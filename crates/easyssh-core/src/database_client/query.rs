//! Query execution and results

use crate::database_client::DatabaseError;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query result cell types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QueryCell {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Blob(Vec<u8>),
    Date(String),
    DateTime(String),
    Json(serde_json::Value),
    Array(Vec<QueryCell>),
}

impl QueryCell {
    pub fn to_string(&self) -> String {
        match self {
            QueryCell::Null => "NULL".to_string(),
            QueryCell::Boolean(b) => b.to_string(),
            QueryCell::Integer(i) => i.to_string(),
            QueryCell::Float(f) => f.to_string(),
            QueryCell::String(s) => s.clone(),
            QueryCell::Blob(b) => format!("<BLOB:{} bytes>", b.len()),
            QueryCell::Date(d) => d.clone(),
            QueryCell::DateTime(d) => d.clone(),
            QueryCell::Json(v) => v.to_string(),
            QueryCell::Array(a) => format!("<ARRAY:{} items>", a.len()),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, QueryCell::Null)
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            QueryCell::String(s) => Some(s.clone()),
            QueryCell::Integer(i) => Some(i.to_string()),
            QueryCell::Float(f) => Some(f.to_string()),
            QueryCell::Boolean(b) => Some(b.to_string()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            QueryCell::Integer(i) => Some(*i),
            QueryCell::Float(f) => Some(*f as i64),
            QueryCell::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            QueryCell::Float(f) => Some(*f),
            QueryCell::Integer(i) => Some(*i as f64),
            QueryCell::String(s) => s.parse().ok(),
            _ => None,
        }
    }
}

/// Query result row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRow {
    pub cells: Vec<QueryCell>,
}

impl QueryRow {
    pub fn get(&self, index: usize) -> Option<&QueryCell> {
        self.cells.get(index)
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Get value by column name (requires column map)
    pub fn get_by_name(&self, name: &str, columns: &[String]) -> Option<&QueryCell> {
        columns
            .iter()
            .position(|c| c == name)
            .and_then(|idx| self.cells.get(idx))
    }
}

/// Query execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<QueryRow>,
    pub execution_time_ms: u64,
    pub affected_rows: Option<u64>,
    pub warning_count: u32,
    pub info_message: Option<String>,
}

impl QueryResult {
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Escape SQL identifier (table name, column name) to prevent SQL injection
    fn escape_identifier(identifier: &str) -> String {
        // Remove any potentially dangerous characters
        let sanitized: String = identifier
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
            .collect();
        // Wrap in double quotes for safety
        format!("\"{}\"", sanitized.replace('"', "\"\""))
    }

    /// Escape string value for SQL to prevent SQL injection
    fn escape_sql_string(value: &str) -> String {
        value.replace('\'', "''")
    }

    /// Convert to CSV format
    pub fn to_csv(&self) -> Result<String, DatabaseError> {
        let mut wtr = csv::WriterBuilder::new()
            .has_headers(true)
            .from_writer(Vec::new());

        // Write headers
        wtr.write_record(&self.columns)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;

        // Write rows
        for row in &self.rows {
            let record: Vec<String> = row.cells.iter().map(|cell| cell.to_string()).collect();
            wtr.write_record(&record)
                .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;
        }

        let data = wtr
            .into_inner()
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;

        String::from_utf8(data).map_err(|e| DatabaseError::ImportExportError(e.to_string()))
    }

    /// Convert to JSON format
    pub fn to_json(&self) -> Result<String, DatabaseError> {
        let mut records = Vec::new();

        for row in &self.rows {
            let mut record = serde_json::Map::new();
            for (i, col) in self.columns.iter().enumerate() {
                let value = if let Some(cell) = row.cells.get(i) {
                    match cell {
                        QueryCell::Null => serde_json::Value::Null,
                        QueryCell::Boolean(b) => serde_json::Value::Bool(*b),
                        QueryCell::Integer(i) => serde_json::Value::Number((*i).into()),
                        QueryCell::Float(f) => serde_json::Value::Number(
                            serde_json::Number::from_f64(*f).unwrap_or(0.into()),
                        ),
                        QueryCell::String(s) => serde_json::Value::String(s.clone()),
                        QueryCell::Blob(b) => serde_json::Value::String(STANDARD.encode(b)),
                        QueryCell::Date(d) => serde_json::Value::String(d.clone()),
                        QueryCell::DateTime(d) => serde_json::Value::String(d.clone()),
                        QueryCell::Json(v) => v.clone(),
                        QueryCell::Array(a) => {
                            let arr: Vec<serde_json::Value> = a
                                .iter()
                                .map(|c| match c {
                                    QueryCell::Null => serde_json::Value::Null,
                                    QueryCell::String(s) => serde_json::Value::String(s.clone()),
                                    QueryCell::Integer(i) => serde_json::Value::Number((*i).into()),
                                    _ => serde_json::Value::String(c.to_string()),
                                })
                                .collect();
                            serde_json::Value::Array(arr)
                        }
                    }
                } else {
                    serde_json::Value::Null
                };
                record.insert(col.clone(), value);
            }
            records.push(serde_json::Value::Object(record));
        }

        serde_json::to_string_pretty(&records)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))
    }

    /// Convert to SQL INSERT statements with proper escaping to prevent SQL injection
    pub fn to_sql_inserts(
        &self,
        table_name: &str,
        _db_type: &str,
    ) -> Result<String, DatabaseError> {
        let mut statements = Vec::new();

        // Escape table name to prevent SQL injection
        let escaped_table = Self::escape_identifier(table_name);

        for row in &self.rows {
            // Escape column names
            let columns: Vec<String> = self
                .columns
                .iter()
                .map(|col| Self::escape_identifier(col))
                .collect();

            let values: Vec<String> = row
                .cells
                .iter()
                .map(|cell| match cell {
                    QueryCell::Null => "NULL".to_string(),
                    QueryCell::String(s) => {
                        format!("'{}'", Self::escape_sql_string(s))
                    }
                    QueryCell::Integer(i) => i.to_string(),
                    QueryCell::Float(f) => f.to_string(),
                    QueryCell::Boolean(b) => {
                        if *b {
                            "TRUE".to_string()
                        } else {
                            "FALSE".to_string()
                        }
                    }
                    _ => format!("'{}'", Self::escape_sql_string(&cell.to_string())),
                })
                .collect();

            let stmt = format!(
                "INSERT INTO {} ({}) VALUES ({});",
                escaped_table,
                columns.join(", "),
                values.join(", ")
            );
            statements.push(stmt);
        }

        Ok(statements.join("\n"))
    }
}

/// Query builder for constructing SQL queries
pub struct QueryBuilder {
    table: String,
    columns: Vec<String>,
    where_clauses: Vec<String>,
    order_by: Vec<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    joins: Vec<String>,
    group_by: Vec<String>,
    having: Vec<String>,
    parameters: HashMap<String, QueryCell>,
}

impl QueryBuilder {
    pub fn new(table: &str) -> Self {
        // Validate table name
        let validated_table =
            Self::validate_identifier(table).unwrap_or_else(|_| "invalid_table".to_string());

        Self {
            table: validated_table,
            columns: vec!["*".to_string()],
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            joins: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            parameters: HashMap::new(),
        }
    }

    /// Validate SQL identifier to prevent injection
    /// Only allows alphanumeric characters, underscores, and dots (for schema.table)
    fn validate_identifier(identifier: &str) -> Result<String, DatabaseError> {
        if identifier.is_empty() {
            return Err(DatabaseError::QueryError(
                "Empty identifier not allowed".to_string(),
            ));
        }

        // Special case for wildcard
        if identifier == "*" {
            return Ok(identifier.to_string());
        }

        // Check for dangerous patterns
        let dangerous_patterns = ["--", "/*", "*/", ";", "'", "\"", "\\", "\n", "\r"];
        for pattern in &dangerous_patterns {
            if identifier.contains(pattern) {
                return Err(DatabaseError::QueryError(format!(
                    "Invalid pattern '{}' in identifier",
                    pattern
                )));
            }
        }

        // Check each character
        for c in identifier.chars() {
            if !c.is_alphanumeric() && c != '_' && c != '.' {
                return Err(DatabaseError::QueryError(format!(
                    "Invalid character '{}' in identifier '{}'",
                    c, identifier
                )));
            }
        }

        Ok(identifier.to_string())
    }

    pub fn select(mut self, columns: Vec<&str>) -> Self {
        // Validate each column name
        self.columns = columns
            .iter()
            .filter_map(|&col| Self::validate_identifier(col).ok())
            .collect();

        if self.columns.is_empty() {
            self.columns = vec!["*".to_string()];
        }
        self
    }

    pub fn where_eq(mut self, column: &str, value: QueryCell) -> Self {
        // Validate column name
        match Self::validate_identifier(column) {
            Ok(valid_col) => {
                let param_name = format!("p{}", self.parameters.len());
                self.parameters.insert(param_name.clone(), value);
                self.where_clauses
                    .push(format!("{} = :{}", valid_col, param_name));
            }
            Err(_) => {
                // Invalid column - add a clause that will never match
                self.where_clauses.push("1=0".to_string());
            }
        }
        self
    }

    pub fn where_like(mut self, column: &str, pattern: &str) -> Self {
        // Validate column name
        match Self::validate_identifier(column) {
            Ok(valid_col) => {
                let param_name = format!("p{}", self.parameters.len());
                self.parameters
                    .insert(param_name.clone(), QueryCell::String(pattern.to_string()));
                self.where_clauses
                    .push(format!("{} LIKE :{}", valid_col, param_name));
            }
            Err(_) => {
                self.where_clauses.push("1=0".to_string());
            }
        }
        self
    }

    pub fn order_by(mut self, column: &str, ascending: bool) -> Self {
        match Self::validate_identifier(column) {
            Ok(valid_col) => {
                let dir = if ascending { "ASC" } else { "DESC" };
                self.order_by.push(format!("{} {}", valid_col, dir));
            }
            Err(_) => {
                // Invalid column - ignore order by
            }
        }
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn offset(mut self, n: usize) -> Self {
        self.offset = Some(n);
        self
    }

    pub fn join(mut self, table: &str, on: &str, join_type: &str) -> Self {
        // Validate all identifiers
        let valid_table =
            Self::validate_identifier(table).unwrap_or_else(|_| "invalid_table".to_string());
        let valid_on = Self::validate_identifier(on).unwrap_or_else(|_| "invalid_on".to_string());
        let valid_join_type =
            Self::validate_identifier(join_type).unwrap_or_else(|_| "JOIN".to_string());

        self.joins.push(format!(
            "{} JOIN {} ON {}",
            valid_join_type, valid_table, valid_on
        ));
        self
    }

    pub fn build(&self) -> String {
        let columns_str = self.columns.join(", ");
        let mut sql = format!("SELECT {} FROM {}", columns_str, self.table);

        for join in &self.joins {
            sql.push_str(&format!(" {}", join));
        }

        if !self.where_clauses.is_empty() {
            sql.push_str(&format!(" WHERE {}", self.where_clauses.join(" AND ")));
        }

        if !self.group_by.is_empty() {
            let valid_group: Vec<String> = self
                .group_by
                .iter()
                .filter_map(|g| Self::validate_identifier(g).ok())
                .collect();
            if !valid_group.is_empty() {
                sql.push_str(&format!(" GROUP BY {}", valid_group.join(", ")));
            }
        }

        if !self.having.is_empty() {
            sql.push_str(&format!(" HAVING {}", self.having.join(" AND ")));
        }

        if !self.order_by.is_empty() {
            sql.push_str(&format!(" ORDER BY {}", self.order_by.join(", ")));
        }

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        sql
    }
}

/// Query formatter for pretty-printing SQL
pub struct QueryFormatter;

impl QueryFormatter {
    pub fn format(sql: &str) -> String {
        // Simple SQL formatting
        let keywords = vec![
            "SELECT", "FROM", "WHERE", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER", "ON", "GROUP",
            "BY", "HAVING", "ORDER", "LIMIT", "OFFSET", "INSERT", "INTO", "VALUES", "UPDATE",
            "SET", "DELETE", "CREATE", "TABLE", "ALTER", "DROP", "INDEX", "UNIQUE", "AND", "OR",
            "NOT", "IN", "EXISTS", "BETWEEN", "LIKE",
        ];

        let mut formatted = String::new();
        let mut prev_was_keyword = false;

        for token in sql.split_whitespace() {
            let upper = token.to_uppercase();
            let is_keyword = keywords.contains(&upper.as_str());

            if is_keyword && !prev_was_keyword {
                formatted.push('\n');
            }

            if formatted.is_empty() || formatted.ends_with('\n') {
                formatted.push_str("    ");
            } else {
                formatted.push(' ');
            }

            if is_keyword {
                formatted.push_str(&upper);
            } else {
                formatted.push_str(token);
            }

            prev_was_keyword = is_keyword;
        }

        formatted.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new("users")
            .select(vec!["id", "name", "email"])
            .where_eq("active", QueryCell::Boolean(true))
            .order_by("name", true)
            .limit(10)
            .build();

        assert!(query.contains("SELECT id, name, email"));
        assert!(query.contains("FROM users"));
        assert!(query.contains("ORDER BY name ASC"));
        assert!(query.contains("LIMIT 10"));
    }

    #[test]
    fn test_result_to_csv() {
        let result = QueryResult {
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![
                QueryRow {
                    cells: vec![
                        QueryCell::Integer(1),
                        QueryCell::String("Alice".to_string()),
                    ],
                },
                QueryRow {
                    cells: vec![QueryCell::Integer(2), QueryCell::String("Bob".to_string())],
                },
            ],
            execution_time_ms: 0,
            affected_rows: None,
            warning_count: 0,
            info_message: None,
        };

        let csv = result.to_csv().unwrap();
        assert!(csv.contains("id,name"));
        assert!(csv.contains("1,Alice"));
        assert!(csv.contains("2,Bob"));
    }
}
