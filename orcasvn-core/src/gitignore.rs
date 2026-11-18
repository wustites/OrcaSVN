//! Minimal `.gitignore` matcher used to hide ignored unversioned files,
//! mirroring `main`'s `src/utils/gitignore.ts`.

use crate::SvnStatus;
use std::path::Path;

#[derive(Debug, Clone)]
struct Rule {
    negation: bool,
    dir_only: bool,
    /// Anchored to the workspace root (leading slash or contains a slash).
    anchored: bool,
    pattern: Vec<char>,
}

#[derive(Debug, Default, Clone)]
pub struct Gitignore {
    rules: Vec<Rule>,
}

impl Gitignore {
    pub fn parse(content: &str) -> Self {
        let mut rules = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (negation, rest) = match trimmed.strip_prefix('!') {
                Some(rest) => (true, rest.trim_start()),
                None => (false, trimmed),
            };
            if rest.is_empty() {
                continue;
            }
            let (dir_only, rest) = match rest.strip_suffix('/') {
                Some(rest) => (true, rest),
                None => (false, rest),
            };
            let (anchored, rest) = match rest.strip_prefix('/') {
                Some(rest) => (true, rest),
                None => (false, rest),
            };
            if rest.is_empty() {
                continue;
            }
            let anchored = anchored || rest.contains('/');
            rules.push(Rule {
                negation,
                dir_only,
                anchored,
                pattern: rest.chars().collect(),
            });
        }
        Self { rules }
    }

    /// Read `<workspace>/<file>`; returns `None` when the file is missing.
    pub fn load(workspace: &str, file: &str) -> Option<Self> {
        if file.trim().is_empty() {
            return None;
        }
        let path = Path::new(workspace).join(file);
        let content = std::fs::read_to_string(path).ok()?;
        Some(Self::parse(&content))
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Whether `path` (workspace-relative, `/` separated) is ignored.
    pub fn is_ignored(&self, path: &str) -> bool {
        let normalized = path
            .replace('\\', "/")
            .trim_start_matches("./")
            .trim_end_matches('/')
            .to_string();
        if normalized.is_empty() {
            return false;
        }
        let mut candidates = vec![normalized.clone()];
        let mut rest = normalized.as_str();
        while let Some((parent, _)) = rest.rsplit_once('/') {
            candidates.push(parent.to_string());
            rest = parent;
        }
        // The last matching rule wins.
        for rule in self.rules.iter().rev() {
            if candidates.iter().any(|candidate| rule.matches(candidate)) {
                return !rule.negation;
            }
        }
        false
    }
}

impl Rule {
    fn matches(&self, candidate: &str) -> bool {
        let chars: Vec<char> = candidate.chars().collect();
        if self.anchored {
            return glob_match(&self.pattern, &chars);
        }
        // Unanchored patterns match the trailing segment or any directory.
        if !candidate.contains('/') {
            return glob_match(&self.pattern, &chars);
        }
        // Match any suffix starting at a path boundary, e.g. `target` matches
        // `a/target` and `build/out` matches `a/build/out`.
        let segments: Vec<&str> = candidate.split('/').collect();
        (0..segments.len()).any(|start| {
            let suffix: Vec<char> = segments[start..].join("/").chars().collect();
            glob_match(&self.pattern, &suffix)
                || (self.dir_only
                    && segments[start..].len() == 1
                    && glob_match(&self.pattern, &suffix))
        })
    }
}

/// Glob matcher supporting `*`, `?`, `**` and character classes.
fn glob_match(pattern: &[char], text: &[char]) -> bool {
    fn go(pattern: &[char], text: &[char], pi: usize, ti: usize) -> bool {
        if pi == pattern.len() {
            return ti == text.len();
        }
        match pattern[pi] {
            '*' => {
                let double = pi + 1 < pattern.len() && pattern[pi + 1] == '*';
                if double {
                    let mut next = pi + 2;
                    if next < pattern.len() && pattern[next] == '/' {
                        next += 1;
                        // `**/` may match zero directories.
                        if go(pattern, text, next, ti) {
                            return true;
                        }
                    }
                    for skip in ti..=text.len() {
                        if go(pattern, text, next, skip) {
                            return true;
                        }
                    }
                    false
                } else {
                    for skip in ti..=text.len() {
                        if skip > ti && text[skip - 1] == '/' {
                            return false;
                        }
                        if go(pattern, text, pi + 1, skip) {
                            return true;
                        }
                    }
                    false
                }
            }
            '?' => ti < text.len() && text[ti] != '/' && go(pattern, text, pi + 1, ti + 1),
            '[' => {
                let mut end = pi + 1;
                while end < pattern.len() && pattern[end] != ']' {
                    end += 1;
                }
                if end >= pattern.len() || ti >= text.len() {
                    return false;
                }
                let mut negated = false;
                let mut start = pi + 1;
                if start < end && (pattern[start] == '!' || pattern[start] == '^') {
                    negated = true;
                    start += 1;
                }
                let mut matched = false;
                let mut index = start;
                while index < end {
                    if index + 2 < end && pattern[index + 1] == '-' {
                        let (from, to) = (pattern[index], pattern[index + 2]);
                        if from <= text[ti] && text[ti] <= to {
                            matched = true;
                        }
                        index += 3;
                    } else {
                        if pattern[index] == text[ti] {
                            matched = true;
                        }
                        index += 1;
                    }
                }
                matched != negated && go(pattern, text, end + 1, ti + 1)
            }
            '\\' => {
                let next = pi + 1;
                if next < pattern.len() && ti < text.len() && pattern[next] == text[ti] {
                    go(pattern, text, next + 1, ti + 1)
                } else {
                    false
                }
            }
            literal => ti < text.len() && text[ti] == literal && go(pattern, text, pi + 1, ti + 1),
        }
    }
    go(pattern, text, 0, 0)
}

/// Drop unversioned entries that the workspace `.gitignore` hides.
pub fn filter_status(entries: Vec<SvnStatus>, gitignore: &Gitignore) -> Vec<SvnStatus> {
    if gitignore.is_empty() {
        return entries;
    }
    entries
        .into_iter()
        .filter(|entry| entry.status_code != "unversioned" || !gitignore.is_ignored(&entry.path))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ignored(gitignore: &str, path: &str) -> bool {
        Gitignore::parse(gitignore).is_ignored(path)
    }

    #[test]
    fn matches_names_and_directories() {
        assert!(ignored("target\n", "target"));
        assert!(ignored("target\n", "a/b/target"));
        assert!(ignored("build/\n", "x/build/out.txt"));
        assert!(ignored("*.log\n", "a/b/x.log"));
        assert!(!ignored("*.log\n", "a/b/x.txt"));
    }

    #[test]
    fn supports_negation_and_anchors() {
        assert!(!ignored("/root.txt\n", "a/root.txt"));
        assert!(ignored("/root.txt\n", "root.txt"));
        assert!(ignored("doc/**\n", "doc/a/b.md"));
        assert!(ignored("**/node_modules\n", "a/node_modules"));
        assert!(!ignored("*.log\n!keep.log\n", "keep.log"));
        assert!(ignored("!keep.log\n*.log\n", "keep.log"));
    }

    #[test]
    fn filters_only_unversioned_entries() {
        let entries = vec![
            SvnStatus {
                path: "a.log".into(),
                status: "unversioned".into(),
                status_code: "unversioned".into(),
                prop_status: "none".into(),
                locked: false,
                history: false,
                switched: false,
                changelist: None,
            },
            SvnStatus {
                path: "a.txt".into(),
                status: "unversioned".into(),
                status_code: "unversioned".into(),
                prop_status: "none".into(),
                locked: false,
                history: false,
                switched: false,
                changelist: None,
            },
            SvnStatus {
                path: "b.log".into(),
                status: "modified".into(),
                status_code: "modified".into(),
                prop_status: "none".into(),
                locked: false,
                history: false,
                switched: false,
                changelist: None,
            },
        ];
        let filtered = filter_status(entries, &Gitignore::parse("*.log\n"));
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|entry| entry.path != "a.log"));
    }
}
