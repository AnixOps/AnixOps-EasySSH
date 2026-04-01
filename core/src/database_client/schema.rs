//! Database schema analysis and introspection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub database_name: String,
    pub tables: Vec<SchemaTable>,
    pub views: Vec<SchemaView>,
    pub procedures: Vec<SchemaProcedure>,
    pub functions: Vec<SchemaFunction>,
    pub sequences: Vec<SchemaSequence>,
    pub enums: Vec<SchemaEnum>,
}

impl DatabaseSchema {
    pub fn new(database_name: String) -> Self {
        Self {
            database_name,
            tables: Vec::new(),
            views: Vec::new(),
            procedures: Vec::new(),
            functions: Vec::new(),
            sequences: Vec::new(),
            enums: Vec::new(),
        }
    }

    pub fn get_table(&self, name: &str) -> Option<&SchemaTable> {
        self.tables.iter().find(|t| t.name == name)
    }

    pub fn get_table_mut(&mut self, name: &str) -> Option<&mut SchemaTable> {
        self.tables.iter_mut().find(|t| t.name == name)
    }

    pub fn table_names(&self) -> Vec<String> {
        self.tables.iter().map(|t| t.name.clone()).collect()
    }

    pub fn view_names(&self) -> Vec<String> {
        self.views.iter().map(|v| v.name.clone()).collect()
    }
}

/// Schema table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaTable {
    pub name: String,
    pub schema: Option<String>,
    pub table_type: SchemaTableType,
    pub columns: Vec<SchemaColumn>,
    pub indexes: Vec<SchemaIndex>,
    pub foreign_keys: Vec<SchemaForeignKey>,
    pub row_count: Option<u64>,
    pub comment: Option<String>,
}

impl SchemaTable {
    pub fn primary_key_columns(&self) -> Vec<&SchemaColumn> {
        self.columns.iter()
            .filter(|c| c.is_primary_key)
            .collect()
    }

    pub fn get_column(&self, name: &str) -> Option<&SchemaColumn> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn get_column_mut(&mut self, name: &str) -> Option<&mut SchemaColumn> {
        self.columns.iter_mut().find(|c| c.name == name)
    }

    pub fn has_column(&self, name: &str) -> bool {
        self.columns.iter().any(|c| c.name == name)
    }

    pub fn foreign_key_for_column(&self, column_name: &str) -> Option<&SchemaForeignKey> {
        self.foreign_keys.iter()
            .find(|fk| fk.column == column_name)
    }
}

/// Schema table type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaTableType {
    Table,
    View,
    System,
    Temporary,
    External,
    Partitioned,
    MaterializedView,
}

/// Schema column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub comment: Option<String>,
    pub extra: Option<String>,
}

impl SchemaColumn {
    pub fn is_required(&self) -> bool {
        !self.nullable && self.default.is_none()
    }

    pub fn rust_type(&self) -> String {
        // Map SQL types to Rust types
        let upper = self.data_type.to_uppercase();
        if upper.contains("INT") {
            if self.nullable { "Option<i64>" } else { "i64" }.to_string()
        } else if upper.contains("FLOAT") || upper.contains("REAL") || upper.contains("DOUBLE") {
            if self.nullable { "Option<f64>" } else { "f64" }.to_string()
        } else if upper.contains("BOOL") {
            if self.nullable { "Option<bool>" } else { "bool" }.to_string()
        } else if upper.contains("TIME") || upper.contains("DATE") {
            if self.nullable { "Option<chrono::NaiveDateTime>" } else { "chrono::NaiveDateTime" }.to_string()
        } else if upper.contains("JSON") {
            "serde_json::Value".to_string()
        } else {
            if self.nullable { "Option<String>" } else { "String" }.to_string()
        }
    }

    pub fn is_numeric(&self) -> bool {
        let upper = self.data_type.to_uppercase();
        upper.contains("INT") || upper.contains("FLOAT") ||
        upper.contains("REAL") || upper.contains("DOUBLE") ||
        upper.contains("DECIMAL") || upper.contains("NUMERIC")
    }

    pub fn is_text(&self) -> bool {
        let upper = self.data_type.to_uppercase();
        upper.contains("CHAR") || upper.contains("TEXT") ||
        upper.contains("VARCHAR") || upper.contains("STRING")
    }
}

/// Schema index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaIndex {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub primary: bool,
    pub index_type: Option<String>,
    pub comment: Option<String>,
}

/// Schema foreign key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaForeignKey {
    pub name: String,
    pub column: String,
    pub referenced_table: String,
    pub referenced_column: String,
    pub on_update: Option<String>,
    pub on_delete: Option<String>,
}

/// Schema view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaView {
    pub name: String,
    pub schema: Option<String>,
    pub definition: String,
    pub comment: Option<String>,
}

/// Schema stored procedure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaProcedure {
    pub name: String,
    pub schema: Option<String>,
    pub parameters: Vec<SchemaParameter>,
    pub return_type: Option<String>,
    pub language: Option<String>,
    pub definition: Option<String>,
    pub comment: Option<String>,
}

/// Schema function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaFunction {
    pub name: String,
    pub schema: Option<String>,
    pub parameters: Vec<SchemaParameter>,
    pub return_type: String,
    pub language: Option<String>,
    pub is_aggregate: bool,
    pub definition: Option<String>,
    pub comment: Option<String>,
}

/// Schema parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaParameter {
    pub name: String,
    pub data_type: String,
    pub mode: ParameterMode,
    pub default_value: Option<String>,
}

/// Parameter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterMode {
    In,
    Out,
    InOut,
}

/// Schema sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSequence {
    pub name: String,
    pub schema: Option<String>,
    pub start_value: i64,
    pub increment: i64,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub cycle: bool,
    pub current_value: Option<i64>,
}

/// Schema enum type (PostgreSQL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEnum {
    pub name: String,
    pub schema: Option<String>,
    pub values: Vec<String>,
}

/// Schema analyzer
pub struct SchemaAnalyzer;

impl SchemaAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze table relationships
    pub fn analyze_relationships(schema: &DatabaseSchema) -> Vec<TableRelationship> {
        let mut relationships = Vec::new();

        for table in &schema.tables {
            for fk in &table.foreign_keys {
                relationships.push(TableRelationship {
                    from_table: table.name.clone(),
                    from_column: fk.column.clone(),
                    to_table: fk.referenced_table.clone(),
                    to_column: fk.referenced_column.clone(),
                    relationship_type: RelationshipType::ManyToOne,
                    constraint_name: fk.name.clone(),
                });
            }
        }

        relationships
    }

    /// Find orphaned tables (no foreign keys to or from)
    pub fn find_orphaned_tables(schema: &DatabaseSchema) -> Vec<String> {
        let relationships = Self::analyze_relationships(schema);
        let related_tables: std::collections::HashSet<String> = relationships.iter()
            .flat_map(|r| vec![r.from_table.clone(), r.to_table.clone()])
            .collect();

        schema.tables.iter()
            .filter(|t| !related_tables.contains(&t.name))
            .map(|t| t.name.clone())
            .collect()
    }

    /// Detect circular references
    pub fn detect_circular_references(schema: &DatabaseSchema) -> Vec<Vec<String>> {
        let relationships = Self::analyze_relationships(schema);
        let mut circles = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for table in &schema.tables {
            let mut path = vec![table.name.clone()];
            Self::find_cycles(&relationships, &mut path, &mut visited, &mut circles);
        }

        circles
    }

    fn find_cycles(
        relationships: &[TableRelationship],
        path: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
        circles: &mut Vec<Vec<String>>,
    ) {
        let current = path.last().unwrap().clone();

        if visited.contains(&current) {
            if let Some(pos) = path.iter().position(|p| p == &current) {
                if pos < path.len() - 1 {
                    circles.push(path[pos..].to_vec());
                }
            }
            return;
        }

        visited.insert(current.clone());

        for rel in relationships {
            if rel.from_table == current {
                path.push(rel.to_table.clone());
                Self::find_cycles(relationships, path, visited, circles);
                path.pop();
            }
        }

        visited.remove(&current);
    }

    /// Generate table statistics
    pub fn generate_statistics(schema: &DatabaseSchema) -> SchemaStatistics {
        let total_tables = schema.tables.len();
        let total_columns: usize = schema.tables.iter()
            .map(|t| t.columns.len())
            .sum();
        let total_indexes: usize = schema.tables.iter()
            .map(|t| t.indexes.len())
            .sum();
        let total_foreign_keys: usize = schema.tables.iter()
            .map(|t| t.foreign_keys.len())
            .sum();

        let avg_columns_per_table = if total_tables > 0 {
            total_columns as f64 / total_tables as f64
        } else {
            0.0
        };

        let tables_with_pk = schema.tables.iter()
            .filter(|t| t.columns.iter().any(|c| c.is_primary_key))
            .count();

        let pk_coverage = if total_tables > 0 {
            (tables_with_pk as f64 / total_tables as f64) * 100.0
        } else {
            0.0
        };

        SchemaStatistics {
            total_tables,
            total_views: schema.views.len(),
            total_columns,
            total_indexes,
            total_foreign_keys,
            avg_columns_per_table,
            tables_with_primary_key: tables_with_pk,
            primary_key_coverage_percent: pk_coverage,
            data_type_distribution: Self::analyze_data_types(schema),
        }
    }

    fn analyze_data_types(schema: &DatabaseSchema) -> HashMap<String, u32> {
        let mut distribution = HashMap::new();

        for table in &schema.tables {
            for column in &table.columns {
                let key = Self::normalize_type(&column.data_type);
                *distribution.entry(key).or_insert(0) += 1;
            }
        }

        distribution
    }

    fn normalize_type(data_type: &str) -> String {
        let upper = data_type.to_uppercase();
        if upper.contains("INT") {
            "INTEGER".to_string()
        } else if upper.contains("CHAR") || upper.contains("TEXT") {
            "TEXT".to_string()
        } else if upper.contains("DATE") || upper.contains("TIME") {
            "DATETIME".to_string()
        } else if upper.contains("BOOL") {
            "BOOLEAN".to_string()
        } else if upper.contains("DECIMAL") || upper.contains("NUMERIC") {
            "DECIMAL".to_string()
        } else if upper.contains("FLOAT") || upper.contains("REAL") {
            "FLOAT".to_string()
        } else if upper.contains("BLOB") || upper.contains("BINARY") {
            "BINARY".to_string()
        } else if upper.contains("JSON") {
            "JSON".to_string()
        } else {
            "OTHER".to_string()
        }
    }
}

impl Default for SchemaAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Table relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRelationship {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
    pub relationship_type: RelationshipType,
    pub constraint_name: String,
}

/// Relationship type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}

/// Schema statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaStatistics {
    pub total_tables: usize,
    pub total_views: usize,
    pub total_columns: usize,
    pub total_indexes: usize,
    pub total_foreign_keys: usize,
    pub avg_columns_per_table: f64,
    pub tables_with_primary_key: usize,
    pub primary_key_coverage_percent: f64,
    pub data_type_distribution: HashMap<String, u32>,
}

/// Schema comparison for diff generation
pub struct SchemaComparer;

impl SchemaComparer {
    /// Compare two schemas and generate diff
    pub fn compare(old: &DatabaseSchema, new: &DatabaseSchema) -> SchemaDiff {
        let mut added_tables = Vec::new();
        let mut removed_tables = Vec::new();
        let mut modified_tables = Vec::new();

        let old_table_names: std::collections::HashSet<_> = old.tables.iter()
            .map(|t| &t.name)
            .collect();
        let new_table_names: std::collections::HashSet<_> = new.tables.iter()
            .map(|t| &t.name)
            .collect();

        // Find added tables
        for table in &new.tables {
            if !old_table_names.contains(&table.name) {
                added_tables.push(table.clone());
            }
        }

        // Find removed tables
        for table in &old.tables {
            if !new_table_names.contains(&table.name) {
                removed_tables.push(table.clone());
            }
        }

        // Find modified tables
        for new_table in &new.tables {
            if let Some(old_table) = old.get_table(&new_table.name) {
                let diff = Self::compare_tables(old_table, new_table);
                if diff.has_changes() {
                    modified_tables.push(diff);
                }
            }
        }

        SchemaDiff {
            added_tables,
            removed_tables,
            modified_tables,
        }
    }

    fn compare_tables(old: &SchemaTable, new: &SchemaTable) -> TableDiff {
        let mut added_columns = Vec::new();
        let mut removed_columns = Vec::new();
        let mut modified_columns = Vec::new();

        let old_col_names: std::collections::HashSet<_> = old.columns.iter()
            .map(|c| &c.name)
            .collect();
        let new_col_names: std::collections::HashSet<_> = new.columns.iter()
            .map(|c| &c.name)
            .collect();

        for col in &new.columns {
            if !old_col_names.contains(&col.name) {
                added_columns.push(col.clone());
            }
        }

        for col in &old.columns {
            if !new_col_names.contains(&col.name) {
                removed_columns.push(col.clone());
            }
        }

        for new_col in &new.columns {
            if let Some(old_col) = old.get_column(&new_col.name) {
                if old_col.data_type != new_col.data_type ||
                   old_col.nullable != new_col.nullable {
                    modified_columns.push(ColumnDiff {
                        name: new_col.name.clone(),
                        old_type: old_col.data_type.clone(),
                        new_type: new_col.data_type.clone(),
                        old_nullable: old_col.nullable,
                        new_nullable: new_col.nullable,
                    });
                }
            }
        }

        TableDiff {
            table_name: new.name.clone(),
            added_columns,
            removed_columns,
            modified_columns,
        }
    }
}

/// Schema difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDiff {
    pub added_tables: Vec<SchemaTable>,
    pub removed_tables: Vec<SchemaTable>,
    pub modified_tables: Vec<TableDiff>,
}

impl SchemaDiff {
    pub fn has_changes(&self) -> bool {
        !self.added_tables.is_empty() ||
        !self.removed_tables.is_empty() ||
        !self.modified_tables.is_empty()
    }

    pub fn generate_sql(&self, db_type: crate::database_client::DatabaseType) -> Vec<String> {
        let mut statements = Vec::new();

        for table in &self.added_tables {
            statements.push(Self::generate_create_table(table, db_type));
        }

        for table in &self.removed_tables {
            statements.push(format!("DROP TABLE {};", table.name));
        }

        for diff in &self.modified_tables {
            statements.extend(Self::generate_alter_table(diff, db_type));
        }

        statements
    }

    fn generate_create_table(table: &SchemaTable, db_type: crate::database_client::DatabaseType) -> String {
        let mut sql = format!("CREATE TABLE {} (\n", table.name);

        let columns_sql: Vec<String> = table.columns.iter()
            .map(|c| {
                let mut col = format!("    {} {}", c.name, c.data_type);
                if !c.nullable {
                    col.push_str(" NOT NULL");
                }
                if let Some(ref default) = c.default {
                    col.push_str(&format!(" DEFAULT {}", default));
                }
                if c.is_primary_key {
                    col.push_str(" PRIMARY KEY");
                }
                col
            })
            .collect();

        sql.push_str(&columns_sql.join(",\n"));
        sql.push_str("\n);");

        sql
    }

    fn generate_alter_table(diff: &TableDiff, db_type: crate::database_client::DatabaseType) -> Vec<String> {
        let mut statements = Vec::new();

        for col in &diff.added_columns {
            statements.push(format!(
                "ALTER TABLE {} ADD COLUMN {} {};",
                diff.table_name, col.name, col.data_type
            ));
        }

        for col in &diff.removed_columns {
            match db_type {
                crate::database_client::DatabaseType::PostgreSQL => {
                    statements.push(format!(
                        "ALTER TABLE {} DROP COLUMN {};",
                        diff.table_name, col.name
                    ));
                }
                _ => {
                    statements.push(format!(
                        "-- SQLite doesn't support DROP COLUMN, recreate table required for {}",
                        col.name
                    ));
                }
            }
        }

        statements
    }
}

/// Table difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDiff {
    pub table_name: String,
    pub added_columns: Vec<SchemaColumn>,
    pub removed_columns: Vec<SchemaColumn>,
    pub modified_columns: Vec<ColumnDiff>,
}

impl TableDiff {
    pub fn has_changes(&self) -> bool {
        !self.added_columns.is_empty() ||
        !self.removed_columns.is_empty() ||
        !self.modified_columns.is_empty()
    }
}

/// Column difference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDiff {
    pub name: String,
    pub old_type: String,
    pub new_type: String,
    pub old_nullable: bool,
    pub new_nullable: bool,
}
