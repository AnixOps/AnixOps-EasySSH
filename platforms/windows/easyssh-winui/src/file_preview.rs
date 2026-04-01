#![allow(dead_code)]

//! File Preview for SFTP File Manager
//! Provides text file preview functionality

use std::collections::HashMap;

pub struct FilePreview {
    pub file_name: String,
    pub content: String,
    pub is_binary: bool,
    pub line_count: usize,
    pub char_count: usize,
    pub language: String,
    pub is_open: bool,
    pub scroll_offset: f32,
    pub search_query: String,
    pub search_matches: Vec<(usize, usize)>, // (line, column) pairs
    pub current_match: Option<usize>,
    pub wrap_text: bool,
    pub font_size: f32,
    pub show_line_numbers: bool,
    pub theme: PreviewTheme,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PreviewTheme {
    Dark,
    Light,
    HighContrast,
}

impl FilePreview {
    pub fn new() -> Self {
        Self {
            file_name: String::new(),
            content: String::new(),
            is_binary: false,
            line_count: 0,
            char_count: 0,
            language: String::from("text"),
            is_open: false,
            scroll_offset: 0.0,
            search_query: String::new(),
            search_matches: Vec::new(),
            current_match: None,
            wrap_text: true,
            font_size: 14.0,
            show_line_numbers: true,
            theme: PreviewTheme::Dark,
        }
    }

    pub fn open(&mut self, file_name: String, content: String) {
        self.file_name = file_name.clone();
        self.content = content.clone();
        self.is_open = true;
        self.scroll_offset = 0.0;
        self.search_query.clear();
        self.search_matches.clear();
        self.current_match = None;

        // Detect if binary
        self.is_binary = content.bytes().any(|b| b == 0);

        // Count lines and chars
        self.line_count = content.lines().count();
        self.char_count = content.chars().count();

        // Detect language
        self.language = Self::detect_language(&file_name);
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.file_name.clear();
        self.content.clear();
        self.search_matches.clear();
        self.current_match = None;
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = query.to_lowercase();
        self.search_matches.clear();
        self.current_match = None;

        if query.is_empty() {
            return;
        }

        let query_lower = query.to_lowercase();

        for (line_idx, line) in self.content.lines().enumerate() {
            let line_lower = line.to_lowercase();
            let mut search_start = 0;

            while let Some(pos) = line_lower[search_start..].find(&query_lower) {
                let actual_pos = search_start + pos;
                self.search_matches.push((line_idx, actual_pos));
                search_start = actual_pos + query.len();
            }
        }

        if !self.search_matches.is_empty() {
            self.current_match = Some(0);
        }
    }

    pub fn next_match(&mut self) {
        if let Some(current) = self.current_match {
            if current + 1 < self.search_matches.len() {
                self.current_match = Some(current + 1);
            } else {
                self.current_match = Some(0); // Wrap around
            }
        }
    }

    pub fn prev_match(&mut self) {
        if let Some(current) = self.current_match {
            if current > 0 {
                self.current_match = Some(current - 1);
            } else {
                self.current_match = Some(self.search_matches.len() - 1); // Wrap to end
            }
        }
    }

    pub fn get_line(&self, line_idx: usize) -> Option<&str> {
        self.content.lines().nth(line_idx)
    }

    pub fn is_too_large(&self) -> bool {
        self.char_count > 1_000_000 || self.line_count > 50_000
    }

    pub fn truncated_content(&self) -> String {
        if self.is_too_large() {
            let lines: Vec<_> = self.content.lines().take(10000).collect();
            format!(
                "{}\n\n[... File too large to preview fully ...]",
                lines.join("\n")
            )
        } else {
            self.content.clone()
        }
    }

    pub fn get_preview_summary(&self) -> String {
        format!(
            "{} | {} lines | {} chars | {}",
            self.file_name,
            Self::format_number(self.line_count),
            Self::format_number(self.char_count),
            self.language
        )
    }

    pub fn can_preview(&self, file_name: &str) -> bool {
        let ext = file_name.split('.').last().unwrap_or("").to_lowercase();

        let previewable = [
            // Text files
            "txt",
            "md",
            "markdown",
            "mdx",
            "rst",
            // Code files
            "rs",
            "js",
            "jsx",
            "ts",
            "tsx",
            "py",
            "java",
            "cpp",
            "c",
            "h",
            "hpp",
            "go",
            "rb",
            "php",
            "swift",
            "kt",
            "scala",
            "r",
            "m",
            "cs",
            "fs",
            "hs",
            "erl",
            "ex",
            "clj",
            "jl",
            "lua",
            "pl",
            "sh",
            "bash",
            "ps1",
            "vim",
            "html",
            "css",
            "scss",
            "sass",
            "less",
            // Config files
            "json",
            "xml",
            "yaml",
            "yml",
            "toml",
            "ini",
            "cfg",
            "conf",
            "properties",
            // Data files
            "csv",
            "tsv",
            "sql",
            "log",
            // Web files
            "htm",
            "xhtml",
            "svg",
            // Other
            "gitignore",
            "dockerfile",
            "makefile",
            "cmake",
            " LICENSE",
            "readme",
        ];

        previewable.contains(&ext.as_str()) || self.is_text_file_by_content(file_name)
    }

    fn is_text_file_by_content(&self, _file_name: &str) -> bool {
        // Try to read first few bytes and check for null bytes
        // This is a placeholder - actual implementation would need file reading
        true
    }

    fn detect_language(file_name: &str) -> String {
        let ext = file_name.split('.').last().unwrap_or("").to_lowercase();
        let name_lower = file_name.to_lowercase();

        let lang_map: HashMap<&str, &str> = [
            ("rs", "Rust"),
            ("js", "JavaScript"),
            ("jsx", "React JSX"),
            ("ts", "TypeScript"),
            ("tsx", "React TSX"),
            ("py", "Python"),
            ("java", "Java"),
            ("cpp", "C++"),
            ("cc", "C++"),
            ("cxx", "C++"),
            ("c", "C"),
            ("h", "C/C++ Header"),
            ("hpp", "C++ Header"),
            ("go", "Go"),
            ("rb", "Ruby"),
            ("php", "PHP"),
            ("swift", "Swift"),
            ("kt", "Kotlin"),
            ("kts", "Kotlin Script"),
            ("scala", "Scala"),
            ("r", "R"),
            ("m", "Objective-C/MATLAB"),
            ("cs", "C#"),
            ("fs", "F#"),
            ("fsx", "F# Script"),
            ("hs", "Haskell"),
            ("lhs", "Literate Haskell"),
            ("erl", "Erlang"),
            ("hrl", "Erlang Header"),
            ("ex", "Elixir"),
            ("exs", "Elixir Script"),
            ("clj", "Clojure"),
            ("cljs", "ClojureScript"),
            ("jl", "Julia"),
            ("lua", "Lua"),
            ("pl", "Perl"),
            ("pm", "Perl Module"),
            ("sh", "Shell"),
            ("bash", "Bash"),
            ("zsh", "Zsh"),
            ("fish", "Fish"),
            ("ps1", "PowerShell"),
            ("vim", "Vim"),
            ("html", "HTML"),
            ("htm", "HTML"),
            ("xhtml", "XHTML"),
            ("css", "CSS"),
            ("scss", "SCSS"),
            ("sass", "Sass"),
            ("less", "Less"),
            ("json", "JSON"),
            ("xml", "XML"),
            ("yaml", "YAML"),
            ("yml", "YAML"),
            ("toml", "TOML"),
            ("ini", "INI"),
            ("cfg", "Config"),
            ("conf", "Config"),
            ("csv", "CSV"),
            ("tsv", "TSV"),
            ("sql", "SQL"),
            ("md", "Markdown"),
            ("markdown", "Markdown"),
            ("mdx", "MDX"),
            ("rst", "reStructuredText"),
            ("txt", "Plain Text"),
            ("log", "Log"),
            ("svg", "SVG"),
        ]
        .iter()
        .cloned()
        .collect();

        // Check for special filenames
        if name_lower == "dockerfile" || name_lower.ends_with("dockerfile") {
            return String::from("Dockerfile");
        }
        if name_lower == "makefile" || name_lower == "gnumakefile" {
            return String::from("Makefile");
        }
        if name_lower.starts_with("readme") {
            return String::from("Readme");
        }
        if name_lower.contains("license") || name_lower.contains("licence") {
            return String::from("License");
        }
        if name_lower == ".gitignore" || name_lower.ends_with(".gitignore") {
            return String::from("Git Ignore");
        }

        lang_map
            .get(ext.as_str())
            .map(|&s| s.to_string())
            .unwrap_or_else(|| String::from("Plain Text"))
    }

    fn format_number(n: usize) -> String {
        if n >= 1_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else if n >= 1_000 {
            format!("{:.1}K", n as f64 / 1_000.0)
        } else {
            format!("{}", n)
        }
    }

    pub fn zoom_in(&mut self) {
        self.font_size = (self.font_size + 2.0).min(32.0);
    }

    pub fn zoom_out(&mut self) {
        self.font_size = (self.font_size - 2.0).max(8.0);
    }

    pub fn reset_zoom(&mut self) {
        self.font_size = 14.0;
    }

    pub fn toggle_wrap(&mut self) {
        self.wrap_text = !self.wrap_text;
    }

    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    pub fn cycle_theme(&mut self) {
        self.theme = match self.theme {
            PreviewTheme::Dark => PreviewTheme::Light,
            PreviewTheme::Light => PreviewTheme::HighContrast,
            PreviewTheme::HighContrast => PreviewTheme::Dark,
        };
    }
}

impl Default for FilePreview {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for FilePreview {
    fn clone(&self) -> Self {
        Self {
            file_name: self.file_name.clone(),
            content: self.content.clone(),
            is_binary: self.is_binary,
            line_count: self.line_count,
            char_count: self.char_count,
            language: self.language.clone(),
            is_open: self.is_open,
            scroll_offset: self.scroll_offset,
            search_query: self.search_query.clone(),
            search_matches: self.search_matches.clone(),
            current_match: self.current_match,
            wrap_text: self.wrap_text,
            font_size: self.font_size,
            show_line_numbers: self.show_line_numbers,
            theme: self.theme,
        }
    }
}
