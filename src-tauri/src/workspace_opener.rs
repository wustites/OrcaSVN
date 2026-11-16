use std::env;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(any(not(target_os = "windows"), test))]
use std::process::Command;

#[derive(Clone, Copy)]
pub enum EditorTarget {
    Vscode,
    AndroidStudio,
    Idea,
    Custom,
}

impl EditorTarget {
    fn label(self) -> &'static str {
        match self {
            Self::Vscode => "VS Code",
            Self::AndroidStudio => "Android Studio",
            Self::Idea => "IntelliJ IDEA",
            Self::Custom => "其他编辑器",
        }
    }

    #[cfg(target_os = "windows")]
    fn executables(self) -> &'static [&'static str] {
        match self {
            Self::Vscode => &["Code.exe", "Code - Insiders.exe"],
            Self::AndroidStudio => &["studio64.exe", "studio.exe"],
            Self::Idea => &["idea64.exe", "idea.exe"],
            Self::Custom => &[],
        }
    }
}

pub fn workspace_directory(path: &str) -> Result<String, String> {
    let directory = fs::canonicalize(path).map_err(|e| format!("工作区目录不可用：{e}"))?;
    if !directory.is_dir() {
        return Err("工作区路径不是文件夹".to_string());
    }
    // 转为绝对路径，避免目录名被编辑器当作命令行选项，同时兼容不识别长路径前缀的程序。
    let directory = directory.to_string_lossy();
    Ok(if let Some(unc) = directory.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        directory
            .strip_prefix(r"\\?\")
            .unwrap_or(&directory)
            .to_string()
    })
}

pub fn open_editor(
    workspace: &str,
    target: EditorTarget,
    executable_path: Option<&str>,
) -> Result<(), String> {
    if let Some(executable) = executable_path.filter(|value| !value.trim().is_empty()) {
        let executable = validate_executable(executable)?;
        return spawn_editor(&executable, workspace).map_err(|e| {
            #[cfg(target_os = "windows")]
            if e == crate::windows_editor_launch::ELEVATION_CANCELLED_MESSAGE {
                return e;
            }
            format!(
                "无法启动 {}：{e}；请检查打开方式设置中的程序路径",
                target.label()
            )
        });
    }
    if matches!(target, EditorTarget::Custom) {
        return Err("请先在打开方式设置中选择其他编辑器的程序路径".to_string());
    }
    let mut last_error = None;
    for executable in editor_candidates(target) {
        if let Err(error) = spawn_editor(&executable, workspace) {
            #[cfg(target_os = "windows")]
            if error == crate::windows_editor_launch::ELEVATION_CANCELLED_MESSAGE {
                return Err(error);
            }
            last_error = Some(error);
        } else {
            return Ok(());
        }
    }
    let reason = last_error
        .map(|error| format!("（{error}）"))
        .unwrap_or_default();
    Err(format!(
        "未能启动 {}{reason}，请先安装该程序，或在打开方式设置中指定程序路径",
        target.label()
    ))
}

fn validate_executable(value: &str) -> Result<PathBuf, String> {
    let executable = PathBuf::from(value.trim());
    if !executable.is_absolute() {
        return Err("程序路径必须是绝对路径，不能包含启动参数".to_string());
    }
    #[cfg(target_os = "macos")]
    if executable.is_dir() && executable.extension().is_some_and(|ext| ext == "app") {
        return Ok(executable);
    }
    if !executable.is_file() {
        return Err("指定的编辑器程序不存在，请重新选择程序路径".to_string());
    }
    #[cfg(target_os = "windows")]
    if !executable
        .extension()
        .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("exe"))
    {
        return Err("请选择编辑器的 .exe 文件，不支持快捷方式或批处理脚本".to_string());
    }
    Ok(executable)
}

#[cfg(any(not(target_os = "windows"), test))]
fn editor_command(executable: &Path, workspace: &str) -> Command {
    #[cfg(target_os = "macos")]
    if executable.extension().is_some_and(|ext| ext == "app") {
        let mut command = Command::new("open");
        command.arg("-a").arg(executable).arg(workspace);
        return command;
    }
    let mut command = Command::new(executable);
    // 程序与工作区分别传参，不通过 shell 拼接命令，带空格和特殊字符的目录不会被执行。
    command.arg(workspace);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn spawn_editor(executable: &Path, workspace: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return crate::windows_editor_launch::spawn_editor(executable, workspace);

    #[cfg(not(target_os = "windows"))]
    editor_command(executable, workspace)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn editor_candidates(target: EditorTarget) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let vendor = match target {
            EditorTarget::AndroidStudio => Some("Google"),
            EditorTarget::Idea => Some("JetBrains"),
            _ => None,
        };
        if let Some(vendor) = vendor {
            append_recent_installations(
                &mut candidates,
                &PathBuf::from(local).join(vendor),
                target,
            );
        }
    }
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path).filter(|path| path.is_absolute()) {
            append_executables(&mut candidates, &directory, target);
            // VS Code 的 PATH 通常指向 bin，直接定位上级 Code.exe，避免调用 code.cmd。
            if directory
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("bin"))
            {
                if let Some(parent) = directory.parent() {
                    append_executables(&mut candidates, parent, target);
                }
            }
        }
    }
    let mut roots: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(env::var_os)
        .map(PathBuf::from)
        .collect();
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        roots.push(local.join("Programs"));
        if !matches!(target, EditorTarget::Vscode) {
            append_toolbox_candidates(
                &mut candidates,
                &local.join("JetBrains/Toolbox/apps"),
                target,
                4,
            );
        }
    }
    for root in roots {
        match target {
            EditorTarget::Vscode => {
                append_executables(&mut candidates, &root.join("Microsoft VS Code"), target);
                append_executables(
                    &mut candidates,
                    &root.join("Microsoft VS Code Insiders"),
                    target,
                );
            }
            EditorTarget::AndroidStudio => {
                append_executables(
                    &mut candidates,
                    &root.join("Android/Android Studio/bin"),
                    target,
                );
                append_product_candidates(&mut candidates, &root.join("Android"), target);
                append_product_candidates(&mut candidates, &root, target);
            }
            EditorTarget::Idea => {
                append_product_candidates(&mut candidates, &root.join("JetBrains"), target);
                append_product_candidates(&mut candidates, &root, target);
            }
            EditorTarget::Custom => {}
        }
    }
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|path| seen.insert(path.to_string_lossy().to_lowercase()));
    candidates
}

#[cfg(target_os = "windows")]
fn append_executables(candidates: &mut Vec<PathBuf>, directory: &Path, target: EditorTarget) {
    for name in target.executables() {
        let executable = directory.join(name);
        if executable.is_file() {
            candidates.push(executable);
        }
    }
}

#[cfg(target_os = "windows")]
fn append_recent_installations(candidates: &mut Vec<PathBuf>, vendor: &Path, target: EditorTarget) {
    // IDE 的 .home 记录实际安装目录，兼容非系统盘和 ZIP 解压安装；优先最近使用的有效版本。
    // 只读取对应产品配置目录中的短路径文件，不扫描项目、日志或整块磁盘。
    let mut records: Vec<_> = child_directories(vendor)
        .into_iter()
        .filter(|directory| {
            let name = directory
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            match target {
                EditorTarget::AndroidStudio => name.starts_with("androidstudio"),
                EditorTarget::Idea => {
                    name.starts_with("intellijidea") || name.starts_with("ideaic")
                }
                _ => false,
            }
        })
        .filter_map(|directory| {
            let record = directory.join(".home");
            let metadata = fs::symlink_metadata(&record).ok()?;
            if !metadata.is_file() || metadata.len() > 32768 {
                return None;
            }
            Some((metadata.modified().ok()?, record))
        })
        .collect();
    records.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    for (_, record) in records {
        let Ok(home) = fs::read_to_string(record) else {
            continue;
        };
        let installation = Path::new(home.trim_start_matches('\u{feff}').trim());
        if installation.is_absolute() {
            append_executables(candidates, &installation.join("bin"), target);
        }
    }
}

#[cfg(target_os = "windows")]
fn child_directories(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut children: Vec<_> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    children.sort_by(|a, b| b.cmp(a));
    children
}

#[cfg(target_os = "windows")]
fn append_product_candidates(candidates: &mut Vec<PathBuf>, root: &Path, target: EditorTarget) {
    for product in child_directories(root) {
        let name = product
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let matches = match target {
            EditorTarget::AndroidStudio => {
                name.starts_with("android studio") || name.starts_with("android-studio")
            }
            EditorTarget::Idea => name.starts_with("intellij") || name.starts_with("idea"),
            _ => false,
        };
        if matches {
            append_executables(candidates, &product.join("bin"), target);
        }
    }
}

#[cfg(target_os = "windows")]
fn append_toolbox_candidates(
    candidates: &mut Vec<PathBuf>,
    root: &Path,
    target: EditorTarget,
    depth: usize,
) {
    append_executables(candidates, &root.join("bin"), target);
    if depth == 0 {
        return;
    }
    // 仅扫描 Toolbox 的有限层级，不递归遍历工作区或整块磁盘，也不跟随目录链接。
    for child in child_directories(root) {
        let name = child
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if ["bin", "lib", "plugins", "jbr", "jre", "license", "help"].contains(&name.as_str()) {
            continue;
        }
        append_toolbox_candidates(candidates, &child, target, depth - 1);
    }
}

#[cfg(target_os = "macos")]
fn editor_candidates(target: EditorTarget) -> Vec<PathBuf> {
    let names: &[&str] = match target {
        EditorTarget::Vscode => &[
            "Visual Studio Code.app",
            "Visual Studio Code - Insiders.app",
        ],
        EditorTarget::AndroidStudio => &["Android Studio.app"],
        EditorTarget::Idea => &[
            "IntelliJ IDEA.app",
            "IntelliJ IDEA CE.app",
            "IntelliJ IDEA Ultimate.app",
        ],
        EditorTarget::Custom => &[],
    };
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Applications"));
    }
    roots
        .into_iter()
        .flat_map(|root| names.iter().map(move |name| root.join(name)))
        .filter(|path| path.is_dir())
        .collect()
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn editor_candidates(target: EditorTarget) -> Vec<PathBuf> {
    let names: &[&str] = match target {
        EditorTarget::Vscode => &["code", "code-insiders"],
        EditorTarget::AndroidStudio => &["android-studio", "studio", "studio.sh"],
        EditorTarget::Idea => &[
            "idea",
            "idea.sh",
            "intellij-idea-ultimate",
            "intellij-idea-community",
        ],
        EditorTarget::Custom => &[],
    };
    let mut candidates = Vec::new();
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path).filter(|path| path.is_absolute()) {
            candidates.extend(
                names
                    .iter()
                    .map(|name| directory.join(name))
                    .filter(|path| path.is_file()),
            );
        }
    }
    let common = match target {
        EditorTarget::AndroidStudio => Some("/opt/android-studio/bin/studio.sh"),
        EditorTarget::Idea => Some("/opt/idea/bin/idea.sh"),
        _ => None,
    };
    if let Some(path) = common.map(PathBuf::from).filter(|path| path.is_file()) {
        candidates.push(path);
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_opener_keeps_special_directory_as_one_argument() {
        let workspace = r"D:\临时代码 & 项目\a;b $(test)";
        let executable = Path::new(r"C:\Program Files\Android\studio64.exe");
        let command = editor_command(executable, workspace);
        assert_eq!(command.get_program(), executable.as_os_str());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![std::ffi::OsStr::new(workspace)]
        );
        assert!(validate_executable("studio64.exe --run").is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn workspace_opener_finds_toolbox_and_rejects_scripts() {
        let root = env::temp_dir().join(format!("orcasvn-editor-test-{}", std::process::id()));
        let product = root.join("IDEA-U/ch-0/2026.1/bin");
        fs::create_dir_all(&product).unwrap();
        let executable = product.join("idea64.exe");
        fs::write(&executable, "").unwrap();
        let script = product.join("idea.cmd");
        fs::write(&script, "").unwrap();
        let mut candidates = Vec::new();
        append_toolbox_candidates(&mut candidates, &root, EditorTarget::Idea, 4);
        assert_eq!(candidates, vec![executable.clone()]);
        assert_eq!(
            validate_executable(executable.to_str().unwrap()).unwrap(),
            executable
        );
        assert!(validate_executable(script.to_str().unwrap()).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn workspace_opener_finds_recent_zip_installation_and_skips_stale_records() {
        let root = env::temp_dir().join(format!("orcasvn-studio-home-test-{}", std::process::id()));
        let vendor = root.join("Google");
        let older = root.join("其他磁盘/Android Studio old/bin");
        let recent = root.join("Program Files/android-studio-quail3-windows/android-studio/bin");
        fs::create_dir_all(&older).unwrap();
        fs::create_dir_all(&recent).unwrap();
        fs::write(older.join("studio64.exe"), "").unwrap();
        fs::write(recent.join("studio64.exe"), "").unwrap();
        for (name, home, seconds) in [
            ("AndroidStudio2024.1", older.parent().unwrap(), 10),
            ("AndroidStudio2025.2.3", recent.parent().unwrap(), 20),
            ("AndroidStudio2026.1", root.as_path(), 30),
            ("UnrelatedProduct", older.parent().unwrap(), 40),
        ] {
            let directory = vendor.join(name);
            fs::create_dir_all(&directory).unwrap();
            let record = directory.join(".home");
            fs::write(&record, format!("\u{feff}{}\r\n", home.display())).unwrap();
            fs::File::options()
                .write(true)
                .open(record)
                .unwrap()
                .set_times(
                    fs::FileTimes::new().set_modified(
                        std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds),
                    ),
                )
                .unwrap();
        }
        let mut candidates = Vec::new();
        append_recent_installations(&mut candidates, &vendor, EditorTarget::AndroidStudio);
        assert_eq!(
            candidates,
            vec![recent.join("studio64.exe"), older.join("studio64.exe")]
        );
        fs::remove_dir_all(root).unwrap();
    }
}
