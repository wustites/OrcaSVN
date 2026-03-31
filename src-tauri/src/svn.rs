use crate::{DiffResult, SvnInfo, SvnLogEntry, SvnStatus};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;
use tokio::task::spawn_blocking;

#[derive(Error, Debug)]
pub enum SvnError {
    #[error("SVN 命令执行失败：{0}")]
    CommandFailed(String),
    #[error("SVN 未安装或不在 PATH 中")]
    SvnNotFound,
    #[error("解析输出失败：{0}")]
    ParseError(#[allow(dead_code)] String),
}

pub async fn execute_svn(args: &[&str], path: Option<&str>) -> Result<String, SvnError> {
    let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let path_str = path.map(|s| s.to_string());

    let result = spawn_blocking(move || {
        let mut cmd = Command::new("svn");
        cmd.args(&args_vec);
        cmd.arg("--non-interactive");
        cmd.arg("--trust-server-cert");

        if let Some(p) = path_str.as_ref() {
            cmd.current_dir(p);
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let output = cmd.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SvnError::SvnNotFound
            } else {
                SvnError::CommandFailed(e.to_string())
            }
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(stdout)
        } else {
            Err(SvnError::CommandFailed(format!(
                "{}\n{}",
                stdout, stderr
            )))
        }
    })
    .await
    .map_err(|e| SvnError::CommandFailed(e.to_string()))??;

    Ok(result)
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

pub async fn commit(path: &str, message: &str, files: Option<&[String]>) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["commit".to_string(), "-m".to_string(), message.to_string()];

    if let Some(file_list) = files {
        for file in file_list {
            args.push(file.clone());
        }
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

pub async fn status(path: &str) -> Result<Vec<SvnStatus>, SvnError> {
    let args = vec!["status", "--xml"];
    let output = execute_svn(&args, Some(path)).await?;
    parse_status_xml(&output)
}

fn parse_status_xml(xml: &str) -> Result<Vec<SvnStatus>, SvnError> {
    let mut statuses = Vec::new();

    // 使用正则表达式解析 XML
    let entry_regex = Regex::new(r#"<entry\s+path="([^"]+)""#).unwrap();
    let status_regex = Regex::new(r#"<wc-status\s+item="([^"]+)"\s+props="([^"]+)"(\s+locked="([^"]+)")?(\s+switched="([^"]+)")?(\s+history="([^"]+)")?"#).unwrap();

    // 按 entry 分割
    let entry_end_regex = Regex::new(r#"</entry>"#).unwrap();

    let mut pos = 0;
    while pos < xml.len() {
        // 在剩余部分查找 <entry>
        let remaining = &xml[pos..];
        if let Some(entry_match) = entry_regex.find(remaining) {
            let entry_start = pos + entry_match.start();
            
            // 找到对应的 </entry>
            if let Some(end_match) = entry_end_regex.find(&xml[entry_start..]) {
                let entry_end = entry_start + end_match.end();
                let entry_xml = &xml[entry_start..entry_end];
                
                // 从 entry 中提取 path
                if let Some(path_cap) = entry_regex.captures(entry_xml) {
                    let path = path_cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                    
                    let status_code;
                    let prop_status;
                    let locked;
                    let switched;
                    let history;

                    if let Some(status_cap) = status_regex.captures(entry_xml) {
                        status_code = status_cap.get(1).map(|m| m.as_str()).unwrap_or(" ").to_string();
                        prop_status = status_cap.get(2).map(|m| m.as_str()).unwrap_or("none").to_string();
                        locked = status_cap.get(4).map(|m| m.as_str() == "true").unwrap_or(false);
                        switched = status_cap.get(6).map(|m| m.as_str() == "true").unwrap_or(false);
                        history = status_cap.get(8).map(|m| m.as_str() == "true").unwrap_or(false);
                    } else {
                        status_code = " ".to_string();
                        prop_status = "none".to_string();
                        locked = false;
                        switched = false;
                        history = false;
                    }

                    statuses.push(SvnStatus {
                        path,
                        status: status_code.clone(),
                        status_code,
                        prop_status,
                        locked,
                        switched,
                        history,
                    });
                }
                
                // 移动到下一个 entry
                pos = entry_end;
            } else {
                // 没有找到 </entry>，退出
                break;
            }
        } else {
            // 没有更多 entry，退出
            break;
        }
    }

    // 如果 XML 解析失败，尝试简单解析命令行输出
    if statuses.is_empty() {
        for line in xml.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("<?") || trimmed.starts_with("<svn") || trimmed.starts_with("<target>") {
                continue;
            }
            if trimmed.starts_with("<") || trimmed.ends_with(">") {
                continue;
            }

            // SVN status 输出格式：第一个字符是状态，第 8 个字符后是路径
            if trimmed.len() >= 9 {
                let status_code = trimmed.chars().next().unwrap_or(' ').to_string();
                let prop_code = trimmed.chars().nth(1).unwrap_or(' ');
                let path = trimmed[8..].trim().to_string();

                if !path.is_empty() {
                    statuses.push(SvnStatus {
                        path,
                        status: status_code.clone(),
                        status_code,
                        prop_status: if prop_code != ' ' { prop_code.to_string() } else { "none".to_string() },
                        locked: trimmed.chars().nth(2) == Some('L'),
                        history: false,
                        switched: false,
                    });
                }
            }
        }
    }

    Ok(statuses)
}

pub async fn log(
    path: &str,
    limit: Option<u32>,
    start_rev: Option<u64>,
    end_rev: Option<u64>,
) -> Result<Vec<SvnLogEntry>, SvnError> {
    let mut args: Vec<String> = vec!["log".to_string(), "--xml".to_string()];

    if let Some(lim) = limit {
        args.push("-l".to_string());
        args.push(lim.to_string());
    }

    if let Some(start) = start_rev {
        if let Some(end) = end_rev {
            args.push("-r".to_string());
            args.push(format!("{}:{}", start, end));
        }
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = execute_svn(&args_refs, Some(path)).await?;
    parse_log_xml(&output)
}

fn parse_log_xml(xml: &str) -> Result<Vec<SvnLogEntry>, SvnError> {
    let mut entries = Vec::new();

    // 使用正则表达式解析 XML
    let logentry_start = Regex::new(r#"<logentry\s+revision="(\d+)""#).unwrap();
    let author_regex = Regex::new(r#"<author>([^<]*)</author>"#).unwrap();
    let date_regex = Regex::new(r#"<date>([^<]*)</date>"#).unwrap();
    let msg_regex = Regex::new(r#"<msg>([^<]*)</msg>"#).unwrap();

    let mut current_pos = 0;

    while let Some(entry_match) = logentry_start.captures(&xml[current_pos..]) {
        let revision = entry_match.get(1).unwrap().as_str().parse::<u64>().unwrap_or(0);

        let entry_end = xml[current_pos..].find("</logentry>")
            .map(|p| current_pos + p + 11)
            .unwrap_or(xml.len());
        let entry_xml = &xml[current_pos..entry_end];

        let author = author_regex.captures(entry_xml)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let date = date_regex.captures(entry_xml)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "".to_string());

        let message = msg_regex.captures(entry_xml)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "".to_string());

        entries.push(SvnLogEntry {
            revision,
            author,
            date,
            message,
        });

        current_pos = entry_end;
    }

    Ok(entries)
}

pub async fn info(path: &str) -> Result<SvnInfo, SvnError> {
    let args = vec!["info", "--xml"];
    let output = execute_svn(&args, Some(path)).await?;
    parse_info_xml(&output)
}

fn parse_info_xml(xml: &str) -> Result<SvnInfo, SvnError> {
    let url_regex = Regex::new(r#"<url>([^<]+)</url>"#).unwrap();
    let root_regex = Regex::new(r#"<root>([^<]+)</root>"#).unwrap();
    let rev_regex = Regex::new(r#"<commit\s+revision="(\d+)""#).unwrap();
    let kind_regex = Regex::new(r#"<entry\s+path="([^"]*)"\s+kind="([^"]+)""#).unwrap();
    let schedule_regex = Regex::new(r#"<wc-info>\s*<schedule>([^<]+)</schedule>"#).unwrap();

    let url = url_regex.captures(xml)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "".to_string());

    let repository_root = root_regex.captures(xml)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "".to_string());

    let revision = rev_regex.captures(xml)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok())
        .unwrap_or(0);

    let (path, node_kind) = kind_regex.captures(xml)
        .map(|c| {
            (
                c.get(1).map(|m| m.as_str().to_string()).unwrap_or_else(|| "".to_string()),
                c.get(2).map(|m| m.as_str().to_string()).unwrap_or_else(|| "file".to_string()),
            )
        })
        .unwrap_or_else(|| ("".to_string(), "file".to_string()));

    let schedule = schedule_regex.captures(xml)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "normal".to_string());

    Ok(SvnInfo {
        path,
        url,
        repository_root,
        revision,
        node_kind,
        schedule,
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlameLine {
    pub revision: u64,
    pub author: String,
    pub line: String,
}

pub async fn blame(path: &str) -> Result<Vec<BlameLine>, SvnError> {
    let args = vec!["blame", "--xml"];
    let output = execute_svn(&args, Some(path)).await?;
    parse_blame_xml(&output, path).await
}

async fn parse_blame_xml(xml: &str, path: &str) -> Result<Vec<BlameLine>, SvnError> {
    let mut lines = Vec::new();

    // 简单解析 blame 输出
    for line in xml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("      ") && trimmed.len() > 20 {
            let parts: Vec<&str> = trimmed.splitn(3, ' ').filter(|s| !s.is_empty()).collect();
            if parts.len() >= 3 {
                if let Ok(revision) = parts[0].parse::<u64>() {
                    lines.push(BlameLine {
                        revision,
                        author: parts[1].to_string(),
                        line: parts[2].to_string(),
                    });
                }
            }
        }
    }

    // 如果没有解析到，尝试非 XML 格式
    if lines.is_empty() {
        let args = vec!["blame"];
        let output = execute_svn(&args, Some(path)).await?;

        for line in output.lines() {
            if line.len() > 15 {
                let parts: Vec<&str> = line.splitn(3, ' ').collect();
                if parts.len() >= 3 {
                    if let Ok(revision) = parts[0].parse::<u64>() {
                        lines.push(BlameLine {
                            revision,
                            author: parts[1].to_string(),
                            line: parts[2].to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(lines)
}

pub async fn diff(path: &str, old_rev: Option<u64>, new_rev: Option<u64>) -> Result<DiffResult, SvnError> {
    let mut args: Vec<String> = vec!["diff".to_string()];

    if let Some(old) = old_rev {
        if let Some(new) = new_rev {
            args.push("-r".to_string());
            args.push(format!("{}:{}", old, new));
        } else {
            args.push("-c".to_string());
            args.push(old.to_string());
        }
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = execute_svn(&args_refs, Some(path)).await?;

    Ok(DiffResult {
        path: path.to_string(),
        diff: output,
        old_revision: old_rev.unwrap_or(0),
        new_revision: new_rev.unwrap_or(0),
    })
}

pub async fn add(path: &str, files: &[String]) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["add".to_string()];
    for file in files {
        args.push(file.clone());
    }
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

pub async fn delete(path: &str, files: &[String]) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["delete".to_string()];
    for file in files {
        args.push(file.clone());
    }
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

pub async fn revert(path: &str, files: &[String]) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["revert".to_string()];
    for file in files {
        args.push(file.clone());
    }
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}

pub async fn resolve(path: &str, files: &[String], strategy: &str) -> Result<String, SvnError> {
    let mut args: Vec<String> = vec!["resolve".to_string(), "--accept".to_string(), strategy.to_string()];
    for file in files {
        args.push(file.clone());
    }
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

pub async fn merge(path: &str, source: &str, rev_start: u64, rev_end: u64) -> Result<String, SvnError> {
    let args: Vec<String> = vec![
        "merge".to_string(),
        "-c".to_string(),
        format!("{}:{}@{}", rev_start, rev_end, source),
    ];
    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    execute_svn(&args_refs, Some(path)).await
}
