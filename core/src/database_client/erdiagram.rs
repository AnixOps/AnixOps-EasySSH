//! ER Diagram generation

use serde::{Deserialize, Serialize};
use crate::database_client::schema::{DatabaseSchema, TableRelationship, RelationshipType, SchemaAnalyzer};

/// ER Diagram node (table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErNode {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub columns: Vec<ErColumn>,
    pub color: String,
}

/// ER Diagram column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErColumn {
    pub name: String,
    pub data_type: String,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    pub is_nullable: bool,
}

/// ER Diagram edge (relationship)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_column: String,
    pub target_column: String,
    pub relationship_type: RelationshipType,
    pub label: String,
    pub cardinality: String,
}

/// ER Diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErDiagram {
    pub name: String,
    pub nodes: Vec<ErNode>,
    pub edges: Vec<ErEdge>,
    pub width: f64,
    pub height: f64,
}

impl ErDiagram {
    pub fn new(name: String) -> Self {
        Self {
            name,
            nodes: Vec::new(),
            edges: Vec::new(),
            width: 1200.0,
            height: 800.0,
        }
    }

    pub fn add_node(&mut self, node: ErNode) {
        self.nodes.push(node);
    }

    pub fn add_edge(&mut self, edge: ErEdge) {
        self.edges.push(edge);
    }

    pub fn get_node(&self, id: &str) -> Option<&ErNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut ErNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Export to SVG format
    pub fn to_svg(&self) -> String {
        let mut svg = format!(
            r#"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">"#,
            self.width, self.height
        );

        // Add styles
        svg.push_str(r##"
  <defs>
    <style>
      .table { fill: #f5f5f5; stroke: #333; stroke-width: 2; }
      .table-header { fill: #4a90d9; }
      .pk { font-weight: bold; color: #d9534f; }
      .fk { color: #5cb85c; }
      .column-text { font-family: monospace; font-size: 12px; }
      .edge { stroke: #666; stroke-width: 1; fill: none; marker-end: url(#arrow); }
    </style>
    <marker id="arrow" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto">
      <path d="M0,0 L0,6 L9,3 z" fill="#666666" />
    </marker>
  </defs>
"##);

        // Draw edges
        for edge in &self.edges {
            let source = self.get_node(&edge.source);
            let target = self.get_node(&edge.target);

            if let (Some(src), Some(tgt)) = (source, target) {
                let (x1, y1) = (src.x + src.width / 2.0, src.y + src.height / 2.0);
                let (x2, y2) = (tgt.x + tgt.width / 2.0, tgt.y + tgt.height / 2.0);

                svg.push_str(&format!(
                    r#"  <line x1="{}" y1="{}" x2="{}" y2="{}" class="edge" />"#,
                    x1, y1, x2, y2
                ));
            }
        }

        // Draw nodes
        for node in &self.nodes {
            // Table rectangle
            svg.push_str(&format!(
                r#"  <rect x="{}" y="{}" width="{}" height="{}" class="table" rx="5" />"#,
                node.x, node.y, node.width, node.height
            ));

            // Header
            let header_height = 30.0;
            svg.push_str(&format!(
                r#"  <rect x="{}" y="{}" width="{}" height="{}" class="table-header" rx="5" />"#,
                node.x, node.y, node.width, header_height
            ));

            // Table name
            svg.push_str(&format!(
                r#"  <text x="{}" y="{}" text-anchor="middle" font-weight="bold" fill="white">{}</text>"#,
                node.x + node.width / 2.0, node.y + 20.0, node.name
            ));

            // Columns
            let mut y_offset = header_height + 15.0;
            for col in &node.columns {
                let mut class = "column-text";
                if col.is_primary_key {
                    class = "column-text pk";
                } else if col.is_foreign_key {
                    class = "column-text fk";
                }

                svg.push_str(&format!(
                    r#"  <text x="{}" y="{}" class="{}">{} {}</text>"#,
                    node.x + 10.0, node.y + y_offset, class,
                    if col.is_primary_key { "🔑" } else if col.is_foreign_key { "🔗" } else { "  " },
                    col.name
                ));

                y_offset += 15.0;
            }
        }

        svg.push_str("</svg>");
        svg
    }

    /// Export to PlantUML format
    pub fn to_plantuml(&self) -> String {
        let mut uml = String::from("@startuml\n\n");

        // Define entities
        for node in &self.nodes {
            uml.push_str(&format!("entity \"{}\" {{\n", node.name));

            for col in &node.columns {
                let prefix = if col.is_primary_key {
                    "* "
                } else if col.is_foreign_key {
                    "# "
                } else {
                    "  "
                };
                uml.push_str(&format!("  {}{}: {}\n", prefix, col.name, col.data_type));
            }

            uml.push_str("}\n\n");
        }

        // Define relationships
        for edge in &self.edges {
            let cardinality = match edge.relationship_type {
                RelationshipType::OneToOne => "||--||",
                RelationshipType::OneToMany => "||--o{",
                RelationshipType::ManyToOne => "}o--||",
                RelationshipType::ManyToMany => "}o--o{",
            };

            uml.push_str(&format!(
                "\"{}\" {} \"{}\"\n",
                edge.source, cardinality, edge.target
            ));
        }

        uml.push_str("\n@enduml");
        uml
    }

    /// Export to Mermaid format
    pub fn to_mermaid(&self) -> String {
        let mut mmd = String::from("erDiagram\n");

        for node in &self.nodes {
            mmd.push_str(&format!("    {} {{\n", node.name));

            for col in &node.columns {
                let dtype = match col.data_type.to_uppercase().as_str() {
                    t if t.contains("INT") => "int",
                    t if t.contains("CHAR") || t.contains("TEXT") => "string",
                    t if t.contains("DATE") || t.contains("TIME") => "datetime",
                    t if t.contains("BOOL") => "bool",
                    t if t.contains("DECIMAL") || t.contains("NUMERIC") || t.contains("FLOAT") => "float",
                    _ => "string",
                };

                let pk_marker = if col.is_primary_key { "PK" } else { "" };
                let fk_marker = if col.is_foreign_key { "FK" } else { "" };
                let markers = format!("{}{}", pk_marker, fk_marker);

                mmd.push_str(&format!(
                    "        {} {} {}\n",
                    dtype, col.name, markers
                ));
            }

            mmd.push_str("    }\n");
        }

        mmd.push('\n');

        for edge in &self.edges {
            let cardinality = match edge.relationship_type {
                RelationshipType::OneToOne => "||--||",
                RelationshipType::OneToMany => "||--o{",
                RelationshipType::ManyToOne => "}o--||",
                RelationshipType::ManyToMany => "}o--o{",
            };

            mmd.push_str(&format!(
                "    {} {} {} : \"{}\"\n",
                edge.source, cardinality, edge.target, edge.label
            ));
        }

        mmd
    }
}

/// ER Diagram generator
pub struct ErDiagramGenerator {
    layout_algorithm: LayoutAlgorithm,
    node_width: f64,
    node_height_per_row: f64,
    horizontal_spacing: f64,
    vertical_spacing: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum LayoutAlgorithm {
    Grid,
    ForceDirected,
    Hierarchical,
    Circular,
}

impl ErDiagramGenerator {
    pub fn new() -> Self {
        Self {
            layout_algorithm: LayoutAlgorithm::Grid,
            node_width: 200.0,
            node_height_per_row: 20.0,
            horizontal_spacing: 50.0,
            vertical_spacing: 50.0,
        }
    }

    pub fn with_algorithm(mut self, algo: LayoutAlgorithm) -> Self {
        self.layout_algorithm = algo;
        self
    }

    /// Generate ER diagram from database schema
    pub fn generate(&self, schema: &DatabaseSchema) -> ErDiagram {
        let mut diagram = ErDiagram::new(schema.database_name.clone());
        let relationships = SchemaAnalyzer::analyze_relationships(schema);

        // Create nodes
        for table in &schema.tables {
            let columns: Vec<ErColumn> = table.columns.iter()
                .map(|c| ErColumn {
                    name: c.name.clone(),
                    data_type: c.data_type.clone(),
                    is_primary_key: c.is_primary_key,
                    is_foreign_key: c.is_foreign_key,
                    is_nullable: c.nullable,
                })
                .collect();

            let height = 40.0 + (columns.len() as f64 * self.node_height_per_row);

            let node = ErNode {
                id: table.name.clone(),
                name: table.name.clone(),
                x: 0.0,
                y: 0.0,
                width: self.node_width,
                height,
                columns,
                color: self.generate_color(&table.name),
            };

            diagram.add_node(node);
        }

        // Create edges
        for rel in &relationships {
            let edge = ErEdge {
                id: format!("{}_{}_{}_{}", rel.from_table, rel.from_column, rel.to_table, rel.to_column),
                source: rel.from_table.clone(),
                target: rel.to_table.clone(),
                source_column: rel.from_column.clone(),
                target_column: rel.to_column.clone(),
                relationship_type: rel.relationship_type,
                label: format!("{} → {}", rel.from_column, rel.to_column),
                cardinality: self.cardinality_string(&rel.relationship_type),
            };

            diagram.add_edge(edge);
        }

        // Apply layout
        self.apply_layout(&mut diagram);

        diagram
    }

    fn generate_color(&self, name: &str) -> String {
        let colors = vec![
            "#4a90d9", "#7ed321", "#f5a623", "#d0021b",
            "#9013fe", "#50e3c2", "#b8e986", "#bd10e0",
            "#417505", "#9b9b9b", "#4a4a4a", "#9c27b0",
        ];

        let hash: usize = name.bytes().map(|b| b as usize).sum();
        colors[hash % colors.len()].to_string()
    }

    fn cardinality_string(&self, rel_type: &RelationshipType) -> String {
        match rel_type {
            RelationshipType::OneToOne => "1:1",
            RelationshipType::OneToMany => "1:N",
            RelationshipType::ManyToOne => "N:1",
            RelationshipType::ManyToMany => "N:M",
        }.to_string()
    }

    fn apply_layout(&self, diagram: &mut ErDiagram) {
        match self.layout_algorithm {
            LayoutAlgorithm::Grid => self.apply_grid_layout(diagram),
            LayoutAlgorithm::ForceDirected => self.apply_force_layout(diagram),
            LayoutAlgorithm::Hierarchical => self.apply_hierarchical_layout(diagram),
            LayoutAlgorithm::Circular => self.apply_circular_layout(diagram),
        }
    }

    fn apply_grid_layout(&self, diagram: &mut ErDiagram) {
        let cols = (diagram.nodes.len() as f64).sqrt().ceil() as usize;
        let cols = cols.max(1);

        for (i, node) in diagram.nodes.iter_mut().enumerate() {
            let row = i / cols;
            let col = i % cols;

            node.x = 50.0 + col as f64 * (self.node_width + self.horizontal_spacing);
            node.y = 50.0 + row as f64 * (150.0 + self.vertical_spacing);
        }

        diagram.width = 100.0 + cols as f64 * (self.node_width + self.horizontal_spacing);
        diagram.height = 100.0 + ((diagram.nodes.len() as f64 / cols as f64).ceil()) *
                         (200.0 + self.vertical_spacing);
    }

    fn apply_force_layout(&self, diagram: &mut ErDiagram) {
        // Initialize random positions
        let width = 1000.0;
        let height = 800.0;

        for node in diagram.nodes.iter_mut() {
            node.x = rand::random::<f64>() * width;
            node.y = rand::random::<f64>() * height;
        }

        // Simple force-directed iterations
        let iterations = 100;
        let k = 100.0; // Ideal edge length
        let c = 0.01; // Repulsion constant

        for _ in 0..iterations {
            // Calculate forces
            let mut forces: Vec<(f64, f64)> = vec![(0.0, 0.0); diagram.nodes.len()];

            // Repulsion between all nodes
            for i in 0..diagram.nodes.len() {
                for j in (i + 1)..diagram.nodes.len() {
                    let dx = diagram.nodes[j].x - diagram.nodes[i].x;
                    let dy = diagram.nodes[j].y - diagram.nodes[i].y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.1);

                    let force = c * k * k / dist;
                    let fx = force * dx / dist;
                    let fy = force * dy / dist;

                    forces[i].0 -= fx;
                    forces[i].1 -= fy;
                    forces[j].0 += fx;
                    forces[j].1 += fy;
                }
            }

            // Attraction along edges
            for edge in &diagram.edges {
                if let (Some(src_idx), Some(tgt_idx)) = (
                    diagram.nodes.iter().position(|n| n.id == edge.source),
                    diagram.nodes.iter().position(|n| n.id == edge.target)
                ) {
                    let dx = diagram.nodes[tgt_idx].x - diagram.nodes[src_idx].x;
                    let dy = diagram.nodes[tgt_idx].y - diagram.nodes[src_idx].y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.1);

                    let force = dist * dist / k;
                    let fx = force * dx / dist * 0.1;
                    let fy = force * dy / dist * 0.1;

                    forces[src_idx].0 += fx;
                    forces[src_idx].1 += fy;
                    forces[tgt_idx].0 -= fx;
                    forces[tgt_idx].1 -= fy;
                }
            }

            // Apply forces
            for i in 0..diagram.nodes.len() {
                diagram.nodes[i].x += forces[i].0 * 0.1;
                diagram.nodes[i].y += forces[i].1 * 0.1;

                // Keep within bounds
                diagram.nodes[i].x = diagram.nodes[i].x.clamp(50.0, width - 50.0);
                diagram.nodes[i].y = diagram.nodes[i].y.clamp(50.0, height - 50.0);
            }
        }

        diagram.width = width;
        diagram.height = height;
    }

    fn apply_hierarchical_layout(&self, _diagram: &mut ErDiagram) {
        // Placeholder for hierarchical layout
    }

    fn apply_circular_layout(&self, _diagram: &mut ErDiagram) {
        // Placeholder for circular layout
    }
}

impl Default for ErDiagramGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// ER Diagram export formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErExportFormat {
    Svg,
    Png,
    Pdf,
    PlantUml,
    Mermaid,
    Dbml,
    Sql,
}

/// DBML (Database Markup Language) generator
pub struct DbmlGenerator;

impl DbmlGenerator {
    pub fn generate(schema: &DatabaseSchema) -> String {
        let mut dbml = String::new();

        // Project header
        dbml.push_str(&format!("Project {} {{\n", schema.database_name));
        dbml.push_str(&format!("  database_type: '{}'\n", "PostgreSQL"));
        dbml.push_str("}\n\n");

        // Tables
        for table in &schema.tables {
            dbml.push_str(&format!("Table {} {{\n", table.name));

            for col in &table.columns {
                let mut modifiers: Vec<String> = Vec::new();
                if col.is_primary_key {
                    modifiers.push("pk".to_string());
                }
                if col.is_foreign_key {
                    modifiers.push("ref".to_string());
                }
                if !col.nullable {
                    modifiers.push("not null".to_string());
                }
                if let Some(ref default) = col.default {
                    modifiers.push(format!("default: {}", default));
                }

                let modifier_str = if modifiers.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", modifiers.join(", "))
                };

                dbml.push_str(&format!(
                    "  {} {}{}\n",
                    col.name, col.data_type, modifier_str
                ));
            }

            // Indexes
            for idx in &table.indexes {
                if idx.unique {
                    dbml.push_str(&format!(
                        "  indexes {{\n    {} [unique]\n  }}\n",
                        idx.columns.join(", ")
                    ));
                }
            }

            dbml.push_str("}\n\n");
        }

        // Relationships
        let relationships = SchemaAnalyzer::analyze_relationships(schema);
        for rel in &relationships {
            let cardinality = match rel.relationship_type {
                RelationshipType::OneToOne => "-",
                RelationshipType::OneToMany => "<",
                RelationshipType::ManyToOne => ">",
                RelationshipType::ManyToMany => "<>",
            };

            dbml.push_str(&format!(
                "Ref: {}.{} {} {}.{}\n",
                rel.from_table, rel.from_column,
                cardinality,
                rel.to_table, rel.to_column
            ));
        }

        dbml
    }
}

impl Default for DbmlGenerator {
    fn default() -> Self {
        Self
    }
}
