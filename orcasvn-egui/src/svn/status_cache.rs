use super::executor::SvnError;
use super::operations::{status, status_for_paths};
use super::status_snapshot::{self, Inventory};
use crate::SvnStatus;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_WORKSPACES: usize = 3;
const FULL_CHECK_INTERVAL: Duration = Duration::from_secs(5 * 60);

type Workspace = Arc<tokio::sync::Mutex<WorkspaceStatus>>;
static WORKSPACES: OnceLock<Mutex<VecDeque<(PathBuf, Workspace)>>> = OnceLock::new();

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
enum QueryMode {
    Full,
    Scoped(usize),
    Reuse,
}

struct WorkspaceStatus {
    entries: Option<Vec<SvnStatus>>,
    inventory: Option<Arc<Inventory>>,
    last_full_check: Option<Instant>,
    pending_save: Option<tokio::task::JoinHandle<()>>,
    #[cfg(test)]
    last_query: Option<QueryMode>,
}

impl WorkspaceStatus {
    fn new() -> Self {
        Self {
            entries: None,
            inventory: None,
            last_full_check: None,
            pending_save: None,
            #[cfg(test)]
            last_query: None,
        }
    }
}

fn get_workspace(root: &Path) -> Result<Workspace, SvnError> {
    let mut workspaces = WORKSPACES
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .map_err(|_| SvnError::CommandFailed("文件状态缓存锁已损坏".into()))?;
    if let Some(index) = workspaces.iter().position(|(path, _)| path == root) {
        let item = workspaces.remove(index).unwrap();
        let workspace = item.1.clone();
        workspaces.push_back(item);
        return Ok(workspace);
    }
    let workspace = Arc::new(tokio::sync::Mutex::new(WorkspaceStatus::new()));
    workspaces.push_back((root.to_path_buf(), workspace.clone()));
    while workspaces.len() > MAX_WORKSPACES {
        workspaces.pop_front();
    }
    Ok(workspace)
}

fn merge_status(previous: &[SvnStatus], fresh: Vec<SvnStatus>, paths: &[String]) -> Vec<SvnStatus> {
    let collapsed: Vec<_> = fresh
        .iter()
        .filter(|entry| matches!(entry.status_code.as_str(), "missing" | "unversioned"))
        .map(|entry| entry.path.clone())
        .collect();
    let mut entries: Vec<_> = previous
        .iter()
        .filter(|entry| {
            !paths.iter().any(|path| entry.path == *path)
                && !collapsed.iter().any(|path| {
                    entry
                        .path
                        .strip_prefix(path)
                        .is_some_and(|tail| tail.starts_with('/'))
                })
        })
        .cloned()
        .collect();
    entries.extend(fresh);
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.dedup_by(|a, b| a.path == b.path);
    entries
}

async fn persist_status(
    workspace: &mut WorkspaceStatus,
    root: &Path,
    directory: PathBuf,
    before: Arc<Inventory>,
    entries: Vec<SvnStatus>,
) {
    if let Some(previous) = workspace.pending_save.take() {
        let _ = previous.await;
    }
    let save_root = root.to_path_buf();
    workspace.pending_save = Some(tokio::task::spawn_blocking(move || {
        let Some(after) = status_snapshot::verify(&save_root, &before) else {
            return;
        };
        if let Err(error) = status_snapshot::write(&directory, &save_root, after, entries) {
            tracing::warn!(%error, "could not persist workspace status");
        }
    }));
}

/// Each scheduled refresh compares filesystem metadata, then scopes SVN queries.
pub async fn cached_status(
    path: &str,
    force: bool,
    cache_dir: Option<PathBuf>,
) -> Result<Vec<SvnStatus>, SvnError> {
    let owned_path = path.to_string();
    let root = tokio::task::spawn_blocking(move || std::fs::canonicalize(owned_path))
        .await
        .map_err(|error| SvnError::CommandFailed(error.to_string()))?
        .map_err(|error| SvnError::CommandFailed(format!("无法访问工作副本：{error}")))?;
    let workspace = get_workspace(&root)?;
    let mut workspace = workspace.lock().await;
    let scan_root = root.clone();
    let restore_dir = if workspace.entries.is_none() && !force {
        cache_dir.clone()
    } else {
        None
    };
    let rescan_due = force
        || (workspace.entries.is_some()
            && workspace
                .last_full_check
                .is_none_or(|time| time.elapsed() >= FULL_CHECK_INTERVAL));
    let previous_inventory = if rescan_due {
        None
    } else {
        workspace.inventory.clone()
    };
    // Run directory I/O away from the async command thread; enumerate once per
    // refresh, including while the app is open, with no watcher registration.
    let (scan, restored) = tokio::task::spawn_blocking(move || {
        let restored =
            restore_dir.and_then(|directory| status_snapshot::read(&directory, &scan_root));
        let baseline = previous_inventory
            .as_deref()
            .or_else(|| restored.as_ref().map(|snapshot| &snapshot.inventory));
        (status_snapshot::scan(&scan_root, baseline), restored)
    })
    .await
    .map_err(|error| SvnError::CommandFailed(error.to_string()))?;
    if let Some(snapshot) = restored {
        tracing::debug!(workspace = %root.display(), "disk status baseline restored");
        workspace.entries = Some(snapshot.entries);
        workspace.inventory = Some(Arc::new(snapshot.inventory));
        workspace.last_full_check = Some(Instant::now());
    }
    let (current, hint, journal_reset) = match scan {
        Ok(scan) => (Some(Arc::new(scan.inventory)), scan.paths, scan.force_full),
        Err(error) => {
            tracing::warn!(%error, "filesystem comparison unavailable; using complete SVN scan");
            (None, None, true)
        }
    };
    // USN reasons remain candidates even when an editor preserves mtime/size.
    let changed = hint.or_else(|| {
        workspace
            .inventory
            .as_deref()
            .zip(current.as_deref())
            .and_then(|(before, after)| status_snapshot::differences(before, after))
    });
    let full = force
        || journal_reset
        || workspace.entries.is_none()
        || changed.is_none()
        || workspace
            .last_full_check
            .is_none_or(|time| time.elapsed() >= FULL_CHECK_INTERVAL);
    let paths = changed.unwrap_or_default();
    let cwd = root.to_string_lossy();
    if !full && paths.is_empty() {
        tracing::debug!(workspace = %cwd, "filesystem unchanged; reusing SVN status");
        #[cfg(test)]
        {
            workspace.last_query = Some(QueryMode::Reuse);
        }
        let entries = workspace.entries.clone().unwrap_or_default();
        if let (Some(directory), Some(before)) = (cache_dir, current.clone()) {
            if status_snapshot::checkpoint_changed(workspace.inventory.as_deref(), &before) {
                persist_status(&mut workspace, &root, directory, before, entries.clone()).await;
            }
        }
        workspace.inventory = current;
        return Ok(entries);
    }
    let mut used_full_scan = full;
    let result = if full {
        tracing::debug!(workspace = %cwd, "complete SVN status scan");
        status(&cwd).await
    } else {
        tracing::debug!(workspace = %cwd, ?paths, "filesystem changes scoped SVN status");
        match status_for_paths(&cwd, &paths).await {
            Ok(fresh) => Ok(merge_status(
                workspace.entries.as_deref().unwrap_or_default(),
                fresh,
                &paths,
            )),
            Err(error) => {
                tracing::debug!(%error, "scoped status failed; retrying complete scan");
                used_full_scan = true;
                status(&cwd).await
            }
        }
    };
    match result {
        Ok(entries) => {
            workspace.entries = Some(entries.clone());
            // Keep the pre-query inventory: any edit during SVN execution will
            // differ on the next poll and be checked again.
            workspace.inventory = current.clone();
            if used_full_scan {
                workspace.last_full_check = Some(Instant::now());
            }
            #[cfg(test)]
            {
                workspace.last_query = Some(if used_full_scan {
                    QueryMode::Full
                } else {
                    QueryMode::Scoped(paths.len())
                });
            }
            if let (Some(directory), Some(before)) = (cache_dir, current) {
                persist_status(&mut workspace, &root, directory, before, entries.clone()).await;
            }
            Ok(entries)
        }
        Err(error) => {
            // Failed queries must not promote a new filesystem baseline.
            workspace.inventory = None;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    async fn cached_status(path: &str, force: bool) -> Result<Vec<SvnStatus>, SvnError> {
        super::cached_status(path, force, None).await
    }

    fn entry(path: &str, code: &str) -> SvnStatus {
        SvnStatus {
            path: path.into(),
            status: code.into(),
            status_code: code.into(),
            prop_status: "none".into(),
            locked: false,
            history: false,
            switched: false,
        }
    }

    #[test]
    fn scoped_merge_removes_cleaned_files_without_erasing_other_paths() {
        let previous = [
            entry("src/a", "modified"),
            entry("src/sub/keep", "modified"),
            entry("src2/a", "modified"),
        ];
        let merged = merge_status(
            &previous,
            vec![entry("src/new", "unversioned")],
            &["src/a".into(), "src/new".into()],
        );
        assert_eq!(
            merged
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            ["src/new", "src/sub/keep", "src2/a"]
        );
        let merged = merge_status(
            &previous,
            vec![entry("src/sub", "missing")],
            &["src/a".into(), "src/sub".into()],
        );
        assert_eq!(
            merged
                .iter()
                .map(|entry| entry.path.as_str())
                .collect::<Vec<_>>(),
            ["src/sub", "src2/a"]
        );
    }

    async fn assert_query_mode(root: &Path, expected: QueryMode) {
        let workspace = get_workspace(&fs::canonicalize(root).unwrap()).unwrap();
        assert_eq!(workspace.lock().await.last_query.as_ref(), Some(&expected));
    }

    #[test]
    fn polling_reuses_unchanged_status_and_forces_metadata_and_periodic_checks() {
        let Some(fixture) = Fixture::new() else {
            return;
        };
        fixture.init();
        runtime().block_on(async {
            let path = fixture.wc.to_str().unwrap();
            cached_status(path, false).await.unwrap();
            assert_query_mode(&fixture.wc, QueryMode::Full).await;
            cached_status(path, false).await.unwrap();
            assert_query_mode(&fixture.wc, QueryMode::Reuse).await;
            fs::write(fixture.wc.join("src/file@name.txt"), "changed\n").unwrap();
            let changed = cached_status(path, false).await.unwrap();
            assert_query_mode(&fixture.wc, QueryMode::Scoped(1)).await;
            assert_eq!(statuses(&changed), statuses(&status(path).await.unwrap()));
            fixture.svn(&["propset", "test:property", "value", path]);
            cached_status(path, false).await.unwrap();
            assert_query_mode(&fixture.wc, QueryMode::Full).await;
            let workspace = get_workspace(&fs::canonicalize(&fixture.wc).unwrap()).unwrap();
            workspace.lock().await.last_full_check = Some(Instant::now() - FULL_CHECK_INTERVAL);
            cached_status(path, false).await.unwrap();
            assert_query_mode(&fixture.wc, QueryMode::Full).await;
            cached_status(path, true).await.unwrap();
            assert_query_mode(&fixture.wc, QueryMode::Full).await;
        });
    }

    async fn flush_snapshot(root: &Path) {
        let workspace = get_workspace(&fs::canonicalize(root).unwrap()).unwrap();
        let mut workspace = workspace.lock().await;
        if let Some(save) = workspace.pending_save.take() {
            save.await.unwrap();
        }
    }

    fn forget_workspace(root: &Path) {
        let root = fs::canonicalize(root).unwrap();
        WORKSPACES
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .retain(|(path, _)| path != &root);
    }

    #[test]
    fn disk_snapshot_reopens_and_checks_offline_changes() {
        let Some(fixture) = Fixture::new() else {
            return;
        };
        fixture.init();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let path = fixture.wc.to_str().unwrap();
            let directory = fixture.root.join("cache");
            let query = || super::cached_status(path, false, Some(directory.clone()));
            assert!(query().await.unwrap().is_empty());
            flush_snapshot(&fixture.wc).await;
            let saved =
                status_snapshot::read(&directory, &fs::canonicalize(&fixture.wc).unwrap()).unwrap();
            forget_workspace(&fixture.wc);
            fs::write(fixture.wc.join("src/file@name.txt"), "edit\n").unwrap();
            fs::rename(
                fixture.wc.join("src/sub/other.txt"),
                fixture.wc.join("src/sub/renamed.txt"),
            )
            .unwrap();
            fs::write(fixture.wc.join("ignored.tmp"), "ignored").unwrap();
            let current = status_snapshot::capture(&fixture.wc).unwrap();
            assert_eq!(
                status_snapshot::differences(&saved.inventory, &current)
                    .unwrap()
                    .len(),
                4
            );
            assert_eq!(
                statuses(&query().await.unwrap()),
                statuses(&status(path).await.unwrap())
            );
            flush_snapshot(&fixture.wc).await;
            forget_workspace(&fixture.wc);
            // Revert one previous modification while closed, retaining unrelated ones.
            fs::write(fixture.wc.join("src/file@name.txt"), "base\n").unwrap();
            assert_eq!(
                statuses(&query().await.unwrap()),
                statuses(&status(path).await.unwrap())
            );
            flush_snapshot(&fixture.wc).await;
            forget_workspace(&fixture.wc);
            fs::create_dir(fixture.wc.join("new-dir")).unwrap();
            fs::write(fixture.wc.join("new-dir/file"), "new").unwrap();
            assert_eq!(
                statuses(&query().await.unwrap()),
                statuses(&status(path).await.unwrap())
            );
            flush_snapshot(&fixture.wc).await;
            forget_workspace(&fixture.wc);
            fixture.svn(&["propset", "test:property", "value", path]);
            assert_eq!(
                statuses(&query().await.unwrap()),
                statuses(&status(path).await.unwrap())
            );
            flush_snapshot(&fixture.wc).await;
            forget_workspace(&fixture.wc);
            // Corrupt disk cache cannot hide changes.
            for item in fs::read_dir(&directory).unwrap() {
                fs::write(item.unwrap().path(), "broken json").unwrap();
            }
            fs::remove_file(fixture.wc.join("src/file@name.txt")).unwrap();
            assert_eq!(
                statuses(&query().await.unwrap()),
                statuses(&status(path).await.unwrap())
            );
            flush_snapshot(&fixture.wc).await;
        });
    }

    #[test]
    fn inventory_detects_same_size_edits_and_does_not_follow_symlinks() {
        let root = std::env::temp_dir().join(format!(
            "orcasvn-inventory-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("file"), "aaaa").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/", root.join("outside")).unwrap();
        let before = status_snapshot::capture(&root).unwrap();
        #[cfg(unix)]
        let original_time = fs::metadata(root.join("file")).unwrap().modified().unwrap();
        // Restoring mtime leaves ctime as the edit signal. Let the filesystem
        // clock advance so both writes cannot share one timestamp tick.
        #[cfg(unix)]
        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::write(root.join("file"), "bbbb").unwrap();
        #[cfg(unix)]
        fs::File::options()
            .write(true)
            .open(root.join("file"))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(original_time))
            .unwrap();
        let after = status_snapshot::capture(&root).unwrap();
        assert_eq!(
            status_snapshot::differences(&before, &after),
            Some(vec!["file".into()])
        );
        assert_eq!(before.len(), if cfg!(unix) { 2 } else { 1 });
        fs::create_dir(root.join("new-dir")).unwrap();
        assert!(
            status_snapshot::differences(&after, &status_snapshot::capture(&root).unwrap())
                .is_none()
        );
        fs::remove_dir_all(root).unwrap();
    }

    struct Fixture {
        root: PathBuf,
        wc: PathBuf,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    impl Fixture {
        fn new() -> Option<Self> {
            if Command::new("svnadmin").arg("--version").output().is_err() {
                eprintln!("Skipping real SVN fixture: svnadmin not available");
                return None;
            }
            let root = std::env::temp_dir().join(format!(
                "orcasvn-scopes-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&root).unwrap();
            let repository = root.join("repo");
            assert!(Command::new("svnadmin")
                .arg("create")
                .arg(&repository)
                .output()
                .unwrap()
                .status
                .success());
            let fixture = Self {
                wc: root.join("wc"),
                root,
            };
            let url = format!(
                "file:///{}",
                repository
                    .to_string_lossy()
                    .replace('\\', "/")
                    .trim_start_matches('/')
            );
            fixture.svn(&["checkout", &url, fixture.wc.to_str().unwrap()]);
            Some(fixture)
        }
        fn svn(&self, args: &[&str]) {
            let output = Command::new("svn").args(args).output().unwrap();
            assert!(
                output.status.success(),
                "{:?}: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        fn init(&self) {
            fs::create_dir_all(self.wc.join("src/sub")).unwrap();
            fs::write(self.wc.join("src/file@name.txt"), "base\n").unwrap();
            fs::write(self.wc.join("src/sub/other.txt"), "base\n").unwrap();
            self.svn(&["add", self.wc.join("src").to_str().unwrap()]);
            self.svn(&[
                "propset",
                "svn:ignore",
                "build\n*.tmp",
                self.wc.to_str().unwrap(),
            ]);
            self.svn(&["commit", self.wc.to_str().unwrap(), "-m", "initial"]);
        }
    }
    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }
    fn statuses(entries: &[SvnStatus]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|entry| (entry.path.clone(), format!("{entry:?}")))
            .collect()
    }

    #[test]
    fn subdirectory_cache_checks_ancestor_administration_in_memory_and_on_disk() {
        let Some(fixture) = Fixture::new() else {
            return;
        };
        fixture.init();
        let subdirectory = fixture.wc.join("src/sub");
        let path = subdirectory.to_str().unwrap();
        let directory = fixture.root.join("cache");
        runtime().block_on(async {
            let query = || super::cached_status(path, false, Some(directory.clone()));
            assert!(query().await.unwrap().is_empty());
            flush_snapshot(&subdirectory).await;

            // Property edits change wc.db in the parent working-copy root,
            // leaving the selected subtree's filesystem inventory unchanged.
            fixture.svn(&["propset", "test:property", "value", path]);
            let current = query().await.unwrap();
            assert_eq!(statuses(&current), statuses(&status(path).await.unwrap()));
            assert!(current
                .iter()
                .any(|entry| entry.path == "." && entry.prop_status == "modified"));
            flush_snapshot(&subdirectory).await;
            forget_workspace(&subdirectory);

            // Reopening must also reject an unchanged subtree paired with an
            // outdated disk snapshot after an external metadata-only revert.
            fixture.svn(&["revert", path]);
            let current = query().await.unwrap();
            assert!(current.is_empty());
            assert_eq!(statuses(&current), statuses(&status(path).await.unwrap()));
            flush_snapshot(&subdirectory).await;
        });
    }

    #[test]
    fn real_svn_scoped_status_preserves_ignore_rules_and_unversioned_directory_collapsing() {
        let Some(fixture) = Fixture::new() else {
            return;
        };
        fixture.init();
        fs::create_dir_all(fixture.wc.join("build/deep")).unwrap();
        fs::write(fixture.wc.join("build/deep/generated.txt"), "ignored").unwrap();
        fs::create_dir_all(fixture.wc.join("new-dir/deep")).unwrap();
        fs::write(fixture.wc.join("new-dir/deep/new.txt"), "new").unwrap();
        runtime().block_on(async {
            let full = status(fixture.wc.to_str().unwrap()).await.unwrap();
            let fresh = status_for_paths(
                fixture.wc.to_str().unwrap(),
                &[
                    "build/deep/generated.txt".into(),
                    "new-dir/deep/new.txt".into(),
                ],
            )
            .await
            .unwrap();
            assert_eq!(
                statuses(&merge_status(
                    &full,
                    fresh,
                    &[
                        "build/deep/generated.txt".into(),
                        "new-dir/deep/new.txt".into()
                    ]
                )),
                statuses(&full)
            );
        });
    }

    #[test]
    fn real_filesystem_changes_update_reverted_renamed_added_and_missing_files() {
        let Some(fixture) = Fixture::new() else {
            return;
        };
        fixture.init();
        runtime().block_on(async {
            let wc = fixture.wc.to_str().unwrap();
            assert!(cached_status(wc, false).await.unwrap().is_empty());
            fs::write(fixture.wc.join("src/file@name.txt"), "changed\n").unwrap();
            assert_eq!(
                statuses(&cached_status(wc, false).await.unwrap()),
                statuses(&status(wc).await.unwrap())
            );
            fs::write(fixture.wc.join("src/file@name.txt"), "base\n").unwrap();
            fs::rename(
                fixture.wc.join("src/sub/other.txt"),
                fixture.wc.join("src/sub/renamed.txt"),
            )
            .unwrap();
            fs::write(fixture.wc.join("added.txt"), "new").unwrap();
            fs::write(fixture.wc.join("ignored.tmp"), "ignored").unwrap();
            let actual = cached_status(wc, false).await.unwrap();
            assert_eq!(statuses(&actual), statuses(&status(wc).await.unwrap()));
            assert!(!actual
                .iter()
                .any(|entry| entry.path.ends_with("ignored.tmp")));
            // Another SVN client changed metadata: complete verification is required.
            fixture.svn(&[
                "delete",
                fixture.wc.join("src").to_str().unwrap(),
                "--force",
            ]);
            let actual = cached_status(wc, false).await.unwrap();
            assert!(actual
                .iter()
                .any(|entry| entry.path == "src/sub/other.txt" && entry.status_code == "deleted"));
            assert_eq!(statuses(&actual), statuses(&status(wc).await.unwrap()));
        });
    }

    #[test]
    #[ignore = "creates a real 20,000-file working copy for performance measurement"]
    fn benchmark_twenty_thousand_files_forty_changes() {
        let Some(fixture) = Fixture::new() else {
            panic!("svnadmin required for benchmark");
        };
        for directory in 0..200 {
            let target = fixture.wc.join(format!("group-{directory:03}"));
            fs::create_dir_all(&target).unwrap();
            for file in 0..100 {
                fs::write(target.join(format!("file-{file:03}.txt")), "baseline\n").unwrap();
            }
        }
        fixture.svn(&["add", fixture.wc.to_str().unwrap(), "--force"]);
        fixture.svn(&["commit", fixture.wc.to_str().unwrap(), "-m", "baseline"]);
        // Model reopening an existing workspace after directory stamps settle.
        std::thread::sleep(Duration::from_millis(1100));
        runtime().block_on(async {
            let wc = fixture.wc.to_str().unwrap();
            let directory = fixture.root.join("cache");
            let initial = Instant::now();
            super::cached_status(wc, false, Some(directory.clone())).await.unwrap();
            flush_snapshot(&fixture.wc).await;
            let initial_ms = initial.elapsed().as_secs_f64() * 1000.;
            for file in 0..40 { fs::write(fixture.wc.join(format!("group-000/file-{file:03}.txt")), "modified\n").unwrap(); }
            let start = Instant::now();
            let scoped = cached_status(wc, false).await.unwrap();
            let scoped_ms = start.elapsed().as_secs_f64() * 1000.;
            assert_query_mode(&fixture.wc, QueryMode::Scoped(40)).await;
            let start = Instant::now();
            let full = status(wc).await.unwrap();
            let full_ms = start.elapsed().as_secs_f64() * 1000.;
            assert_eq!(scoped.len(), 40);
            assert_eq!(statuses(&scoped), statuses(&full));
            let start = Instant::now();
            cached_status(wc, false).await.unwrap();
            let unchanged_ms = start.elapsed().as_secs_f64() * 1000.;
            flush_snapshot(&fixture.wc).await;
            forget_workspace(&fixture.wc);
            let start = Instant::now();
            let reopened = super::cached_status(wc, false, Some(directory.clone())).await.unwrap();
            let reopen_changed_ms = start.elapsed().as_secs_f64() * 1000.;
            assert_query_mode(&fixture.wc, QueryMode::Scoped(40)).await;
            assert_eq!(statuses(&reopened), statuses(&full));
            flush_snapshot(&fixture.wc).await;
            forget_workspace(&fixture.wc);
            let start = Instant::now();
            let reopened = super::cached_status(wc, false, Some(directory)).await.unwrap();
            let reopen_unchanged_ms = start.elapsed().as_secs_f64() * 1000.;
            assert_query_mode(&fixture.wc, QueryMode::Reuse).await;
            flush_snapshot(&fixture.wc).await;
            assert_eq!(statuses(&reopened), statuses(&full));
            let start = Instant::now();
            let inventory = status_snapshot::capture(&fixture.wc).unwrap();
            let inventory_ms = start.elapsed().as_secs_f64() * 1000.;
            let start = Instant::now();
            let (incremental, stats) = status_snapshot::capture_measured(&fixture.wc, Some(&inventory)).unwrap();
            let incremental_ms = start.elapsed().as_secs_f64() * 1000.;
            assert_eq!(inventory, incremental);
            #[cfg(unix)]
            assert!(stats.reused_directories >= 200);
            eprintln!("Incremental inventory: {incremental_ms:.1} ms, enumerated dirs {}, reused dirs {}, checked files {}", stats.enumerated_directories, stats.reused_directories, stats.checked_files);
            eprintln!("Filesystem inventory only: {inventory_ms:.1} ms");
            eprintln!("Disk snapshot reopen: 40 offline changes {reopen_changed_ms:.1} ms, unchanged {reopen_unchanged_ms:.1} ms (includes inventory and JSON loading)");
            eprintln!("20,000 files / 40 changes: initial + snapshot {initial_ms:.1} ms, filesystem-scoped {scoped_ms:.1} ms, full SVN {full_ms:.1} ms, unchanged {unchanged_ms:.1} ms");
        });
    }

    #[test]
    #[ignore = "read-only startup benchmark; set ORCASVN_BENCH_WORKSPACE"]
    fn benchmark_workspace_startup() {
        let path = std::env::var("ORCASVN_BENCH_WORKSPACE")
            .expect("set ORCASVN_BENCH_WORKSPACE to a real SVN working copy");
        let root = fs::canonicalize(&path).unwrap();
        let temporary = Fixture {
            wc: root.clone(),
            root: std::env::temp_dir().join(format!(
                "orcasvn-startup-bench-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            )),
        };
        let inventory = status_snapshot::capture(&root).unwrap();
        eprintln!(
            "Workspace: {path}; inventory entries: {}; USN accessible: {}; optimized: {}",
            inventory.len(),
            inventory.journal_available(),
            !cfg!(debug_assertions)
        );
        runtime().block_on(async {
            let expected = statuses(&status(&path).await.unwrap());
            eprintln!("Changed status entries: {}", expected.len());
            for scenario in ["no-cache", "disk-cache"] {
                let mut measurements = Vec::new();
                for iteration in 0..7 {
                    let directory = if scenario == "no-cache" {
                        temporary.root.join(format!("cold-{iteration}"))
                    } else {
                        temporary.root.join("cold-6")
                    };
                    // A fresh in-memory workspace models application reopening.
                    if WORKSPACES.get().is_some() { forget_workspace(&root); }
                    let started = Instant::now();
                    let revision_path = path.clone();
                    let revision = tokio::spawn(async move {
                        let started = Instant::now();
                        super::super::operations::local_revision(&revision_path).await.unwrap();
                        started.elapsed().as_secs_f64() * 1000.
                    });
                    let metadata_path = path.clone();
                    let metadata = (scenario == "no-cache").then(|| tokio::spawn(async move {
                        super::super::operations::info(&metadata_path).await
                    }));
                    let status_started = Instant::now();
                    let entries = super::cached_status(&path, false, Some(directory)).await.unwrap();
                    let status_ms = status_started.elapsed().as_secs_f64() * 1000.;
                    let revision_ms = revision.await.unwrap();
                    if let Some(metadata) = metadata { metadata.await.unwrap().unwrap(); }
                    let total_ms = started.elapsed().as_secs_f64() * 1000.;
                    assert_eq!(statuses(&entries), expected, "workspace changed during benchmark");
                    assert_query_mode(&root, if scenario == "no-cache" { QueryMode::Full } else { QueryMode::Reuse }).await;
                    // Excluded from foreground time; settle before next reopening.
                    flush_snapshot(&root).await;
                    measurements.push(total_ms);
                    eprintln!("STARTUP {scenario} run {}: status={status_ms:.1} ms, revision={revision_ms:.1} ms, total={total_ms:.1} ms", iteration + 1);
                }
                measurements.sort_by(f64::total_cmp);
                eprintln!("SUMMARY {scenario}: min={:.1} ms median={:.1} ms max={:.1} ms (7 runs)",
                    measurements[0], measurements[3], measurements[6]);
            }
            forget_workspace(&root);
        });
    }
}
