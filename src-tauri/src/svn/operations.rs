use crate::{DiffResult, SvnAuthUser, SvnInfo, SvnLogEntry, SvnStatus};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::executor::{execute_svn, SvnError};
use super::parser::{parse_blame_text, parse_info_xml, parse_log_xml, parse_status_xml};

fn append_targets(args: &mut Vec<String>, targets: &[String]) {
    if targets.is_empty() {
        return;
    }
    args.push("--".to_string());
    args.extend(targets.iter().cloned());
}

fn normalize_diff_target(file: &str) -> String {
    if file.contains("://") || !file.contains('@') || file.ends_with('@') {
        file.to_string()
    } else {
        format!("{}@", file)
    }
}

fn normalize_svn_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn normalize_workspace_diff_file(workspace: &str, file: &str) -> Result<String, SvnError> {
    if file.contains("://") {
        return Ok(file.to_string());
    }

    let file_path = Path::new(file);
    if !file_path.is_absolute() {
        return Ok(normalize_svn_path(file));
    }

    let workspace_path = Path::new(workspace)
        .canonicalize()
        .map_err(|e| SvnError::CommandFailed(format!("无法访问工作副本目录：{}", e)))?;
    let canonical_file = file_path
        .canonicalize()
        .map_err(|e| SvnError::CommandFailed(format!("无法访问文件：{}", e)))?;

    if !canonical_file.starts_with(&workspace_path) {
        return Err(SvnError::InvalidArguments(
            "文件必须位于当前工作副本内".to_string(),
        ));
    }

    let relative = canonical_file
        .strip_prefix(&workspace_path)
        .map_err(|e| SvnError::CommandFailed(format!("无法解析工作副本相对路径：{}", e)))?;

    Ok(normalize_svn_path(&relative.to_string_lossy()))
}

fn build_diff_args(
    file: &str,
    old_rev: Option<u64>,
    new_rev: Option<u64>,
) -> Result<Vec<String>, SvnError> {
    let mut args = vec!["diff".to_string()];

    match (old_rev, new_rev) {
        (Some(old), Some(new)) => {
            args.push("-r".to_string());
            args.push(format!("{}:{}", old, new));
        }
        (Some(change), None) => {
            args.push("-c".to_string());
            args.push(change.to_string());
        }
        (None, Some(_)) => {
            return Err(SvnError::InvalidArguments(
                "新版本不能在没有旧版本的情况下单独指定".to_string(),
            ));
        }
        (None, None) => {}
    }

    append_targets(&mut args, &[normalize_diff_target(file)]);
    Ok(args)
}

pub async fn checkout(url: &str, path: &str, revision: Option<u64>) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["checkout".to_string(), url.to_string(), path.to_string()];

    if let Some(rev) = revision {
        args.insert(1, "-r".to_string());
        args.insert(2, rev.to_string());
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, None).await
}

pub async fn update(path: &str, revision: Option<u64>) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["update".to_string()];

    if let Some(rev) = revision {
        args.push("-r".to_string());
        args.push(rev.to_string());
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

fn resolve_workspace_file_path(workspace: &str, file: &str) -> Result<PathBuf, SvnError> {
    let workspace_path = Path::new(workspace)
        .canonicalize()
        .map_err(|e| SvnError::CommandFailed(format!("无法访问工作副本目录：{}", e)))?;
    let file_path = workspace_path.join(file);
    let canonical_file = file_path
        .canonicalize()
        .map_err(|e| SvnError::CommandFailed(format!("无法访问文件：{}", e)))?;

    if !canonical_file.starts_with(&workspace_path) {
        return Err(SvnError::InvalidArguments(
            "文件必须位于当前工作副本内".to_string(),
        ));
    }

    Ok(canonical_file)
}

fn build_unversioned_file_diff(file: &str, content: &str) -> String {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let has_trailing_newline = normalized.ends_with('\n');
    let mut lines: Vec<&str> = normalized.split('\n').collect();
    if has_trailing_newline {
        lines.pop();
    }

    let mut diff = String::new();
    diff.push_str(&format!("Index: {}\n", file));
    diff.push_str("===================================================================\n");
    diff.push_str(&format!("--- {}\t(nonexistent)\n", file));
    diff.push_str(&format!("+++ {}\t(working copy)\n", file));
    diff.push_str(&format!("@@ -0,0 +1,{} @@\n", lines.len()));
    for line in &lines {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    if !has_trailing_newline && !lines.is_empty() {
        diff.push_str("\\ No newline at end of file\n");
    }
    diff
}

fn workspace_child_exists(workspace: &str, file: &str) -> bool {
    let Ok(root) = Path::new(workspace).canonicalize() else {
        return false;
    };
    let Ok(child) = root.join(file).canonicalize() else {
        return false;
    };
    child.starts_with(&root)
}

async fn is_unversioned_target(workspace: &str, file: &str) -> Result<bool, SvnError> {
    let target = normalize_diff_target(file);
    let args = vec!["status", "--xml", "--", target.as_str()];
    match execute_svn(&args, Some(workspace)).await {
        Ok(output) => {
            let statuses = parse_status_xml(&output)?;
            let normalized_file = normalize_svn_path(file);
            let marked = statuses.iter().any(|status| {
                status.status_code == "unversioned"
                    && normalize_svn_path(&status.path) == normalized_file
            }) || (statuses.len() == 1 && statuses[0].status_code == "unversioned");
            if marked {
                return Ok(true);
            }
            // A file inside an unversioned directory yields an empty status
            // target instead of E155010. An unchanged versioned file does too,
            // so confirm with info before treating the path as new content.
            if statuses.is_empty() && workspace_child_exists(workspace, file) {
                return match execute_svn(&["info", "--xml", "--", target.as_str()], Some(workspace))
                    .await
                {
                    Ok(_) => Ok(false),
                    Err(error) => {
                        let text = error.to_string();
                        if text.contains("E155010")
                            || text.contains("W155010")
                            || text.contains("was not found")
                            || text.contains("not a working copy")
                        {
                            Ok(true)
                        } else {
                            Err(error)
                        }
                    }
                };
            }
            Ok(false)
        }
        Err(error) => {
            let text = error.to_string();
            if (text.contains("E155010") || text.contains("was not found"))
                && workspace_child_exists(workspace, file)
            {
                Ok(true)
            } else {
                Err(error)
            }
        }
    }
}

fn build_unversioned_directory_diff(file: &str, directory: &Path) -> String {
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(directory) {
        for entry in entries.flatten() {
            let mut name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_dir() {
                name.push('/');
            }
            names.push(name);
        }
    }
    names.sort();
    let mut diff = String::new();
    diff.push_str(&format!("Index: {file}\n"));
    diff.push_str("===================================================================\n");
    diff.push_str(&format!("--- {file}\t(nonexistent)\n"));
    diff.push_str(&format!("+++ {file}\t(unversioned directory)\n"));
    diff.push_str(&format!("@@ -0,0 +1,{} @@\n", names.len()));
    for name in &names {
        diff.push('+');
        diff.push_str(name);
        diff.push('\n');
    }
    diff
}

pub async fn remote_info(path: &str) -> Result<SvnInfo, SvnError> {
    let args = vec!["info", "-r", "HEAD", "--xml"];
    let output = execute_svn(&args, Some(path)).await?;
    parse_info_xml(&output)
}

pub async fn commit(
    path: &str,
    message: &str,
    files: Option<&[String]>,
) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["commit".to_string(), "-m".to_string(), message.to_string()];

    if let Some(file_list) = files {
        if file_list.is_empty() {
            return Err(SvnError::InvalidArguments(
                "显式文件列表不能为空；省略文件列表可提交整个工作副本".to_string(),
            ));
        }
        // SVN directory deletion/replacement is atomic even with --depth empty.
        // Validate against fresh status so deselected descendants cannot slip in.
        let statuses = status(path).await?;
        let selected: HashSet<String> = file_list
            .iter()
            .map(|file| {
                normalize_svn_path(file)
                    .trim_start_matches("./")
                    .trim_end_matches('/')
                    .to_string()
            })
            .collect();
        for entry in &statuses {
            let target = normalize_svn_path(&entry.path);
            if !selected.contains(&target) {
                continue;
            }
            if entry.history && Path::new(path).join(&entry.path).is_dir() {
                return Err(SvnError::InvalidArguments(format!(
                    "复制或移动的目录 {} 包含隐式子项，暂不支持通过部分提交处理；请使用 SVN 客户端提交完整的复制或移动操作", entry.path
                )));
            }
            if matches!(entry.status_code.as_str(), "deleted" | "replaced") {
                let prefix = format!("{}/", target);
                if statuses.iter().any(|child| {
                    let child_path = normalize_svn_path(&child.path);
                    child_path.starts_with(&prefix) && !selected.contains(&child_path)
                }) {
                    return Err(SvnError::InvalidArguments(format!(
                        "目录 {} 的删除或替换必须包含全部子项，请完整勾选该目录或取消该目录的选择",
                        entry.path
                    )));
                }
            }
        }
        args.extend(["--depth".to_string(), "empty".to_string()]);
        append_targets(&mut args, file_list);
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

pub async fn status(path: &str) -> Result<Vec<SvnStatus>, SvnError> {
    let output = execute_svn(&["status", "--xml"], Some(path)).await?;
    let mut entries = parse_status_xml(&output)?;
    // Default status collapses deleted directories. Expand only affected
    // targets, avoiding a second verbose traversal of the whole workspace.
    let targets: Vec<_> = entries
        .iter()
        .filter(|entry| matches!(entry.status_code.as_str(), "deleted" | "replaced"))
        .map(|entry| normalize_diff_target(&entry.path))
        .collect();
    for group in targets.chunks(16) {
        let mut args: Vec<String> = ["status", "--xml", "--verbose", "--"]
            .iter()
            .map(|arg| arg.to_string())
            .collect();
        args.extend(group.iter().cloned());
        let refs: Vec<_> = args.iter().map(String::as_str).collect();
        let output = execute_svn(&refs, Some(path)).await?;
        entries.extend(parse_status_xml(&output)?);
    }
    let mut changed = std::collections::BTreeMap::new();
    for entry in entries {
        if !matches!(entry.status_code.as_str(), "normal" | "none")
            || matches!(entry.prop_status.as_str(), "modified" | "conflicted")
        {
            changed.insert(entry.path.clone(), entry);
        }
    }
    Ok(changed.into_values().collect())
}

/// Query only exact paths. The filesystem event queue chooses these
/// targets; a complete status scan remains the fallback for unknown scope.
pub(super) async fn status_for_paths(
    path: &str,
    paths: &[String],
) -> Result<Vec<SvnStatus>, SvnError> {
    let mut entries = Vec::new();
    // Batch the identified paths; oversized platform argv limits fall back
    // to a complete scan in the caller.
    for group in paths.chunks(64) {
        let mut args: Vec<String> = ["status", "--xml", "--depth", "empty", "--"]
            .iter()
            .map(|arg| arg.to_string())
            .collect();
        args.extend(group.iter().map(|target| normalize_diff_target(target)));
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = execute_svn(&refs, Some(path)).await?;
        entries.extend(parse_status_xml(&output)?);
    }
    Ok(entries
        .into_iter()
        .filter(|entry| {
            !matches!(entry.status_code.as_str(), "normal" | "none" | "ignored")
                || matches!(entry.prop_status.as_str(), "modified" | "conflicted")
        })
        .collect())
}

fn build_log_args(
    limit: Option<u32>,
    start_rev: Option<u64>,
    end_rev: Option<u64>,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "log".to_string(),
        "--xml".to_string(),
        "--verbose".to_string(),
    ];

    if let Some(lim) = limit {
        args.push("-l".to_string());
        args.push(lim.to_string());
    }

    // Scan newest first so filtered batches and their revision cursor agree.
    if date_from.is_some() || date_to.is_some() {
        let newest = if let Some(rev) = start_rev {
            rev.to_string()
        } else if let Some(to) = date_to {
            // The date picker supplies a day, while SVN date revisions use an
            // instant. Include commits throughout the selected end day.
            format!("{{{}T23:59:59.999999}}", to)
        } else {
            "HEAD".to_string()
        };
        let oldest = date_from
            .map(|from| format!("{{{}}}", from))
            .unwrap_or_else(|| "1".to_string());
        args.push("-r".to_string());
        args.push(format!("{}:{}", newest, oldest));
    } else if let Some(start) = start_rev {
        if let Some(end) = end_rev {
            args.push("-r".to_string());
            args.push(format!("{}:{}", start, end));
        } else {
            args.push("-r".to_string());
            args.push(format!("{}:HEAD", start));
        }
    } else {
        args.push("-r".to_string());
        args.push("HEAD:1".to_string());
    }

    args
}

async fn fetch_log_batch(
    path: &str,
    limit: Option<u32>,
    start_rev: Option<u64>,
    end_rev: Option<u64>,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Result<Vec<SvnLogEntry>, SvnError> {
    let args = build_log_args(limit, start_rev, end_rev, date_from, date_to);
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = execute_svn(&args_refs, Some(path)).await?;
    let entries = parse_log_xml(&output)?;
    Ok(entries
        .into_iter()
        .filter(|entry| log_entry_matches_dates(entry, date_from, date_to))
        .collect())
}

fn log_entry_matches_dates(
    entry: &SvnLogEntry,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> bool {
    if date_from.is_none() && date_to.is_none() {
        return true;
    }
    let Ok(timestamp) = DateTime::parse_from_rfc3339(&entry.date) else {
        return false;
    };
    let local_day = timestamp
        .with_timezone(&Local)
        .format("%Y-%m-%d")
        .to_string();
    date_from.is_none_or(|from| local_day.as_str() >= from)
        && date_to.is_none_or(|to| local_day.as_str() <= to)
}

fn log_entry_matches(entry: &SvnLogEntry, keyword: Option<&str>, author: Option<&str>) -> bool {
    if let Some(author_filter) = author.map(str::trim).filter(|value| !value.is_empty()) {
        if !entry.author.eq_ignore_ascii_case(author_filter) {
            return false;
        }
    }

    let Some(keyword) = keyword.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };
    let keyword = keyword.to_lowercase();

    entry.message.to_lowercase().contains(&keyword)
}

fn append_unique_log_matches(
    matches: &mut Vec<SvnLogEntry>,
    seen_revisions: &mut HashSet<u64>,
    batch: &[SvnLogEntry],
    keyword: Option<&str>,
    author: Option<&str>,
    limit: Option<u32>,
) -> bool {
    for entry in batch {
        if log_entry_matches(entry, keyword, author) && seen_revisions.insert(entry.revision) {
            matches.push(entry.clone());
            if limit.is_some_and(|limit| matches.len() >= limit as usize) {
                return true;
            }
        }
    }

    false
}

#[allow(clippy::too_many_arguments)]
async fn filtered_log(
    path: &str,
    limit: Option<u32>,
    start_rev: Option<u64>,
    end_rev: Option<u64>,
    keyword: Option<&str>,
    author: Option<&str>,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Result<Vec<SvnLogEntry>, SvnError> {
    let scan_limit = limit.unwrap_or(200).saturating_mul(4).max(200);
    let mut cursor = start_rev;
    let mut matches = Vec::new();
    let mut seen_revisions = HashSet::new();

    loop {
        let batch =
            fetch_log_batch(path, Some(scan_limit), cursor, end_rev, date_from, date_to).await?;
        if batch.is_empty() {
            break;
        }

        if append_unique_log_matches(
            &mut matches,
            &mut seen_revisions,
            &batch,
            keyword,
            author,
            limit,
        ) {
            return Ok(matches);
        }

        let oldest_revision = batch.last().map(|entry| entry.revision).unwrap_or(1);
        if oldest_revision <= 1 || batch.len() < scan_limit as usize {
            break;
        }
        cursor = Some(oldest_revision - 1);
    }

    Ok(matches)
}

#[allow(clippy::too_many_arguments)]
pub async fn log(
    path: &str,
    limit: Option<u32>,
    start_rev: Option<u64>,
    end_rev: Option<u64>,
    keyword: Option<&str>,
    author: Option<&str>,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Result<Vec<SvnLogEntry>, SvnError> {
    let has_keyword = keyword.is_some_and(|value| !value.trim().is_empty());
    let has_author = author.is_some_and(|value| !value.trim().is_empty());
    if has_keyword || has_author {
        return filtered_log(
            path,
            limit.map(|limit| limit.max(1)),
            start_rev,
            end_rev,
            keyword,
            author,
            date_from,
            date_to,
        )
        .await;
    }

    fetch_log_batch(path, limit, start_rev, end_rev, date_from, date_to).await
}

pub async fn current_user(path: &str) -> Result<Option<SvnAuthUser>, SvnError> {
    let info = info(path).await?;
    let output = execute_svn(&["auth"], Some(path)).await?;
    Ok(parse_auth_user(&output, &info.repository_root))
}

pub async fn info(path: &str) -> Result<SvnInfo, SvnError> {
    let args = vec!["info", "--xml"];
    let output = execute_svn(&args, Some(path)).await?;
    parse_info_xml(&output)
}

/// Highest revision already present in the working copy.
///
/// `svn info` on the root reports the directory base. A partial commit
/// (`svn commit --depth empty` with an explicit file list) advances the
/// committed files but leaves that base behind, so the update page would
/// treat the revision just created as incoming. `svnversion` prints the
/// mixed range (`2:3M`); the upper bound is the newest revision the WC has.
pub fn parse_svnversion_upper(output: &str) -> Option<u64> {
    let token = output.split_whitespace().next()?;
    let numeric: String = token
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == ':')
        .collect();
    let upper = numeric.split(':').next_back()?;
    if upper.is_empty() {
        None
    } else {
        upper.parse().ok()
    }
}

pub async fn local_revision(path: &str) -> Result<u64, SvnError> {
    let version_output =
        super::executor::execute_program(super::executor::svnversion_executable(), &[path], None)
            .await;
    if let Ok(output) = version_output {
        if let Some(revision) = parse_svnversion_upper(&output) {
            return Ok(revision);
        }
    }

    let args = vec!["info", "--show-item", "revision"];
    let output = execute_svn(&args, Some(path)).await?;
    output
        .trim()
        .parse()
        .map_err(|_| SvnError::ParseError(format!("无法解析本地工作副本版本：{}", output.trim())))
}

pub fn parse_update_conflicts(output: &str) -> Vec<String> {
    let mut conflicts = Vec::new();
    for line in output.lines() {
        if let Some(path) = conflict_path_from_update_line(line) {
            if !conflicts.contains(&path) {
                conflicts.push(path);
            }
        }
    }
    if conflicts.is_empty() {
        for line in output.lines() {
            let trimmed = line.trim();
            if conflict_summary_count(trimmed).is_some_and(|count| count > 0) {
                conflicts.push(trimmed.to_string());
            }
        }
    }
    conflicts
}

fn conflict_path_from_update_line(line: &str) -> Option<String> {
    let split = line.find("  ")?;
    let status = line[..split].trim();
    let path = line[split..].trim();
    if status.is_empty()
        || path.is_empty()
        || !status.chars().all(|ch| ch.is_ascii_uppercase())
        || !status.contains('C')
    {
        return None;
    }
    Some(path.to_string())
}

fn conflict_summary_count(line: &str) -> Option<u64> {
    const LABELS: [&str; 3] = ["Text conflicts:", "Tree conflicts:", "Property conflicts:"];
    let count = LABELS
        .iter()
        .find_map(|label| line.strip_prefix(label))?
        .trim();
    count.parse().ok()
}

fn parse_auth_user(output: &str, repository_root: &str) -> Option<SvnAuthUser> {
    let root_key = normalize_url_key(repository_root);
    let host_key = extract_url_host_key(repository_root);

    output
        .split("------------------------------------------------------------------------")
        .filter_map(|block| {
            let mut realm = "";
            let mut username = "";
            for line in block.lines() {
                if let Some(value) = line.strip_prefix("Authentication realm:") {
                    realm = value.trim();
                } else if let Some(value) = line.strip_prefix("Username:") {
                    username = value.trim();
                }
            }
            if username.is_empty() {
                return None;
            }

            let normalized_realm = normalize_url_key(realm);
            let matches_root = !root_key.is_empty() && normalized_realm.contains(&root_key);
            let matches_host = host_key
                .as_ref()
                .is_some_and(|host| normalized_realm.contains(host));
            if matches_root || matches_host {
                Some(SvnAuthUser {
                    username: username.to_string(),
                    realm: realm.to_string(),
                })
            } else {
                None
            }
        })
        .next()
}

fn normalize_url_key(value: &str) -> String {
    value
        .trim()
        .trim_matches('<')
        .trim_matches('>')
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn extract_url_host_key(value: &str) -> Option<String> {
    let normalized = normalize_url_key(value);
    let (_, rest) = normalized.split_once("://")?;
    let host = rest.split('/').next()?.trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlameLine {
    pub revision: u64,
    pub author: String,
    pub line: String,
}

pub async fn blame(workspace: &str, file: &str) -> Result<Vec<BlameLine>, SvnError> {
    let args = vec!["blame", "--", file];
    let output = execute_svn(&args, Some(workspace)).await?;
    parse_blame_text(&output)
}

pub async fn diff(
    workspace: &str,
    file: &str,
    old_rev: Option<u64>,
    new_rev: Option<u64>,
) -> Result<DiffResult, SvnError> {
    let diff_file = normalize_workspace_diff_file(workspace, file)?;

    if old_rev.is_none()
        && new_rev.is_none()
        && is_unversioned_target(workspace, &diff_file).await?
    {
        let file_path = resolve_workspace_file_path(workspace, &diff_file)?;
        let diff = if file_path.is_dir() {
            build_unversioned_directory_diff(&diff_file, &file_path)
        } else {
            let content = fs::read(&file_path)
                .map_err(|e| SvnError::CommandFailed(format!("无法读取未版本文件：{}", e)))?;
            let content = String::from_utf8_lossy(&content);
            build_unversioned_file_diff(&diff_file, content.as_ref())
        };
        return Ok(DiffResult {
            path: diff_file,
            diff,
            old_revision: 0,
            new_revision: 0,
        });
    }

    let args = build_diff_args(&diff_file, old_rev, new_rev)?;
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = execute_svn(&args_refs, Some(workspace)).await?;
    let (result_old_rev, result_new_rev) = match (old_rev, new_rev) {
        (Some(old), Some(new)) => (old, new),
        (Some(change), None) => (change.saturating_sub(1), change),
        _ => (0, 0),
    };

    Ok(DiffResult {
        path: diff_file,
        diff: output,
        old_revision: result_old_rev,
        new_revision: result_new_rev,
    })
}

fn patch_output_has_conflicts(output: &str) -> bool {
    output.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("C ")
            || trimmed.contains(".svnpatch.rej")
            || trimmed.to_ascii_lowercase().contains("rejected hunk")
    })
}

pub async fn apply_patch(workspace: &str, patch: &str, reverse: bool) -> Result<String, SvnError> {
    if patch.trim().is_empty() {
        return Err(SvnError::InvalidArguments("补丁内容不能为空".to_string()));
    }

    let workspace_path = Path::new(workspace)
        .canonicalize()
        .map_err(|e| SvnError::CommandFailed(format!("无法访问工作副本目录：{}", e)))?;
    if !workspace_path.is_dir() {
        return Err(SvnError::InvalidArguments(
            "工作副本路径必须是目录".to_string(),
        ));
    }

    let patch_path = std::env::temp_dir().join(format!(
        "orcasvn-stash-{}-{}.patch",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    fs::write(&patch_path, patch)
        .map_err(|e| SvnError::CommandFailed(format!("无法创建临时补丁文件：{}", e)))?;

    let mut args = vec!["patch".to_string()];
    if reverse {
        args.push("--reverse-diff".to_string());
    }
    args.push(patch_path.to_string_lossy().into_owned());
    let workspace_cwd = workspace_path.to_string_lossy().into_owned();

    let mut dry_run_args = args.clone();
    dry_run_args.insert(1, "--dry-run".to_string());
    let dry_run_refs: Vec<&str> = dry_run_args.iter().map(|arg| arg.as_str()).collect();
    let dry_run = execute_svn(&dry_run_refs, Some(&workspace_cwd)).await;
    if let Ok(output) = &dry_run {
        if patch_output_has_conflicts(output) {
            let _ = fs::remove_file(&patch_path);
            return Err(SvnError::CommandFailed(
                "补丁与当前工作副本冲突，未应用任何更改".to_string(),
            ));
        }
    }
    if let Err(error) = dry_run {
        let _ = fs::remove_file(&patch_path);
        return Err(error);
    }

    let args_refs: Vec<&str> = args.iter().map(|arg| arg.as_str()).collect();
    let result = execute_svn(&args_refs, Some(&workspace_cwd))
        .await
        .and_then(|output| {
            if patch_output_has_conflicts(&output) {
                Err(SvnError::CommandFailed(
                    "补丁应用时发生冲突，贮藏记录已保留".to_string(),
                ))
            } else {
                Ok(output)
            }
        });
    let _ = fs::remove_file(&patch_path);
    result
}

const FALSE_BINARY_READ_LIMIT: u64 = 8 * 1024 * 1024;

/// SVN marks a file `application/octet-stream` when most bytes are non-ASCII,
/// which includes ordinary UTF-8 text that is entirely CJK. That blocks diff
/// and blame. Only undo the mark for files this add just scheduled, and only
/// when the bytes are conservative UTF-8 text.
fn is_conservative_utf8_text(bytes: &[u8]) -> bool {
    if bytes.contains(&0) {
        return false;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let mut controls = 0usize;
    let mut total = 0usize;
    for ch in text.chars() {
        total += 1;
        if ch.is_control() && !matches!(ch, '\n' | '\r' | '\t') {
            controls += 1;
        }
    }
    controls == 0 || controls.saturating_mul(20) <= total
}

async fn clear_autodetected_binary_mime(
    workspace: &str,
    targets: &[String],
) -> Result<(), SvnError> {
    if targets.is_empty() {
        return Ok(());
    }
    let mut args = vec!["status".to_string(), "--xml".to_string(), "--".to_string()];
    args.extend(targets.iter().cloned());
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = execute_svn(&refs, Some(workspace)).await?;
    let statuses = parse_status_xml(&output)?;
    let root = Path::new(workspace);
    for entry in statuses {
        if entry.status_code != "added" {
            continue;
        }
        let full = root.join(&entry.path);
        if !full.is_file() {
            continue;
        }
        let length = fs::metadata(&full)
            .map_err(|error| SvnError::CommandFailed(format!("无法读取已添加文件：{error}")))?
            .len();
        if length > FALSE_BINARY_READ_LIMIT {
            continue;
        }
        let bytes = fs::read(&full)
            .map_err(|error| SvnError::CommandFailed(format!("无法读取已添加文件：{error}")))?;
        if !is_conservative_utf8_text(&bytes) {
            continue;
        }
        let target = normalize_diff_target(&entry.path);
        let mime = execute_svn(
            &["propget", "svn:mime-type", "--", target.as_str()],
            Some(workspace),
        )
        .await;
        let Ok(mime) = mime else {
            continue;
        };
        if mime.trim() != "application/octet-stream" {
            continue;
        }
        execute_svn(
            &["propdel", "svn:mime-type", "--", target.as_str()],
            Some(workspace),
        )
        .await?;
    }
    Ok(())
}

pub async fn add(path: &str, files: &[String]) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["add".to_string()];
    append_targets(&mut args, files);
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = execute_svn(&args_refs, Some(path)).await?;
    clear_autodetected_binary_mime(path, files).await?;
    Ok(output)
}

pub async fn delete(path: &str, files: &[String]) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["delete".to_string()];
    append_targets(&mut args, files);
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

pub async fn revert(path: &str, files: &[String]) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["revert".to_string()];
    append_targets(&mut args, files);
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

pub async fn resolve(path: &str, files: &[String], strategy: &str) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec![
        "resolve".to_string(),
        "--accept".to_string(),
        strategy.to_string(),
    ];
    append_targets(&mut args, files);
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

pub async fn cleanup(path: &str) -> Result<String, SvnError> {
    let args = vec!["cleanup"];
    execute_svn(&args, Some(path)).await
}

pub async fn switch_cmd(path: &str, url: &str) -> Result<String, SvnError> {
    let args = vec!["switch", url];
    execute_svn(&args, Some(path)).await
}

pub async fn merge(
    path: &str,
    source: &str,
    rev_start: u64,
    rev_end: u64,
) -> Result<String, SvnError> {
    let revision = format!("{rev_start}:{rev_end}");
    let args = [
        "merge".to_string(),
        "-r".to_string(),
        revision,
        source.to_string(),
    ];
    let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    execute_svn(&args_refs, Some(path)).await
}

#[cfg(test)]
mod tests {
    use super::{
        add, append_targets, append_unique_log_matches, blame, build_diff_args, build_log_args,
        build_unversioned_file_diff, commit, diff, is_conservative_utf8_text, local_revision,
        log_entry_matches, log_entry_matches_dates, normalize_diff_target, normalize_svn_path,
        parse_svnversion_upper, parse_update_conflicts, patch_output_has_conflicts,
    };
    use crate::svn::executor::SvnError;
    use crate::{SvnLogEntry, SvnLogPath};
    use std::collections::HashSet;

    #[test]
    fn target_arguments_are_separated_from_options() {
        let mut args = vec!["add".to_string()];
        append_targets(
            &mut args,
            &["-leading-dash.txt".to_string(), "src/main.rs".to_string()],
        );
        assert_eq!(args, ["add", "--", "-leading-dash.txt", "src/main.rs"]);
    }

    #[test]
    fn empty_targets_do_not_add_separator() {
        let mut args = vec!["commit".to_string()];
        append_targets(&mut args, &[]);
        assert_eq!(args, ["commit"]);
    }

    #[test]
    fn patch_conflicts_are_detected_before_pop_removes_the_entry() {
        assert!(patch_output_has_conflicts(
            "C         src/main.rs\n>         rejected hunk @@ -1 +1 @@"
        ));
        assert!(patch_output_has_conflicts("path/to/file.svnpatch.rej"));
        assert!(patch_output_has_conflicts("> rejected hunk @@ -2 +2 @@"));
        assert!(!patch_output_has_conflicts(
            "U         src/main.rs\n> applied hunk with offset 2"
        ));
    }

    #[test]
    fn commit_rejects_an_explicit_empty_file_list() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(commit(".", "message", Some(&[])));
        assert!(matches!(result, Err(SvnError::InvalidArguments(_))));
    }

    // Uses an isolated local repository; never touches a user's working copy.
    #[test]
    fn selected_commit_preserves_unselected_children_and_other_directories() {
        use std::process::Command;
        if Command::new("svnadmin").arg("--version").output().is_err() {
            eprintln!("Skipping local repository test: svnadmin is unavailable");
            return;
        }
        let directory = std::env::temp_dir().join(format!(
            "orcasvn-commit-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let repository = directory.join("repository");
        let wc = directory.join("wc");
        let created = Command::new("svnadmin")
            .arg("create")
            .arg(&repository)
            .output()
            .unwrap();
        assert!(created.status.success());
        let url = format!(
            "file:///{}",
            repository
                .to_string_lossy()
                .replace('\\', "/")
                .trim_start_matches('/')
        );
        let run = |args: &[&str]| {
            let output = Command::new("svn")
                .args(args)
                .current_dir(if wc.exists() { &wc } else { &directory })
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).into_owned()
        };
        run(&["checkout", &url, wc.to_str().unwrap()]);
        std::fs::create_dir_all(wc.join("A")).unwrap();
        std::fs::create_dir_all(wc.join("B")).unwrap();
        for name in ["A/selected.txt", "A/keep.txt", "B/keep.txt"] {
            std::fs::write(wc.join(name), "before").unwrap();
        }
        run(&["add", "A", "B"]);
        run(&["commit", "-m", "initial"]);
        for name in ["A/selected.txt", "A/keep.txt", "B/keep.txt"] {
            std::fs::write(wc.join(name), "after").unwrap();
        }
        run(&["propset", "test:property", "changed", "A"]);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime
            .block_on(commit(
                wc.to_str().unwrap(),
                "selected",
                Some(&["A".into(), "A/selected.txt".into()]),
            ))
            .unwrap();
        let remaining = run(&["status"]);
        assert!(remaining.contains("A/keep.txt") || remaining.contains("A\\keep.txt"));
        assert!(remaining.contains("B/keep.txt") || remaining.contains("B\\keep.txt"));
        assert!(!remaining.contains("selected.txt"));
        assert_eq!(run(&["cat", "-r", "HEAD", "A/keep.txt"]), "before");
        assert_eq!(run(&["cat", "-r", "HEAD", "A/selected.txt"]), "after");
        std::fs::create_dir(wc.join("A/new")).unwrap();
        std::fs::write(wc.join("A/new/chosen.txt"), "chosen").unwrap();
        std::fs::write(wc.join("A/new/excluded.txt"), "excluded").unwrap();
        run(&["add", "A/new"]);
        runtime
            .block_on(commit(
                wc.to_str().unwrap(),
                "new directory",
                Some(&["A/new".into(), "A/new/chosen.txt".into()]),
            ))
            .unwrap();
        assert!(run(&["status"]).contains("excluded.txt"));
        assert!(!run(&["list", "-r", "HEAD", "A/new"]).contains("excluded.txt"));
        run(&["update"]);
        run(&["delete", "--force", "A"]);
        let partial_delete = runtime.block_on(commit(
            wc.to_str().unwrap(),
            "partial deletion",
            Some(&["A".into(), "A/selected.txt".into()]),
        ));
        assert!(matches!(partial_delete, Err(SvnError::InvalidArguments(_))));
        assert!(run(&["list", "-r", "HEAD", &url]).contains("A/"));
        let deletions = runtime
            .block_on(super::status(wc.to_str().unwrap()))
            .unwrap()
            .into_iter()
            .filter(|entry| {
                entry.path == "A" || entry.path.starts_with("A/") || entry.path.starts_with("A\\")
            })
            .map(|entry| entry.path)
            .collect::<Vec<_>>();
        runtime
            .block_on(commit(
                wc.to_str().unwrap(),
                "complete deletion",
                Some(&deletions),
            ))
            .unwrap();
        assert!(!run(&["list", "-r", "HEAD", &url]).contains("A/"));
        run(&["copy", "B", "C"]);
        let copied_directory = runtime.block_on(commit(
            wc.to_str().unwrap(),
            "copied directory",
            Some(&["C".into()]),
        ));
        assert!(matches!(
            copied_directory,
            Err(SvnError::InvalidArguments(_))
        ));
        assert!(!run(&["list", "-r", "HEAD", &url]).contains("C/"));
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn diff_builds_working_copy_arguments() {
        assert_eq!(
            build_diff_args("src/main.rs", None, None).unwrap(),
            ["diff", "--", "src/main.rs"]
        );
    }

    #[test]
    fn diff_builds_revision_range_arguments() {
        assert_eq!(
            build_diff_args("src/main.rs", Some(10), Some(12)).unwrap(),
            ["diff", "-r", "10:12", "--", "src/main.rs"]
        );
    }

    #[test]
    fn diff_builds_change_arguments() {
        assert_eq!(
            build_diff_args("https://example.test/repo/file@12", Some(12), None).unwrap(),
            [
                "diff",
                "-c",
                "12",
                "--",
                "https://example.test/repo/file@12"
            ]
        );
    }

    #[test]
    fn diff_rejects_new_revision_without_old_revision() {
        assert!(matches!(
            build_diff_args("src/main.rs", None, Some(12)),
            Err(SvnError::InvalidArguments(_))
        ));
    }

    #[test]
    fn diff_escapes_peg_revision_marker_in_local_path() {
        assert_eq!(
            normalize_diff_target("src/name@domain.txt"),
            "src/name@domain.txt@"
        );
        assert_eq!(
            normalize_diff_target("src/name@domain.txt@"),
            "src/name@domain.txt@"
        );
    }

    #[test]
    fn diff_keeps_url_peg_revision() {
        assert_eq!(
            normalize_diff_target("https://example.test/repo/file@12"),
            "https://example.test/repo/file@12"
        );
    }

    #[test]
    fn normalizes_svn_paths_to_forward_slashes() {
        assert_eq!(normalize_svn_path("src\\main.rs"), "src/main.rs");
    }

    #[test]
    fn keeps_relative_diff_target_normalized() {
        assert_eq!(
            super::normalize_workspace_diff_file(".", "src\\main.rs").unwrap(),
            "src/main.rs"
        );
    }

    #[test]
    fn unversioned_file_diff_is_rendered_as_added_file() {
        let diff = build_unversioned_file_diff("src/new.txt", "hello\nworld\n");

        assert!(diff.contains("Index: src/new.txt\n"));
        assert!(diff.contains("--- src/new.txt\t(nonexistent)\n"));
        assert!(diff.contains("+++ src/new.txt\t(working copy)\n"));
        assert!(diff.contains("@@ -0,0 +1,2 @@\n"));
        assert!(diff.contains("+hello\n+world\n"));
    }

    #[test]
    fn log_keyword_does_not_match_changed_path_or_action_label() {
        let entry = SvnLogEntry {
            revision: 8,
            author: "mark_chen".to_string(),
            date: "2026-06-26T00:00:00Z".to_string(),
            message: "seed".to_string(),
            changed_paths: vec![SvnLogPath {
                path: "/trunk/add-file.txt".to_string(),
                action: "A".to_string(),
            }],
        };

        assert!(!log_entry_matches(&entry, Some("add"), None));
        assert!(!log_entry_matches(&entry, Some("新增"), None));
        assert!(!log_entry_matches(&entry, Some("mark"), None));
    }

    #[test]
    fn log_date_range_scans_newest_first_and_includes_end_day() {
        let args = build_log_args(None, None, None, Some("2026-06-01"), Some("2026-06-26"));
        assert_eq!(args[4], "{2026-06-26T23:59:59.999999}:{2026-06-01}");
        let next = build_log_args(
            Some(50),
            Some(99),
            None,
            Some("2026-06-01"),
            Some("2026-06-26"),
        );
        assert_eq!(next[6], "99:{2026-06-01}");
        let end_only = build_log_args(None, None, None, None, Some("2026-06-26"));
        assert_eq!(end_only[4], "{2026-06-26T23:59:59.999999}:1");
    }

    #[test]
    fn log_date_filter_excludes_revision_before_start_day() {
        let entry = SvnLogEntry {
            revision: 8,
            author: "author".to_string(),
            date: "2026-06-25T12:00:00Z".to_string(),
            message: "earlier".to_string(),
            changed_paths: Vec::new(),
        };
        assert!(!log_entry_matches_dates(&entry, Some("2026-06-26"), None));
        assert!(log_entry_matches_dates(&entry, None, Some("2026-06-26")));
    }

    #[test]
    fn log_keyword_matches_commit_message() {
        let entry = SvnLogEntry {
            revision: 4,
            author: "mark_chen".to_string(),
            date: "2026-04-12T00:00:00Z".to_string(),
            message: "conf: add api key".to_string(),
            changed_paths: Vec::new(),
        };

        assert!(log_entry_matches(&entry, Some("add"), None));
    }

    #[test]
    fn log_author_filter_matches_exact_author_case_insensitively() {
        let entry = SvnLogEntry {
            revision: 8,
            author: "mark_chen".to_string(),
            date: "2026-06-26T00:00:00Z".to_string(),
            message: "seed".to_string(),
            changed_paths: Vec::new(),
        };

        assert!(log_entry_matches(&entry, None, Some("MARK_CHEN")));
        assert!(!log_entry_matches(&entry, None, Some("other")));
    }

    #[test]
    fn filtered_log_discards_duplicate_revisions() {
        let entry = SvnLogEntry {
            revision: 8,
            author: "mark_chen".to_string(),
            date: "2026-06-26T00:00:00Z".to_string(),
            message: "seed".to_string(),
            changed_paths: Vec::new(),
        };
        let batch = vec![entry.clone(), entry];
        let mut matches = Vec::new();
        let mut seen_revisions = HashSet::new();

        let reached_limit = append_unique_log_matches(
            &mut matches,
            &mut seen_revisions,
            &batch,
            None,
            Some("mark_chen"),
            None,
        );

        assert!(!reached_limit);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].revision, 8);
    }

    #[test]
    fn svnversion_upper_bound_ignores_mixed_revision_suffixes() {
        assert_eq!(parse_svnversion_upper("3"), Some(3));
        assert_eq!(parse_svnversion_upper("2:3M\n"), Some(3));
        assert_eq!(parse_svnversion_upper("4123:4168MS"), Some(4168));
        assert_eq!(parse_svnversion_upper("0:1P"), Some(1));
        assert_eq!(parse_svnversion_upper("Unversioned directory"), None);
    }

    #[test]
    fn utf8_text_detection_keeps_binaries_and_accepts_cjk() {
        assert!(is_conservative_utf8_text(
            "纯中文内容\n没有英文\n".as_bytes()
        ));
        assert!(is_conservative_utf8_text(
            b"note\n\xe4\xbd\xa0\xe5\xa5\xbd\n"
        ));
        assert!(is_conservative_utf8_text(b""));
        assert!(!is_conservative_utf8_text(b"\0\x01binary"));
        assert!(!is_conservative_utf8_text(&[0xff, 0xfe, 0x00]));
    }

    #[test]
    fn update_conflict_parser_reads_paths_and_summaries() {
        let output = "\
Updating '.':\n\
C    文档/说明.txt\n\
Updated to revision 5.\n\
Summary of conflicts:\n\
  Text conflicts: 1\n";
        assert_eq!(
            parse_update_conflicts(output),
            vec!["文档/说明.txt".to_string()]
        );

        let property = "\
Updating '.':\n\
 C   src/props.txt\n\
Summary of conflicts:\n\
  Property conflicts: 1\n";
        assert_eq!(
            parse_update_conflicts(property),
            vec!["src/props.txt".to_string()]
        );

        let summary_only = "\
Summary of conflicts:\n\
  Tree conflicts: 2\n\
  Text conflicts: 0\n";
        assert_eq!(
            parse_update_conflicts(summary_only),
            vec!["Tree conflicts: 2".to_string()]
        );
        assert!(parse_update_conflicts("Updated to revision 4.\n").is_empty());
    }

    #[test]
    fn partial_commit_revision_text_mime_and_nested_unversioned_diff() {
        for tool in ["svn", "svnadmin", "svnversion"] {
            if std::process::Command::new(tool)
                .arg("--version")
                .output()
                .is_err()
            {
                eprintln!("skipping working-copy fix test: {tool} is unavailable");
                return;
            }
        }

        let directory = std::env::temp_dir().join(format!(
            "orcasvn-fixes-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(directory.clone());
        let repository = directory.join("repo");
        let created = std::process::Command::new("svnadmin")
            .args(["create", repository.to_str().unwrap()])
            .env("LC_ALL", "C.UTF-8")
            .output()
            .unwrap();
        assert!(created.status.success());
        let url = format!("file://{}", repository.display());
        let wc = directory.join("wc");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime
            .block_on(super::checkout(&url, wc.to_str().unwrap(), None))
            .unwrap();
        std::fs::write(wc.join("plain.txt"), "hello\n").unwrap();
        runtime
            .block_on(add(wc.to_str().unwrap(), &["plain.txt".into()]))
            .unwrap();
        runtime
            .block_on(commit(wc.to_str().unwrap(), "initial", None))
            .unwrap();

        std::fs::create_dir(wc.join("文档")).unwrap();
        std::fs::write(wc.join("文档/说明.txt"), "你好，世界\n").unwrap();
        std::fs::write(wc.join("纯中文.txt"), "纯中文内容\n没有英文\n").unwrap();
        std::fs::write(wc.join("binary.bin"), b"\0\x01not-text").unwrap();
        std::fs::write(wc.join("plain.txt"), "hello\nedited\n").unwrap();
        runtime
            .block_on(add(
                wc.to_str().unwrap(),
                &["纯中文.txt".into(), "binary.bin".into()],
            ))
            .unwrap();

        let mime = |name: &str| {
            std::process::Command::new("svn")
                .args(["propget", "svn:mime-type", "--non-interactive", "--", name])
                .current_dir(&wc)
                .env("LC_ALL", "C.UTF-8")
                .output()
                .unwrap()
        };
        let cjk_mime = mime("纯中文.txt");
        assert!(
            !cjk_mime.status.success()
                || !String::from_utf8_lossy(&cjk_mime.stdout).contains("octet-stream"),
            "CJK text kept a binary mime type: {}",
            String::from_utf8_lossy(&cjk_mime.stdout)
        );
        let binary_mime = mime("binary.bin");
        assert!(
            String::from_utf8_lossy(&binary_mime.stdout).contains("octet-stream"),
            "real binary lost its mime type: {}",
            String::from_utf8_lossy(&binary_mime.stderr)
        );

        let text_diff = runtime
            .block_on(diff(wc.to_str().unwrap(), "纯中文.txt", None, None))
            .unwrap();
        assert!(
            text_diff.diff.contains("纯中文内容"),
            "diff hid CJK text: {}",
            text_diff.diff
        );
        assert!(!text_diff.diff.contains("Cannot display"));

        let nested = runtime
            .block_on(diff(wc.to_str().unwrap(), "文档/说明.txt", None, None))
            .unwrap();
        assert!(
            nested.diff.contains("你好，世界"),
            "nested unversioned diff failed: {}",
            nested.diff
        );

        let committed = runtime
            .block_on(commit(
                wc.to_str().unwrap(),
                "只提交说明",
                Some(&["plain.txt".into()]),
            ))
            .unwrap();
        assert!(committed.contains("Committed revision"));
        let reported = runtime
            .block_on(local_revision(wc.to_str().unwrap()))
            .unwrap();
        let directory_revision = std::process::Command::new("svn")
            .args(["info", "--show-item", "revision", "--non-interactive"])
            .current_dir(&wc)
            .env("LC_ALL", "C.UTF-8")
            .output()
            .unwrap();
        let directory_revision = String::from_utf8_lossy(&directory_revision.stdout)
            .trim()
            .parse::<u64>()
            .unwrap();
        assert!(
            reported > directory_revision,
            "local revision {reported} did not advance past the directory base {directory_revision}"
        );

        runtime
            .block_on(commit(
                wc.to_str().unwrap(),
                "提交纯中文",
                Some(&["纯中文.txt".into()]),
            ))
            .unwrap();
        let blamed = runtime
            .block_on(blame(wc.to_str().unwrap(), "纯中文.txt"))
            .unwrap();
        assert!(
            blamed.iter().any(|line| line.line.contains("纯中文内容")),
            "blame hid CJK text: {blamed:?}"
        );
    }
}
