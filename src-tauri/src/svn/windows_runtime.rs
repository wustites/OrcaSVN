use std::fs;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use zip::ZipArchive;

use super::executor::SvnError;

const ARCHIVE: &[u8] = include_bytes!(env!("ORCASVN_RUNTIME_ARCHIVE"));
static RUNTIME: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

pub(super) fn executable() -> Result<PathBuf, SvnError> {
    let mut cached = RUNTIME
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| SvnError::CommandFailed("SVN runtime lock is poisoned".into()))?;
    if let Some(path) = cached.as_ref() {
        return Ok(path.clone());
    }
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("OrcaSVN")
        .join("svn-runtime");
    let path = prepare(&base)
        .map_err(|error| SvnError::CommandFailed(format!("无法准备内置 UTF-8 SVN：{error}")))?;
    *cached = Some(path.clone());
    Ok(path)
}

fn prepare(base: &Path) -> io::Result<PathBuf> {
    let identity = format!("{:x}", Sha256::digest(ARCHIVE));
    let stable = base.join(&identity);
    let mut archive = ZipArchive::new(Cursor::new(ARCHIVE))?;
    if matches_archive(&stable, &mut archive)? {
        return Ok(stable.join("bin/svn.exe"));
    }
    fs::create_dir_all(base)?;
    let stage = base.join(format!(
        ".{identity}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir(&stage)?;
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(stage.clone());
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let relative = file
            .enclosed_name()
            .ok_or_else(|| io::Error::other("unsafe runtime path"))?;
        if file.is_dir() {
            continue;
        }
        let target = stage.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::File::create(target)?;
        io::copy(&mut file, &mut output)?;
    }
    if !matches_archive(&stage, &mut archive)? {
        return Err(io::Error::other("SVN runtime verification failed"));
    }
    match fs::rename(&stage, &stable) {
        Ok(()) => Ok(stable.join("bin/svn.exe")),
        Err(_) if matches_archive(&stable, &mut archive)? => Ok(stable.join("bin/svn.exe")),
        Err(_) => {
            // A damaged or in-use old cache must not prevent recovery. Leave
            // that directory untouched and publish the verified private copy.
            let recovered =
                stage.with_file_name(format!("{identity}-{}-recovered", std::process::id()));
            fs::rename(&stage, &recovered)?;
            Ok(recovered.join("bin/svn.exe"))
        }
    }
}

fn matches_archive(directory: &Path, archive: &mut ZipArchive<Cursor<&[u8]>>) -> io::Result<bool> {
    if !directory.join("bin/svn.exe").is_file() {
        return Ok(false);
    }
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let relative = file
            .enclosed_name()
            .ok_or_else(|| io::Error::other("unsafe runtime path"))?;
        if file.is_dir() {
            continue;
        }
        let expected_path = directory.join(relative);
        let actual = match fs::read(expected_path) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(false),
        };
        let mut expected = Vec::new();
        file.read_to_end(&mut expected)?;
        if Sha256::digest(&actual) != Sha256::digest(&expected) {
            return Ok(false);
        }
    }
    // Refuse additional binaries: Windows DLL lookup must see only this build.
    let mut actual_names: Vec<_> = fs::read_dir(directory.join("bin"))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<io::Result<_>>()?;
    let mut expected_names = Vec::new();
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        if let Some(path) = file.enclosed_name() {
            if path.parent() == Some(Path::new("bin")) && !file.is_dir() {
                expected_names.push(path.file_name().unwrap().to_owned());
            }
        }
    }
    actual_names.sort();
    expected_names.sort();
    Ok(actual_names == expected_names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_is_reused_and_corrupt_cache_recovers_without_overwriting_it() {
        let root = std::env::temp_dir().join(format!(
            "orcasvn-runtime-test-中文🐋-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let first = prepare(&root).unwrap();
        assert_eq!(prepare(&root).unwrap(), first);
        let version = std::process::Command::new(&first)
            .args(["--version", "--quiet"])
            .output()
            .unwrap();
        assert!(version.status.success(), "{version:?}");
        assert_eq!(
            String::from_utf8_lossy(&version.stdout).trim(),
            "1.14.5-SlikSvn"
        );
        fs::write(&first, b"damaged").unwrap();
        let second = prepare(&root).unwrap();
        assert_ne!(first, second);
        assert_eq!(fs::read(first).unwrap(), b"damaged");
        assert!(fs::metadata(second).unwrap().len() > 100_000);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn concurrent_process_preparations_publish_one_verified_runtime() {
        let root = std::env::temp_dir().join(format!(
            "orcasvn-runtime-race-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let root = root.clone();
                std::thread::spawn(move || prepare(&root).unwrap())
            })
            .collect();
        let paths: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert!(paths.iter().all(|path| path == &paths[0]));
        fs::remove_dir_all(root).unwrap();
    }
}
