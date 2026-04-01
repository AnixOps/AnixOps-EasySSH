//! Query editor with syntax highlighting support

use serde::{Deserialize, Serialize};
use crate::database_client::{DatabaseType, DatabaseError};

/// Syntax highlighting theme
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorTheme {
    Light,
    Dark,
    HighContrast,
}

/// SQL dialect for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlDialect {
    Standard,
    MySQL,
    PostgreSQL,
    SQLite,
    MongoDB,
    Redis,
}

impl SqlDialect {
    pub fn from_db_type(db_type: DatabaseType) -> Self {
        match db_type {
            DatabaseType::MySQL => SqlDialect::MySQL,
            DatabaseType::PostgreSQL => SqlDialect::PostgreSQL,
            DatabaseType::SQLite => SqlDialect::SQLite,
            DatabaseType::MongoDB => SqlDialect::MongoDB,
            DatabaseType::Redis => SqlDialect::Redis,
        }
    }
}

/// Token types for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenType {
    Keyword,
    Identifier,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Function,
    Type,
    Variable,
    Whitespace,
    Unknown,
}

/// Token with position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub token_type: TokenType,
    pub text: String,
    pub start_pos: usize,
    pub end_pos: usize,
    pub line: usize,
    pub column: usize,
}

/// Syntax highlighter
pub struct SyntaxHighlighter {
    dialect: SqlDialect,
}

impl SyntaxHighlighter {
    pub fn new(dialect: SqlDialect) -> Self {
        Self { dialect }
    }

    /// Tokenize SQL query
    pub fn tokenize(&self, sql: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = sql.chars().collect();
        let mut pos = 0;
        let mut line = 1;
        let mut col = 1;

        while pos < chars.len() {
            let start_pos = pos;
            let start_line = line;
            let start_col = col;

            let (token_type, len) = self.next_token(&chars, pos);

            // Calculate line/column changes
            for i in 0..len {
                if chars[pos + i] == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
            }

            let text: String = chars[pos..pos + len].iter().collect();

            tokens.push(Token {
                token_type,
                text,
                start_pos,
                end_pos: pos + len,
                line: start_line,
                column: start_col,
            });

            pos += len;
        }

        tokens
    }

    fn next_token(&self, chars: &[char], pos: usize) -> (TokenType, usize) {
        let c = chars[pos];

        // Whitespace
        if c.is_whitespace() {
            let len = chars[pos..].iter()
                .position(|&ch| !ch.is_whitespace())
                .unwrap_or(chars.len() - pos);
            return (TokenType::Whitespace, len);
        }

        // Line comment
        if c == '-' && pos + 1 < chars.len() && chars[pos + 1] == '-' {
            let len = chars[pos..].iter()
                .position(|&ch| ch == '\n')
                .unwrap_or(chars.len() - pos);
            return (TokenType::Comment, len);
        }

        // Block comment
        if c == '/' && pos + 1 < chars.len() && chars[pos + 1] == '*' {
            if let Some(end) = chars[pos + 2..].windows(2).position(|w| w == ['*', '/']) {
                return (TokenType::Comment, end + 4);
            }
            return (TokenType::Comment, chars.len() - pos);
        }

        // String literal
        if c == '\'' || c == '"' {
            let quote = c;
            let mut len = 1;
            while pos + len < chars.len() {
                if chars[pos + len] == quote {
                    len += 1;
                    // Check for escaped quote
                    if pos + len < chars.len() && chars[pos + len] == quote {
                        len += 1;
                    } else {
                        break;
                    }
                } else {
                    len += 1;
                }
            }
            return (TokenType::String, len);
        }

        // Number
        if c.is_ascii_digit() {
            let len = chars[pos..].iter()
                .position(|&ch| !ch.is_ascii_digit() && ch != '.' && ch != 'e' && ch != 'E' && ch != '-' && ch != '+')
                .unwrap_or(chars.len() - pos);
            return (TokenType::Number, len);
        }

        // Identifier or keyword
        if c.is_alphabetic() || c == '_' {
            let len = chars[pos..].iter()
                .position(|&ch| !ch.is_alphanumeric() && ch != '_')
                .unwrap_or(chars.len() - pos);

            let word: String = chars[pos..pos + len].iter().collect();
            let upper = word.to_uppercase();

            // Check if keyword
            if self.is_keyword(&upper) {
                return (TokenType::Keyword, len);
            }

            // Check if function
            if pos + len < chars.len() && chars[pos + len] == '(' {
                return (TokenType::Function, len);
            }

            // Check if type
            if self.is_type(&upper) {
                return (TokenType::Type, len);
            }

            return (TokenType::Identifier, len);
        }

        // Operator
        if "+-*/=<>!%&|^".contains(c) {
            let len = if pos + 1 < chars.len() {
                let two_char = format!("{}{}", c, chars[pos + 1]);
                if ["<=", ">=", "<>", "!=", "<=>", "||", "&&", "->", "->>"].contains(&two_char.as_str()) {
                    2
                } else {
                    1
                }
            } else {
                1
            };
            return (TokenType::Operator, len);
        }

        // Punctuation
        if "(),;.:[]{}".contains(c) {
            return (TokenType::Punctuation, 1);
        }

        // Variable (for some dialects)
        if c == '@' || c == ':' || c == '$' || c == '?' {
            let len = chars[pos..].iter()
                .skip(1)
                .position(|&ch| !ch.is_alphanumeric() && ch != '_')
                .map(|p| p + 1)
                .unwrap_or(chars.len() - pos);
            return (TokenType::Variable, len);
        }

        (TokenType::Unknown, 1)
    }

    fn is_keyword(&self, word: &str) -> bool {
        let keywords = match self.dialect {
            SqlDialect::Standard | SqlDialect::MySQL | SqlDialect::PostgreSQL | SqlDialect::SQLite => vec![
                "SELECT", "INSERT", "UPDATE", "DELETE", "FROM", "WHERE", "JOIN",
                "INNER", "LEFT", "RIGHT", "FULL", "OUTER", "CROSS", "ON", "USING",
                "AND", "OR", "NOT", "NULL", "IS", "IN", "EXISTS", "BETWEEN", "LIKE",
                "GROUP", "BY", "HAVING", "ORDER", "ASC", "DESC", "LIMIT", "OFFSET",
                "CREATE", "ALTER", "DROP", "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA",
                "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "UNIQUE", "DEFAULT", "AUTO_INCREMENT",
                "INT", "INTEGER", "VARCHAR", "TEXT", "DATE", "DATETIME", "TIMESTAMP", "BOOL", "BOOLEAN",
                "AS", "DISTINCT", "ALL", "UNION", "INTERSECT", "EXCEPT",
                "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION",
                "IF", "ELSE", "WHILE", "CASE", "WHEN", "THEN", "END",
            ],
            SqlDialect::MongoDB => vec![
                "find", "findOne", "insert", "insertOne", "insertMany",
                "update", "updateOne", "updateMany", "delete", "deleteOne", "deleteMany",
                "aggregate", "count", "distinct", "sort", "limit", "skip",
                "db", "collection", "match", "project", "group", "lookup",
            ],
            SqlDialect::Redis => vec![
                "GET", "SET", "DEL", "EXISTS", "EXPIRE", "TTL",
                "HGET", "HSET", "HDEL", "HGETALL", "HMSET",
                "LPUSH", "RPUSH", "LPOP", "RPOP", "LRANGE", "LLEN",
                "SADD", "SREM", "SMEMBERS", "SISMEMBER",
                "ZADD", "ZREM", "ZRANGE", "ZREVRANGE", "ZSCORE",
                "KEYS", "SCAN", "FLUSHDB", "FLUSHALL", "INFO", "CONFIG",
            ],
        };

        keywords.contains(&word)
    }

    fn is_type(&self, word: &str) -> bool {
        let types = vec![
            "INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT",
            "VARCHAR", "CHAR", "TEXT", "STRING",
            "FLOAT", "DOUBLE", "REAL", "DECIMAL", "NUMERIC",
            "DATE", "TIME", "DATETIME", "TIMESTAMP",
            "BOOLEAN", "BOOL",
            "BLOB", "BINARY", "VARBINARY",
            "JSON", "XML",
        ];

        types.contains(&word)
    }

    /// Generate HTML with syntax highlighting
    pub fn to_html(&self, sql: &str, theme: EditorTheme) -> String {
        let tokens = self.tokenize(sql);
        let mut html = String::new();

        html.push_str(r#"<pre class="sql-editor" style="font-family: monospace;"#);
        match theme {
            EditorTheme::Light => html.push_str(r#" background: #fff; color: #333;"#),
            EditorTheme::Dark => html.push_str(r#" background: #1e1e1e; color: #d4d4d4;"#),
            EditorTheme::HighContrast => html.push_str(r#" background: #000; color: #fff;"#),
        }
        html.push_str(">");

        for token in tokens {
            let color = self.token_color(token.token_type, theme);
            let escaped = Self::escape_html(&token.text);
            html.push_str(&format!(
                r#"<span style="color: {}">{}</span>"#,
                color, escaped
            ));
        }

        html.push_str("</pre>");
        html
    }

    fn token_color(&self, token_type: TokenType, theme: EditorTheme) -> &'static str {
        match theme {
            EditorTheme::Light => match token_type {
                TokenType::Keyword => "#0000ff",
                TokenType::String => "#008000",
                TokenType::Number => "#ff00ff",
                TokenType::Comment => "#808080",
                TokenType::Function => "#795e26",
                TokenType::Type => "#267f99",
                TokenType::Operator => "#000000",
                TokenType::Identifier => "#001080",
                _ => "#000000",
            },
            EditorTheme::Dark => match token_type {
                TokenType::Keyword => "#569cd6",
                TokenType::String => "#ce9178",
                TokenType::Number => "#b5cea8",
                TokenType::Comment => "#6a9955",
                TokenType::Function => "#dcdcaa",
                TokenType::Type => "#4ec9b0",
                TokenType::Operator => "#d4d4d4",
                TokenType::Identifier => "#9cdcfe",
                _ => "#d4d4d4",
            },
            EditorTheme::HighContrast => match token_type {
                TokenType::Keyword => "#ffff00",
                TokenType::String => "#00ff00",
                TokenType::Number => "#ff00ff",
                TokenType::Comment => "#808080",
                TokenType::Function => "#00ffff",
                TokenType::Type => "#ffaa00",
                TokenType::Operator => "#ffffff",
                TokenType::Identifier => "#ffffff",
                _ => "#ffffff",
            },
        }
    }

    fn escape_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}

/// Query editor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryEditorState {
    pub query_text: String,
    pub cursor_position: CursorPosition,
    pub selection: Option<TextSelection>,
    pub dirty: bool,
    pub last_saved: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: usize,
    pub column: usize,
    pub absolute: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TextSelection {
    pub start: CursorPosition,
    pub end: CursorPosition,
}

/// Auto-completer for SQL
pub struct SqlAutoCompleter {
    dialect: SqlDialect,
    schema_tables: Vec<String>,
    schema_columns: std::collections::HashMap<String, Vec<String>>,
}

impl SqlAutoCompleter {
    pub fn new(dialect: SqlDialect) -> Self {
        Self {
            dialect,
            schema_tables: Vec::new(),
            schema_columns: std::collections::HashMap::new(),
        }
    }

    pub fn update_schema(&mut self, tables: Vec<String>, columns: std::collections::HashMap<String, Vec<String>>) {
        self.schema_tables = tables;
        self.schema_columns = columns;
    }

    /// Get completions at position
    pub fn complete(&self, sql: &str, pos: usize) -> Vec<CompletionItem> {
        let prefix = Self::get_word_prefix(sql, pos);
        let mut completions = Vec::new();

        if prefix.is_empty() {
            return completions;
        }

        let prefix_upper = prefix.to_uppercase();

        // Keywords
        for keyword in self.get_keywords() {
            if keyword.to_uppercase().starts_with(&prefix_upper) {
                completions.push(CompletionItem {
                    label: keyword.clone(),
                    kind: CompletionKind::Keyword,
                    detail: Some("Keyword".to_string()),
                    insert_text: Some(keyword),
                });
            }
        }

        // Tables
        for table in &self.schema_tables {
            if table.to_uppercase().starts_with(&prefix_upper) {
                completions.push(CompletionItem {
                    label: table.clone(),
                    kind: CompletionKind::Table,
                    detail: Some("Table".to_string()),
                    insert_text: Some(table.clone()),
                });
            }
        }

        // Columns for current table context
        if let Some(table) = self.infer_table_context(sql, pos) {
            if let Some(columns) = self.schema_columns.get(&table) {
                for col in columns {
                    if col.to_uppercase().starts_with(&prefix_upper) {
                        completions.push(CompletionItem {
                            label: col.clone(),
                            kind: CompletionKind::Column,
                            detail: Some(format!("Column of {}", table)),
                            insert_text: Some(col.clone()),
                        });
                    }
                }
            }
        }

        completions
    }

    fn get_word_prefix(sql: &str, pos: usize) -> String {
        let before = &sql[..pos.min(sql.len())];
        before.chars().rev()
            .take_while(|&c| c.is_alphanumeric() || c == '_' || c == '.')
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    }

    fn infer_table_context(&self, sql: &str, _pos: usize) -> Option<String> {
        // Simple table detection - look for FROM or JOIN clauses
        let upper = sql.to_uppercase();

        for keyword in ["FROM", "JOIN"] {
            if let Some(pos) = upper.find(keyword) {
                let after = &sql[pos + keyword.len()..];
                let table = after.split_whitespace().next()?;
                return Some(table.trim_matches(&['(', ')', ',', ';', '\n'][..]).to_string());
            }
        }

        None
    }

    fn get_keywords(&self) -> Vec<String> {
        let base = vec![
            "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE",
            "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "ON", "USING",
            "AND", "OR", "NOT", "NULL", "IS", "IN", "EXISTS",
            "GROUP", "BY", "HAVING", "ORDER", "ASC", "DESC",
            "LIMIT", "OFFSET", "UNION", "DISTINCT", "ALL",
            "CREATE", "ALTER", "DROP", "TABLE", "INDEX", "VIEW",
            "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "UNIQUE",
        ];

        base.iter().map(|&s| s.to_string()).collect()
    }
}

/// Completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
    // Database-specific
    Table,
    Column,
    Database,
    Schema,
    Trigger,
    View,
    Index,
}

/// Query formatter/beautifier
pub struct QueryFormatter;

impl QueryFormatter {
    pub fn format(sql: &str, dialect: SqlDialect) -> String {
        let highlighter = SyntaxHighlighter::new(dialect);
        let tokens = highlighter.tokenize(sql);

        let mut formatted = String::new();
        let mut indent_level = 0;
        let mut prev_was_newline = true;

        for token in tokens {
            match token.token_type {
                TokenType::Keyword => {
                    let upper = token.text.to_uppercase();

                    // New line before certain keywords
                    if ["SELECT", "FROM", "WHERE", "JOIN", "LEFT", "RIGHT", "INNER",
                        "OUTER", "GROUP", "ORDER", "HAVING", "UNION", "INSERT", "UPDATE",
                        "DELETE", "CREATE", "ALTER", "DROP"].contains(&upper.as_str()) {
                        if !formatted.is_empty() && !prev_was_newline {
                            formatted.push('\n');
                        }

                        // Adjust indent for closing keywords
                        if ["FROM", "WHERE", "GROUP", "ORDER", "HAVING"].contains(&upper.as_str()) {
                            indent_level = 1;
                        }
                    }

                    formatted.push_str(&"  ".repeat(indent_level));
                    formatted.push_str(&upper);
                    prev_was_newline = false;
                }
                TokenType::Whitespace => {
                    if token.text.contains('\n') {
                        formatted.push('\n');
                        prev_was_newline = true;
                    } else if !prev_was_newline {
                        formatted.push(' ');
                    }
                }
                _ => {
                    if prev_was_newline {
                        formatted.push_str(&"  ".repeat(indent_level));
                        prev_was_newline = false;
                    } else {
                        formatted.push(' ');
                    }
                    formatted.push_str(&token.text);
                }
            }
        }

        formatted.trim().to_string()
    }
}
