//! `cleany` command-line front end.
//!
//! ```text
//! cleany scan <path> [--json] [--cross-mounts]
//! cleany --version
//! ```
//!
//! Arguments are parsed by hand to keep the binary dependency-free.

use std::ffi::OsString;
use std::fmt;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use cleany_core::{CancelToken, NodeKind, Progress, ScanOptions, ScanResult, Tree, scan};
use serde_json::json;

const USAGE: &str = "\
usage: cleany scan <path> [--json] [--cross-mounts]
       cleany --version

  --json          print the result as JSON on stdout
  --cross-mounts  descend into other filesystems / volumes";

/// How many of the largest direct children the text report lists.
const TOP_CHILDREN: usize = 20;

/// A parsed command line.
#[derive(Debug, PartialEq, Eq)]
enum Command {
    Scan(ScanArgs),
    Version,
    Help,
}

/// Arguments of `cleany scan`.
#[derive(Debug, PartialEq, Eq)]
struct ScanArgs {
    path: PathBuf,
    json: bool,
    cross_mounts: bool,
}

/// A command-line usage error.
#[derive(Debug, PartialEq, Eq)]
enum UsageError {
    MissingCommand,
    UnknownCommand(OsString),
    UnknownFlag(OsString),
    MissingPath,
    ExtraArgument(OsString),
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => f.write_str("missing command"),
            Self::UnknownCommand(c) => write!(f, "unknown command `{}`", c.to_string_lossy()),
            Self::UnknownFlag(c) => write!(f, "unknown option `{}`", c.to_string_lossy()),
            Self::MissingPath => f.write_str("`scan` needs a path"),
            Self::ExtraArgument(a) => write!(f, "unexpected argument `{}`", a.to_string_lossy()),
        }
    }
}

/// Parses arguments after the program name. Paths stay `OsString`s so
/// non-UTF-8 names work.
fn parse_args<I: IntoIterator<Item = OsString>>(args: I) -> Result<Command, UsageError> {
    let mut args = args.into_iter();
    let Some(cmd) = args.next() else {
        return Err(UsageError::MissingCommand);
    };
    match cmd.to_str() {
        Some("scan") => {}
        Some("--version" | "-V") => return Ok(Command::Version),
        Some("--help" | "-h" | "help") => return Ok(Command::Help),
        _ => return Err(UsageError::UnknownCommand(cmd)),
    }

    let mut path = None;
    let mut json = false;
    let mut cross_mounts = false;
    let mut flags_done = false;
    for arg in args {
        match arg.to_str() {
            Some("--") if !flags_done => flags_done = true,
            Some("--json") if !flags_done => json = true,
            Some("--cross-mounts") if !flags_done => cross_mounts = true,
            Some("--help" | "-h") if !flags_done => return Ok(Command::Help),
            Some(s) if !flags_done && s.starts_with('-') && s.len() > 1 => {
                return Err(UsageError::UnknownFlag(arg));
            }
            _ if path.is_none() => path = Some(PathBuf::from(arg)),
            _ => return Err(UsageError::ExtraArgument(arg)),
        }
    }
    let path = path.ok_or(UsageError::MissingPath)?;
    Ok(Command::Scan(ScanArgs {
        path,
        json,
        cross_mounts,
    }))
}

fn main() -> ExitCode {
    match parse_args(std::env::args_os().skip(1)) {
        Ok(Command::Scan(args)) => run_scan(&args),
        Ok(Command::Version) => {
            println!("{}", version());
            ExitCode::SUCCESS
        }
        Ok(Command::Help) => {
            println!("{}\n\n{USAGE}", version());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("cleany: {e}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn version() -> String {
    format!(
        "cleany {} ({}/{})",
        env!("CARGO_PKG_VERSION"),
        cleany_platform::Os::current().as_str(),
        std::env::consts::ARCH
    )
}

fn run_scan(args: &ScanArgs) -> ExitCode {
    let options = ScanOptions {
        cross_mounts: args.cross_mounts,
        ..ScanOptions::default()
    };
    let live = io::stderr().is_terminal();
    let result = scan(&args.path, &options, &CancelToken::new(), |p| {
        if live {
            show_progress(p);
        }
    });
    if live {
        eprint!("\r\x1b[2K");
    }
    let result = match result {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cleany: {e}");
            return ExitCode::FAILURE;
        }
    };

    let out = if args.json {
        json_report(&result)
    } else {
        text_report(&result)
    };
    let mut stdout = io::stdout().lock();
    if writeln!(stdout, "{out}").is_err() {
        return ExitCode::FAILURE;
    }
    if !args.json && !result.issues.is_empty() {
        eprintln!(
            "cleany: {} entr{} could not be read",
            result.issues.len(),
            if result.issues.len() == 1 { "y" } else { "ies" }
        );
    }
    ExitCode::SUCCESS
}

fn show_progress(p: &Progress) {
    eprint!(
        "\r\x1b[2K{} dirs, {} files, {}",
        p.dirs,
        p.files,
        human_bytes(p.allocated)
    );
}

fn kind_str(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::File => "file",
        NodeKind::Dir => "dir",
        NodeKind::Symlink => "symlink",
        NodeKind::Other => "other",
    }
}

/// Direct children of the root, largest allocated size first.
fn sorted_children(tree: &Tree) -> Vec<cleany_core::NodeId> {
    let mut kids: Vec<_> = tree.children(tree.root()).collect();
    kids.sort_by_key(|&id| std::cmp::Reverse(tree.node(id).map_or(0, |n| n.size().allocated)));
    kids
}

fn json_report(r: &ScanResult) -> String {
    let tree = &r.tree;
    let root = tree.node(tree.root());
    let size = root.map(cleany_core::Node::size).unwrap_or_default();
    let children: Vec<_> = sorted_children(tree)
        .into_iter()
        .filter_map(|id| {
            let n = tree.node(id)?;
            Some(json!({
                "name": tree.name(id)?.to_string_lossy(),
                "kind": kind_str(n.kind()),
                "logical": n.size().logical,
                "allocated": n.size().allocated,
                "files": n.file_count(),
            }))
        })
        .collect();
    let issues: Vec<_> = r
        .issues
        .iter()
        .map(|i| json!({ "path": i.path.to_string_lossy(), "error": i.error.to_string() }))
        .collect();
    let value = json!({
        "root": tree.path(tree.root()).map(|p| p.to_string_lossy().into_owned()),
        "logical": size.logical,
        "allocated": size.allocated,
        "files": r.stats.files,
        "dirs": r.stats.dirs,
        "symlinks": r.stats.symlinks,
        "others": r.stats.others,
        "skipped_mounts": r.stats.skipped_mounts,
        "hardlinks": r.stats.hardlinks,
        "placeholders": r.stats.placeholders,
        "elapsed_ms": u64::try_from(r.stats.elapsed.as_millis()).unwrap_or(u64::MAX),
        "children": children,
        "issues": issues,
    });
    value.to_string()
}

fn text_report(r: &ScanResult) -> String {
    use fmt::Write as _;
    let tree = &r.tree;
    let size = tree
        .node(tree.root())
        .map(cleany_core::Node::size)
        .unwrap_or_default();
    let root = tree
        .path(tree.root())
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let mut s = String::new();
    let _ = writeln!(
        s,
        "{root}\n  {} on disk, {} logical",
        human_bytes(size.allocated),
        human_bytes(size.logical)
    );
    let _ = writeln!(
        s,
        "  {} files, {} dirs, {} symlinks in {:.2?}",
        r.stats.files, r.stats.dirs, r.stats.symlinks, r.stats.elapsed
    );
    if r.stats.hardlinks > 0 {
        let _ = writeln!(s, "  {} extra hard links counted once", r.stats.hardlinks);
    }
    if r.stats.placeholders > 0 {
        let _ = writeln!(
            s,
            "  {} cloud placeholders not downloaded",
            r.stats.placeholders
        );
    }
    let kids = sorted_children(tree);
    if !kids.is_empty() {
        s.push('\n');
    }
    for &id in kids.iter().take(TOP_CHILDREN) {
        let (Some(n), Some(name)) = (tree.node(id), tree.name(id)) else {
            continue;
        };
        let slash = if n.kind() == NodeKind::Dir { "/" } else { "" };
        let _ = writeln!(
            s,
            "  {:>10}  {}{slash}",
            human_bytes(n.size().allocated),
            name.to_string_lossy()
        );
    }
    if kids.len() > TOP_CHILDREN {
        let _ = writeln!(s, "  … {} more", kids.len() - TOP_CHILDREN);
    }
    s.trim_end().to_owned()
}

/// Formats bytes with binary units, e.g. `1.5 MiB`.
fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    if n < 1024 {
        return format!("{n} B");
    }
    #[expect(clippy::cast_precision_loss, reason = "display only, one decimal")]
    let mut v = n as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    format!("{v:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::{Command, ScanArgs, UsageError, human_bytes, parse_args};
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> Result<Command, UsageError> {
        parse_args(args.iter().map(OsString::from))
    }

    #[test]
    fn parses_scan_with_flags_in_any_order() {
        let want = Command::Scan(ScanArgs {
            path: PathBuf::from("/data"),
            json: true,
            cross_mounts: true,
        });
        assert_eq!(
            parse(&["scan", "--json", "/data", "--cross-mounts"]),
            Ok(want)
        );
    }

    #[test]
    fn double_dash_allows_dash_paths() {
        let Ok(Command::Scan(a)) = parse(&["scan", "--", "--json"]) else {
            panic!("expected scan");
        };
        assert_eq!(a.path, PathBuf::from("--json"));
        assert!(!a.json);
    }

    #[test]
    fn rejects_bad_command_lines() {
        assert_eq!(parse(&[]), Err(UsageError::MissingCommand));
        assert_eq!(parse(&["scan"]), Err(UsageError::MissingPath));
        assert!(matches!(
            parse(&["nuke"]),
            Err(UsageError::UnknownCommand(_))
        ));
        assert!(matches!(
            parse(&["scan", "-x", "/"]),
            Err(UsageError::UnknownFlag(_))
        ));
        assert!(matches!(
            parse(&["scan", "/a", "/b"]),
            Err(UsageError::ExtraArgument(_))
        ));
    }

    #[test]
    fn version_and_help() {
        assert_eq!(parse(&["--version"]), Ok(Command::Version));
        assert_eq!(parse(&["scan", "--help"]), Ok(Command::Help));
    }

    #[test]
    fn human_bytes_uses_binary_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1536), "1.5 KiB");
        assert_eq!(human_bytes(10 * 1024 * 1024), "10.0 MiB");
        assert_eq!(human_bytes(u64::MAX), "16384.0 PiB");
    }
}
