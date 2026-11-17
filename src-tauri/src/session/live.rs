//! Live updates of a finished scan.
//!
//! One thread per watched scan takes the OS change notifications, merges
//! them with [`Coalescer`] and patches the finished tree directory by
//! directory with [`refresh`]. Changes that arrive while the scan still
//! runs wait for its `Done`. The thread ends when its channel closes, which
//! happens when the scan's watch guard is dropped: the scan was replaced,
//! cancelled or failed, or its live updates stopped with `Outdated`.
//!
//! Locks: the state lock is never held while a disk is read or an event is
//! sent, and the sink is always locked before the state.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cleany_core::refresh::{self, Change, Coalescer, Depth, DirRequest, DirUpdate, Outcome};
use cleany_core::{CancelToken, RefreshPlan, ScanOptions, Tree};
use cleany_platform::{ChangeKind, FsChange};

use super::{ScanEvent, ScanId, Sink, State, entry, lock, lock_sink};

/// How often a quiet channel is checked for a batch that became ready.
const TICK: Duration = Duration::from_millis(200);

/// Reads of one directory thrown away because the app changed the tree
/// meanwhile, before that directory is left to the next notification.
const RETRIES: usize = 3;

/// Nodes patches may add beyond the scanned tree before only a new scan
/// makes sense (on top of doubling it).
const GROWTH: usize = 100_000;

/// Starts the live thread of scan `id`, rooted at `root`.
pub(super) fn spawn(
    state: Arc<Mutex<State>>,
    id: ScanId,
    root: PathBuf,
    rx: mpsc::Receiver<FsChange>,
    sink: Sink,
) -> io::Result<()> {
    thread::Builder::new()
        .name(format!("cleany-live-{}", id.0))
        .spawn(move || run(&state, id, &root, &rx, &sink))
        .map(drop)
}

fn run(state: &Mutex<State>, id: ScanId, root: &Path, rx: &mpsc::Receiver<FsChange>, sink: &Sink) {
    let mut pending = Coalescer::new(root);
    loop {
        match rx.recv_timeout(TICK) {
            Ok(change) => push(&mut pending, &change),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(change) = rx.try_recv() {
            push(&mut pending, &change);
        }
        if pending.is_empty() || !pending.ready(Instant::now()) {
            continue;
        }
        match phase(state, id) {
            Phase::Scanning => continue,
            Phase::Gone => return,
            Phase::Done => {}
        }
        let round = match pending.take() {
            RefreshPlan::Full => Round::Outdated,
            RefreshPlan::Dirs(dirs) => refresh_all(state, id, &dirs),
        };
        let go_on = match round {
            Round::Quiet => true,
            Round::Changed => report(state, id, sink),
            Round::Outdated => {
                outdated(state, id, sink);
                false
            }
            Round::Gone => false,
        };
        if !go_on {
            return;
        }
    }
}

fn push(pending: &mut Coalescer, change: &FsChange) {
    let kind = match change.kind {
        ChangeKind::Entry => Change::Entry,
        ChangeKind::Subtree => Change::Subtree,
    };
    pending.push(&change.path, kind, Instant::now());
}

enum Phase {
    /// The scan still runs; changes wait.
    Scanning,
    /// The scan's tree is current.
    Done,
    /// Another scan replaced it, or it ended without a tree.
    Gone,
}

fn phase(state: &Mutex<State>, id: ScanId) -> Phase {
    match &*lock(state) {
        State::Scanning { id: cur, .. } if *cur == id => Phase::Scanning,
        State::Done { id: cur, .. } if *cur == id => Phase::Done,
        _ => Phase::Gone,
    }
}

/// What one batch of changes did to the tree.
#[derive(Debug, PartialEq, Eq)]
enum Round {
    Quiet,
    Changed,
    Outdated,
    Gone,
}

fn refresh_all(state: &Mutex<State>, id: ScanId, dirs: &[(PathBuf, Depth)]) -> Round {
    // Directories already refreshed in this batch, and how deep; unknown
    // paths fall back to an ancestor, which may then come up twice.
    let mut seen = HashMap::new();
    let mut changed = false;
    for (path, depth) in dirs {
        match refresh_one(state, id, path, *depth, &mut seen) {
            Round::Quiet => {}
            Round::Changed => changed = true,
            stop @ (Round::Outdated | Round::Gone) => return stop,
        }
    }
    if changed {
        Round::Changed
    } else {
        Round::Quiet
    }
}

fn refresh_one(
    state: &Mutex<State>,
    id: ScanId,
    path: &Path,
    depth: Depth,
    seen: &mut HashMap<PathBuf, Depth>,
) -> Round {
    for _ in 0..RETRIES {
        let prepared = match prepare(state, id, path, depth, seen) {
            Prep::Ready(p) => p,
            Prep::Skip => return Round::Quiet,
            Prep::Gone => return Round::Gone,
        };
        let Prepared {
            dir,
            depth,
            req,
            options,
            edits,
        } = prepared;
        let update = refresh::read(req, &options, &CancelToken::new());
        match commit(state, id, edits, update) {
            Commit::Stale => {}
            Commit::Applied(changed) => {
                seen.insert(dir, depth);
                return if changed {
                    Round::Changed
                } else {
                    Round::Quiet
                };
            }
            Commit::Outdated => return Round::Outdated,
            Commit::Gone => return Round::Gone,
        }
    }
    // The app keeps changing the tree; its own moves raise notifications
    // that refresh this directory again.
    Round::Quiet
}

/// A directory read about to happen, taken under the lock.
struct Prepared {
    dir: PathBuf,
    depth: Depth,
    req: DirRequest,
    options: ScanOptions,
    /// The tree's edit count when `req` was taken.
    edits: u64,
}

enum Prep {
    Ready(Prepared),
    /// Nothing to do: the directory was already refreshed in this batch.
    Skip,
    Gone,
}

fn prepare(
    state: &Mutex<State>,
    id: ScanId,
    path: &Path,
    depth: Depth,
    seen: &HashMap<PathBuf, Depth>,
) -> Prep {
    let mut state = lock(state);
    let Ok(scan) = state.finished_mut(id) else {
        return Prep::Gone;
    };
    let Some((dir, depth, req)) = target(&scan.tree, path, depth) else {
        return Prep::Skip;
    };
    if seen.get(&dir).is_some_and(|&done| done >= depth) {
        return Prep::Skip;
    }
    Prep::Ready(Prepared {
        dir,
        depth,
        req,
        options: scan.options.clone(),
        edits: scan.edits,
    })
}

/// The directory to refresh for a change at `path`: `path` itself when the
/// tree has it as a directory, else its nearest ancestor that is one (a
/// shallow listing of which picks up everything new below it).
fn target(tree: &Tree, path: &Path, depth: Depth) -> Option<(PathBuf, Depth, DirRequest)> {
    let mut dir = path;
    let mut depth = depth;
    loop {
        if let Some(req) = refresh::request(tree, dir, depth) {
            return Some((dir.to_path_buf(), depth, req));
        }
        // The root is always a directory of the tree, so this stops there
        // for any path below it.
        dir = dir.parent()?;
        depth = Depth::Shallow;
    }
}

enum Commit {
    Applied(bool),
    /// The app changed the tree after the read began; read again.
    Stale,
    Outdated,
    Gone,
}

fn commit(state: &Mutex<State>, id: ScanId, edits: u64, update: DirUpdate) -> Commit {
    let mut state = lock(state);
    let Ok(scan) = state.finished_mut(id) else {
        return Commit::Gone;
    };
    if scan.edits != edits {
        return Commit::Stale;
    }
    match refresh::apply(&mut scan.tree, update) {
        Ok(applied) => match applied.outcome {
            Outcome::Unchanged | Outcome::Skipped => Commit::Applied(false),
            Outcome::Changed(_) => {
                scan.sorted = None;
                Commit::Applied(true)
            }
            Outcome::RootGone => Commit::Outdated,
        },
        Err(e) => {
            eprintln!("cleany: live update stopped: {e}");
            Commit::Outdated
        }
    }
}

/// Sends `Changed` for the patched tree, or `Outdated` when patches grew it
/// past what a rescan would cost. `false` once live updates stopped.
fn report(state: &Mutex<State>, id: ScanId, sink: &Sink) -> bool {
    let mut out = lock_sink(sink);
    let event = {
        let mut state = lock(state);
        let Ok(scan) = state.finished_mut(id) else {
            return false;
        };
        let limit = scan
            .base_len
            .saturating_mul(2)
            .max(scan.base_len.saturating_add(GROWTH));
        match entry(&scan.tree, scan.tree.root()) {
            Some(root) if scan.tree.len() <= limit => {
                scan.revision = scan.revision.wrapping_add(1);
                ScanEvent::Changed {
                    scan: id,
                    revision: scan.revision,
                    root,
                }
            }
            _ => {
                scan.watch = None;
                ScanEvent::Outdated { scan: id }
            }
        }
    };
    let go_on = matches!(event, ScanEvent::Changed { .. });
    (out)(event);
    go_on
}

/// Stops live updates of scan `id` and says so.
fn outdated(state: &Mutex<State>, id: ScanId, sink: &Sink) {
    let mut out = lock_sink(sink);
    {
        let mut state = lock(state);
        let Ok(scan) = state.finished_mut(id) else {
            return;
        };
        scan.watch = None;
    }
    (out)(ScanEvent::Outdated { scan: id });
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread;
    use std::time::Duration;

    use cleany_core::refresh::{self, Depth};
    use cleany_core::{CancelToken, ScanError, ScanOptions, ScanResult, scan};
    use cleany_platform::{ChangeKind, FsChange};

    use super::super::{
        ScanEvent, ScanId, Sessions, Sink, State, TrashOutcome, WatchFn, finish, lock, lock_sink,
        new_sink,
    };
    use super::{Commit, Prep, Round, commit, prepare, refresh_all, spawn, target};

    const WAIT: Duration = Duration::from_secs(30);

    /// A fresh directory under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("cleany-live-{}-{tag}-{n}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn entry_change(path: PathBuf) -> FsChange {
        FsChange {
            path,
            kind: ChangeKind::Entry,
        }
    }

    /// A watcher that hands its senders to the test instead of the OS.
    fn fake_watch() -> (WatchFn, Arc<Mutex<Vec<mpsc::Sender<FsChange>>>>) {
        let senders = Arc::new(Mutex::new(Vec::new()));
        let keep = Arc::clone(&senders);
        let watch: WatchFn = Arc::new(move |_: &Path, tx| {
            keep.lock().unwrap().push(tx);
            Some(Box::new(()) as Box<dyn Send>)
        });
        (watch, senders)
    }

    /// The next event that is not progress.
    fn next(rx: &mpsc::Receiver<ScanEvent>) -> ScanEvent {
        loop {
            match rx.recv_timeout(WAIT).unwrap() {
                ScanEvent::Progress { .. } => {}
                other => return other,
            }
        }
    }

    /// A scan `ScanId(1)` of `dir` that is still running, with its live
    /// thread started; events go to the returned receiver.
    struct Live {
        state: Arc<Mutex<State>>,
        id: ScanId,
        sink: Sink,
        changes: mpsc::Sender<FsChange>,
        events: mpsc::Receiver<ScanEvent>,
    }

    impl Live {
        fn start(dir: &Path) -> Self {
            let state = Arc::new(Mutex::new(State::Idle));
            let id = ScanId(1);
            let (etx, events) = mpsc::channel();
            let sink = new_sink(move |e| {
                let _ = etx.send(e);
            });
            let (changes, rx) = mpsc::channel();
            *lock(&state) = State::Scanning {
                id,
                cancel: CancelToken::new(),
                sink: Arc::clone(&sink),
                watch: Some(Box::new(())),
            };
            spawn(
                Arc::clone(&state),
                id,
                dir.to_path_buf(),
                rx,
                Arc::clone(&sink),
            )
            .unwrap();
            Self {
                state,
                id,
                sink,
                changes,
                events,
            }
        }

        /// Ends the scan with `result` the way the scan thread does.
        fn finish(&self, result: Result<ScanResult, ScanError>) {
            let mut out = lock_sink(&self.sink);
            let last = finish(&self.state, self.id, ScanOptions::default(), result);
            (out)(last);
        }

        fn has(&self, path: &Path) -> bool {
            match &*lock(&self.state) {
                State::Done { scan, .. } => scan.tree.lookup(path).is_some(),
                _ => panic!("scan not done"),
            }
        }
    }

    fn scanned(dir: &Path) -> Result<ScanResult, ScanError> {
        scan(dir, &ScanOptions::default(), &CancelToken::new(), |_| {})
    }

    #[test]
    fn changes_patch_done_tree_and_emit_changed() {
        let dir = TempDir::new("patch");
        fs::write(dir.path().join("a"), [0u8; 100]).unwrap();
        let (watch, senders) = fake_watch();
        let s = Sessions::with_watch(watch);
        let (tx, rx) = mpsc::channel();
        let id = s
            .start(dir.path().to_path_buf(), ScanOptions::default(), move |e| {
                let _ = tx.send(e);
            })
            .unwrap();
        assert!(matches!(next(&rx), ScanEvent::Done { .. }));

        fs::write(dir.path().join("b"), [0u8; 100]).unwrap();
        let tx = senders.lock().unwrap()[0].clone();
        tx.send(entry_change(dir.path().join("b"))).unwrap();
        match next(&rx) {
            ScanEvent::Changed {
                scan,
                revision,
                root,
            } => {
                assert_eq!(scan, id);
                assert_eq!(revision, 1);
                assert_eq!(root.files, 2);
            }
            other => panic!("expected changed, got {other:?}"),
        }
        let names: Vec<_> = s
            .children(
                id,
                0,
                super::super::SortKey::Name,
                super::super::SortOrder::Asc,
                0,
                10,
            )
            .unwrap()
            .items
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn changes_buffered_during_scan_apply_after_done() {
        let dir = TempDir::new("buffer");
        let live = Live::start(dir.path());
        let result = scanned(dir.path());
        // Lands on disk after the scan read the folder, before `Done`.
        fs::write(dir.path().join("late"), [0u8; 10]).unwrap();
        live.changes
            .send(entry_change(dir.path().join("late")))
            .unwrap();
        thread::sleep(Duration::from_millis(1_500));
        assert!(live.events.try_recv().is_err(), "nothing before done");

        live.finish(result);
        assert!(matches!(next(&live.events), ScanEvent::Done { .. }));
        assert!(matches!(
            next(&live.events),
            ScanEvent::Changed { revision: 1, .. }
        ));
        assert!(live.has(&dir.path().join("late")));
    }

    #[test]
    fn changes_for_a_replaced_scan_are_dropped() {
        let dir = TempDir::new("replaced");
        let live = Live::start(dir.path());
        live.finish(scanned(dir.path()));
        assert!(matches!(next(&live.events), ScanEvent::Done { .. }));
        *lock(&live.state) = State::Cancelled { id: ScanId(2) };

        fs::write(dir.path().join("new"), [0u8; 10]).unwrap();
        live.changes
            .send(entry_change(dir.path().join("new")))
            .unwrap();
        thread::sleep(Duration::from_millis(2_000));
        assert!(live.events.try_recv().is_err());
    }

    #[test]
    fn trash_during_refresh_is_not_undone() {
        let dir = TempDir::new("trash");
        let c = dir.path().join("C");
        fs::create_dir(&c).unwrap();
        fs::write(c.join("x"), [0u8; 10]).unwrap();
        let s = Sessions::with_watch(Arc::new(|_: &Path, _| None));
        let id = ScanId(1);
        *lock(&s.state) = State::Scanning {
            id,
            cancel: CancelToken::new(),
            sink: new_sink(|_| {}),
            watch: None,
        };
        let _ = finish(&s.state, id, ScanOptions::default(), scanned(dir.path()));

        // A read of the root begins while `C` is still in the tree...
        let Prep::Ready(p) = prepare(&s.state, id, dir.path(), Depth::Shallow, &HashMap::new())
        else {
            panic!("expected a read");
        };
        let update = refresh::read(p.req, &p.options, &CancelToken::new());
        // ...the app moves `C` to the trash (on disk it stays here)...
        let c_id = match &*lock(&s.state) {
            State::Done { scan, .. } => {
                u32::try_from(scan.tree.lookup(&c).unwrap().index()).unwrap()
            }
            _ => panic!("scan not done"),
        };
        s.trash(id, &[c_id], |_, _, _| TrashOutcome::Trashed)
            .unwrap();
        // ...so the read, which still saw `C`, must not bring it back.
        assert!(matches!(
            commit(&s.state, id, p.edits, update),
            Commit::Stale
        ));
        match &*lock(&s.state) {
            State::Done { scan, .. } => assert!(scan.tree.lookup(&c).is_none()),
            _ => panic!("scan not done"),
        }
    }

    #[test]
    fn unknown_dir_refreshes_first_known_ancestor() {
        let dir = TempDir::new("ancestor");
        let live = Live::start(dir.path());
        live.finish(scanned(dir.path()));
        let deep = dir.path().join("new").join("deep");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("f"), [0u8; 10]).unwrap();

        {
            let state = lock(&live.state);
            let State::Done { scan, .. } = &*state else {
                panic!("scan not done");
            };
            let (at, depth, _) = target(&scan.tree, &deep, Depth::Subtree).unwrap();
            assert_eq!((at.as_path(), depth), (dir.path(), Depth::Shallow));
        }
        assert_eq!(
            refresh_all(&live.state, live.id, &[(deep.clone(), Depth::Subtree)]),
            Round::Changed
        );
        assert!(live.has(&deep.join("f")));
    }

    #[test]
    fn root_gone_emits_outdated_and_stops() {
        let dir = TempDir::new("gone");
        let live = Live::start(dir.path());
        live.finish(scanned(dir.path()));
        assert!(matches!(next(&live.events), ScanEvent::Done { .. }));

        fs::remove_dir_all(dir.path()).unwrap();
        live.changes
            .send(entry_change(dir.path().to_path_buf()))
            .unwrap();
        assert_eq!(
            format!("{:?}", next(&live.events)),
            format!("{:?}", ScanEvent::Outdated { scan: live.id })
        );
        match &*lock(&live.state) {
            State::Done { scan, .. } => assert!(scan.watch.is_none()),
            _ => panic!("scan not done"),
        }
    }

    #[test]
    fn attach_replaces_the_event_channel() {
        let dir = TempDir::new("attach");
        let (watch, senders) = fake_watch();
        let s = Sessions::with_watch(watch);
        let (tx, first) = mpsc::channel();
        let id = s
            .start(dir.path().to_path_buf(), ScanOptions::default(), move |e| {
                let _ = tx.send(e);
            })
            .unwrap();
        assert!(matches!(next(&first), ScanEvent::Done { .. }));

        let (tx, second) = mpsc::channel();
        let summary = s
            .attach(move |e| {
                let _ = tx.send(e);
            })
            .unwrap();
        assert_eq!(summary.scan, id);

        fs::write(dir.path().join("n"), [0u8; 10]).unwrap();
        let changes = senders.lock().unwrap()[0].clone();
        changes.send(entry_change(dir.path().join("n"))).unwrap();
        assert!(matches!(next(&second), ScanEvent::Changed { .. }));
        assert!(first.try_recv().is_err());
    }
}
