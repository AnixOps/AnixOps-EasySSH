//! Read-only discovery of concrete aliases from OpenSSH configuration files.
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SshConfigDiscovery {
    pub aliases: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn scan_default_ssh_config() -> SshConfigDiscovery {
    let Some(home) = dirs::home_dir() else {
        return SshConfigDiscovery::default();
    };
    scan_ssh_config(&home.join(".ssh").join("config"))
}

pub fn scan_ssh_config(path: &Path) -> SshConfigDiscovery {
    let mut aliases = BTreeSet::new();
    let mut warnings = Vec::new();
    scan(
        path,
        false,
        &mut BTreeSet::new(),
        &mut aliases,
        &mut warnings,
    );
    SshConfigDiscovery {
        aliases: aliases.into_iter().collect(),
        warnings,
    }
}

fn scan(
    path: &Path,
    include: bool,
    visited: &mut BTreeSet<PathBuf>,
    aliases: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
) {
    let path = path.to_path_buf();
    if !visited.insert(path.clone()) {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        if include {
            warnings.push(format!(
                "Could not read SSH config include: {}",
                path.display()
            ));
        }
        return;
    };
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        let mut parts = line.split_whitespace();
        let Some(keyword) = parts.next() else {
            continue;
        };
        if keyword.eq_ignore_ascii_case("host") {
            for alias in parts {
                if !alias.contains(['*', '?', '!']) {
                    aliases.insert(alias.to_owned());
                }
            }
        } else if keyword.eq_ignore_ascii_case("include") {
            for item in parts {
                let include = PathBuf::from(item);
                let pattern = if include.is_absolute() {
                    include
                } else {
                    base.join(include)
                };
                let matched = expand_glob(&pattern);
                if matched.is_empty() {
                    warnings.push(format!(
                        "Could not read SSH config include: {}",
                        pattern.display()
                    ));
                }
                for include in matched {
                    scan(&include, true, visited, aliases, warnings);
                }
            }
        }
    }
}

fn expand_glob(pattern: &Path) -> Vec<PathBuf> {
    let text = pattern.to_string_lossy();
    if !text.contains(['*', '?']) {
        return pattern
            .is_file()
            .then(|| pattern.to_path_buf())
            .into_iter()
            .collect();
    }
    let parent = pattern.parent().unwrap_or_else(|| Path::new("."));
    let Some(name) = pattern.file_name().and_then(|name| name.to_str()) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut result: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|value| glob_match(name, value))
        })
        .collect();
    result.sort();
    result
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let (mut p, mut t, mut star, mut mark) = (0, 0, None, 0);
    let bytes = pattern.as_bytes();
    let value = text.as_bytes();
    while t < value.len() {
        if p < bytes.len() && (bytes[p] == b'?' || bytes[p] == value[t]) {
            p += 1;
            t += 1;
        } else if p < bytes.len() && bytes[p] == b'*' {
            star = Some(p);
            p += 1;
            mark = t;
        } else if let Some(saved) = star {
            p = saved + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    while p < bytes.len() && bytes[p] == b'*' {
        p += 1;
    }
    p == bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_nested_includes_and_skips_patterns() {
        let root = std::env::temp_dir().join(format!("easyssh-config-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("parts")).unwrap();
        fs::write(
            root.join("config"),
            "Host main *.wild !skip\nInclude parts/*\n",
        )
        .unwrap();
        fs::write(root.join("parts/a"), "Host nested\n").unwrap();
        let found = scan_ssh_config(&root.join("config"));
        assert_eq!(found.aliases, ["main", "nested"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unreadable_includes_are_warnings_not_failures() {
        let root = std::env::temp_dir().join(format!("easyssh-config-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("config"), "Host usable\nInclude missing.conf\n").unwrap();
        let found = scan_ssh_config(&root.join("config"));
        assert_eq!(found.aliases, ["usable"]);
        assert_eq!(found.warnings.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }
}
