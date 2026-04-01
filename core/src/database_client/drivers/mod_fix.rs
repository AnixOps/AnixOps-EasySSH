/// Helper function to escape SQL identifiers
pub fn escape_identifier(ident: &str, db_type: DatabaseType) -> String {
    match db_type {
        DatabaseType::MySQL => format!("`{}`", ident.replace('`', "``")),
        DatabaseType::PostgreSQL => format!("\"{}\"", ident.replace('"', "\"\"")),
        DatabaseType::SQLite => format!("\"{}\"", ident.replace('"', "\"\"")),
        _ => ident.to_string(),
    }
}
