use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;
pub const A4_WIDTH_MM: f32 = 210.0;
pub const A4_HEIGHT_MM: f32 = 297.0;
pub const A3_WIDTH_MM: f32 = 297.0;
pub const A3_HEIGHT_MM: f32 = 420.0;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Project {
    pub schema_version: u32,
    pub id: Uuid,
    pub title: String,
    pub symbol_definitions: Vec<SymbolDefinition>,
    pub pages: Vec<Page>,
}

impl Project {
    pub fn new() -> Self {
        let definitions = starter_symbols();
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: Uuid::new_v4(),
            title: "Untitled electrical project".to_owned(),
            symbol_definitions: definitions,
            pages: vec![Page::new("Page 1".to_owned())],
        }
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Page {
    pub id: Uuid,
    pub name: String,
    pub diagram_type: DiagramType,
    pub sheet_size: SheetSize,
    pub orientation: Orientation,
    pub title_block: TitleBlock,
    pub symbols: Vec<PlacedSymbol>,
    pub connections: Vec<Connection>,
    pub annotations: Vec<Annotation>,
}

impl Page {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.clone(),
            diagram_type: DiagramType::Control,
            sheet_size: SheetSize::A4,
            orientation: Orientation::Landscape,
            title_block: TitleBlock {
                project_title: String::new(),
                page_title: name,
                page_number: "1".to_owned(),
                revision: String::new(),
                prepared_by: String::new(),
            },
            symbols: Vec::new(),
            connections: Vec::new(),
            annotations: Vec::new(),
        }
    }

    pub fn dimensions_mm(&self) -> (f32, f32) {
        let (width, height) = match self.sheet_size {
            SheetSize::A4 => (A4_WIDTH_MM, A4_HEIGHT_MM),
            SheetSize::A3 => (A3_WIDTH_MM, A3_HEIGHT_MM),
        };
        match self.orientation {
            Orientation::Portrait => (width, height),
            Orientation::Landscape => (height, width),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagramType {
    Control,
    PowerSingleLine,
    PowerMultiLine,
}

impl DiagramType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Control => "Control",
            Self::PowerSingleLine => "Power single-line",
            Self::PowerMultiLine => "Power multi-line",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SheetSize {
    A4,
    A3,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TitleBlock {
    pub project_title: String,
    pub page_title: String,
    pub page_number: String,
    pub revision: String,
    pub prepared_by: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SymbolDefinition {
    pub id: Uuid,
    pub name: String,
    pub category: SymbolCategory,
    pub glyph: SymbolGlyph,
    pub pins: Vec<PinDefinition>,
    pub local_library_ref: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolCategory {
    ProtectionSwitching,
    Control,
    PowerEquipment,
    WiringInfrastructure,
}

impl SymbolCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::ProtectionSwitching => "Protection & switching",
            Self::Control => "Control",
            Self::PowerEquipment => "Power equipment",
            Self::WiringInfrastructure => "Wiring infrastructure",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolGlyph {
    Breaker,
    Contactor,
    PushButton,
    IndicatorLamp,
    Motor,
    Transformer,
    Terminal,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PinDefinition {
    pub id: String,
    pub name: String,
    pub position: PointMm,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlacedSymbol {
    pub id: Uuid,
    pub definition_id: Uuid,
    pub designation: String,
    pub label: String,
    pub position: PointMm,
    pub rotation_quarters: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Connection {
    pub id: Uuid,
    pub from: PinRef,
    pub to: PinRef,
    pub vertices: Vec<PointMm>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PinRef {
    pub symbol_id: Uuid,
    pub pin_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Annotation {
    pub id: Uuid,
    pub text: String,
    pub position: PointMm,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PointMm {
    pub x: f32,
    pub y: f32,
}

fn starter_symbols() -> Vec<SymbolDefinition> {
    [
        (
            "Circuit breaker",
            SymbolCategory::ProtectionSwitching,
            SymbolGlyph::Breaker,
        ),
        (
            "Contactor",
            SymbolCategory::ProtectionSwitching,
            SymbolGlyph::Contactor,
        ),
        (
            "Push button",
            SymbolCategory::Control,
            SymbolGlyph::PushButton,
        ),
        (
            "Indicator lamp",
            SymbolCategory::Control,
            SymbolGlyph::IndicatorLamp,
        ),
        ("Motor", SymbolCategory::PowerEquipment, SymbolGlyph::Motor),
        (
            "Transformer",
            SymbolCategory::PowerEquipment,
            SymbolGlyph::Transformer,
        ),
        (
            "Terminal",
            SymbolCategory::WiringInfrastructure,
            SymbolGlyph::Terminal,
        ),
    ]
    .into_iter()
    .map(|(name, category, glyph)| SymbolDefinition {
        id: Uuid::new_v4(),
        name: name.to_owned(),
        category,
        glyph,
        pins: vec![
            PinDefinition {
                id: "1".to_owned(),
                name: "1".to_owned(),
                position: PointMm { x: 0.0, y: -12.0 },
            },
            PinDefinition {
                id: "2".to_owned(),
                name: "2".to_owned(),
                position: PointMm { x: 0.0, y: 12.0 },
            },
        ],
        local_library_ref: None,
    })
    .collect()
}
