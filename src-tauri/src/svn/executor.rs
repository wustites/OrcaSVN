use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::task::spawn_blocking;

pub const SVN_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Error, Debug)]
pub enum SvnError {
    #[error("SVN 命令执行失败：{0}")]
    CommandFailed(String),
    #[error("SVN 未安装或不在 PATH 中")]
    SvnNotFound,
    #[error("解析输出失败：{0}")]
    ParseError(String),
    #[error("无效的 SVN 参数：{0}")]
    InvalidArguments(String),
    #[error("SVN 操作超时")]
    Timeout,
}

pub async fn execute_svn(args: &[&str], path: Option<&str>) -> Result<String, SvnError> {
    execute_svn_inner(args, path).await
}

fn build_command_args(args: &[String]) -> Vec<String> {
    let mut command_args = vec!["--non-interactive".to_string()];
    command_args.extend(args.iter().cloned());
    command_args
}

async fn execute_svn_inner(args: &[&str], path: Option<&str>) -> Result<String, SvnError> {
    let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let path_str = path.map(|s| s.to_string());

    spawn_blocking(move || {
        let mut cmd = Command::new("svn");
        // Global options must precede a possible `--` target separator.
        cmd.args(build_command_args(&args_vec));
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Some(p) = path_str.as_ref() {
            cmd.current_dir(p);
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SvnError::SvnNotFound
            } else {
                SvnError::CommandFailed(e.to_string())
            }
        })?;

        // Drain both pipes while SVN is running. Large XML responses such as
        // verbose logs can otherwise fill an OS pipe and block the child.
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| SvnError::CommandFailed("无法读取 SVN 标准输出".to_string()))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| SvnError::CommandFailed("无法读取 SVN 错误输出".to_string()))?;
        let stdout_reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        });
        let stderr_reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).map(|_| bytes)
        });

        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() >= SVN_TIMEOUT => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(SvnError::Timeout);
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(e) => return Err(SvnError::CommandFailed(e.to_string())),
            }
        };

        let stdout = stdout_reader
            .join()
            .map_err(|_| SvnError::CommandFailed("读取 SVN 标准输出失败".to_string()))?
            .map_err(|e| SvnError::CommandFailed(e.to_string()))?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| SvnError::CommandFailed("读取 SVN 错误输出失败".to_string()))?
            .map_err(|e| SvnError::CommandFailed(e.to_string()))?;
        let stdout = String::from_utf8_lossy(&stdout).to_string();
        let stderr = String::from_utf8_lossy(&stderr).to_string();

        if status.success() {
            Ok(stdout)
        } else {
            Err(SvnError::CommandFailed(format!("{}\n{}", stdout, stderr)))
        }
    })
    .await
    .map_err(|e| SvnError::CommandFailed(format!("SVN 进程异常退出：{}", e)))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_svn_stdout() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("failed to build tokio runtime");

        let result = runtime.block_on(execute_svn(&["--version", "--quiet"], None));

        match result {
            Ok(output) => assert!(!output.trim().is_empty()),
            Err(SvnError::SvnNotFound) => {}
            Err(err) => panic!("unexpected svn executor error: {err}"),
        }
    }

    #[test]
    fn global_options_precede_target_separator() {
        let args = vec![
            "diff".to_string(),
            "-c".to_string(),
            "12".to_string(),
            "--".to_string(),
            "https://example.test/repo/file@12".to_string(),
        ];

        assert_eq!(
            build_command_args(&args),
            [
                "--non-interactive",
                "diff",
                "-c",
                "12",
                "--",
                "https://example.test/repo/file@12"
            ]
        );
    }
}
