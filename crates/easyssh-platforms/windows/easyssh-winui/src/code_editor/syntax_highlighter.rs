#![allow(dead_code)]

//! Syntax Highlighter - Stub

/// Token type for syntax highlighting
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    Keyword,
    Identifier,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Whitespace,
    Unknown,
}

/// Language
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Unknown,
    PlainText,
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    C,
    Cpp,
    Java,
    CSharp,
    Html,
    Css,
    Json,
    Yaml,
    Toml,
    Sql,
    Bash,
    Markdown,
}

impl Language {
    pub fn from_extension(_ext: &str) -> Self {
        Language::Unknown
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
            Language::Go => "go",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Java => "java",
            Language::CSharp => "csharp",
            Language::Html => "html",
            Language::Css => "css",
            Language::Json => "json",
            Language::Yaml => "yaml",
            Language::Toml => "toml",
            Language::Sql => "sql",
            Language::Bash => "bash",
            Language::Markdown => "markdown",
            Language::PlainText => "plaintext",
            Language::Unknown => "unknown",
        }
    }
}

/// Syntax highlighter
pub struct SyntaxHighlighter;

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self
    }

    pub fn highlight(&self, _code: &str, _language: Language) -> Vec<Token> {
        vec![]
    }

    pub fn highlight_line(&self, _line: &str, _language: Language) -> Vec<(TokenType, String)> {
        vec![]
    }
}

/// Token
#[derive(Clone, Debug)]
pub struct Token {
    pub text: String,
    pub token_type: TokenType,
}

impl Token {
    pub fn new(text: String, token_type: TokenType) -> Self {
        Self { text, token_type }
    }
}
