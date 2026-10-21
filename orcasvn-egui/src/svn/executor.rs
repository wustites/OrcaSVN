use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{OnceLock, RwLock};
use std::time::Duration;
use thiserror::Error;
use tokio::process::Command;
use tokio::time::timeout;

pub const SVN_TIMEOUT: Duration = Duration::from_secs(120);
static SVN_EXECUTABLE: OnceLock<RwLock<PathBuf>> = OnceLock::new();

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
    let executable = current_svn_executable()?;
    execute_svn_inner(executable, args, path).await
}

pub async fn configure_svn_executable(executable: Option<&str>) -> Result<String, SvnError> {
    let executable = normalize_svn_executable(executable);

    let version = execute_svn_inner(executable.clone(), &["--version", "--quiet"], None).await?;

    let mut configured = svn_executable_lock()
        .write()
        .map_err(|_| SvnError::CommandFailed("SVN 可执行文件配置锁已损坏".to_string()))?;
    *configured = executable;
    Ok(version.trim().to_string())
}

fn normalize_svn_executable(executable: Option<&str>) -> PathBuf {
    executable
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("svn"))
}

fn svn_executable_lock() -> &'static RwLock<PathBuf> {
    SVN_EXECUTABLE.get_or_init(|| RwLock::new(PathBuf::from("svn")))
}

pub(super) fn current_svn_executable() -> Result<PathBuf, SvnError> {
    svn_executable_lock()
        .read()
        .map(|path| path.clone())
        .map_err(|_| SvnError::CommandFailed("SVN 可执行文件配置锁已损坏".to_string()))
}

fn build_command_args(args: &[String]) -> Vec<String> {
    let mut command_args = vec!["--non-interactive".to_string()];
    command_args.extend(args.iter().cloned());
    command_args
}

async fn execute_svn_inner(
    executable: PathBuf,
    args: &[&str],
    path: Option<&str>,
) -> Result<String, SvnError> {
    let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut cmd = Command::new(&executable);
    cmd.args(build_command_args(&args_vec));
    // SVN converts paths and messages using the process locale. Desktop launches
    // may inherit no locale, an ASCII locale, or a non-UTF-8 locale even though
    // Rust passes UTF-8 arguments. Override LC_ALL (which takes precedence over
    // LANG and LC_CTYPE) for the child only, keeping its output UTF-8 as well.
    #[cfg(unix)]
    {
        // macOS ships en_US.UTF-8; C.UTF-8 is available on Linux without
        // installing or generating an additional language locale.
        #[cfg(target_os = "macos")]
        const SVN_LOCALE: &str = "en_US.UTF-8";
        #[cfg(not(target_os = "macos"))]
        const SVN_LOCALE: &str = "C.UTF-8";
        cmd.env("LC_ALL", SVN_LOCALE);
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    // Dropping the output future on timeout kills the child rather than leaving
    // an SVN process running. Tokio drains stdout and stderr concurrently.
    cmd.kill_on_drop(true);
    if let Some(path) = path {
        cmd.current_dir(path);
    }

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    collect_command_output(&mut cmd, SVN_TIMEOUT).await
}

async fn collect_command_output(cmd: &mut Command, deadline: Duration) -> Result<String, SvnError> {
    let output = timeout(deadline, cmd.output())
        .await
        .map_err(|_| SvnError::Timeout)?
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                SvnError::SvnNotFound
            } else {
                SvnError::CommandFailed(error.to_string())
            }
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(SvnError::CommandFailed(format!("{}\n{}", stdout, stderr)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[cfg(unix)]
    #[test]
    fn non_ascii_paths_work_without_a_utf8_parent_locale() {
        const CHILD_MARKER: &str = "ORCASVN_ENCODING_TEST_CHILD";
        if std::env::var_os(CHILD_MARKER).is_none() {
            for executable in ["svn", "svnadmin"] {
                if std::process::Command::new(executable)
                    .arg("--version")
                    .output()
                    .is_err()
                {
                    eprintln!("Skipping encoding regression test: {executable} is unavailable");
                    return;
                }
            }
            // Isolate parent locale changes from other tests running in parallel.
            for locale in [None, Some("C"), Some("invalid_ORCASVN_locale")] {
                let mut child = std::process::Command::new(std::env::current_exe().unwrap());
                child.args([
                    "--exact",
                    "svn::executor::tests::non_ascii_paths_work_without_a_utf8_parent_locale",
                    "--nocapture",
                ]);
                child.env(CHILD_MARKER, "1");
                for variable in ["LC_ALL", "LC_CTYPE", "LANG"] {
                    match locale {
                        Some(value) => {
                            child.env(variable, value);
                        }
                        None => {
                            child.env_remove(variable);
                        }
                    }
                }
                let output = child.output().unwrap();
                assert!(
                    output.status.success(),
                    "encoding regression failed with parent locale {locale:?}:\n{}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            return;
        }

        let directory = std::env::temp_dir().join(format!(
            "orcasvn-encoding-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(directory.clone());
        let repository = directory.join("repository");
        let created = std::process::Command::new("svnadmin")
            .arg("create")
            .arg(&repository)
            .output()
            .unwrap();
        assert!(created.status.success());
        let url = format!("file://{}", repository.to_str().unwrap());
        let wc = directory.join("工作副本");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let run = |args: &[&str], path: Option<&str>| {
            runtime
                .block_on(execute_svn_inner(PathBuf::from("svn"), args, path))
                .unwrap()
        };
        run(&["checkout", &url, wc.to_str().unwrap()], None);
        std::fs::create_dir(wc.join("子目录")).unwrap();
        std::fs::write(wc.join("子目录/文件.xlsx"), "中文内容").unwrap();
        run(&["add", "子目录"], Some(wc.to_str().unwrap()));
        run(
            &["commit", "-m", "提交中文文件", "子目录/文件.xlsx", "子目录"],
            Some(wc.to_str().unwrap()),
        );
        let log = run(&["log", "--xml", "-v", &url], None);
        assert!(log.contains("提交中文文件"));
        assert!(log.contains("/子目录/文件.xlsx"));
        let second_wc = directory.join("第二副本");
        run(
            &[
                "checkout",
                &format!("{url}/子目录"),
                second_wc.to_str().unwrap(),
            ],
            None,
        );
        assert_eq!(
            std::fs::read_to_string(second_wc.join("文件.xlsx")).unwrap(),
            "中文内容"
        );
    }

    #[test]
    fn captures_svn_stdout() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");

        let result = runtime.block_on(execute_svn(&["--version", "--quiet"], None));

        match result {
            Ok(output) => assert!(!output.trim().is_empty()),
            Err(SvnError::SvnNotFound) => {}
            Err(err) => panic!("unexpected svn executor error: {err}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn drains_large_stdout_and_stderr_without_blocking() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "i=0; while [ $i -lt 10000 ]; do echo output; echo error >&2; i=$((i+1)); done",
        ]);
        command.kill_on_drop(true);
        let output = runtime
            .block_on(collect_command_output(
                &mut command,
                Duration::from_secs(10),
            ))
            .unwrap();
        assert_eq!(output.lines().count(), 10000);
    }

    #[cfg(unix)]
    #[test]
    fn timeout_kills_the_child() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut command = Command::new("sh");
        command.args(["-c", "echo $$; exec sleep 10"]);
        command.kill_on_drop(true);
        // Spawn explicitly so the test can check that the timed-out PID exits.
        command.stdout(Stdio::piped());
        runtime.block_on(async {
            let child = command.spawn().unwrap();
            let pid = child.id().unwrap();
            assert!(timeout(Duration::from_millis(30), child.wait_with_output())
                .await
                .is_err());
            for _ in 0..100 {
                let alive = std::process::Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .output()
                    .unwrap()
                    .status
                    .success();
                if !alive {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("timed-out child is still alive");
        });
    }

    #[test]
    fn empty_executable_uses_default_command() {
        assert_eq!(normalize_svn_executable(Some("   ")), Path::new("svn"));
        assert_eq!(normalize_svn_executable(None), Path::new("svn"));
    }

    #[test]
    fn configured_executable_is_trimmed() {
        assert_eq!(
            normalize_svn_executable(Some("  C:\\Tools\\svn.exe  ")),
            Path::new("C:\\Tools\\svn.exe")
        );
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
