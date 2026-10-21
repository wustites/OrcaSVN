//! NTFS journal records are candidates; SVN remains the authority for status.
#![cfg_attr(not(windows), allow(dead_code))]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io;

const DIRECTORY: u32 = 0x10;
const REPARSE: u32 = 0x400;
const CLOSE: u32 = 0x8000_0000;
const CREATE: u32 = 0x100;
const DELETE: u32 = 0x200;
const OLD_NAME: u32 = 0x1000;
const NEW_NAME: u32 = 0x2000;
const STRUCTURAL: u32 = CREATE | DELETE | OLD_NAME | NEW_NAME | 0x10000 | 0x100000;
const MAX_PATHS: usize = 256;
const MAX_NODES: usize = 500_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Journal {
    pub volume: String,
    pub serial: u32,
    pub id: u64,
    pub next: i64,
    pub root_id: u64,
    pub directories: BTreeMap<u64, String>,
    pub files: BTreeMap<u64, Vec<String>>,
}

#[derive(Debug)]
struct Record {
    file: u64,
    parent: u64,
    usn: i64,
    reason: u32,
    attributes: u32,
    name: String,
}

pub(super) struct Delta {
    pub journal: Journal,
    pub paths: Vec<String>,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn field<const N: usize>(buffer: &[u8], offset: usize) -> io::Result<[u8; N]> {
    buffer
        .get(offset..offset + N)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| invalid("truncated Windows record"))
}
fn name(buffer: &[u8], offset: usize, length: usize) -> io::Result<String> {
    if length == 0 || length % 2 != 0 || offset % 2 != 0 {
        return Err(invalid("invalid UTF-16 record name"));
    }
    let bytes = buffer
        .get(
            offset
                ..offset
                    .checked_add(length)
                    .ok_or_else(|| invalid("name overflow"))?,
        )
        .ok_or_else(|| invalid("truncated record name"))?;
    let units: Vec<_> = bytes
        .chunks_exact(2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect();
    let name = String::from_utf16(&units).map_err(|_| invalid("non-Unicode Windows filename"))?;
    if name.contains(['/', '\\', '\0', ':']) {
        return Err(invalid("unsafe Windows filename"));
    }
    Ok(name)
}

fn parse_page(buffer: &[u8]) -> io::Result<(i64, Vec<Record>)> {
    let next = i64::from_le_bytes(field(buffer, 0)?);
    let mut offset = 8;
    let mut records = Vec::new();
    while offset < buffer.len() {
        let length = u32::from_le_bytes(field(buffer, offset)?) as usize;
        let end = offset
            .checked_add(length)
            .ok_or_else(|| invalid("USN record overflow"))?;
        if length < 60 || length % 8 != 0 || end > buffer.len() {
            return Err(invalid("invalid USN record length"));
        }
        let record = &buffer[offset..end];
        if u16::from_le_bytes(field(record, 4)?) != 2 {
            return Err(invalid("unsupported USN record version"));
        }
        let name_length = u16::from_le_bytes(field(record, 56)?) as usize;
        let name_offset = u16::from_le_bytes(field(record, 58)?) as usize;
        if name_offset < 60 {
            return Err(invalid("invalid USN name offset"));
        }
        records.push(Record {
            file: u64::from_le_bytes(field(record, 8)?),
            parent: u64::from_le_bytes(field(record, 16)?),
            usn: i64::from_le_bytes(field(record, 24)?),
            reason: u32::from_le_bytes(field(record, 40)?),
            attributes: u32::from_le_bytes(field(record, 52)?),
            name: name(record, name_offset, name_length)?,
        });
        offset = end;
    }
    Ok((next, records))
}

fn admin(path: &str) -> bool {
    path.split('/')
        .any(|part| part.eq_ignore_ascii_case(".svn") || part.eq_ignore_ascii_case("_svn"))
}
fn excluded(path: &str) -> bool {
    let mut parts = path.split('/');
    while let Some(part) = parts.next() {
        if part.eq_ignore_ascii_case(".svn") || part.eq_ignore_ascii_case("_svn") {
            return parts.next().is_some_and(|part| {
                part.eq_ignore_ascii_case("tmp") || part.eq_ignore_ascii_case("pristine")
            });
        }
    }
    false
}

fn apply_record(
    journal: &mut Journal,
    record: Record,
    paths: &mut BTreeSet<String>,
) -> io::Result<()> {
    if record.reason & !CLOSE == 0 {
        return Ok(());
    }
    let parent = journal.directories.get(&record.parent);
    let path = parent.map(|parent| {
        if parent.is_empty() {
            record.name.clone()
        } else {
            format!("{parent}/{}", record.name)
        }
    });
    if path.as_deref().is_some_and(excluded) {
        return Ok(());
    }
    if record.file == journal.root_id && record.reason & STRUCTURAL != 0 {
        return Err(invalid("working-copy root moved"));
    }
    if record.attributes & DIRECTORY != 0 || journal.directories.contains_key(&record.file) {
        if (parent.is_some() || journal.directories.contains_key(&record.file))
            && record.reason & STRUCTURAL != 0
        {
            return Err(invalid("working-copy directory structure changed"));
        }
        // Directory timestamps/attributes alone do not alter SVN properties.
        return Ok(());
    }
    if record.attributes & REPARSE != 0
        && (path.is_some() || journal.files.contains_key(&record.file))
    {
        return Err(invalid("reparse point requires filesystem scan"));
    }
    if let Some(aliases) = journal.files.get(&record.file) {
        for alias in aliases {
            if admin(alias) {
                return Err(invalid("SVN administrative metadata changed"));
            }
            paths.insert(alias.clone());
        }
    }
    if let Some(path) = path {
        if admin(&path) {
            return Err(invalid("SVN administrative metadata changed"));
        }
        // A data/attribute record may name a DOS short alias. Existing file IDs
        // already identify all indexed paths; only structural records add names.
        if record.reason & STRUCTURAL == 0 && journal.files.contains_key(&record.file) {
            if paths.len() > MAX_PATHS {
                return Err(invalid("journal changes exceed scoped limit"));
            }
            return Ok(());
        }
        paths.insert(path.clone());
        let aliases = journal.files.entry(record.file).or_default();
        if record.reason & DELETE != 0
            || (record.reason & OLD_NAME != 0 && record.reason & NEW_NAME == 0)
        {
            aliases.retain(|alias| alias != &path);
        } else if !aliases.contains(&path) {
            aliases.push(path);
            aliases.sort();
        }
        if aliases.is_empty() {
            journal.files.remove(&record.file);
        }
    }
    if paths.len() > MAX_PATHS || journal.files.len() > MAX_NODES {
        return Err(invalid("journal changes exceed scoped limit"));
    }
    Ok(())
}

fn continuous(journal: &Journal, id: u64, first: i64, lowest: i64, next: i64) -> bool {
    journal.id == id
        && journal.next >= first
        && journal.next >= lowest
        && journal.next <= next
        && journal.next >= 0
}

pub(super) fn valid(journal: &Journal, expected: &[(String, bool)]) -> bool {
    if journal.next < 0
        || journal.volume.is_empty()
        || journal
            .directories
            .get(&journal.root_id)
            .is_none_or(|path| !path.is_empty())
    {
        return false;
    }
    let mut actual = BTreeMap::new();
    for path in journal.directories.values().filter(|path| !path.is_empty()) {
        if actual.insert(path.as_str(), true).is_some() {
            return false;
        }
    }
    for aliases in journal.files.values() {
        if aliases.is_empty() {
            return false;
        }
        for path in aliases {
            if actual.insert(path.as_str(), false).is_some() {
                return false;
            }
        }
    }
    actual.len() == expected.len()
        && expected
            .iter()
            .all(|(path, directory)| actual.get(path.as_str()) == Some(directory))
}

#[cfg(windows)]
pub(super) use native::{begin, probe};

#[cfg(windows)]
mod native {
    use super::*;
    use std::ffi::OsStr;
    use std::mem::{offset_of, size_of};
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr::{null, null_mut};
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_NO_MORE_FILES, GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::*;
    use windows_sys::Win32::System::Ioctl::{
        FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V0,
        USN_JOURNAL_DATA_V0,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;

    #[repr(C, align(8))]
    #[derive(Clone)]
    struct AlignedWord(u64);
    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(Some(0)).collect()
    }
    fn open(path: &OsStr, access: u32, flags: u32) -> io::Result<Handle> {
        let path = wide(path);
        // No privilege adjustment or elevation: access denial simply falls back.
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                access,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                flags,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Handle(handle))
        }
    }
    fn info(handle: &Handle) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        if unsafe { GetFileInformationByHandle(handle.0, &mut info) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(invalid("reparse point in workspace"));
        }
        Ok(info)
    }
    fn file_id(info: &BY_HANDLE_FILE_INFORMATION) -> u64 {
        (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow)
    }
    fn query(handle: &Handle) -> io::Result<USN_JOURNAL_DATA_V0> {
        let mut data = USN_JOURNAL_DATA_V0::default();
        let mut length = 0;
        if unsafe {
            DeviceIoControl(
                handle.0,
                FSCTL_QUERY_USN_JOURNAL,
                null(),
                0,
                (&mut data as *mut USN_JOURNAL_DATA_V0).cast(),
                size_of::<USN_JOURNAL_DATA_V0>() as u32,
                &mut length,
                null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if length < size_of::<USN_JOURNAL_DATA_V0>() as u32 {
            return Err(invalid("truncated journal identity"));
        }
        Ok(data)
    }
    pub(in crate::svn) struct Session {
        handle: Handle,
        journal: Journal,
    }
    pub(in crate::svn) fn begin(root: &Path) -> io::Result<Session> {
        let root_handle = open(
            root.as_os_str(),
            FILE_READ_ATTRIBUTES,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
        )?;
        let root_info = info(&root_handle)?;
        let root_wide = wide(root.as_os_str());
        let mut mount = vec![0u16; 32768];
        if unsafe { GetVolumePathNameW(root_wide.as_ptr(), mount.as_mut_ptr(), mount.len() as u32) }
            == 0
        {
            return Err(io::Error::last_os_error());
        }
        let mut filesystem = [0u16; 32];
        if unsafe {
            GetVolumeInformationW(
                mount.as_ptr(),
                null_mut(),
                0,
                null_mut(),
                null_mut(),
                null_mut(),
                filesystem.as_mut_ptr(),
                filesystem.len() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let filesystem = String::from_utf16(
            &filesystem[..filesystem
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(filesystem.len())],
        )
        .map_err(|_| invalid("invalid filesystem name"))?;
        if filesystem != "NTFS" {
            return Err(invalid("USN optimization requires local NTFS"));
        }
        let mut volume = [0u16; 128];
        if unsafe {
            GetVolumeNameForVolumeMountPointW(
                mount.as_ptr(),
                volume.as_mut_ptr(),
                volume.len() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let volume = String::from_utf16(
            &volume[..volume.iter().position(|&c| c == 0).unwrap_or(volume.len())],
        )
        .map_err(|_| invalid("invalid volume name"))?
        .trim_end_matches('\\')
        .to_owned();
        let handle = open(OsStr::new(&volume), GENERIC_READ, 0)?;
        let data = query(&handle)?;
        let root_id = file_id(&root_info);
        Ok(Session {
            handle,
            journal: Journal {
                volume,
                serial: root_info.dwVolumeSerialNumber,
                id: data.UsnJournalID,
                next: data.NextUsn,
                root_id,
                directories: BTreeMap::from([(root_id, String::new())]),
                files: BTreeMap::new(),
            },
        })
    }
    impl Session {
        pub(in crate::svn) fn index(
            mut self,
            root: &Path,
            expected: &[(String, bool)],
        ) -> io::Result<Journal> {
            let mut pending = vec![String::new()];
            let mut count = 0;
            while let Some(parent) = pending.pop() {
                let handle = open(
                    root.join(&parent).as_os_str(),
                    FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES,
                    FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                )?;
                let metadata = info(&handle)?;
                if self.journal.directories.get(&file_id(&metadata)) != Some(&parent)
                    || metadata.dwVolumeSerialNumber != self.journal.serial
                {
                    return Err(invalid(
                        "directory identity changed during journal indexing",
                    ));
                }
                // FILE_ID_BOTH_DIR_INFO needs 8-byte alignment. No per-file opens.
                let mut storage = vec![AlignedWord(0); 8192];
                loop {
                    if unsafe {
                        GetFileInformationByHandleEx(
                            handle.0,
                            FileIdBothDirectoryInfo,
                            storage.as_mut_ptr().cast(),
                            (storage.len() * 8) as u32,
                        )
                    } == 0
                    {
                        let error = io::Error::last_os_error();
                        if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                            break;
                        }
                        return Err(error);
                    }
                    let bytes = unsafe {
                        std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), storage.len() * 8)
                    };
                    let mut offset = 0;
                    loop {
                        let header = bytes
                            .get(offset..)
                            .ok_or_else(|| invalid("directory buffer overflow"))?;
                        if header.len() < offset_of!(FILE_ID_BOTH_DIR_INFO, FileName) {
                            return Err(invalid("truncated directory entry"));
                        }
                        let next = u32::from_le_bytes(field(
                            header,
                            offset_of!(FILE_ID_BOTH_DIR_INFO, NextEntryOffset),
                        )?) as usize;
                        let length = u32::from_le_bytes(field(
                            header,
                            offset_of!(FILE_ID_BOTH_DIR_INFO, FileNameLength),
                        )?) as usize;
                        let entry_length = if next == 0 { header.len() } else { next };
                        let entry = header
                            .get(..entry_length)
                            .ok_or_else(|| invalid("directory entry overflow"))?;
                        let name =
                            name(entry, offset_of!(FILE_ID_BOTH_DIR_INFO, FileName), length)?;
                        if name != "." && name != ".." {
                            let path = if parent.is_empty() {
                                name
                            } else {
                                format!("{parent}/{name}")
                            };
                            if !excluded(&path) {
                                let attributes = u32::from_le_bytes(field(
                                    entry,
                                    offset_of!(FILE_ID_BOTH_DIR_INFO, FileAttributes),
                                )?);
                                if attributes & REPARSE != 0 {
                                    return Err(invalid(
                                        "reparse point requires metadata fallback",
                                    ));
                                }
                                let id = u64::from_le_bytes(field(
                                    entry,
                                    offset_of!(FILE_ID_BOTH_DIR_INFO, FileId),
                                )?);
                                if attributes & DIRECTORY != 0 {
                                    if self.journal.directories.insert(id, path.clone()).is_some() {
                                        return Err(invalid("duplicate directory identity"));
                                    }
                                    pending.push(path);
                                } else {
                                    self.journal.files.entry(id).or_default().push(path);
                                }
                                count += 1;
                                if count > MAX_NODES {
                                    return Err(invalid("journal index too large"));
                                }
                            }
                        }
                        if next == 0 {
                            break;
                        }
                        if next < offset_of!(FILE_ID_BOTH_DIR_INFO, FileName) || next % 8 != 0 {
                            return Err(invalid("invalid next directory entry"));
                        }
                        offset = offset
                            .checked_add(next)
                            .ok_or_else(|| invalid("directory offset overflow"))?;
                    }
                }
            }
            if !valid(&self.journal, expected) {
                return Err(invalid("journal index differs from filesystem inventory"));
            }
            for aliases in self.journal.files.values_mut() {
                aliases.sort();
            }
            Ok(self.journal)
        }
    }
    pub(in crate::svn) fn probe(root: &Path, previous: &Journal) -> io::Result<Delta> {
        let session = begin(root)?;
        if session.journal.volume != previous.volume
            || session.journal.serial != previous.serial
            || session.journal.root_id != previous.root_id
        {
            return Err(invalid("cached journal volume/root changed"));
        }
        let data = query(&session.handle)?;
        if !continuous(
            previous,
            data.UsnJournalID,
            data.FirstUsn,
            data.LowestValidUsn,
            data.NextUsn,
        ) {
            return Err(invalid("USN history expired or journal recreated"));
        }
        let boundary = data.NextUsn;
        let mut journal = previous.clone();
        let mut paths = BTreeSet::new();
        let mut position = previous.next;
        let start = Instant::now();
        let mut scanned = 0usize;
        let mut buffer = vec![AlignedWord(0); 8192];
        while position < boundary {
            let input = READ_USN_JOURNAL_DATA_V0 {
                StartUsn: position,
                ReasonMask: u32::MAX,
                ReturnOnlyOnClose: 0,
                Timeout: 0,
                BytesToWaitFor: 0,
                UsnJournalID: previous.id,
            };
            let mut returned = 0;
            if unsafe {
                DeviceIoControl(
                    session.handle.0,
                    FSCTL_READ_USN_JOURNAL,
                    (&input as *const READ_USN_JOURNAL_DATA_V0).cast(),
                    size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                    buffer.as_mut_ptr().cast(),
                    (buffer.len() * 8) as u32,
                    &mut returned,
                    null_mut(),
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            if returned as usize > buffer.len() * 8 {
                return Err(invalid("journal buffer overflow"));
            }
            let bytes = unsafe {
                std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), returned as usize)
            };
            let (next, records) = parse_page(bytes)?;
            if next <= position {
                return Err(invalid("journal reader made no progress"));
            }
            for record in records {
                if record.usn < position || record.usn >= next {
                    return Err(invalid("USN record outside returned range"));
                }
                if record.usn < boundary {
                    apply_record(&mut journal, record, &mut paths)?;
                }
            }
            position = next.min(boundary);
            scanned += returned as usize;
            if scanned > 16 * 1024 * 1024 || start.elapsed() > Duration::from_secs(2) {
                return Err(invalid("journal replay limit exceeded"));
            }
        }
        journal.next = boundary;
        Ok(Delta {
            journal,
            paths: paths.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn journal() -> Journal {
        Journal {
            volume: "volume".into(),
            serial: 1,
            id: 9,
            next: 100,
            root_id: 1,
            directories: BTreeMap::from([(1, "".into()), (2, "src".into()), (3, ".svn".into())]),
            files: BTreeMap::from([(10, vec!["src/old".into()])]),
        }
    }
    fn record(file: u64, parent: u64, name: &str, reason: u32) -> Record {
        Record {
            file,
            parent,
            name: name.into(),
            usn: 100,
            reason,
            attributes: 0,
        }
    }
    fn bytes(name: &str, reason: u32) -> Vec<u8> {
        let name: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let size = (60 + name.len() + 7) & !7;
        let mut bytes = vec![0u8; 8 + size];
        bytes[..8].copy_from_slice(&200i64.to_le_bytes());
        let data = &mut bytes[8..];
        data[..4].copy_from_slice(&(size as u32).to_le_bytes());
        data[4..6].copy_from_slice(&2u16.to_le_bytes());
        data[8..16].copy_from_slice(&10u64.to_le_bytes());
        data[16..24].copy_from_slice(&2u64.to_le_bytes());
        data[24..32].copy_from_slice(&100i64.to_le_bytes());
        data[40..44].copy_from_slice(&reason.to_le_bytes());
        data[56..58].copy_from_slice(&(name.len() as u16).to_le_bytes());
        data[58..60].copy_from_slice(&60u16.to_le_bytes());
        data[60..60 + name.len()].copy_from_slice(&name);
        bytes
    }
    #[test]
    fn parses_aligned_utf16_usn_records_and_rejects_corruption() {
        let valid = bytes("中文.txt", 1);
        let (next, parsed) = parse_page(&valid).unwrap();
        assert_eq!(next, 200);
        assert_eq!(parsed[0].name, "中文.txt");
        assert_eq!(parsed[0].usn, 100);
        for length in [0, 7, 9, valid.len() - 1] {
            assert!(parse_page(&valid[..length]).is_err());
        }
        let mut wrong_version = valid.clone();
        wrong_version[12..14].copy_from_slice(&3u16.to_le_bytes());
        assert!(parse_page(&wrong_version).is_err());
        let mut offset = valid.clone();
        offset[66..68].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(parse_page(&offset).is_err());
        let mut length = valid.clone();
        length[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse_page(&length).is_err());
        assert!(parse_page(&bytes("bad/path", 1)).is_err());
        let mut surrogate = bytes("a", 1);
        surrogate[68..70].copy_from_slice(&0xd800u16.to_le_bytes());
        assert!(parse_page(&surrogate).is_err());
        assert!(parse_page(&200i64.to_le_bytes()).unwrap().1.is_empty());
    }
    #[test]
    fn replay_tracks_delete_and_both_rename_names_including_accumulated_reasons() {
        let mut journal = journal();
        let mut paths = BTreeSet::new();
        apply_record(&mut journal, record(10, 2, "old", OLD_NAME), &mut paths).unwrap();
        apply_record(
            &mut journal,
            record(10, 1, "new", OLD_NAME | NEW_NAME),
            &mut paths,
        )
        .unwrap();
        apply_record(
            &mut journal,
            record(10, 1, "new", CLOSE | OLD_NAME | NEW_NAME),
            &mut paths,
        )
        .unwrap();
        assert_eq!(paths.into_iter().collect::<Vec<_>>(), ["new", "src/old"]);
        assert_eq!(journal.files[&10], ["new"]);
        let mut paths = BTreeSet::new();
        apply_record(&mut journal, record(10, 1, "new", DELETE), &mut paths).unwrap();
        assert!(!journal.files.contains_key(&10));
        assert_eq!(paths.into_iter().collect::<Vec<_>>(), ["new"]);
    }
    #[test]
    fn external_hardlink_edits_target_known_working_copy_aliases() {
        let mut journal = journal();
        journal.files.get_mut(&10).unwrap().push("another".into());
        let mut paths = BTreeSet::new();
        apply_record(&mut journal, record(10, 900, "outside", 1), &mut paths).unwrap();
        assert_eq!(
            paths.into_iter().collect::<Vec<_>>(),
            ["another", "src/old"]
        );
    }
    #[test]
    fn short_names_in_data_records_do_not_create_duplicate_inventory_paths() {
        let mut journal = journal();
        let mut paths = BTreeSet::new();
        apply_record(&mut journal, record(10, 2, "OLD~1", 1), &mut paths).unwrap();
        assert_eq!(paths.into_iter().collect::<Vec<_>>(), ["src/old"]);
        assert_eq!(journal.files[&10], ["src/old"]);
    }
    #[test]
    fn outside_close_and_admin_temporary_records_do_not_query_svn() {
        let mut journal = journal();
        let mut paths = BTreeSet::new();
        for event in [
            record(77, 900, "outside", 1),
            record(10, 2, "old", CLOSE),
            record(78, 3, "tmp", CREATE),
        ] {
            apply_record(&mut journal, event, &mut paths).unwrap();
        }
        assert!(paths.is_empty());
    }
    #[test]
    fn structural_admin_reparse_and_excessive_changes_force_scan() {
        let cases = [
            record(70, 3, "wc.db", 1),
            record(1, 900, "moved-root", NEW_NAME),
            Record {
                attributes: DIRECTORY,
                ..record(2, 900, "moved-src", NEW_NAME)
            },
            Record {
                attributes: DIRECTORY,
                ..record(77, 1, "new-dir", CREATE)
            },
            Record {
                attributes: REPARSE,
                ..record(10, 2, "old", 1)
            },
        ];
        for event in cases {
            assert!(apply_record(&mut journal(), event, &mut BTreeSet::new()).is_err());
        }
        let mut journal = journal();
        let mut paths = BTreeSet::new();
        for index in 0..MAX_PATHS {
            apply_record(
                &mut journal,
                record(1000 + index as u64, 1, &format!("file-{index}"), CREATE),
                &mut paths,
            )
            .unwrap();
        }
        assert!(apply_record(
            &mut journal,
            record(2000, 1, "overflow", CREATE),
            &mut paths
        )
        .is_err());
    }
    #[test]
    fn journal_reset_trim_and_cursor_ahead_are_not_cache_hits() {
        let journal = journal();
        assert!(continuous(&journal, 9, 0, 0, 200));
        assert!(!continuous(&journal, 10, 0, 0, 200));
        assert!(!continuous(&journal, 9, 101, 0, 200));
        assert!(!continuous(&journal, 9, 0, 101, 200));
        assert!(!continuous(&journal, 9, 0, 0, 99));
    }
    #[test]
    fn serialized_identity_index_must_match_the_filesystem_inventory() {
        let journal = journal();
        let expected = vec![
            ("src".into(), true),
            (".svn".into(), true),
            ("src/old".into(), false),
        ];
        assert!(valid(&journal, &expected));
        let mut broken = journal.clone();
        broken.files.get_mut(&10).unwrap().push("extra".into());
        assert!(!valid(&broken, &expected));
        let restored: Journal =
            serde_json::from_str(&serde_json::to_string(&journal).unwrap()).unwrap();
        assert_eq!(journal, restored);
    }
    #[cfg(windows)]
    #[test]
    fn native_ntfs_journal_replays_offline_edits_hardlinks_rename_and_delete() {
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};
        let base = std::env::temp_dir().join(format!(
            "orcasvn-native-usn-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = base.join("wc");
        fs::create_dir_all(&root).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(base.clone());
        fs::write(root.join("file"), "base\n").unwrap();
        fs::hard_link(root.join("file"), base.join("outside-link")).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let session = match begin(&root) {
            Ok(session) => session,
            Err(error) => {
                if std::env::var_os("ORCASVN_REQUIRE_USN_TEST").is_some() {
                    panic!("native USN test required: {error}");
                }
                eprintln!("Native USN unavailable; metadata fallback applies: {error}");
                return;
            }
        };
        let journal = session.index(&root, &[("file".into(), false)]).unwrap();
        let modified = fs::metadata(root.join("file")).unwrap().modified().unwrap();
        fs::write(base.join("outside-link"), "edit\n").unwrap();
        fs::File::options()
            .write(true)
            .open(root.join("file"))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(modified))
            .unwrap();
        let edited = probe(&root, &journal).unwrap();
        assert_eq!(edited.paths, ["file"]);
        fs::rename(root.join("file"), root.join("renamed")).unwrap();
        let renamed = probe(&root, &edited.journal).unwrap();
        assert_eq!(renamed.paths, ["file", "renamed"]);
        fs::remove_file(root.join("renamed")).unwrap();
        let deleted = probe(&root, &renamed.journal).unwrap();
        assert_eq!(deleted.paths, ["renamed"]);
        fs::create_dir(root.join("new-directory")).unwrap();
        assert!(probe(&root, &deleted.journal).is_err());
    }
}
