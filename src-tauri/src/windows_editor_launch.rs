use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use windows_sys::Win32::Foundation::{ERROR_CANCELLED, ERROR_ELEVATION_REQUIRED};
use windows_sys::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub const ELEVATION_CANCELLED_MESSAGE: &str = "已取消系统权限确认，未打开编辑器";

pub fn spawn_editor(executable: &Path, workspace: &str) -> Result<(), String> {
    match Command::new(executable)
        .arg(workspace)
        .creation_flags(0x08000000)
        .spawn()
    {
        Ok(_) => Ok(()),
        Err(error) if error.raw_os_error() == Some(ERROR_ELEVATION_REQUIRED as i32) => {
            // 只在程序本身要求提升权限时交给 Windows 打开；系统确认仍由用户决定。
            shell_open_editor(executable, OsStr::new(workspace))
        }
        Err(error) => Err(format!("启动编辑器失败：{error}")),
    }
}

fn terminated_wide(value: &OsStr) -> Result<Vec<u16>, String> {
    let mut encoded: Vec<u16> = value.encode_wide().collect();
    if encoded.contains(&0) {
        return Err("程序或工作区路径包含无效字符".to_string());
    }
    encoded.push(0);
    Ok(encoded)
}

fn quoted_argument(value: &OsStr) -> Result<Vec<u16>, String> {
    let encoded = terminated_wide(value)?;
    let mut quoted = vec![b'"' as u16];
    let mut backslashes = 0;
    for &unit in &encoded[..encoded.len() - 1] {
        if unit == b'\\' as u16 {
            backslashes += 1;
            continue;
        }
        // 按 Windows 参数规则处理引号前的反斜杠，保留路径中的空格及特殊字符。
        let count = if unit == b'"' as u16 {
            backslashes * 2 + 1
        } else {
            backslashes
        };
        quoted.extend(std::iter::repeat_n(b'\\' as u16, count));
        quoted.push(unit);
        backslashes = 0;
    }
    // 目录以反斜杠结尾时也不能吞掉闭合引号，例如磁盘根目录。
    quoted.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2));
    quoted.extend([b'"' as u16, 0]);
    Ok(quoted)
}

fn shell_open_editor(executable: &Path, workspace: &OsStr) -> Result<(), String> {
    let executable = terminated_wide(executable.as_os_str())?;
    let parameters = quoted_argument(workspace)?;
    let verb = terminated_wide(OsStr::new("open"))?;
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpVerb: verb.as_ptr(),
        lpFile: executable.as_ptr(),
        lpParameters: parameters.as_ptr(),
        nShow: SW_SHOWNORMAL,
        ..Default::default()
    };
    // 所有 UTF-16 缓冲区在同步调用期间保持有效，不执行 cmd 或 PowerShell 命令。
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let error = io::Error::last_os_error();
        return Err(shell_error(&error));
    }
    Ok(())
}

fn shell_error(error: &io::Error) -> String {
    if error.raw_os_error() == Some(ERROR_CANCELLED as i32) {
        ELEVATION_CANCELLED_MESSAGE.to_string()
    } else {
        format!("系统未能启动编辑器：{error}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_editor_launch_quotes_one_directory_argument() {
        for (input, expected) in [
            (
                r"D:\临时代码 & 项目\a;b $(test)",
                "\"D:\\临时代码 & 项目\\a;b $(test)\"",
            ),
            (r"D:\", "\"D:\\\\\""),
            ("D:\\a\\\"b", "\"D:\\a\\\\\\\"b\""),
        ] {
            let quoted = quoted_argument(OsStr::new(input)).unwrap();
            assert_eq!(quoted.last(), Some(&0));
            assert_eq!(
                String::from_utf16(&quoted[..quoted.len() - 1]).unwrap(),
                expected
            );
        }
        assert!(quoted_argument(OsStr::new("D:\\a\0b")).is_err());
    }

    #[test]
    fn windows_editor_launch_reports_cancelled_permission() {
        assert_eq!(
            shell_error(&io::Error::from_raw_os_error(ERROR_CANCELLED as i32)),
            ELEVATION_CANCELLED_MESSAGE
        );
        assert!(shell_error(&io::Error::from_raw_os_error(5)).starts_with("系统未能启动编辑器："));
    }
}
