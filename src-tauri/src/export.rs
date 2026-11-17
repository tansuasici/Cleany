//! Writes scan rows to a CSV or JSON file, for spreadsheets and for agents
//! that read the scan outside the app.
//!
//! The UI names a scope and a format; the save dialog runs in Rust and the
//! tree supplies every path, so no path ever comes from the webview.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::session::{
    EntryKind, ExportHead, ExportProgress, ExportRow, ExportScope, ExportSink, ScanId,
    SessionError, Sessions,
};

/// Column names, in order. CSV writes them as its header; JSON items use the
/// same keys.
pub const COLUMNS: [&str; 11] = [
    "path",
    "name",
    "kind",
    "allocated_bytes",
    "logical_bytes",
    "files",
    "modified_unix",
    "modified_iso",
    "depth",
    "hardlink",
    "placeholder",
];

/// File format of an export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    /// Comma separated values, RFC 4180.
    Csv,
    /// One JSON document: `meta`, `items`, then `rows`.
    Json,
}

impl Format {
    /// File extension, without the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }

    /// Name of the save dialog's file filter.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Json => "JSON",
        }
    }
}

/// Why an export failed. Mirrors `ExportError` in `ui/src/lib/ipc.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ExportError {
    /// The id is not the current scan.
    StaleScan,
    /// The current scan has no tree.
    NotReady,
    /// The node index is not in the tree.
    UnknownNode,
    /// The file could not be written.
    Io {
        /// The OS error, for the log; the UI shows its own text.
        message: String,
    },
    /// The user stopped the export; the partial file was removed.
    Cancelled,
    /// The save dialog failed.
    Dialog {
        /// What went wrong.
        message: String,
    },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleScan => f.write_str("scan was replaced by a newer one"),
            Self::NotReady => f.write_str("scan has no results"),
            Self::UnknownNode => f.write_str("unknown node"),
            Self::Io { message } => write!(f, "cannot write the file: {message}"),
            Self::Cancelled => f.write_str("export stopped"),
            Self::Dialog { message } => write!(f, "save dialog failed: {message}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<SessionError> for ExportError {
    fn from(e: SessionError) -> Self {
        match e {
            SessionError::StaleScan => Self::StaleScan,
            SessionError::NotReady => Self::NotReady,
            SessionError::UnknownNode => Self::UnknownNode,
            // Not produced by an export; keep the text if one ever is.
            SessionError::InvalidPath | SessionError::Spawn { .. } => Self::Io {
                message: e.to_string(),
            },
        }
    }
}

impl From<io::Error> for ExportError {
    fn from(e: io::Error) -> Self {
        Self::Io {
            message: e.to_string(),
        }
    }
}

/// What a finished export reports back to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    /// Rows written, not counting the CSV header.
    pub rows: u64,
    /// Size of the written file.
    pub bytes: u64,
    /// The file's name, without its folder.
    pub file_name: String,
}

/// What a running export tells the UI. Mirrors `ExportEvent` in
/// `ui/src/lib/ipc.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ExportEvent {
    /// The file was chosen and writing starts.
    Started {
        /// The file's name, without its folder.
        file_name: String,
    },
    /// Rows written so far.
    Progress {
        /// Rows written.
        rows: u64,
        /// Rows expected, when known; an upper bound.
        total: Option<u64>,
    },
}

/// Least time between two [`ExportEvent::Progress`] events.
pub const PROGRESS_EVERY: Duration = Duration::from_millis(150);

/// The running export's stop request and the last finished file. One
/// export runs at a time; the UI starts the next only after the last one
/// ended.
#[derive(Debug, Default)]
pub struct Exports {
    cancel: AtomicBool,
    last: Mutex<Option<PathBuf>>,
}

impl Exports {
    /// Clears an old stop request before a new export starts.
    pub fn begin(&self) {
        self.cancel.store(false, Ordering::Relaxed);
    }

    /// Asks the running export to stop at its next batch.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Remembers `path` as the last finished export, for [`Self::last`].
    pub fn finished(&self, path: PathBuf) {
        *self.last.lock().unwrap_or_else(PoisonError::into_inner) = Some(path);
    }

    /// The file the last successful export wrote, if any. Kept here so the
    /// UI can ask to show it without sending a path.
    pub fn last(&self) -> Option<PathBuf> {
        self.last
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Reports progress through `send`, at most every [`PROGRESS_EVERY`]
    /// (the first report always goes out), and fails with
    /// [`ExportError::Cancelled`] once a stop was asked for.
    pub fn reporter<'a>(
        &'a self,
        mut send: impl FnMut(ExportProgress) + 'a,
    ) -> impl FnMut(ExportProgress) -> Result<(), ExportError> + 'a {
        let mut last: Option<Instant> = None;
        move |p| {
            if self.cancel.load(Ordering::Relaxed) {
                return Err(ExportError::Cancelled);
            }
            if last.is_none_or(|t| t.elapsed() >= PROGRESS_EVERY) {
                last = Some(Instant::now());
                send(p);
            }
            Ok(())
        }
    }
}

/// Writes `scope` of scan `id` to `path` as `format`. The rows go to a
/// hidden file next to `path` first, renamed over `path` only once complete,
/// so a failed or stopped export leaves an existing file at `path` as it
/// was and no partial file behind. `progress` is passed to
/// [`Sessions::export`].
///
/// # Errors
/// The session's errors for a stale scan or unknown node,
/// [`ExportError::Io`] when the file cannot be written, or whatever
/// `progress` failed with.
pub fn write_file(
    sessions: &Sessions,
    id: ScanId,
    scope: ExportScope,
    format: Format,
    path: &Path,
    now: SystemTime,
    progress: impl FnMut(ExportProgress) -> Result<(), ExportError>,
) -> Result<ExportReport, ExportError> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let name = name.ok_or_else(|| ExportError::Io {
        message: "the chosen path has no file name".to_owned(),
    })?;
    let part = path.with_file_name(format!(".{name}.cleany-part"));
    let result = write_to(sessions, id, scope, format, &part, now, progress)
        .and_then(|rows| Ok((rows, fs::rename(&part, path)?)));
    match result {
        Ok((rows, ())) => Ok(ExportReport {
            rows,
            bytes: fs::metadata(path)?.len(),
            file_name: name,
        }),
        Err(e) => {
            // Best effort: the error being reported matters more than this one.
            let _ = fs::remove_file(&part);
            Err(e)
        }
    }
}

fn write_to(
    sessions: &Sessions,
    id: ScanId,
    scope: ExportScope,
    format: Format,
    path: &Path,
    now: SystemTime,
    progress: impl FnMut(ExportProgress) -> Result<(), ExportError>,
) -> Result<u64, ExportError> {
    let file = BufWriter::new(File::create(path)?);
    let (rows, file) = match format {
        Format::Csv => {
            let mut sink = CsvSink::new(file);
            let rows = sessions.export(id, scope, &mut sink, progress)?;
            (rows, sink.finish()?)
        }
        Format::Json => {
            let mut sink = JsonSink::new(file, scope, unix_secs(now));
            let rows = sessions.export(id, scope, &mut sink, progress)?;
            (rows, sink.finish(rows)?)
        }
    };
    let file = file.into_inner().map_err(io::IntoInnerError::into_error)?;
    file.sync_all()?;
    Ok(rows)
}

/// The name the save dialog suggests: `cleany-<scope name>-<date>.<ext>`.
/// Characters a file name should not carry become `-`; the disk root, which
/// has no name, becomes `disk`.
#[must_use]
pub fn default_name(scope_name: &str, format: Format, now: SystemTime) -> String {
    let mut name: String = scope_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    name = name.trim_matches(|c| c == '-' || c == '.').to_owned();
    if name.is_empty() {
        name = "disk".to_owned();
    }
    let iso = iso_utc(unix_secs(now));
    let date = iso.get(..10).unwrap_or("");
    format!("cleany-{name}-{date}.{}", format.extension())
}

fn unix_secs(t: SystemTime) -> i64 {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
        Err(e) => i64::try_from(e.duration().as_secs()).map_or(i64::MIN, |s| -s),
    }
}

/// `secs` since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ`, in UTC.
#[must_use]
pub fn iso_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's
/// `civil_from_days`).
const fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

const fn kind_text(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::File => "file",
        EntryKind::Dir => "dir",
        EntryKind::Symlink => "symlink",
        EntryKind::Other => "other",
    }
}

/// An unknown modification time (0) is left empty rather than 1970.
fn modified_iso(secs: i64) -> String {
    if secs == 0 {
        String::new()
    } else {
        iso_utc(secs)
    }
}

/// Writes rows as RFC 4180 CSV: a header line, CRLF line ends, and fields
/// quoted when they hold a comma, a quote or a line break.
pub struct CsvSink<W: Write> {
    out: W,
}

impl<W: Write> CsvSink<W> {
    /// A sink writing to `out`.
    pub const fn new(out: W) -> Self {
        Self { out }
    }

    /// Flushes and hands the writer back.
    ///
    /// # Errors
    /// The writer's error.
    pub fn finish(mut self) -> io::Result<W> {
        self.out.flush()?;
        Ok(self.out)
    }

    fn line<'a>(&mut self, fields: impl IntoIterator<Item = &'a str>) -> io::Result<()> {
        for (i, field) in fields.into_iter().enumerate() {
            if i > 0 {
                self.out.write_all(b",")?;
            }
            if field.contains([',', '"', '\n', '\r']) {
                self.out.write_all(b"\"")?;
                self.out.write_all(field.replace('"', "\"\"").as_bytes())?;
                self.out.write_all(b"\"")?;
            } else {
                self.out.write_all(field.as_bytes())?;
            }
        }
        self.out.write_all(b"\r\n")
    }
}

impl<W: Write> ExportSink for CsvSink<W> {
    type Error = ExportError;

    fn head(&mut self, _head: &ExportHead) -> Result<(), ExportError> {
        Ok(self.line(COLUMNS)?)
    }

    fn row(&mut self, row: &ExportRow) -> Result<(), ExportError> {
        let e = &row.entry;
        let path = row.path.to_string_lossy();
        let cells = [
            path.into_owned(),
            e.name.clone(),
            kind_text(e.kind).to_owned(),
            e.allocated.to_string(),
            e.logical.to_string(),
            e.files.to_string(),
            e.modified.to_string(),
            modified_iso(e.modified),
            row.depth.to_string(),
            e.hardlink.to_string(),
            e.placeholder.to_string(),
        ];
        Ok(self.line(cells.iter().map(String::as_str))?)
    }
}

/// Writes one JSON document: `meta` first, then `items` streamed one by
/// one, then `rows`, the item count, once it is known.
pub struct JsonSink<W: Write> {
    out: W,
    scope: ExportScope,
    exported_at: i64,
    first: bool,
}

impl<W: Write> JsonSink<W> {
    /// A sink writing to `out`; `exported_at` is in Unix seconds.
    pub const fn new(out: W, scope: ExportScope, exported_at: i64) -> Self {
        Self {
            out,
            scope,
            exported_at,
            first: true,
        }
    }

    /// Closes the document with the row count, flushes and hands the
    /// writer back.
    ///
    /// # Errors
    /// The writer's error.
    pub fn finish(mut self, rows: u64) -> io::Result<W> {
        write!(self.out, "\n],\"rows\":{rows}}}\n")?;
        self.out.flush()?;
        Ok(self.out)
    }
}

impl<W: Write> ExportSink for JsonSink<W> {
    type Error = ExportError;

    fn head(&mut self, head: &ExportHead) -> Result<(), ExportError> {
        let meta = json!({
            "tool": "Cleany",
            "version": env!("CARGO_PKG_VERSION"),
            "exported_at": iso_utc(self.exported_at),
            "scan_root": head.scan_root.to_string_lossy(),
            "scope": self.scope,
            "scope_path": head.scope_path.to_string_lossy(),
            "columns": COLUMNS,
            "units": {
                "allocated_bytes": "bytes on disk",
                "logical_bytes": "bytes, file length",
                "modified_unix": "seconds since 1970-01-01 UTC, 0 when unknown",
                "modified_iso": "UTC, empty when unknown",
                "depth": "levels below scope_path",
            },
        });
        self.out.write_all(b"{\"meta\":")?;
        serde_json::to_writer(&mut self.out, &meta).map_err(io::Error::from)?;
        self.out.write_all(b",\"items\":[")?;
        Ok(())
    }

    fn row(&mut self, row: &ExportRow) -> Result<(), ExportError> {
        let e = &row.entry;
        let item = json!({
            "path": row.path.to_string_lossy(),
            "name": e.name,
            "kind": kind_text(e.kind),
            "allocated_bytes": e.allocated,
            "logical_bytes": e.logical,
            "files": e.files,
            "modified_unix": e.modified,
            "modified_iso": modified_iso(e.modified),
            "depth": row.depth,
            "hardlink": e.hardlink,
            "placeholder": e.placeholder,
        });
        self.out
            .write_all(if self.first { b"\n" } else { b",\n" })?;
        self.first = false;
        serde_json::to_writer(&mut self.out, &item).map_err(io::Error::from)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        COLUMNS, CsvSink, ExportError, Exports, Format, JsonSink, default_name, iso_utc, write_file,
    };
    use crate::session::{
        EntryKind, ExportHead, ExportProgress, ExportRow, ExportScope, ExportSink, NodeEntry,
        ScanId, Sessions, SortKey, SortOrder,
    };
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    fn head() -> ExportHead {
        ExportHead {
            scan_root: PathBuf::from("/r"),
            scope_path: PathBuf::from("/r"),
        }
    }

    fn row(path: &str, name: &str, modified: i64) -> ExportRow {
        ExportRow {
            path: PathBuf::from(path),
            depth: 1,
            entry: NodeEntry {
                id: 3,
                name: name.to_owned(),
                kind: EntryKind::File,
                logical: 50,
                allocated: 4096,
                files: 1,
                modified,
                has_children: false,
                hardlink: false,
                placeholder: true,
            },
        }
    }

    #[test]
    fn csv_quotes_commas_quotes_and_newlines() {
        let mut sink = CsvSink::new(Vec::new());
        sink.head(&head()).unwrap();
        sink.row(&row("/r/plain", "plain", 0)).unwrap();
        sink.row(&row("/r/a,b \"q\"\nx", "a,b \"q\"\nx", 1_700_000_000))
            .unwrap();
        let text = String::from_utf8(sink.finish().unwrap()).unwrap();
        let mut lines = text.split("\r\n");
        assert_eq!(lines.next(), Some(COLUMNS.join(",").as_str()));
        assert_eq!(
            lines.next(),
            Some("/r/plain,plain,file,4096,50,1,0,,1,false,true")
        );
        assert_eq!(
            lines.next(),
            Some(
                "\"/r/a,b \"\"q\"\"\nx\",\"a,b \"\"q\"\"\nx\",file,4096,50,1,1700000000,\
                 2023-11-14T22:13:20Z,1,false,true"
            )
        );
        assert_eq!(lines.next(), Some(""));
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn json_has_meta_and_valid_items() {
        let scope = ExportScope::Children {
            node: 0,
            sort: SortKey::Name,
            order: SortOrder::Asc,
        };
        let mut sink = JsonSink::new(Vec::new(), scope, 1_700_000_000);
        sink.head(&head()).unwrap();
        sink.row(&row("/r/a", "a", 0)).unwrap();
        sink.row(&row("/r/\"b\"", "\"b\"", 86_400)).unwrap();
        let bytes = sink.finish(2).unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(doc["rows"], 2);
        assert_eq!(doc["meta"]["tool"], "Cleany");
        assert_eq!(doc["meta"]["exported_at"], "2023-11-14T22:13:20Z");
        assert_eq!(doc["meta"]["scan_root"], "/r");
        assert_eq!(doc["meta"]["scope"]["kind"], "children");
        assert_eq!(doc["meta"]["scope"]["sort"], "name");
        let items = doc["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[1]["name"], "\"b\"");
        assert_eq!(items[1]["modified_iso"], "1970-01-02T00:00:00Z");
        assert_eq!(items[0]["modified_iso"], "");
        assert_eq!(items[0]["allocated_bytes"], 4096);
        for col in COLUMNS {
            assert!(items[0].get(col).is_some(), "item lacks {col}");
        }
    }

    #[test]
    fn json_with_no_rows_is_valid() {
        let mut sink = JsonSink::new(Vec::new(), ExportScope::Subtree { node: 0 }, 0);
        sink.head(&head()).unwrap();
        let bytes = sink.finish(0).unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(doc["items"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn iso_handles_leap_days_and_before_1970() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(iso_utc(-1), "1969-12-31T23:59:59Z");
    }

    #[test]
    fn default_name_is_safe_and_dated() {
        let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert_eq!(
            default_name("Projeler", Format::Csv, now),
            "cleany-Projeler-2023-11-14.csv"
        );
        assert_eq!(
            default_name("/", Format::Json, now),
            "cleany-disk-2023-11-14.json"
        );
        assert_eq!(
            default_name("a/b:c d", Format::Csv, now),
            "cleany-a-b-c-d-2023-11-14.csv"
        );
    }

    #[test]
    fn failed_export_keeps_what_was_there() {
        let dir = std::env::temp_dir().join(format!("cleany-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.csv");
        let id: ScanId = serde_json::from_str("7").unwrap();
        let err = write_file(
            &Sessions::default(),
            id,
            ExportScope::Subtree { node: 0 },
            Format::Csv,
            &path,
            UNIX_EPOCH,
            |_| Ok(()),
        );
        assert_eq!(err, Err(ExportError::StaleScan));
        assert!(!path.exists());
        std::fs::write(&path, "keep").unwrap();
        let err = write_file(
            &Sessions::default(),
            id,
            ExportScope::Subtree { node: 0 },
            Format::Json,
            &path,
            UNIX_EPOCH,
            |_| Ok(()),
        );
        assert_eq!(err, Err(ExportError::StaleScan));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn reporter_throttles_and_stops_on_cancel() {
        let exports = Exports::default();
        let mut sent = Vec::new();
        {
            let mut report = exports.reporter(|p| sent.push(p.rows));
            let at = |rows| ExportProgress { rows, total: None };
            assert_eq!(report(at(0)), Ok(()));
            // Too soon after the first report: counted, not sent.
            assert_eq!(report(at(1)), Ok(()));
            exports.cancel();
            assert_eq!(report(at(2)), Err(ExportError::Cancelled));
        }
        assert_eq!(sent, [0]);
        exports.begin();
        let mut report = exports.reporter(|_| {});
        assert_eq!(
            report(ExportProgress {
                rows: 0,
                total: None
            }),
            Ok(())
        );
    }

    #[test]
    fn format_from_wire_name() {
        let f: Format = serde_json::from_str("\"json\"").unwrap();
        assert_eq!(f, Format::Json);
        assert_eq!(Format::Csv.extension(), "csv");
    }
}
