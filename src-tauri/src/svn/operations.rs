use crate::{DiffResult, SvnAuthUser, SvnInfo, SvnLogEntry, SvnStatus};
use serde::{Deserialize, Serialize};
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

async fn is_unversioned_file(workspace: &str, file: &str) -> Result<bool, SvnError> {
    let target = normalize_diff_target(file);
    let args = vec!["status", "--xml", "--", target.as_str()];
    let output = execute_svn(&args, Some(workspace)).await?;
    let statuses = parse_status_xml(&output)?;
    let normalized_file = normalize_svn_path(file);
    Ok(statuses.iter().any(|status| {
        status.status_code == "unversioned" && normalize_svn_path(&status.path) == normalized_file
    }) || (statuses.len() == 1 && statuses[0].status_code == "unversioned"))
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
        append_targets(&mut args, file_list);
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

pub async fn status(path: &str) -> Result<Vec<SvnStatus>, SvnError> {
    let args = vec!["status", "--xml"];
    let output = execute_svn(&args, Some(path)).await?;
    parse_status_xml(&output)
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

    // 日期范围优先于修订号范围（服务器端过滤）。
    if let Some(from) = date_from {
        let end = date_to.unwrap_or("HEAD");
        let range = if let Some(rev) = start_rev {
            // load-more：用修订号作为上界，但保持下界为开始日期。
            format!("{{{}}}:{}", from, rev)
        } else {
            format!("{{{}}}:{{{}}}", from, end)
        };
        args.push("-r".to_string());
        args.push(range);
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
    parse_log_xml(&output)
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

    loop {
        let batch =
            fetch_log_batch(path, Some(scan_limit), cursor, end_rev, date_from, date_to).await?;
        if batch.is_empty() {
            break;
        }

        for entry in &batch {
            if log_entry_matches(entry, keyword, author) {
                matches.push(entry.clone());
                if limit.is_some_and(|limit| matches.len() >= limit as usize) {
                    return Ok(matches);
                }
            }
        }

        let oldest_revision = batch.last().map(|entry| entry.revision).unwrap_or(1);
        if oldest_revision <= 1 || batch.len() < scan_limit as usize {
            break;
        }
        cursor = Some(oldest_revision - 1);
    }

    Ok(matches)
}

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

    if old_rev.is_none() && new_rev.is_none() && is_unversioned_file(workspace, &diff_file).await? {
        let file_path = resolve_workspace_file_path(workspace, &diff_file)?;
        let content = fs::read(&file_path)
            .map_err(|e| SvnError::CommandFailed(format!("无法读取未版本文件：{}", e)))?;
        let content = String::from_utf8_lossy(&content);
        return Ok(DiffResult {
            path: diff_file.clone(),
            diff: build_unversioned_file_diff(&diff_file, content.as_ref()),
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

pub async fn add(path: &str, files: &[String]) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["add".to_string()];
    append_targets(&mut args, files);
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
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
    let args = vec![
        "merge".to_string(),
        "-r".to_string(),
        format!("{}:{}", rev_start, rev_end),
        source.to_string(),
    ];
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

#[cfg(test)]
mod tests {
    use super::{
        append_targets, build_diff_args, build_unversioned_file_diff, commit, log_entry_matches,
        normalize_diff_target, normalize_svn_path,
    };
    use crate::svn::executor::SvnError;
    use crate::{SvnLogEntry, SvnLogPath};

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
    fn commit_rejects_an_explicit_empty_file_list() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("failed to build tokio runtime");
        let result = runtime.block_on(commit(".", "message", Some(&[])));
        assert!(matches!(result, Err(SvnError::InvalidArguments(_))));
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
}
