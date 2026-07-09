use std::fs::{File, OpenOptions};
use std::path::Path;

use fs4::fs_std::FileExt;

/// Holds an OS advisory lock (`flock`/`LockFileEx`) on `<calm_dir>/indexer.lock`
/// for the lifetime of this handle. Dropping it (including on process exit,
/// even a crash — the OS releases `flock`s automatically when the holding
/// process dies) releases the lock.
///
/// Exists because every `calm serve` process independently spawned its own
/// background indexer + file watcher against the same shared `index.db` —
/// harmless with a single process, but running several concurrently (e.g.
/// multiple editor/MCP-client sessions on the same project_root) meant N
/// redundant reindex loops racing each other, mitigated only by
/// `open_writer`'s `busy_timeout`, not prevented.
pub struct IndexerLock(#[allow(dead_code)] File);

/// Tries to become this project's sole indexer/watcher owner.
///
/// `Some(lock)` — this process acquired the lock and must run the
/// background indexer + watcher; keep the returned guard alive for as long
/// as that's true.
/// `None` — another live process already holds it. The caller should skip
/// spawning its own indexer/watcher entirely and just serve tool calls
/// read-only against the DB the owning process keeps fresh — still fully
/// functional, just not the one doing the (re)indexing.
pub fn try_acquire(calm_dir: &Path) -> Option<IndexerLock> {
    let path = calm_dir.join("indexer.lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .ok()?;
    file.try_lock_exclusive().ok()?;
    Some(IndexerLock(file))
}

/// Blocks until this process becomes the project's sole indexer/watcher
/// owner. Used by a process that initially lost the `try_acquire` race to
/// promote itself automatically if the current owner ever exits later in
/// the session — gracefully (clean shutdown drops the lock) or not (a
/// crash/OOM/SIGKILL still releases the OS `flock` the moment the process
/// dies, same guarantee `try_acquire`'s own doc comment relies on). Without
/// this, a lock-loser process that started before the owner died would stay
/// read-only for its entire remaining lifetime — nothing else in this
/// codebase ever retries the initial `try_acquire`.
///
/// Unlike `try_acquire`, this call can block indefinitely (as long as some
/// other process holds the lock). Callers must run it on a dedicated
/// blocking thread (e.g. inside `tokio::task::spawn_blocking`), never on an
/// async reactor thread — blocking a reactor thread here would stall every
/// other task sharing that thread for as long as some other process keeps
/// the lock.
pub fn acquire_blocking(calm_dir: &Path) -> std::io::Result<IndexerLock> {
    let path = calm_dir.join("indexer.lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    file.lock_exclusive()?;
    Ok(IndexerLock(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_acquirer_succeeds_second_fails_until_first_drops() {
        let dir = tempfile::tempdir().unwrap();
        let first = try_acquire(dir.path());
        assert!(first.is_some(), "first acquirer must succeed");

        let second = try_acquire(dir.path());
        assert!(
            second.is_none(),
            "second acquirer must fail while first still holds the lock"
        );

        drop(first);
        let third = try_acquire(dir.path());
        assert!(third.is_some(), "lock must be acquirable after release");
    }

    #[test]
    fn acquire_blocking_waits_for_release_then_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let first = try_acquire(dir.path()).unwrap();

        let dir_path = dir.path().to_path_buf();
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let _second = acquire_blocking(&dir_path).unwrap();
            tx.send(()).unwrap();
        });

        assert!(
            rx.recv_timeout(std::time::Duration::from_millis(200)).is_err(),
            "acquire_blocking must not return while the first lock is held"
        );

        drop(first);
        assert!(
            rx.recv_timeout(std::time::Duration::from_secs(2)).is_ok(),
            "acquire_blocking must succeed once the first lock is released"
        );
        handle.join().unwrap();
    }
}
