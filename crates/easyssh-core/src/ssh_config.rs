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
        0,
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
    depth: usize,
    visited: &mut BTreeSet<PathBuf>,
    aliases: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
) {
    if depth > 16 {
        warnings.push(format!(
            "SSH config include depth exceeded at {}",
            path.display()
        ));
        return;
    }
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
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
        let parts = ssh_words(raw);
        let mut parts = parts.iter().map(String::as_str);
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
                let include = expand_path_token(item);
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
                    scan(&include, true, depth + 1, visited, aliases, warnings);
                }
            }
        }
    }
}

/// Parses the deliberately small OpenSSH grammar needed for `Host` and
/// `Include`: quoted paths are one token, and comments start outside quotes.
fn ssh_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            if quote.is_some() && matches!(ch, '\\' | '"') {
                word.push(ch);
            } else {
                word.push('\\');
                word.push(ch);
            }
            escaped = false;
        } else if ch == '\\' && quote.is_some() {
            escaped = true;
        } else if let Some(delimiter) = quote {
            if ch == delimiter {
                quote = None;
            } else {
                word.push(ch);
            }
        } else if matches!(ch, '\'' | '\"') {
            quote = Some(ch);
        } else if ch == '#' {
            break;
        } else if ch == '='
            && (word.eq_ignore_ascii_case("host") || word.eq_ignore_ascii_case("include"))
        {
            words.push(std::mem::take(&mut word));
        } else if ch.is_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(ch);
        }
    }
    if escaped {
        word.push('\\');
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

fn expand_path_token(token: &str) -> PathBuf {
    let mut value = token.to_owned();
    if let Some(home) = dirs::home_dir() {
        if value == "~" {
            return home;
        }
        if let Some(rest) = value
            .strip_prefix("~/")
            .or_else(|| value.strip_prefix("~\\"))
        {
            return home.join(rest);
        }
    }
    for (name, replacement) in [("%USERPROFILE%", std::env::var("USERPROFILE").ok())] {
        if let Some(replacement) = replacement {
            value = value.replace(name, &replacement);
            value = value.replace(&name.to_ascii_lowercase(), &replacement);
        }
    }
    PathBuf::from(value)
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

    #[test]
    fn quoted_include_paths_are_scanned() {
        let root = std::env::temp_dir().join(format!("easyssh-config-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("with spaces")).unwrap();
        fs::write(root.join("config"), "Include \"with spaces/hosts.conf\"\n").unwrap();
        fs::write(root.join("with spaces/hosts.conf"), "Host quoted\n").unwrap();
        assert_eq!(scan_ssh_config(&root.join("config")).aliases, ["quoted"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn path_tokens_expand_home_and_userprofile() {
        let home = dirs::home_dir().expect("test environment has a home directory");
        assert_eq!(expand_path_token("~/ssh/config"), home.join("ssh/config"));
        if let Ok(profile) = std::env::var("USERPROFILE") {
            assert_eq!(
                expand_path_token("%USERPROFILE%/ssh/config"),
                PathBuf::from(profile).join("ssh/config")
            );
        }
    }
}
