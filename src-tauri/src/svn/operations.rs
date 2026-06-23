use crate::{DiffResult, SvnInfo, SvnLogEntry, SvnStatus};
use serde::{Deserialize, Serialize};

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

pub async fn log(
    path: &str,
    limit: Option<u32>,
    start_rev: Option<u64>,
    end_rev: Option<u64>,
    keyword: Option<&str>,
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Result<Vec<SvnLogEntry>, SvnError> {
    let mut args: Vec<String> = vec!["log".to_string(), "--xml".to_string(), "--verbose".to_string()];

    if let Some(lim) = limit {
        args.push("-l".to_string());
        args.push(lim.to_string());
    }

    // 日期范围优先于修订号范围（服务器端过滤）
    if let Some(from) = date_from {
        let end = date_to.unwrap_or("HEAD");
        let range = if let Some(rev) = start_rev {
            // load-more：用修订号作为上界，但保持下界为开始日期
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
    }

    // 关键字搜索（--search 自 Subversion 1.8 支持）
    if let Some(kw) = keyword {
        if !kw.is_empty() {
            args.push("--search".to_string());
            args.push(kw.to_string());
        }
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = execute_svn(&args_refs, Some(path)).await?;
    parse_log_xml(&output)
}

pub async fn info(path: &str) -> Result<SvnInfo, SvnError> {
    let args = vec!["info", "--xml"];
    let output = execute_svn(&args, Some(path)).await?;
    parse_info_xml(&output)
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
    let args = build_diff_args(file, old_rev, new_rev)?;
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = execute_svn(&args_refs, Some(workspace)).await?;
    let (result_old_rev, result_new_rev) = match (old_rev, new_rev) {
        (Some(old), Some(new)) => (old, new),
        (Some(change), None) => (change.saturating_sub(1), change),
        _ => (0, 0),
    };

    Ok(DiffResult {
        path: file.to_string(),
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
    use super::{append_targets, build_diff_args, commit, normalize_diff_target};
    use crate::svn::executor::SvnError;

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
}
