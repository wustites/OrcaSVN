use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use windows_sys::Win32::Globalization::{
    GetACP, WideCharToMultiByte, CP_UTF8, WC_NO_BEST_FIT_CHARS,
};

use super::executor::{collect_command_output, SvnError};

// WideCharToMultiByte in old svn.exe replaces characters with '?' or a best-fit
// character *before* SVN sees the target. This can also select a different file.
pub(super) fn can_encode(value: &str, code_page: u32) -> bool {
    if value.is_empty() || code_page == CP_UTF8 {
        return true;
    }
    let wide: Vec<u16> = value.encode_utf16().collect();
    let Ok(length) = i32::try_from(wide.len()) else {
        return false;
    };
    let mut substituted = 0;
    // Querying the required buffer size also reports default-character use.
    let size = unsafe {
        WideCharToMultiByte(
            code_page,
            WC_NO_BEST_FIT_CHARS,
            wide.as_ptr(),
            length,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            &mut substituted,
        )
    };
    size > 0 && substituted == 0
}

pub(super) async fn validate_arguments(executable: &Path, args: &[&str]) -> Result<(), SvnError> {
    let code_page = unsafe { GetACP() };
    if args.iter().all(|arg| can_encode(arg, code_page)) {
        return Ok(());
    }
    // Do not assume all Windows SVN builds use ANSI: newer/native Unicode
    // clients and UTF-8-manifest builds can handle the original arguments.
    if supports_unicode_arguments(executable).await? {
        return Ok(());
    }
    Err(SvnError::InvalidArguments(format!(
        "当前 SVN 命令行程序无法无损接收这些 Unicode 参数（Windows 代码页 {code_page}）。为避免路径变成问号或操作到其他文件，本次操作未执行。请在设置中选择支持 Unicode 命令行参数的 SVN 程序，或使用当前系统代码页可表示的路径和参数。"
    )))
}

struct ProbeDirectory(PathBuf);

impl Drop for ProbeDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn supports_unicode_arguments(executable: &Path) -> Result<bool, SvnError> {
    let directory = std::env::temp_dir().join(format!(
        "orcasvn-unicode-probe-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir(&directory)
        .map_err(|error| SvnError::CommandFailed(format!("无法创建 SVN 编码检测目录：{error}")))?;
    let _cleanup = ProbeDirectory(directory.clone());
    let config = directory.join("unicode-🐋");
    let mut command = Command::new(executable);
    // `auth` with a brand-new explicit configuration directory is local-only,
    // touches no working copy, and cannot read the user's authentication cache.
    // Legacy clients can exit 0 after a config warning, so check the exact path.
    command.args(["auth", "--non-interactive", "--config-dir"]);
    command.arg(&config);
    command.current_dir(&directory);
    command.creation_flags(0x08000000);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    command.kill_on_drop(true);
    match collect_command_output(&mut command, Duration::from_secs(10)).await {
        Ok(_) => Ok(config.join("config").is_file()),
        Err(SvnError::CommandFailed(_)) => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_check_rejects_replacement_and_best_fit_characters() {
        assert!(can_encode("ASCII path/file.txt", 1252));
        assert!(can_encode("café.txt", 1252));
        assert!(!can_encode("甲.txt", 1252));
        assert!(!can_encode("Ａ.txt", 1252));
        assert!(can_encode("中文 空格.txt", 936));
        assert!(!can_encode("emoji-🐋.txt", 936));
        assert!(can_encode("中文 🐋.txt", CP_UTF8));
    }
}
