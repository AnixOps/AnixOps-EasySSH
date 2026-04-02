//! Data import and export functionality

use crate::database_client::query::QueryCell;
use crate::database_client::{DatabaseConfig, DatabaseError, DatabaseType, QueryResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Import/Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataFormat {
    Csv,
    Json,
    Jsonl,
    Sql,
    Xml,
    Excel,
    Parquet,
    Markdown,
    Html,
}

impl DataFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            DataFormat::Csv => "csv",
            DataFormat::Json => "json",
            DataFormat::Jsonl => "jsonl",
            DataFormat::Sql => "sql",
            DataFormat::Xml => "xml",
            DataFormat::Excel => "xlsx",
            DataFormat::Parquet => "parquet",
            DataFormat::Markdown => "md",
            DataFormat::Html => "html",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            DataFormat::Csv => "text/csv",
            DataFormat::Json => "application/json",
            DataFormat::Jsonl => "application/x-jsonlines",
            DataFormat::Sql => "application/sql",
            DataFormat::Xml => "application/xml",
            DataFormat::Excel => {
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            }
            DataFormat::Parquet => "application/octet-stream",
            DataFormat::Markdown => "text/markdown",
            DataFormat::Html => "text/html",
        }
    }
}

/// Import configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    pub format: DataFormat,
    pub table_name: String,
    pub has_header: bool,
    pub delimiter: String,
    pub encoding: String,
    pub skip_lines: usize,
    pub date_format: Option<String>,
    pub on_duplicate: DuplicateAction,
    pub batch_size: usize,
    pub preview_only: bool,
}

impl ImportConfig {
    pub fn new(format: DataFormat, table_name: String) -> Self {
        Self {
            format,
            table_name,
            has_header: true,
            delimiter: ",".to_string(),
            encoding: "UTF-8".to_string(),
            skip_lines: 0,
            date_format: None,
            on_duplicate: DuplicateAction::Skip,
            batch_size: 1000,
            preview_only: false,
        }
    }
}

/// Duplicate handling action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DuplicateAction {
    Skip,
    Replace,
    Ignore,
    Abort,
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    pub format: DataFormat,
    pub include_headers: bool,
    pub delimiter: String,
    pub encoding: String,
    pub date_format: String,
    pub null_value: String,
    pub bool_true: String,
    pub bool_false: String,
    pub max_rows: Option<usize>,
    pub include_create_table: bool,
    pub include_indexes: bool,
    pub table_name: String,
}

impl ExportConfig {
    pub fn new(format: DataFormat, table_name: String) -> Self {
        Self {
            format,
            include_headers: true,
            delimiter: ",".to_string(),
            encoding: "UTF-8".to_string(),
            date_format: "%Y-%m-%d %H:%M:%S".to_string(),
            null_value: "NULL".to_string(),
            bool_true: "true".to_string(),
            bool_false: "false".to_string(),
            max_rows: None,
            include_create_table: false,
            include_indexes: false,
            table_name,
        }
    }
}

/// Import result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub rows_imported: u64,
    pub rows_skipped: u64,
    pub rows_failed: u64,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub duration_ms: u64,
    pub preview: Option<Vec<Vec<String>>>,
}

/// Export result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub success: bool,
    pub rows_exported: u64,
    pub file_path: String,
    pub file_size_bytes: u64,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

/// Data importer
pub struct DataImporter;

impl DataImporter {
    pub fn new() -> Self {
        Self
    }

    /// Import data from file
    pub async fn import(
        &self,
        file_path: &Path,
        config: &ImportConfig,
    ) -> Result<ImportResult, DatabaseError> {
        let start = std::time::Instant::now();
        let mut result = ImportResult {
            success: true,
            rows_imported: 0,
            rows_skipped: 0,
            rows_failed: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms: 0,
            preview: None,
        };

        match config.format {
            DataFormat::Csv => {
                self.import_csv(file_path, config, &mut result).await?;
            }
            DataFormat::Json => {
                self.import_json(file_path, config, &mut result).await?;
            }
            DataFormat::Jsonl => {
                self.import_jsonl(file_path, config, &mut result).await?;
            }
            DataFormat::Sql => {
                self.import_sql(file_path, config, &mut result).await?;
            }
            _ => {
                return Err(DatabaseError::ImportExportError(format!(
                    "Import format {:?} not yet supported",
                    config.format
                )));
            }
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    async fn import_csv(
        &self,
        file_path: &Path,
        config: &ImportConfig,
        result: &mut ImportResult,
    ) -> Result<(), DatabaseError> {
        let file = std::fs::File::open(file_path)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(config.has_header)
            .delimiter(config.delimiter.as_bytes()[0])
            .from_reader(file);

        let headers: Vec<String> = if config.has_header {
            rdr.headers()
                .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        // Preview mode - just read first few rows
        if config.preview_only {
            let mut preview = Vec::new();
            for record in rdr.records().take(5) {
                let record = record.map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;
                preview.push(record.iter().map(|s| s.to_string()).collect());
            }
            result.preview = Some(preview);
            return Ok(());
        }

        // Actual import
        let mut batch: Vec<Vec<String>> = Vec::with_capacity(config.batch_size);

        for record in rdr.records() {
            match record {
                Ok(rec) => {
                    let row: Vec<String> = rec.iter().map(|s| s.to_string()).collect();
                    batch.push(row);

                    if batch.len() >= config.batch_size {
                        // Insert batch
                        result.rows_imported += batch.len() as u64;
                        batch.clear();
                    }
                }
                Err(e) => {
                    result.rows_failed += 1;
                    result.errors.push(e.to_string());
                }
            }
        }

        // Insert remaining batch
        if !batch.is_empty() {
            result.rows_imported += batch.len() as u64;
        }

        Ok(())
    }

    async fn import_json(
        &self,
        file_path: &Path,
        config: &ImportConfig,
        result: &mut ImportResult,
    ) -> Result<(), DatabaseError> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;

        let data: Vec<serde_json::Map<String, serde_json::Value>> = serde_json::from_str(&content)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;

        result.rows_imported = data.len() as u64;
        Ok(())
    }

    async fn import_jsonl(
        &self,
        file_path: &Path,
        config: &ImportConfig,
        result: &mut ImportResult,
    ) -> Result<(), DatabaseError> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;

        let mut count = 0;
        for line in content.lines().skip(config.skip_lines) {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(line) {
                Ok(_) => count += 1,
                Err(e) => {
                    result.rows_failed += 1;
                    result.errors.push(e.to_string());
                }
            }
        }

        result.rows_imported = count;
        Ok(())
    }

    async fn import_sql(
        &self,
        file_path: &Path,
        _config: &ImportConfig,
        result: &mut ImportResult,
    ) -> Result<(), DatabaseError> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;

        // Simple SQL parsing - split by semicolons
        let statements: Vec<&str> = content.split(';').collect();
        result.rows_imported = statements.len() as u64;

        Ok(())
    }

    /// Generate preview of import data
    pub async fn preview(
        &self,
        file_path: &Path,
        format: DataFormat,
        max_rows: usize,
    ) -> Result<Vec<Vec<String>>, DatabaseError> {
        let mut config = ImportConfig::new(format, String::new());
        config.preview_only = true;

        let mut result = self.import(file_path, &config).await?;

        result
            .preview
            .take()
            .ok_or_else(|| DatabaseError::ImportExportError("No preview available".to_string()))
    }
}

impl Default for DataImporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Data exporter
pub struct DataExporter;

impl DataExporter {
    pub fn new() -> Self {
        Self
    }

    /// Export query result to file
    pub async fn export_result(
        &self,
        result: &QueryResult,
        file_path: &Path,
        config: &ExportConfig,
    ) -> Result<ExportResult, DatabaseError> {
        let start = std::time::Instant::now();

        let data = match config.format {
            DataFormat::Csv => result.to_csv()?,
            DataFormat::Json => result.to_json()?,
            DataFormat::Sql => result.to_sql_inserts(&config.table_name, "")?,
            DataFormat::Markdown => self.to_markdown(result, config)?,
            DataFormat::Html => self.to_html(result, config)?,
            _ => {
                return Err(DatabaseError::ImportExportError(format!(
                    "Export format {:?} not yet supported",
                    config.format
                )));
            }
        };

        std::fs::write(file_path, data)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;

        let metadata = std::fs::metadata(file_path)
            .map_err(|e| DatabaseError::ImportExportError(e.to_string()))?;

        Ok(ExportResult {
            success: true,
            rows_exported: result.rows.len() as u64,
            file_path: file_path.to_string_lossy().to_string(),
            file_size_bytes: metadata.len(),
            errors: Vec::new(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn to_markdown(
        &self,
        result: &QueryResult,
        config: &ExportConfig,
    ) -> Result<String, DatabaseError> {
        let mut md = String::new();

        // Header
        if config.include_headers {
            md.push('|');
            for col in &result.columns {
                md.push_str(&format!(" {} |", col));
            }
            md.push('\n');

            // Separator
            md.push('|');
            for _ in &result.columns {
                md.push_str(" --- |");
            }
            md.push('\n');
        }

        // Data
        for row in &result.rows {
            md.push('|');
            for cell in &row.cells {
                let val = match cell {
                    QueryCell::Null => config.null_value.clone(),
                    QueryCell::Boolean(true) => config.bool_true.clone(),
                    QueryCell::Boolean(false) => config.bool_false.clone(),
                    _ => cell.to_string(),
                };
                md.push_str(&format!(" {} |", val));
            }
            md.push('\n');
        }

        Ok(md)
    }

    fn to_html(
        &self,
        result: &QueryResult,
        config: &ExportConfig,
    ) -> Result<String, DatabaseError> {
        let mut html = String::from(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Query Results</title>
    <style>
        table { border-collapse: collapse; width: 100%; font-family: sans-serif; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #4a90d9; color: white; }
        tr:nth-child(even) { background-color: #f2f2f2; }
        .null { color: #999; font-style: italic; }
    </style>
</head>
<body>
    <table>
"#,
        );

        // Headers
        if config.include_headers {
            html.push_str("        <tr>\n");
            for col in &result.columns {
                html.push_str(&format!("            <th>{}</th>\n", col));
            }
            html.push_str("        </tr>\n");
        }

        // Data
        for row in &result.rows {
            html.push_str("        <tr>\n");
            for cell in &row.cells {
                let val = match cell {
                    QueryCell::Null => format!(r#"<td class="null">{}</td>"#, config.null_value),
                    _ => format!("<td>{}</td>", cell.to_string()),
                };
                html.push_str(&format!("            {}\n", val));
            }
            html.push_str("        </tr>\n");
        }

        html.push_str(
            r#"    </table>
</body>
</html>"#,
        );

        Ok(html)
    }

    /// Export database schema
    pub async fn export_schema(
        &self,
        _config: &DatabaseConfig,
        _file_path: &Path,
        _include_data: bool,
    ) -> Result<ExportResult, DatabaseError> {
        // Would export full database schema + optionally data
        Ok(ExportResult {
            success: true,
            rows_exported: 0,
            file_path: _file_path.to_string_lossy().to_string(),
            file_size_bytes: 0,
            errors: Vec::new(),
            duration_ms: 0,
        })
    }
}

impl Default for DataExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Import/Export manager
pub struct ImportExportManager {
    importer: DataImporter,
    exporter: DataExporter,
}

impl ImportExportManager {
    pub fn new() -> Self {
        Self {
            importer: DataImporter::new(),
            exporter: DataExporter::new(),
        }
    }

    pub fn importer(&self) -> &DataImporter {
        &self.importer
    }

    pub fn exporter(&self) -> &DataExporter {
        &self.exporter
    }

    /// Detect format from file extension
    pub fn detect_format(path: &Path) -> Option<DataFormat> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "csv" => Some(DataFormat::Csv),
                "json" => Some(DataFormat::Json),
                "jsonl" => Some(DataFormat::Jsonl),
                "sql" => Some(DataFormat::Sql),
                "xml" => Some(DataFormat::Xml),
                "xlsx" | "xls" => Some(DataFormat::Excel),
                "parquet" => Some(DataFormat::Parquet),
                "md" | "markdown" => Some(DataFormat::Markdown),
                "html" | "htm" => Some(DataFormat::Html),
                _ => None,
            })
    }

    /// Get available formats for import
    pub fn import_formats() -> Vec<DataFormat> {
        vec![
            DataFormat::Csv,
            DataFormat::Json,
            DataFormat::Jsonl,
            DataFormat::Sql,
            DataFormat::Xml,
            DataFormat::Excel,
        ]
    }

    /// Get available formats for export
    pub fn export_formats() -> Vec<DataFormat> {
        vec![
            DataFormat::Csv,
            DataFormat::Json,
            DataFormat::Jsonl,
            DataFormat::Sql,
            DataFormat::Xml,
            DataFormat::Excel,
            DataFormat::Markdown,
            DataFormat::Html,
        ]
    }
}

impl Default for ImportExportManager {
    fn default() -> Self {
        Self::new()
    }
}
