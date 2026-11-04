//! Ordinary directory permissions suffice; this is an in-process fallback for
//! volumes whose USN journal cannot be read. Offline changes still require the
//! existing snapshot comparison, and lost notifications always request a scan.
use std::collections::BTreeSet;
use std::io;
use std::path::{Component, Path};

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn paths(bytes: &[u8]) -> io::Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    let mut offset = 0usize;
    loop {
        let record = bytes
            .get(offset..)
            .ok_or_else(|| invalid("notification offset"))?;
        if record.len() < 12 {
            return Err(invalid("truncated directory notification"));
        }
        let next = u32::from_le_bytes(record[0..4].try_into().unwrap()) as usize;
        let action = u32::from_le_bytes(record[4..8].try_into().unwrap());
        let length = u32::from_le_bytes(record[8..12].try_into().unwrap()) as usize;
        if !(1..=5).contains(&action) || length == 0 || length % 2 != 0 {
            return Err(invalid("invalid directory notification"));
        }
        let end = 12usize
            .checked_add(length)
            .ok_or_else(|| invalid("name overflow"))?;
        let name = record
            .get(12..end)
            .ok_or_else(|| invalid("truncated notification name"))?;
        let utf16: Vec<_> = name
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let name = String::from_utf16(&utf16)
            .map_err(|_| invalid("invalid notification UTF-16"))?
            .replace('\\', "/");
        if name.contains('\0')
            || name.contains(':')
            || Path::new(&name)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(invalid("unsafe notification path"));
        }
        // These trees cannot affect SVN status. Keep all other administration
        // changes: wc.db/property/ignore changes require complete verification.
        if !super::status_usn::excluded(&name) {
            paths.insert(name);
            if paths.len() > 256 {
                return Err(invalid("directory notifications exceed scoped limit"));
            }
        }
        if next == 0 {
            break;
        }
        if next < end || next % 4 != 0 || next >= record.len() {
            return Err(invalid("invalid next notification offset"));
        }
        offset = offset
            .checked_add(next)
            .ok_or_else(|| invalid("notification offset overflow"))?;
    }
    Ok(paths.into_iter().collect())
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::cell::UnsafeCell;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{PathBuf, Prefix};
    use std::ptr::{null, null_mut};
    use std::sync::mpsc;
    use std::thread::JoinHandle;
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::*;
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_FIXED;
    use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

    const BUFFER_WORDS: usize = 16 * 1024;
    const BUFFER_BYTES: usize = BUFFER_WORDS * 4;

    fn open(root: &Path, access: u32, flags: u32) -> io::Result<HANDLE> {
        let name: Vec<_> = root.as_os_str().encode_wide().chain(Some(0)).collect();
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                flags | FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(handle)
        }
    }

    fn identity(handle: HANDLE) -> io::Result<(u32, u32, u32)> {
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        if unsafe { GetFileInformationByHandle(handle, &mut info) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(invalid("reparse point at watched root"));
        }
        Ok((
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
        ))
    }

    /// Remote and removable volumes can miss or delay notifications, for example
    /// another SMB client's edit, so they keep the per-poll metadata comparison.
    /// Canonical paths resolve mapped and `subst` drives before this check.
    pub(in crate::svn) fn supports_notifications(root: &Path) -> bool {
        let Some(Component::Prefix(prefix)) = root.components().next() else {
            return false;
        };
        let (Prefix::Disk(drive) | Prefix::VerbatimDisk(drive)) = prefix.kind() else {
            return false;
        };
        let name = [drive as u16, b':' as u16, b'\\' as u16, 0];
        unsafe { GetDriveTypeW(name.as_ptr()) == DRIVE_FIXED }
    }

    type PollReply = mpsc::Sender<io::Result<Vec<String>>>;

    /// Windows cancels a thread's pending I/O when that thread exits, so the
    /// request must not be issued from Tokio's short-lived blocking pool. One
    /// dedicated thread issues and polls it for the lifetime of the watcher.
    pub(in crate::svn) struct Watcher {
        commands: Option<mpsc::Sender<(PathBuf, PollReply)>>,
        thread: Option<JoinHandle<()>>,
    }

    impl Watcher {
        pub(in crate::svn) fn new(root: &Path) -> io::Result<Self> {
            let (commands, requests) = mpsc::channel::<(PathBuf, PollReply)>();
            let (started, ready) = mpsc::channel();
            let watch_root = root.to_path_buf();
            let thread = std::thread::Builder::new()
                .name("orcasvn-status-watch".into())
                .spawn(move || {
                    let mut request = match Request::new(&watch_root) {
                        Ok(request) => {
                            let _ = started.send(Ok(()));
                            request
                        }
                        Err(error) => {
                            let _ = started.send(Err(error));
                            return;
                        }
                    };
                    while let Ok((root, reply)) = requests.recv() {
                        let _ = reply.send(request.poll(&root));
                    }
                })?;
            let watcher = Self {
                commands: Some(commands),
                thread: Some(thread),
            };
            ready
                .recv()
                .map_err(|_| io::Error::other("directory notification thread stopped"))??;
            Ok(watcher)
        }

        pub(in crate::svn) fn poll(&mut self, root: &Path) -> io::Result<Vec<String>> {
            let stopped = || io::Error::other("directory notification thread stopped");
            let (reply, response) = mpsc::channel();
            self.commands
                .as_ref()
                .ok_or_else(stopped)?
                .send((root.to_path_buf(), reply))
                .map_err(|_| stopped())?;
            response.recv().map_err(|_| stopped())?
        }
    }

    impl Drop for Watcher {
        fn drop(&mut self) {
            // Closing the channel ends the thread, which cancels the request and
            // waits for the kernel to release its buffers.
            self.commands = None;
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    struct Request {
        handle: HANDLE,
        identity: (u32, u32, u32),
        // Stable, aligned allocations: Windows owns these addresses until the
        // overlapped operation completes, even if the Watcher is moved.
        // UnsafeCell permits asynchronous kernel writes while the owner is
        // polled; no Rust references to the contents exist during pending I/O.
        buffer: Box<UnsafeCell<[u32; BUFFER_WORDS]>>,
        overlapped: Box<UnsafeCell<OVERLAPPED>>,
        pending: bool,
    }

    impl Request {
        fn new(root: &Path) -> io::Result<Self> {
            let handle = open(root, FILE_LIST_DIRECTORY, FILE_FLAG_OVERLAPPED)?;
            let mut request = Self {
                handle,
                identity: (0, 0, 0),
                buffer: Box::new(UnsafeCell::new([0; BUFFER_WORDS])),
                overlapped: Box::new(UnsafeCell::new(OVERLAPPED::default())),
                pending: false,
            };
            request.identity = identity(handle)?;
            request.arm()?;
            Ok(request)
        }

        fn arm(&mut self) -> io::Result<()> {
            // The previous request has completed before this allocation is
            // reset or reused (including error/cancellation paths).
            unsafe {
                *self.overlapped.get() = OVERLAPPED::default();
            }
            let success = unsafe {
                ReadDirectoryChangesW(
                    self.handle,
                    self.buffer.get().cast(),
                    BUFFER_BYTES as u32,
                    1,
                    FILE_NOTIFY_CHANGE_FILE_NAME
                        | FILE_NOTIFY_CHANGE_DIR_NAME
                        | FILE_NOTIFY_CHANGE_ATTRIBUTES
                        | FILE_NOTIFY_CHANGE_SIZE
                        | FILE_NOTIFY_CHANGE_LAST_WRITE
                        | FILE_NOTIFY_CHANGE_CREATION
                        | FILE_NOTIFY_CHANGE_SECURITY,
                    null_mut(),
                    self.overlapped.get(),
                    None,
                )
            };
            if success == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
                    return Err(error);
                }
            }
            self.pending = true;
            Ok(())
        }

        fn poll(&mut self, root: &Path) -> io::Result<Vec<String>> {
            // A watched directory can be moved/replaced without a child event.
            let current = open(root, FILE_READ_ATTRIBUTES, 0)?;
            let current_identity = identity(current);
            unsafe {
                CloseHandle(current);
            }
            if current_identity? != self.identity {
                return Err(invalid("watched root replaced"));
            }
            let mut all = BTreeSet::new();
            // Bound work on a busy tree. On failure the caller discards this
            // watcher, starts a new one, and establishes a fresh full baseline.
            for _ in 0..16 {
                let mut count = 0;
                if unsafe { GetOverlappedResult(self.handle, self.overlapped.get(), &mut count, 0) }
                    == 0
                {
                    let error = io::Error::last_os_error();
                    if error.raw_os_error() == Some(ERROR_IO_INCOMPLETE as i32) {
                        return Ok(all.into_iter().collect());
                    }
                    self.pending = false;
                    return Err(error);
                }
                self.pending = false;
                if count == 0 || count as usize > BUFFER_BYTES {
                    return Err(invalid("directory notification buffer overflow"));
                }
                let bytes =
                    unsafe { std::slice::from_raw_parts(self.buffer.get().cast(), count as usize) };
                let changed = paths(bytes)?;
                // Re-arm before metadata/SVN work. Windows queues notifications
                // on this same directory handle between consecutive requests.
                self.arm()?;
                all.extend(changed);
                if all.len() > 256 {
                    return Err(invalid("too many directory changes"));
                }
            }
            Err(invalid("directory notification drain limit exceeded"))
        }
    }

    impl Drop for Request {
        fn drop(&mut self) {
            unsafe {
                if self.pending {
                    CancelIoEx(self.handle, self.overlapped.get());
                    let mut count = 0;
                    // Cancellation alone does not release the kernel's buffer
                    // references. Wait for completion before freeing memory.
                    GetOverlappedResult(self.handle, self.overlapped.get(), &mut count, 1);
                }
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(windows)]
pub(super) use native::{supports_notifications, Watcher};

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, action: u32) -> Vec<u8> {
        let name: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let mut bytes = Vec::new();
        bytes.extend(0u32.to_le_bytes());
        bytes.extend(action.to_le_bytes());
        bytes.extend((name.len() as u32).to_le_bytes());
        bytes.extend(name);
        bytes
    }

    #[test]
    fn notifications_keep_both_rename_paths_and_ignore_only_pristine_and_tmp() {
        let mut first = record("src\\旧名.txt", 4);
        while first.len() % 4 != 0 {
            first.push(0);
        }
        let next = first.len() as u32;
        first[..4].copy_from_slice(&next.to_le_bytes());
        first.extend(record("src\\新名.txt", 5));
        assert_eq!(paths(&first).unwrap(), ["src/新名.txt", "src/旧名.txt"]);
        assert!(paths(&record(".svn\\pristine\\a", 3)).unwrap().is_empty());
        assert!(paths(&record(".svn\\tmp\\a", 3)).unwrap().is_empty());
        assert_eq!(
            paths(&record("src\\pristine\\a", 3)).unwrap(),
            ["src/pristine/a"]
        );
        assert_eq!(paths(&record(".svn\\wc.db", 3)).unwrap(), [".svn/wc.db"]);
    }

    #[test]
    fn malformed_or_unsafe_notifications_cannot_reuse_status() {
        for bytes in [
            vec![],
            vec![0; 12],
            record("../escape", 3),
            record("C:\\escape", 3),
            record("/escape", 3),
            record("file", 9),
        ] {
            assert!(paths(&bytes).is_err());
        }
        let mut truncated = record("file", 3);
        truncated.pop();
        assert!(paths(&truncated).is_err());
    }

    #[test]
    fn notification_batches_enforce_the_scoped_limit() {
        let mut bytes = Vec::new();
        for index in 0..257 {
            let mut next = record(&format!("file-{index}"), 3);
            if index < 256 {
                while next.len() % 4 != 0 {
                    next.push(0);
                }
                let length = next.len() as u32;
                next[..4].copy_from_slice(&length.to_le_bytes());
            }
            bytes.extend(next);
        }
        assert!(paths(&bytes).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn native_notifications_detect_edits_rename_delete_and_root_replacement() {
        use std::fs;
        use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

        let base = std::env::temp_dir().join(format!(
            "orcasvn-watch-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(base.join("wc")).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        let root = fs::canonicalize(base.join("wc")).unwrap();
        fs::write(root.join("file"), "base\n").unwrap();
        assert!(supports_notifications(&root));
        assert!(!supports_notifications(Path::new(r"\\server\share\wc")));
        // The issuing thread exits immediately, as a Tokio blocking thread does
        // when idle. Its exit must not cancel the outstanding request.
        let mut watcher = std::thread::spawn({
            let root = root.clone();
            move || Watcher::new(&root).expect("directory notifications need no volume privileges")
        })
        .join()
        .unwrap();
        assert!(watcher.poll(&root).unwrap().is_empty());

        fn wait(watcher: &mut Watcher, root: &Path, expected: &[&str]) {
            let started = Instant::now();
            let mut seen = BTreeSet::new();
            loop {
                seen.extend(watcher.poll(root).unwrap());
                if expected.iter().all(|path| seen.contains(*path)) {
                    return;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(3),
                    "missing events: {seen:?}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        fs::write(root.join("file"), "edit\n").unwrap();
        wait(&mut watcher, &root, &["file"]);
        fs::rename(root.join("file"), root.join("renamed")).unwrap();
        wait(&mut watcher, &root, &["file", "renamed"]);
        fs::remove_file(root.join("renamed")).unwrap();
        wait(&mut watcher, &root, &["renamed"]);
        fs::rename(&root, base.join("moved")).unwrap();
        fs::create_dir(&root).unwrap();
        assert!(watcher.poll(&root).is_err());
        // Drop cancels and joins an outstanding request before freeing buffers.
        drop(watcher);
    }
}
