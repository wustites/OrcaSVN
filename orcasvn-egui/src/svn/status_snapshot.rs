//! A disk snapshot is usable only after comparing a fresh filesystem inventory.
use crate::SvnStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

// Older snapshots did not track administration outside the selected directory.
const VERSION: u32 = 6;
const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FILES: usize = 500_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Stamp {
    #[serde(rename = "k")]
    kind: u8,
    #[serde(rename = "s")]
    size: u64,
    #[serde(rename = "m")]
    modified: u128,
    #[serde(rename = "c")]
    changed: i128,
    #[serde(rename = "l", skip_serializing_if = "Option::is_none", default)]
    link: Option<PathBuf>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct DirectoryStamp {
    modified: u128,
    changed: i128,
    device: u64,
    inode: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Directory {
    stamp: DirectoryStamp,
    children: Vec<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Inventory {
    files: Vec<(String, Stamp)>,
    directories: BTreeMap<String, Directory>,
    configuration: BTreeMap<String, u64>,
    administration: BTreeMap<PathBuf, Stamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    journal: Option<super::status_usn::Journal>,
}
impl Inventory {
    #[cfg(test)]
    pub fn journal_available(&self) -> bool {
        self.journal.is_some()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.files.len()
    }
}
fn configuration() -> io::Result<BTreeMap<String, u64>> {
    let mut directories = vec![PathBuf::from("/etc/subversion")];
    for variable in ["HOME", "USERPROFILE", "APPDATA"] {
        if let Some(home) = std::env::var_os(variable) {
            directories.push(PathBuf::from(&home).join(if variable == "APPDATA" {
                "Subversion"
            } else {
                ".subversion"
            }));
        }
    }
    let mut result = BTreeMap::new();
    let executable = super::executor::current_svn_executable().map_err(io::Error::other)?;
    result.insert(format!("executable:{}", executable.display()), 0);
    for directory in directories {
        for name in ["config", "servers"] {
            let path = directory.join(name);
            match fs::metadata(&path) {
                Ok(metadata) => {
                    if metadata.len() > 1024 * 1024 {
                        return Err(io::Error::other("SVN configuration too large"));
                    }
                    // Hash configuration; never persist potential credentials.
                    let hash = fs::read(&path)?
                        .iter()
                        .fold(0xcbf29ce484222325u64, |hash, byte| {
                            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
                        });
                    result.insert(path.to_string_lossy().into_owned(), hash);
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(result)
}

#[derive(Default)]
pub(super) struct CaptureStats {
    pub enumerated_directories: usize,
    pub reused_directories: usize,
    pub checked_files: usize,
}

fn directory_stamp(metadata: &fs::Metadata) -> io::Result<DirectoryStamp> {
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::other("directory was replaced"));
    }
    #[cfg(unix)]
    let (changed, device, inode) = {
        use std::os::unix::fs::MetadataExt;
        (
            i128::from(metadata.ctime()) * 1_000_000_000 + i128::from(metadata.ctime_nsec()),
            metadata.dev(),
            metadata.ino(),
        )
    };
    #[cfg(not(unix))]
    let (changed, device, inode) = (0, 0, 0);
    Ok(DirectoryStamp {
        modified: metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos(),
        changed,
        device,
        inode,
    })
}

fn can_reuse(previous: &DirectoryStamp, current: &DirectoryStamp) -> bool {
    // Windows enumeration already supplies file metadata; individually stating
    // cached paths there would add expensive calls. Coarse Unix ctimes also
    // request enumeration rather than trusting an ambiguous directory stamp.
    let stable_age = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|now| {
            now.as_nanos()
                .checked_sub(u128::try_from(current.changed).ok()?)
        })
        .is_some_and(|age| age >= 1_000_000_000);
    cfg!(unix) && previous == current && current.changed % 1_000_000_000 != 0 && stable_age
}

fn directory_entry() -> Stamp {
    Stamp {
        kind: 0,
        size: 0,
        modified: 0,
        changed: 0,
        link: None,
    }
}

fn file_stamp(metadata: &fs::Metadata, path: &Path) -> io::Result<Stamp> {
    let kind = if metadata.is_file() {
        1
    } else if metadata.file_type().is_symlink() {
        2
    } else {
        return Err(io::Error::other("file type changed or is unsupported"));
    };
    #[cfg(unix)]
    let changed = {
        use std::os::unix::fs::MetadataExt;
        i128::from(metadata.ctime()) * 1_000_000_000 + i128::from(metadata.ctime_nsec())
    };
    #[cfg(not(unix))]
    let changed = 0;
    Ok(Stamp {
        kind,
        size: metadata.len(),
        modified: metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos(),
        changed,
        link: if kind == 2 {
            Some(fs::read_link(path)?)
        } else {
            None
        },
    })
}

fn ancestor_administration(root: &Path) -> io::Result<BTreeMap<PathBuf, Stamp>> {
    let mut files = BTreeMap::new();
    for ancestor in root.ancestors() {
        for name in [".svn", "_svn"] {
            let directory = ancestor.join(name);
            match fs::symlink_metadata(&directory) {
                Ok(metadata) => {
                    directory_stamp(&metadata)?;
                    // Administration inside the selected directory is already
                    // part of its filesystem inventory (and optional USN index).
                    if ancestor == root {
                        return Ok(files);
                    }
                    let mut pending = vec![directory];
                    while let Some(directory) = pending.pop() {
                        files.insert(directory.clone(), directory_entry());
                        for entry in fs::read_dir(&directory)? {
                            let entry = entry?;
                            if entry.file_name() == "pristine" || entry.file_name() == "tmp" {
                                continue;
                            }
                            let path = entry.path();
                            let metadata = entry.metadata()?;
                            if metadata.is_dir() {
                                pending.push(path);
                            } else {
                                files.insert(path.clone(), file_stamp(&metadata, &path)?);
                            }
                            if files.len() + pending.len() > MAX_FILES {
                                return Err(io::Error::other(
                                    "SVN administration inventory too large",
                                ));
                            }
                        }
                    }
                    return Ok(files);
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(files)
}

pub(super) fn capture(root: &Path) -> io::Result<Inventory> {
    capture_incremental(root, None)
}

pub(super) fn capture_incremental(
    root: &Path,
    previous: Option<&Inventory>,
) -> io::Result<Inventory> {
    #[cfg(windows)]
    let journal_start = super::status_usn::begin(root).ok();
    let (inventory, stats) = capture_measured(root, previous)?;
    #[cfg(windows)]
    let inventory = {
        let mut inventory = inventory;
        if let Some(session) = journal_start {
            let expected = inventory
                .files
                .iter()
                .map(|(path, stamp)| (path.clone(), stamp.kind == 0))
                .collect::<Vec<_>>();
            match session.index(root, &expected) {
                Ok(journal) => inventory.journal = Some(journal),
                Err(error) => {
                    tracing::debug!(%error, "USN index unavailable; retaining metadata scans")
                }
            }
        }
        inventory
    };
    tracing::debug!(
        enumerated_directories = stats.enumerated_directories,
        reused_directories = stats.reused_directories,
        checked_files = stats.checked_files,
        "filesystem inventory captured"
    );
    Ok(inventory)
}

pub(super) fn capture_measured(
    root: &Path,
    previous: Option<&Inventory>,
) -> io::Result<(Inventory, CaptureStats)> {
    let configuration = configuration()?;
    let administration = ancestor_administration(root)?;
    if let Some(previous) = previous {
        if let Some(result) =
            capture_unchanged_directories(root, previous, &configuration, &administration)
        {
            return Ok(result);
        }
    }
    let mut files = Vec::new();
    let mut directories = BTreeMap::new();
    let mut stats = CaptureStats::default();
    let mut pending = vec![(String::new(), false)];
    while let Some((relative, administrative)) = pending.pop() {
        let directory = root.join(&relative);
        let stamp = directory_stamp(&fs::symlink_metadata(&directory)?)?;
        let cached = previous
            .and_then(|before| before.directories.get(&relative))
            .filter(|before| can_reuse(&before.stamp, &stamp));
        directories.insert(
            relative.clone(),
            Directory {
                stamp,
                children: Vec::new(),
            },
        );
        if let Some(cached) = cached {
            stats.reused_directories += 1;
            for &index in &cached.children {
                let (path, before) = &previous.unwrap().files[index];
                if before.kind == 0 {
                    let name = path.rsplit('/').next().unwrap();
                    pending.push((
                        path.clone(),
                        administrative || name == ".svn" || name == "_svn",
                    ));
                    files.push((path.clone(), directory_entry()));
                } else {
                    stats.checked_files += 1;
                    let absolute = root.join(path);
                    files.push((
                        path.clone(),
                        file_stamp(&fs::symlink_metadata(&absolute)?, &absolute)?,
                    ));
                }
            }
        } else {
            stats.enumerated_directories += 1;
            for entry in fs::read_dir(&directory)? {
                let entry = entry?;
                let name = entry.file_name();
                if administrative && (name == "pristine" || name == "tmp") {
                    continue;
                }
                let name = name
                    .to_str()
                    .ok_or_else(|| io::Error::other("non-UTF8 path"))?;
                #[cfg(unix)]
                if name.contains('\\') {
                    return Err(io::Error::other("ambiguous SVN path"));
                }
                let path = if relative.is_empty() {
                    name.to_owned()
                } else {
                    format!("{relative}/{name}")
                };
                let file_type = entry.file_type()?;
                if file_type.is_dir() {
                    pending.push((
                        path.clone(),
                        administrative || name == ".svn" || name == "_svn",
                    ));
                    files.push((path, directory_entry()));
                } else {
                    if file_type.is_symlink() && (name == ".svn" || name == "_svn") {
                        return Err(io::Error::other("linked SVN administration directory"));
                    }
                    stats.checked_files += 1;
                    files.push((path, file_stamp(&entry.metadata()?, &entry.path())?));
                }
            }
        }
        if files.len() > MAX_FILES {
            return Err(io::Error::other("filesystem inventory too large"));
        }
    }
    files.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    for (index, (path, _)) in files.iter().enumerate() {
        let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
        directories
            .get_mut(parent)
            .ok_or_else(|| io::Error::other("incomplete directory inventory"))?
            .children
            .push(index);
    }
    Ok((
        Inventory {
            files,
            directories,
            configuration,
            administration,
            journal: None,
        },
        stats,
    ))
}

fn capture_unchanged_directories(
    root: &Path,
    previous: &Inventory,
    configuration: &BTreeMap<String, u64>,
    administration: &BTreeMap<PathBuf, Stamp>,
) -> Option<(Inventory, CaptureStats)> {
    if previous.configuration != *configuration
        || previous.administration != *administration
        || previous.directories.is_empty()
    {
        return None;
    }
    for (path, directory) in &previous.directories {
        let current = directory_stamp(&fs::symlink_metadata(root.join(path)).ok()?).ok()?;
        if !can_reuse(&directory.stamp, &current) {
            return None;
        }
    }
    // Sorted paths and directory child indices can be retained exactly when
    // all directories are unchanged. File contents are still checked via stat.
    let mut files = Vec::with_capacity(previous.files.len());
    let mut stats = CaptureStats {
        reused_directories: previous.directories.len(),
        ..CaptureStats::default()
    };
    for (path, before) in &previous.files {
        let stamp = if before.kind == 0 {
            directory_entry()
        } else {
            stats.checked_files += 1;
            let absolute = root.join(path);
            file_stamp(&fs::symlink_metadata(&absolute).ok()?, &absolute).ok()?
        };
        if stamp.kind != before.kind {
            return None;
        }
        files.push((path.clone(), stamp));
    }
    Some((
        Inventory {
            files,
            directories: previous.directories.clone(),
            configuration: configuration.clone(),
            administration: administration.clone(),
            journal: None,
        },
        stats,
    ))
}

pub(super) struct Scan {
    pub inventory: Inventory,
    pub paths: Option<Vec<String>>,
    pub force_full: bool,
}

pub(super) fn scan(root: &Path, previous: Option<&Inventory>) -> io::Result<Scan> {
    #[cfg(windows)]
    if let Some(previous) = previous.filter(|previous| previous.journal.is_some()) {
        match capture_usn(root, previous) {
            Ok((inventory, paths)) => {
                tracing::debug!(
                    changed_paths = paths.len(),
                    "USN journal scoped filesystem query"
                );
                return Ok(Scan {
                    inventory,
                    paths: Some(paths),
                    force_full: false,
                });
            }
            Err(error) => {
                tracing::debug!(%error, "USN history unavailable or changes unsafe; full status required");
                return capture(root).map(|inventory| Scan {
                    inventory,
                    paths: None,
                    force_full: true,
                });
            }
        }
    }
    let inventory = capture_incremental(root, previous).or_else(|error| {
        if previous.is_some() {
            tracing::debug!(%error, "directory cache changed during scan; retrying full enumeration");
            capture(root)
        } else { Err(error) }
    })?;
    // Enabling USN after metadata-only caching establishes a new trustworthy
    // baseline; do not reuse status that predates this journal checkpoint.
    let force_full = inventory.journal.is_some()
        && previous
            .and_then(|before| before.journal.as_ref())
            .is_none();
    Ok(Scan {
        inventory,
        paths: None,
        force_full,
    })
}

#[cfg(windows)]
fn capture_usn(root: &Path, previous: &Inventory) -> io::Result<(Inventory, Vec<String>)> {
    if configuration()? != previous.configuration {
        return Err(io::Error::other("SVN configuration changed"));
    }
    if ancestor_administration(root)? != previous.administration {
        return Err(io::Error::other("SVN administrative metadata changed"));
    }
    let delta = super::status_usn::probe(root, previous.journal.as_ref().unwrap())?;
    let mut inventory = previous.clone();
    let mut updates = BTreeMap::new();
    let mut parents = std::collections::BTreeSet::new();
    for path in &delta.paths {
        let absolute = root.join(path);
        let stamp = match fs::symlink_metadata(&absolute) {
            Ok(metadata) => Some(file_stamp(&metadata, &absolute)?),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        parents.insert(path.rsplit_once('/').map_or("", |(parent, _)| parent));
        updates.insert(path.as_str(), stamp);
    }
    if !updates.is_empty() {
        let mut files = Vec::with_capacity(inventory.files.len() + updates.len());
        for (path, before) in &inventory.files {
            match updates.remove(path.as_str()) {
                Some(Some(stamp)) => files.push((path.clone(), stamp)),
                Some(None) => {}
                None => files.push((path.clone(), before.clone())),
            }
        }
        for (path, stamp) in updates {
            if let Some(stamp) = stamp {
                files.push((path.to_owned(), stamp));
            }
        }
        if files.len() > MAX_FILES {
            return Err(io::Error::other("USN inventory too large"));
        }
        files.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        inventory.files = files;
        for parent in parents {
            inventory
                .directories
                .get_mut(parent)
                .ok_or_else(|| io::Error::other("USN parent not in cached directories"))?
                .stamp = directory_stamp(&fs::symlink_metadata(root.join(parent))?)?;
        }
        for directory in inventory.directories.values_mut() {
            directory.children.clear();
        }
        for (index, (path, _)) in inventory.files.iter().enumerate() {
            let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
            inventory
                .directories
                .get_mut(parent)
                .ok_or_else(|| io::Error::other("USN directory structure incomplete"))?
                .children
                .push(index);
        }
    }
    let expected = inventory
        .files
        .iter()
        .map(|(path, stamp)| (path.clone(), stamp.kind == 0))
        .collect::<Vec<_>>();
    if !super::status_usn::valid(&delta.journal, &expected) {
        return Err(io::Error::other(
            "USN identity index inconsistent after changes",
        ));
    }
    inventory.journal = Some(delta.journal);
    Ok((inventory, delta.paths))
}

pub(super) fn checkpoint_changed(before: Option<&Inventory>, after: &Inventory) -> bool {
    after.journal.as_ref().is_some_and(|journal| {
        before
            .and_then(|inventory| inventory.journal.as_ref())
            .is_none_or(|before| before.next != journal.next || before.id != journal.id)
    })
}

pub(super) fn verify(root: &Path, before: &Inventory) -> Option<Inventory> {
    #[cfg(windows)]
    if let Some(journal) = &before.journal {
        if configuration().ok()? != before.configuration {
            return None;
        }
        if ancestor_administration(root).ok()? != before.administration {
            return None;
        }
        let delta = super::status_usn::probe(root, journal).ok()?;
        if !delta.paths.is_empty() {
            return None;
        }
        let mut checked = before.clone();
        checked.journal = Some(delta.journal);
        return Some(checked);
    }
    let after = capture(root).ok()?;
    (before == &after).then_some(after)
}

/// None requests a full scan (SVN metadata or directory structure changed).
pub(super) fn differences(before: &Inventory, after: &Inventory) -> Option<Vec<String>> {
    if before.configuration != after.configuration || before.administration != after.administration
    {
        return None;
    }
    let before = &before.files;
    let after = &after.files;
    let mut old = before.iter().peekable();
    let mut new = after.iter().peekable();
    let mut paths = Vec::new();
    while old.peek().is_some() || new.peek().is_some() {
        let (path, previous, current) = match (old.peek(), new.peek()) {
            (Some((a, _)), Some((b, _))) if a == b => {
                let (path, previous) = old.next().unwrap();
                let (_, current) = new.next().unwrap();
                (path, Some(previous), Some(current))
            }
            (Some((a, _)), Some((b, _))) if a < b => {
                let (path, previous) = old.next().unwrap();
                (path, Some(previous), None)
            }
            (Some(_), None) => {
                let (path, previous) = old.next().unwrap();
                (path, Some(previous), None)
            }
            _ => {
                let (path, current) = new.next().unwrap();
                (path, None, Some(current))
            }
        };
        if previous == current {
            continue;
        }
        if path.split('/').any(|part| part == ".svn" || part == "_svn")
            || previous.is_some_and(|stamp| stamp.kind == 0)
            || current.is_some_and(|stamp| stamp.kind == 0)
        {
            return None;
        }
        paths.push(path.clone());
        if paths.len() > 256 {
            return None;
        }
    }
    Some(paths)
}

#[derive(Serialize, Deserialize)]
pub(super) struct Snapshot {
    version: u32,
    root: PathBuf,
    saved: u64,
    pub inventory: Inventory,
    pub entries: Vec<SvnStatus>,
}
fn location(directory: &Path, root: &Path) -> PathBuf {
    // Deterministic filename; the serialized root also guards hash collisions.
    let hash = root
        .to_string_lossy()
        .bytes()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
    directory.join(format!("status-{hash:016x}.json"))
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn valid_directory_index(inventory: &Inventory) -> bool {
    if !inventory.directories.contains_key("") || inventory.directories.len() > MAX_FILES + 1 {
        return false;
    }
    let mut seen = vec![false; inventory.files.len()];
    for (path, stamp) in &inventory.files {
        if path.is_empty()
            || stamp.kind > 2
            || Path::new(path)
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return false;
        }
        if stamp.kind == 0 && !inventory.directories.contains_key(path) {
            return false;
        }
    }
    for (parent, directory) in &inventory.directories {
        if !parent.is_empty()
            && !inventory
                .files
                .binary_search_by(|(path, _)| path.cmp(parent))
                .ok()
                .is_some_and(|index| inventory.files[index].1.kind == 0)
        {
            return false;
        }
        if directory.children.windows(2).any(|pair| pair[0] >= pair[1]) {
            return false;
        }
        for &index in &directory.children {
            let Some((path, _)) = inventory.files.get(index) else {
                return false;
            };
            if seen[index] || path.rsplit_once('/').map_or("", |(parent, _)| parent) != parent {
                return false;
            }
            seen[index] = true;
        }
    }
    seen.into_iter().all(|item| item)
}

pub(super) fn read(directory: &Path, root: &Path) -> Option<Snapshot> {
    let path = location(directory, root);
    if fs::metadata(&path).ok()?.len() > MAX_BYTES {
        return None;
    }
    let snapshot: Snapshot = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    (snapshot.version == VERSION
        && snapshot.root == root
        && snapshot.inventory.files.len() <= MAX_FILES
        && valid_directory_index(&snapshot.inventory)
        && snapshot.inventory.journal.as_ref().is_none_or(|journal| {
            super::status_usn::valid(
                journal,
                &snapshot
                    .inventory
                    .files
                    .iter()
                    .map(|(path, stamp)| (path.clone(), stamp.kind == 0))
                    .collect::<Vec<_>>(),
            )
        })
        && snapshot
            .inventory
            .files
            .windows(2)
            .all(|pair| pair[0].0 < pair[1].0)
        && now().checked_sub(snapshot.saved)? < 24 * 60 * 60)
        .then_some(snapshot)
}
pub(super) fn write(
    directory: &Path,
    root: &Path,
    inventory: Inventory,
    entries: Vec<SvnStatus>,
) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let snapshot = Snapshot {
        version: VERSION,
        root: root.to_path_buf(),
        saved: now(),
        inventory,
        entries,
    };
    let bytes = serde_json::to_vec(&snapshot)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(io::Error::other("status snapshot too large"));
    }
    let path = location(directory, root);
    let temp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    // Standard rename replaces an existing file on Unix and Windows. Each
    // writer uses its own temporary file, including after cache eviction.
    let saved = fs::write(&temp, bytes).and_then(|_| fs::rename(&temp, &path));
    if saved.is_err() {
        let _ = fs::remove_file(&temp);
    }
    saved?;
    // Bound disk usage to five recent workspaces.
    let mut files: Vec<_> = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("status-") && name.ends_with(".json"))
        })
        .collect();
    files.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());
    let remove = files.len().saturating_sub(5);
    for entry in files.into_iter().take(remove) {
        let _ = fs::remove_file(entry.path());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inventory(count: usize) -> Inventory {
        Inventory {
            files: (0..count)
                .map(|index| {
                    (
                        format!("file-{index:03}"),
                        Stamp {
                            kind: 1,
                            size: 1,
                            modified: 1,
                            changed: 1,
                            link: None,
                        },
                    )
                })
                .collect(),
            directories: BTreeMap::from([(
                String::new(),
                Directory {
                    stamp: DirectoryStamp {
                        modified: 1,
                        changed: 1,
                        device: 0,
                        inode: 0,
                    },
                    children: (0..count).collect(),
                },
            )]),
            configuration: BTreeMap::new(),
            administration: BTreeMap::new(),
            journal: None,
        }
    }
    #[test]
    fn directory_reuse_detects_edits_deletions_moves_and_nested_structure() {
        let root = std::env::temp_dir().join(format!(
            "orcasvn-directory-reuse-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("a")).unwrap();
        fs::create_dir_all(root.join("b")).unwrap();
        fs::write(root.join("a/edit"), "base").unwrap();
        fs::write(root.join("a/delete"), "delete").unwrap();
        fs::write(root.join("b/move"), "move").unwrap();
        // Let directory timestamps leave the conservative one-second window.
        #[cfg(unix)]
        std::thread::sleep(std::time::Duration::from_millis(1100));
        // Exercise metadata directory reuse independently of optional USN
        // checkpoints, which are present on privileged Windows CI runners.
        let (initial, _) = capture_measured(&root, None).unwrap();
        let (unchanged, stats) = capture_measured(&root, Some(&initial)).unwrap();
        assert_eq!(initial, unchanged);
        #[cfg(unix)]
        assert_eq!(stats.enumerated_directories, 0);
        assert_eq!(stats.checked_files, 3);
        fs::write(root.join("a/edit"), "edit").unwrap();
        let (edited, _stats) = capture_measured(&root, Some(&unchanged)).unwrap();
        assert_eq!(edited, capture_measured(&root, None).unwrap().0);
        assert_eq!(
            differences(&unchanged, &edited),
            Some(vec!["a/edit".into()])
        );
        #[cfg(unix)]
        assert_eq!(_stats.enumerated_directories, 0);
        #[cfg(unix)]
        let modified = fs::metadata(root.join("a")).unwrap().modified().unwrap();
        fs::remove_file(root.join("a/delete")).unwrap();
        #[cfg(unix)]
        fs::File::open(root.join("a"))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(modified))
            .unwrap();
        let (deleted, _stats) = capture_measured(&root, Some(&edited)).unwrap();
        assert_eq!(deleted, capture_measured(&root, None).unwrap().0);
        assert_eq!(
            differences(&edited, &deleted),
            Some(vec!["a/delete".into()])
        );
        #[cfg(unix)]
        assert_eq!(_stats.enumerated_directories, 1);
        fs::rename(root.join("b/move"), root.join("a/moved")).unwrap();
        let (moved, _stats) = capture_measured(&root, Some(&deleted)).unwrap();
        assert_eq!(moved, capture_measured(&root, None).unwrap().0);
        assert_eq!(
            differences(&deleted, &moved),
            Some(vec!["a/moved".into(), "b/move".into()])
        );
        #[cfg(unix)]
        assert_eq!(_stats.enumerated_directories, 2);
        fs::create_dir(root.join("b/new-dir")).unwrap();
        fs::write(root.join("b/new-dir/new"), "new").unwrap();
        let (nested, _) = capture_measured(&root, Some(&moved)).unwrap();
        assert_eq!(nested, capture_measured(&root, None).unwrap().0);
        assert!(differences(&moved, &nested).is_none());
        fs::remove_dir_all(root.join("a")).unwrap();
        let (removed, _) = capture_measured(&root, Some(&nested)).unwrap();
        assert_eq!(removed, capture_measured(&root, None).unwrap().0);
        assert!(differences(&nested, &removed).is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_directory_indices_and_coarse_or_replaced_directories_are_rejected() {
        let mut cached = inventory(1);
        assert!(valid_directory_index(&cached));
        cached.directories.get_mut("").unwrap().children[0] = 100;
        assert!(!valid_directory_index(&cached));
        let stamp = DirectoryStamp {
            modified: 1,
            changed: 0,
            device: 1,
            inode: 1,
        };
        assert!(!can_reuse(&stamp, &stamp));
        let high_resolution = DirectoryStamp {
            changed: 1,
            ..stamp
        };
        let replaced = DirectoryStamp {
            inode: 2,
            ..high_resolution.clone()
        };
        assert!(!can_reuse(&high_resolution, &replaced));
    }

    #[test]
    fn concurrent_snapshot_writers_keep_inventory_and_status_paired() {
        let root = std::env::temp_dir().join(format!(
            "orcasvn-snapshot-writers-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let directory = root.join("cache");
        fs::create_dir_all(&directory).unwrap();
        let threads: Vec<_> = (0..8)
            .map(|value| {
                let root = root.clone();
                let directory = directory.clone();
                std::thread::spawn(move || {
                    let mut files = inventory(1);
                    files.files[0].1.modified = value;
                    let entries = vec![SvnStatus {
                        path: "file-000".into(),
                        status: "modified".into(),
                        status_code: value.to_string(),
                        prop_status: "none".into(),
                        locked: false,
                        history: false,
                        switched: false,
                    }];
                    write(&directory, &root, files, entries).unwrap();
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let snapshot = read(&directory, &root).unwrap();
        assert_eq!(
            snapshot.inventory.files[0].1.modified.to_string(),
            snapshot.entries[0].status_code
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn temporary_svn_files_are_skipped_and_metadata_files_invalidate_cache() {
        let root = std::env::temp_dir().join(format!(
            "orcasvn-inventory-metadata-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join(".svn/tmp")).unwrap();
        fs::create_dir_all(root.join(".svn/pristine/aa")).unwrap();
        fs::create_dir_all(root.join("src/tmp")).unwrap();
        fs::write(root.join(".svn/wc.db"), "baseline").unwrap();
        let before = capture(&root).unwrap();
        fs::write(root.join(".svn/tmp/scratch"), "temporary").unwrap();
        fs::write(root.join(".svn/pristine/aa/base"), "pristine").unwrap();
        let temporary = capture(&root).unwrap();
        assert_eq!(differences(&before, &temporary), Some(vec![]));
        fs::write(root.join("src/tmp/file"), "normal file").unwrap();
        let normal = capture(&root).unwrap();
        assert_eq!(
            differences(&temporary, &normal),
            Some(vec!["src/tmp/file".into()])
        );
        fs::write(root.join(".svn/wc.db"), "metadata update").unwrap();
        assert!(differences(&normal, &capture(&root).unwrap()).is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn configuration_change_and_large_scope_require_complete_scan() {
        let before = inventory(0);
        assert_eq!(differences(&before, &inventory(256)).unwrap().len(), 256);
        assert!(differences(&before, &inventory(257)).is_none());
        let mut changed_configuration = inventory(0);
        changed_configuration
            .configuration
            .insert("config".into(), 1);
        assert!(differences(&before, &changed_configuration).is_none());
    }
}
