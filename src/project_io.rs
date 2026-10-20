use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::document::{CURRENT_SCHEMA_VERSION, Project};

#[derive(Debug)]
pub enum ProjectIoError {
    Io(std::io::Error),
    Json(serde_json::Error),
    UnsupportedSchema(u32),
}

impl fmt::Display for ProjectIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported project schema version {version}")
            }
        }
    }
}

impl Error for ProjectIoError {}

impl From<std::io::Error> for ProjectIoError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ProjectIoError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub fn load(path: impl AsRef<Path>) -> Result<Project, ProjectIoError> {
    let contents = fs::read_to_string(path)?;
    from_json(&contents)
}

pub fn save(path: impl AsRef<Path>, project: &Project) -> Result<(), ProjectIoError> {
    let contents = serde_json::to_string_pretty(project)?;
    fs::write(path, contents)?;
    Ok(())
}

pub fn from_json(contents: &str) -> Result<Project, ProjectIoError> {
    let project: Project = serde_json::from_str(contents)?;
    if project.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(ProjectIoError::UnsupportedSchema(project.schema_version));
    }
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{
        Annotation, Connection, DiagramType, Orientation, PinRef, PointMm, SheetSize,
    };

    #[test]
    fn project_json_round_trip_preserves_electrical_document_data() {
        let mut project = Project::new();
        let page = &mut project.pages[0];
        page.diagram_type = DiagramType::PowerMultiLine;
        page.sheet_size = SheetSize::A3;
        page.orientation = Orientation::Portrait;
        page.title_block.prepared_by = "Engineer".to_owned();
        page.symbols.push(crate::document::PlacedSymbol {
            id: uuid::Uuid::new_v4(),
            definition_id: project.symbol_definitions[0].id,
            designation: "Q1".to_owned(),
            label: "Main isolator".to_owned(),
            position: PointMm { x: 45.0, y: 72.0 },
            rotation_quarters: 1,
        });
        let symbol_id = page.symbols[0].id;
        page.connections.push(Connection {
            id: uuid::Uuid::new_v4(),
            from: PinRef {
                symbol_id,
                pin_id: "1".to_owned(),
            },
            to: PinRef {
                symbol_id,
                pin_id: "2".to_owned(),
            },
            vertices: vec![PointMm { x: 45.0, y: 30.0 }],
        });
        page.annotations.push(Annotation {
            id: uuid::Uuid::new_v4(),
            text: "Incoming supply".to_owned(),
            position: PointMm { x: 32.0, y: 24.0 },
        });
        let contents = serde_json::to_string(&project).expect("serialize project");
        let restored = from_json(&contents).expect("load project JSON");

        assert_eq!(restored.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(restored.id, project.id);
        assert_eq!(restored.title, project.title);
        assert_eq!(restored.pages.len(), project.pages.len());
        assert_eq!(
            restored.pages[0].dimensions_mm(),
            project.pages[0].dimensions_mm()
        );
        assert_eq!(restored.pages[0].diagram_type, DiagramType::PowerMultiLine);
        assert_eq!(restored.pages[0].orientation, Orientation::Portrait);
        assert_eq!(restored.pages[0].title_block.prepared_by, "Engineer");
        assert_eq!(restored.pages[0].symbols[0].designation, "Q1");
        assert_eq!(restored.pages[0].symbols[0].rotation_quarters, 1);
        assert_eq!(restored.pages[0].connections[0].from.pin_id, "1");
        assert_eq!(restored.pages[0].connections[0].to.pin_id, "2");
        assert_eq!(restored.pages[0].annotations[0].text, "Incoming supply");
        assert_eq!(
            restored.symbol_definitions.len(),
            project.symbol_definitions.len()
        );
    }

    #[test]
    fn future_schema_versions_are_rejected() {
        let contents = format!(
            r#"{{"schema_version":{},"id":"{}","title":"x","symbol_definitions":[],"pages":[]}}"#,
            CURRENT_SCHEMA_VERSION + 1,
            uuid::Uuid::new_v4()
        );

        assert!(matches!(
            from_json(&contents),
            Err(ProjectIoError::UnsupportedSchema(version)) if version == CURRENT_SCHEMA_VERSION + 1
        ));
    }
}
