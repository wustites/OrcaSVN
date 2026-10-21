//! Desktop integration: launch arguments, file manager, editor and terminal.

use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum OpenTarget {
    Explorer,
    Vscode,
    Terminal,
}

/// Workspace directory passed on the command line, e.g. `orcasvn <path>`.
pub fn launch_workspace_path() -> Option<String> {
    launch_workspace_arg(
        env::args_os().skip(1).map(PathBuf::from),
        &env::current_dir().ok()?,
    )
}

fn launch_workspace_arg(args: impl Iterator<Item = PathBuf>, cwd: &Path) -> Option<String> {
    let arg = args
        .into_iter()
        .find(|arg| !arg.to_string_lossy().starts_with('-'))?;
    let path = fs::canonicalize(cwd.join(arg)).ok()?;
    if !path.is_dir() {
        return None;
    }
    let path = path.to_string_lossy().into_owned();
    Some(
        path.strip_prefix(r"\\?\")
            .map(str::to_string)
            .unwrap_or(path),
    )
}

pub fn open_workspace_target(path: String, target: OpenTarget) -> Result<(), String> {
    match target {
        OpenTarget::Explorer => open_file_manager(&path),
        OpenTarget::Vscode => {
            let mut code_command = Command::new("code");
            code_command.arg(&path);
            if spawn_command(code_command).is_ok() {
                return Ok(());
            }

            for executable in vscode_candidates() {
                let mut command = Command::new(executable);
                command.arg(&path);
                if spawn_command(command).is_ok() {
                    return Ok(());
                }
            }

            #[cfg(target_os = "macos")]
            {
                let mut command = Command::new("open");
                command.args(["-a", "Visual Studio Code", &path]);
                if spawn_command(command).is_ok() {
                    return Ok(());
                }
            }

            Err("failed to open workspace: VS Code executable was not found".to_string())
        }
        OpenTarget::Terminal => open_terminal(&path),
    }
}

fn open_file_manager(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let mut command = Command::new("xdg-open");

    command.arg(path);
    spawn_command(command)
}

pub fn reveal_workspace_file(path: String, file: String) -> Result<(), String> {
    let workspace = fs::canonicalize(&path).map_err(|e| format!("invalid workspace path: {e}"))?;
    let target = resolve_workspace_child(&workspace, &file)?;

    #[cfg(target_os = "windows")]
    {
        let reveal_target = if target.exists() {
            target
        } else {
            target.parent().unwrap_or(&workspace).to_path_buf()
        };
        let mut command = Command::new("explorer");
        if reveal_target.is_file() {
            command.arg(format!("/select,{}", reveal_target.display()));
        } else {
            command.arg(reveal_target);
        }
        return spawn_command(command);
    }

    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        if target.exists() {
            command.arg("-R").arg(target);
        } else {
            command.arg(target.parent().unwrap_or(&workspace));
        }
        return spawn_command(command);
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let directory = if target.is_dir() {
            target.as_path()
        } else {
            target.parent().unwrap_or(&workspace)
        };
        let mut command = Command::new("xdg-open");
        command.arg(directory);
        spawn_command(command)
    }
}

fn spawn_command(mut command: Command) -> Result<(), String> {
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to open workspace: {e}"))
}

pub fn delete_unversioned(path: &str, files: &[String]) -> Result<(), String> {
    let workspace = fs::canonicalize(&path).map_err(|e| format!("invalid workspace path: {e}"))?;

    for file in files {
        let target = resolve_workspace_child(&workspace, &file)?;
        if !target.exists() {
            return Err(format!("file does not exist: {file}"));
        }

        let canonical_target = fs::canonicalize(&target)
            .map_err(|e| format!("failed to resolve file path '{file}': {e}"))?;
        if !canonical_target.starts_with(&workspace) {
            return Err(format!("refusing to delete outside workspace: {file}"));
        }

        if canonical_target.is_dir() {
            fs::remove_dir_all(&canonical_target)
                .map_err(|e| format!("failed to delete directory '{file}': {e}"))?;
        } else {
            fs::remove_file(&canonical_target)
                .map_err(|e| format!("failed to delete file '{file}': {e}"))?;
        }
    }
    Ok(())
}

fn resolve_workspace_child(workspace: &Path, file: &str) -> Result<PathBuf, String> {
    let relative = Path::new(file);
    if relative.is_absolute() {
        return Err(format!("absolute paths are not allowed: {file}"));
    }

    let mut target = workspace.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(part) => target.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("invalid path outside workspace: {file}"));
            }
        }
    }

    Ok(target)
}

fn vscode_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        candidates
            .push(PathBuf::from(&local_app_data).join("Programs\\Microsoft VS Code\\Code.exe"));
        candidates.push(
            PathBuf::from(&local_app_data)
                .join("Programs\\Microsoft VS Code Insiders\\Code - Insiders.exe"),
        );
    }

    if let Ok(program_files) = env::var("ProgramFiles") {
        candidates.push(PathBuf::from(&program_files).join("Microsoft VS Code\\Code.exe"));
        candidates.push(
            PathBuf::from(&program_files).join("Microsoft VS Code Insiders\\Code - Insiders.exe"),
        );
    }

    if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
        candidates.push(PathBuf::from(&program_files_x86).join("Microsoft VS Code\\Code.exe"));
        candidates.push(
            PathBuf::from(&program_files_x86)
                .join("Microsoft VS Code Insiders\\Code - Insiders.exe"),
        );
    }

    candidates
}

#[cfg(target_os = "windows")]
fn open_terminal(path: &str) -> Result<(), String> {
    let workspace = PathBuf::from(path);
    let mut errors = Vec::new();

    let mut windows_terminal = Command::new("wt");
    windows_terminal.args(["-d", path]);
    match spawn_command(windows_terminal) {
        Ok(()) => return Ok(()),
        Err(error) => errors.push(format!("wt: {error}")),
    }

    let mut pwsh = Command::new("pwsh");
    pwsh.arg("-NoExit").current_dir(&workspace);
    match spawn_command(pwsh) {
        Ok(()) => return Ok(()),
        Err(error) => errors.push(format!("pwsh: {error}")),
    }

    let mut powershell = Command::new("powershell");
    powershell.arg("-NoExit").current_dir(&workspace);
    match spawn_command(powershell) {
        Ok(()) => return Ok(()),
        Err(error) => errors.push(format!("powershell: {error}")),
    }

    let mut cmd = Command::new("cmd");
    cmd.arg("/K").current_dir(&workspace);
    match spawn_command(cmd) {
        Ok(()) => Ok(()),
        Err(error) => {
            errors.push(format!("cmd: {error}"));
            Err(format!("failed to open terminal: {}", errors.join("; ")))
        }
    }
}

#[cfg(target_os = "macos")]
fn open_terminal(path: &str) -> Result<(), String> {
    let mut command = Command::new("open");
    command.args(["-a", "Terminal", path]);
    spawn_command(command)
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn open_terminal(path: &str) -> Result<(), String> {
    let candidates: [(&str, &[&str]); 4] = [
        ("x-terminal-emulator", &["--working-directory", path]),
        ("gnome-terminal", &["--working-directory", path]),
        ("konsole", &["--workdir", path]),
        ("xfce4-terminal", &["--working-directory", path]),
    ];
    let mut errors = Vec::new();
    for (executable, args) in candidates {
        let mut command = Command::new(executable);
        command.args(args);
        match spawn_command(command) {
            Ok(()) => return Ok(()),
            Err(error) => errors.push(format!("{executable}: {error}")),
        }
    }
    Err(format!("failed to open terminal: {}", errors.join("; ")))
}

#[cfg(test)]
mod tests {
    use super::launch_workspace_arg;
    use std::path::PathBuf;

    #[test]
    fn resolves_first_directory_argument_and_skips_flags() {
        let cwd = std::env::temp_dir();
        let expected = std::fs::canonicalize(&cwd).unwrap();
        let args = vec![PathBuf::from("-psn_0_123"), PathBuf::from(".")];
        let resolved = launch_workspace_arg(args.into_iter(), &cwd).unwrap();
        assert!(expected.to_string_lossy().ends_with(&resolved));
        assert!(launch_workspace_arg(std::iter::empty(), &cwd).is_none());
    }
}
