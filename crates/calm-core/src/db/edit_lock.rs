use std::fs::{File, OpenOptions};
use std::path::Path;

use fs4::fs_std::FileExt;

/// Holds an OS advisory lock (`flock`/`LockFileEx`) on `<calm_dir>/edit.lock`
/// for the duration of one `edit_lines`/`edit_symbol` call — serializes the
/// read -> hash-check -> write -> reindex sequence *across* `calm serve`
/// processes.
///
/// Deliberately a separate lock file from `instance_lock`'s
/// `indexer.lock`, not a reuse of it: `indexer.lock` means "I am this
/// project's indexer/watcher owner" and only one process ever holds it,
/// whereas *any* process — owner or not — must be able to edit, they just
/// need to serialize with each other while doing so. Conflating the two
/// would mean only the indexer-lock owner could ever safely edit, which is
/// wrong.
///
/// `CalmServer::edit_lock` (an in-process `Mutex<()>`, see
/// `crates/calm-server/src/tools/edit.rs`) already serializes this same
/// sequence *within* one process — that Mutex is constructed fresh per
/// process, so it does nothing to protect two different `calm serve`
/// processes (e.g. one spawned by Cursor, one by Claude Code, both on the
/// same project) editing the same file at the same time. Without this,
/// both processes can read the same pre-edit hash before either writes,
/// both pass `expected_hash` validation, and the second writer's full-file
/// replace silently discards the first writer's change — the exact TOCTOU
/// the in-process `Mutex` already closes *within* a process, still open
/// *across* processes.
pub struct EditLock(#[allow(dead_code)] File);

/// Blocks until this process holds the project's edit lock. Blocking
/// (`lock_exclusive`), not `try_lock_exclusive`: a caller here should wait
/// for another process's in-flight edit to finish, not fail outright —
/// mirrors how the in-process `Mutex` already blocks rather than erroring
/// on contention. Returns `Err` only on an OS-level failure to open/lock
/// the file (e.g. permissions, disk full); contention itself is not an
/// error, the call just waits.
pub fn acquire(calm_dir: &Path) -> std::io::Result<EditLock> {
    let path = calm_dir.join("edit.lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    file.lock_exclusive()?;
    Ok(EditLock(file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn second_acquirer_blocks_until_first_drops() {
        let dir = tempfile::tempdir().unwrap();
        let first = acquire(dir.path()).unwrap();

        let dir_path = dir.path().to_path_buf();
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let _second = acquire(&dir_path).unwrap();
            tx.send(()).unwrap();
        });

        assert!(
            rx.recv_timeout(Duration::from_millis(200)).is_err(),
            "second acquirer must still be blocked while first holds the lock"
        );

        drop(first);
        assert!(
            rx.recv_timeout(Duration::from_secs(2)).is_ok(),
            "second acquirer must proceed once first drops"
        );
        handle.join().unwrap();
    }

    #[test]
    fn acquire_creates_lock_file_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!dir.path().join("edit.lock").exists());
        let _lock = acquire(dir.path()).unwrap();
        assert!(dir.path().join("edit.lock").exists());
    }
}
