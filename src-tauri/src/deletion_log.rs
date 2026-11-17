//! The local deletion log: one JSON line per item Cleany tried to remove,
//! whatever came of it. Kept in the app's log folder as `deletions.jsonl`,
//! with one older file (`deletions.1.jsonl`) once it passes [`MAX_BYTES`].

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use cleany_core::NodeKind;
use serde::Serialize;

use crate::session::{Deletion, TrashOutcome};

/// The log's file name inside the log folder.
pub const FILE_NAME: &str = "deletions.jsonl";
/// The older file kept after a rollover.
const OLD_FILE_NAME: &str = "deletions.1.jsonl";
/// Size past which the log rolls over to [`OLD_FILE_NAME`].
const MAX_BYTES: u64 = 5 * 1024 * 1024;

/// One line of the log.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Line<'a> {
    /// UTC, RFC 3339 to the second.
    time: &'a str,
    /// What was asked: `trash` or `delete`.
    action: &'static str,
    /// The item's path; non-UTF-8 bytes become U+FFFD.
    path: Option<String>,
    kind: &'static str,
    /// `trashed`, `deleted`, `gone`, `protected` or `failed`.
    outcome: &'static str,
    /// Where it went: `osTrash` (the OS does not say where inside it) or
    /// `permanent` for a delete that skips the trash.
    to: &'static str,
    /// Logical bytes, as the scan saw them.
    logical: u64,
    /// Allocated bytes, as the scan saw them.
    allocated: u64,
    /// Why it failed, for `failed`.
    message: Option<&'a str>,
}

/// How a request removed its items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    /// Moved to the OS trash.
    Trash,
    /// Deleted for good, without the trash.
    Delete,
}

impl Removal {
    /// The `action` and `to` fields of a [`Line`].
    const fn names(self) -> (&'static str, &'static str) {
        match self {
            Self::Trash => ("trash", "osTrash"),
            Self::Delete => ("delete", "permanent"),
        }
    }
}

/// Appends one line per attempt to the log in `dir`, creating the folder
/// when needed and rolling the file over past [`MAX_BYTES`].
///
/// # Errors
/// Any I/O error; the caller reports it and carries on, since the items
/// were already removed.
pub fn append(dir: &Path, removal: Removal, deletions: &[Deletion]) -> io::Result<()> {
    append_at(dir, removal, deletions, &rfc3339(SystemTime::now()))
}

/// [`append`] with the time given, for tests.
fn append_at(dir: &Path, removal: Removal, deletions: &[Deletion], time: &str) -> io::Result<()> {
    if deletions.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(dir)?;
    let path = dir.join(FILE_NAME);
    if fs::metadata(&path).is_ok_and(|m| m.len() >= MAX_BYTES) {
        fs::rename(&path, dir.join(OLD_FILE_NAME))?;
    }
    let (action, to) = removal.names();
    let mut text = String::new();
    for d in deletions {
        let line = Line {
            time,
            action,
            path: d.path.as_ref().map(|p| p.to_string_lossy().into_owned()),
            kind: kind_name(d.kind),
            outcome: outcome_name(&d.outcome),
            to,
            logical: d.freed.logical,
            allocated: d.freed.allocated,
            message: match &d.outcome {
                TrashOutcome::Failed { message } => Some(message),
                _ => None,
            },
        };
        let json = serde_json::to_string(&line).map_err(io::Error::other)?;
        text.push_str(&json);
        text.push('\n');
    }
    // One write per request, so lines from one request stay together.
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?
        .write_all(text.as_bytes())
}

const fn kind_name(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Dir => "dir",
        NodeKind::File => "file",
        NodeKind::Symlink => "symlink",
        NodeKind::Other => "other",
    }
}

const fn outcome_name(outcome: &TrashOutcome) -> &'static str {
    match outcome {
        TrashOutcome::Trashed => "trashed",
        TrashOutcome::Deleted => "deleted",
        TrashOutcome::Gone => "gone",
        TrashOutcome::Protected => "protected",
        TrashOutcome::Failed { .. } => "failed",
    }
}

/// `t` as UTC RFC 3339 to the second (`2026-11-16T14:03:22Z`); times before
/// 1970 print as the epoch.
fn rfc3339(t: SystemTime) -> String {
    let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// The proleptic Gregorian date `days` after 1970-01-01, after Howard
/// Hinnant's `civil_from_days`.
const fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::{FILE_NAME, MAX_BYTES, OLD_FILE_NAME, Removal, append_at, rfc3339};
    use crate::session::{Deletion, Freed, TrashOutcome};
    use cleany_core::NodeKind;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cleany-dlog-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn deletion(path: &str, outcome: TrashOutcome) -> Deletion {
        Deletion {
            path: Some(PathBuf::from(path)),
            kind: NodeKind::File,
            freed: Freed {
                logical: 10,
                allocated: 4096,
            },
            outcome,
        }
    }

    #[test]
    fn rfc3339_known_dates() {
        let at = |s| rfc3339(UNIX_EPOCH + Duration::from_secs(s));
        assert_eq!(at(0), "1970-01-01T00:00:00Z");
        assert_eq!(at(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(at(1_709_208_000), "2024-02-29T12:00:00Z");
        assert_eq!(at(1_735_689_599), "2024-12-31T23:59:59Z");
        assert_eq!(at(1_794_837_802), "2026-11-16T14:03:22Z");
    }

    #[test]
    fn appends_one_line_per_item_across_calls() {
        let dir = scratch("append");
        let failed = TrashOutcome::Failed {
            message: "denied".to_owned(),
        };
        append_at(
            &dir,
            Removal::Trash,
            &[deletion("/r/a", TrashOutcome::Trashed)],
            "T1",
        )
        .unwrap();
        append_at(
            &dir,
            Removal::Trash,
            &[
                deletion("/r/b", TrashOutcome::Protected),
                deletion("/r/c", failed),
            ],
            "T2",
        )
        .unwrap();
        append_at(&dir, Removal::Trash, &[], "T3").unwrap();
        let text = fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        let lines: Vec<serde_json::Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0]["time"], "T1");
        assert_eq!(lines[0]["action"], "trash");
        assert_eq!(lines[0]["path"], "/r/a");
        assert_eq!(lines[0]["kind"], "file");
        assert_eq!(lines[0]["outcome"], "trashed");
        assert_eq!(lines[0]["to"], "osTrash");
        assert_eq!(lines[0]["allocated"], 4096);
        assert_eq!(lines[0]["message"], serde_json::Value::Null);
        assert_eq!(lines[1]["outcome"], "protected");
        assert_eq!(lines[2]["outcome"], "failed");
        assert_eq!(lines[2]["message"], "denied");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn deletes_are_logged_as_permanent() {
        let dir = scratch("delete");
        append_at(
            &dir,
            Removal::Delete,
            &[deletion("/r/a", TrashOutcome::Deleted)],
            "T",
        )
        .unwrap();
        let text = fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        let line: serde_json::Value = serde_json::from_str(text.trim_end()).unwrap();
        assert_eq!(line["action"], "delete");
        assert_eq!(line["outcome"], "deleted");
        assert_eq!(line["to"], "permanent");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rolls_over_past_the_cap() {
        let dir = scratch("roll");
        fs::create_dir_all(&dir).unwrap();
        let big = vec![b'x'; usize::try_from(MAX_BYTES).unwrap()];
        fs::write(dir.join(FILE_NAME), &big).unwrap();
        fs::write(dir.join(OLD_FILE_NAME), b"older").unwrap();
        append_at(
            &dir,
            Removal::Trash,
            &[deletion("/r/a", TrashOutcome::Gone)],
            "T",
        )
        .unwrap();
        assert_eq!(
            fs::metadata(dir.join(OLD_FILE_NAME)).unwrap().len(),
            MAX_BYTES
        );
        let text = fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert_eq!(text.lines().count(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unwritable_place_is_an_error_not_a_panic() {
        let dir = scratch("blocked");
        fs::create_dir_all(dir.parent().unwrap()).unwrap();
        // A file where the folder should be.
        fs::write(&dir, b"").unwrap();
        assert!(
            append_at(
                &dir,
                Removal::Trash,
                &[deletion("/r/a", TrashOutcome::Trashed)],
                "T"
            )
            .is_err()
        );
        fs::remove_file(&dir).unwrap();
    }
}
