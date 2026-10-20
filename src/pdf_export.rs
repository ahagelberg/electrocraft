use std::fs;
use std::io;
use std::path::Path;

use printpdf::PdfDocument;
use printpdf::font::BuiltinFont;
use printpdf::graphics::{Line, LinePoint, Point};
use printpdf::ops::{Op, PdfPage};
use printpdf::serialize::PdfSaveOptions;
use printpdf::text::TextItem;
use printpdf::units::{Mm, Pt};

use crate::document::{Page, PlacedSymbol, PointMm, Project, SymbolDefinition, SymbolGlyph};

const SHEET_MARGIN_MM: f32 = 10.0;
const TITLE_BLOCK_WIDTH_MM: f32 = 180.0;
const TITLE_BLOCK_HEIGHT_MM: f32 = 34.0;
const TITLE_BLOCK_SPLIT_X_RATIO: f32 = 0.62;
const TITLE_BLOCK_SPLIT_Y_RATIO: f32 = 0.56;
const SYMBOL_SCALE: f32 = 0.25;
const SYMBOL_HALF_HEIGHT_MM: f32 = 16.0;
const SYMBOL_LINE_WIDTH_MM: f32 = 0.3;
const CONNECTION_LINE_WIDTH_MM: f32 = 0.25;
const FRAME_LINE_WIDTH_MM: f32 = 0.35;
const CIRCLE_SEGMENTS: usize = 24;
const QUARTER_TURNS_PER_REVOLUTION: u8 = 4;
const PT_PER_MM: f32 = 72.0 / 25.4;
const TITLE_FONT_SIZE_PT: f32 = 8.0;
const BODY_FONT_SIZE_PT: f32 = 7.0;
const SMALL_FONT_SIZE_PT: f32 = 6.0;
const INVALID_FILENAME_CHARACTERS: &str = "<>:\"/\\|?*";

pub fn suggested_filename(title: &str) -> String {
    let stem: String = title
        .chars()
        .map(|character| {
            if character.is_control() || INVALID_FILENAME_CHARACTERS.contains(character) {
                '_'
            } else {
                character
            }
        })
        .collect();
    let stem = stem.trim().trim_matches('.');
    if stem.is_empty() {
        "electrocraft.pdf".to_owned()
    } else {
        format!("{stem}.pdf")
    }
}

pub fn write_project(project: &Project, path: &Path) -> io::Result<()> {
    let bytes = export_project(project)?;
    fs::write(path, bytes)
}

fn export_project(project: &Project) -> io::Result<Vec<u8>> {
    let document = build_document(project)?;
    let mut warnings = Vec::new();
    Ok(document.save(&PdfSaveOptions::default(), &mut warnings))
}

fn build_document(project: &Project) -> io::Result<PdfDocument> {
    if project.pages.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "project has no pages to export",
        ));
    }
    let mut document = PdfDocument::new(&project.title);
    let pages = project
        .pages
        .iter()
        .map(|page| draw_page(page, &project.symbol_definitions))
        .collect();
    document.with_pages(pages);
    Ok(document)
}

fn draw_page(page: &Page, definitions: &[SymbolDefinition]) -> PdfPage {
    let (width, height) = page.dimensions_mm();
    let mut ops = Vec::new();
    draw_frame(&mut ops, width, height);
    draw_title_block(&mut ops, page, width, height);
    draw_connections(&mut ops, page, definitions, height);
    for symbol in &page.symbols {
        let Some(definition) = definitions
            .iter()
            .find(|definition| definition.id == symbol.definition_id)
        else {
            continue;
        };
        draw_symbol(&mut ops, symbol, definition, height);
    }
    for annotation in &page.annotations {
        add_text(
            &mut ops,
            height,
            annotation.position.x,
            annotation.position.y,
            &annotation.text,
            BuiltinFont::Helvetica,
            BODY_FONT_SIZE_PT,
        );
    }
    PdfPage::new(Mm(width), Mm(height), ops)
}

fn draw_frame(ops: &mut Vec<Op>, width: f32, height: f32) {
    draw_path(
        ops,
        &[
            (SHEET_MARGIN_MM, SHEET_MARGIN_MM),
            (width - SHEET_MARGIN_MM, SHEET_MARGIN_MM),
            (width - SHEET_MARGIN_MM, height - SHEET_MARGIN_MM),
            (SHEET_MARGIN_MM, height - SHEET_MARGIN_MM),
        ],
        height,
        true,
        FRAME_LINE_WIDTH_MM,
    );
}

fn draw_title_block(ops: &mut Vec<Op>, page: &Page, width: f32, height: f32) {
    let block_width = TITLE_BLOCK_WIDTH_MM.min(width - SHEET_MARGIN_MM * 2.0);
    let block_height = TITLE_BLOCK_HEIGHT_MM.min(height - SHEET_MARGIN_MM * 2.0);
    let left = width - SHEET_MARGIN_MM - block_width;
    let top = height - SHEET_MARGIN_MM - block_height;
    let split_x = left + block_width * TITLE_BLOCK_SPLIT_X_RATIO;
    let split_y = top + block_height * TITLE_BLOCK_SPLIT_Y_RATIO;
    draw_path(
        ops,
        &[
            (left, top),
            (left + block_width, top),
            (left + block_width, top + block_height),
            (left, top + block_height),
        ],
        height,
        true,
        FRAME_LINE_WIDTH_MM,
    );
    draw_line(
        ops,
        (split_x, top),
        (split_x, top + block_height),
        height,
        FRAME_LINE_WIDTH_MM,
    );
    draw_line(
        ops,
        (left, split_y),
        (left + block_width, split_y),
        height,
        FRAME_LINE_WIDTH_MM,
    );
    add_text(
        ops,
        height,
        left + 3.0,
        top + 8.0,
        &page.title_block.project_title,
        BuiltinFont::HelveticaBold,
        TITLE_FONT_SIZE_PT,
    );
    add_text(
        ops,
        height,
        split_x + 3.0,
        top + 8.0,
        &page.title_block.prepared_by,
        BuiltinFont::Helvetica,
        BODY_FONT_SIZE_PT,
    );
    add_text(
        ops,
        height,
        left + 3.0,
        split_y + 8.0,
        &page.title_block.page_title,
        BuiltinFont::Helvetica,
        BODY_FONT_SIZE_PT,
    );
    add_text(
        ops,
        height,
        split_x + 3.0,
        split_y + 8.0,
        &format!(
            "{}  Rev {}",
            page.title_block.page_number, page.title_block.revision
        ),
        BuiltinFont::Helvetica,
        BODY_FONT_SIZE_PT,
    );
    add_text(
        ops,
        height,
        left,
        top - 3.0,
        page.diagram_type.label(),
        BuiltinFont::Helvetica,
        SMALL_FONT_SIZE_PT,
    );
}

fn draw_connections(ops: &mut Vec<Op>, page: &Page, definitions: &[SymbolDefinition], height: f32) {
    for connection in &page.connections {
        let Some(start) = pin_position(
            page,
            definitions,
            connection.from.symbol_id,
            &connection.from.pin_id,
        ) else {
            continue;
        };
        let Some(end) = pin_position(
            page,
            definitions,
            connection.to.symbol_id,
            &connection.to.pin_id,
        ) else {
            continue;
        };
        let mut points = Vec::with_capacity(connection.vertices.len() + 2);
        points.push((start.x, start.y));
        points.extend(
            connection
                .vertices
                .iter()
                .map(|vertex| (vertex.x, vertex.y)),
        );
        points.push((end.x, end.y));
        draw_path(ops, &points, height, false, CONNECTION_LINE_WIDTH_MM);
    }
}

fn pin_position(
    page: &Page,
    definitions: &[SymbolDefinition],
    symbol_id: uuid::Uuid,
    pin_id: &str,
) -> Option<PointMm> {
    let symbol = page.symbols.iter().find(|symbol| symbol.id == symbol_id)?;
    let definition = definitions
        .iter()
        .find(|definition| definition.id == symbol.definition_id)?;
    let pin = definition.pins.iter().find(|pin| pin.id == pin_id)?;
    let (x, y) = rotate(
        pin.position.x * SYMBOL_SCALE,
        pin.position.y * SYMBOL_SCALE,
        symbol.rotation_quarters,
    );
    Some(PointMm {
        x: symbol.position.x + x,
        y: symbol.position.y + y,
    })
}

fn draw_symbol(
    ops: &mut Vec<Op>,
    symbol: &PlacedSymbol,
    definition: &SymbolDefinition,
    height: f32,
) {
    let top = symbol_point(symbol, 0.0, -SYMBOL_HALF_HEIGHT_MM);
    let upper = symbol_point(symbol, 0.0, -7.0);
    let lower = symbol_point(symbol, 0.0, 7.0);
    let bottom = symbol_point(symbol, 0.0, SYMBOL_HALF_HEIGHT_MM);
    draw_line(ops, top, upper, height, SYMBOL_LINE_WIDTH_MM);
    draw_line(ops, lower, bottom, height, SYMBOL_LINE_WIDTH_MM);
    match definition.glyph {
        SymbolGlyph::Breaker => {
            draw_line(
                ops,
                symbol_point(symbol, -5.0, 5.0),
                symbol_point(symbol, 6.0, -6.0),
                height,
                SYMBOL_LINE_WIDTH_MM,
            );
        }
        SymbolGlyph::Contactor => {
            draw_line(
                ops,
                symbol_point(symbol, 0.0, -7.0),
                symbol_point(symbol, 8.0, -2.0),
                height,
                SYMBOL_LINE_WIDTH_MM,
            );
            draw_rotated_rectangle(ops, symbol, 16.0, 0.0, 12.0, 18.0, height);
        }
        SymbolGlyph::PushButton => {
            draw_line(
                ops,
                symbol_point(symbol, -8.0, 0.0),
                symbol_point(symbol, 8.0, 0.0),
                height,
                SYMBOL_LINE_WIDTH_MM,
            );
            draw_line(
                ops,
                symbol_point(symbol, 0.0, -7.0),
                symbol_point(symbol, 5.0, -12.0),
                height,
                SYMBOL_LINE_WIDTH_MM,
            );
        }
        SymbolGlyph::IndicatorLamp => {
            draw_circle(ops, symbol, 0.0, 0.0, 7.0, height);
            draw_line(
                ops,
                symbol_point(symbol, -4.0, -4.0),
                symbol_point(symbol, 4.0, 4.0),
                height,
                SYMBOL_LINE_WIDTH_MM,
            );
            draw_line(
                ops,
                symbol_point(symbol, -4.0, 4.0),
                symbol_point(symbol, 4.0, -4.0),
                height,
                SYMBOL_LINE_WIDTH_MM,
            );
        }
        SymbolGlyph::Motor => {
            draw_circle(ops, symbol, 0.0, 0.0, 8.0, height);
            let center = symbol_point(symbol, 0.0, 0.0);
            add_text(
                ops,
                height,
                center.0,
                center.1 + 1.2,
                "M",
                BuiltinFont::Helvetica,
                BODY_FONT_SIZE_PT,
            );
        }
        SymbolGlyph::Transformer => {
            draw_circle(ops, symbol, -4.0, -4.0, 6.0, height);
            draw_circle(ops, symbol, 4.0, 4.0, 6.0, height);
        }
        SymbolGlyph::Terminal => {
            draw_circle(ops, symbol, 0.0, 0.0, 5.0, height);
            draw_line(
                ops,
                symbol_point(symbol, -8.0, 0.0),
                symbol_point(symbol, 8.0, 0.0),
                height,
                SYMBOL_LINE_WIDTH_MM,
            );
        }
    }
    let label = format!("{}  {}", symbol.designation, symbol.label);
    add_text(
        ops,
        height,
        symbol.position.x,
        symbol.position.y + (SYMBOL_HALF_HEIGHT_MM + 4.0) * SYMBOL_SCALE,
        &label,
        BuiltinFont::Helvetica,
        BODY_FONT_SIZE_PT,
    );
}

fn draw_rotated_rectangle(
    ops: &mut Vec<Op>,
    symbol: &PlacedSymbol,
    center_x: f32,
    center_y: f32,
    width: f32,
    height: f32,
    page_height: f32,
) {
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    draw_path(
        ops,
        &[
            symbol_point(symbol, center_x - half_width, center_y - half_height),
            symbol_point(symbol, center_x + half_width, center_y - half_height),
            symbol_point(symbol, center_x + half_width, center_y + half_height),
            symbol_point(symbol, center_x - half_width, center_y + half_height),
        ],
        page_height,
        true,
        SYMBOL_LINE_WIDTH_MM,
    );
}

fn draw_circle(ops: &mut Vec<Op>, symbol: &PlacedSymbol, x: f32, y: f32, radius: f32, height: f32) {
    let points: Vec<(f32, f32)> = (0..CIRCLE_SEGMENTS)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / CIRCLE_SEGMENTS as f32;
            symbol_point(symbol, x + radius * angle.cos(), y + radius * angle.sin())
        })
        .collect();
    draw_path(ops, &points, height, true, SYMBOL_LINE_WIDTH_MM);
}

fn symbol_point(symbol: &PlacedSymbol, x: f32, y: f32) -> (f32, f32) {
    let (x, y) = rotate(x * SYMBOL_SCALE, y * SYMBOL_SCALE, symbol.rotation_quarters);
    (symbol.position.x + x, symbol.position.y + y)
}

fn rotate(x: f32, y: f32, quarter_turns: u8) -> (f32, f32) {
    match quarter_turns % QUARTER_TURNS_PER_REVOLUTION {
        0 => (x, y),
        1 => (-y, x),
        2 => (-x, -y),
        _ => (y, -x),
    }
}

fn draw_line(ops: &mut Vec<Op>, start: (f32, f32), end: (f32, f32), height: f32, width: f32) {
    draw_path(ops, &[start, end], height, false, width);
}

fn draw_path(ops: &mut Vec<Op>, points: &[(f32, f32)], height: f32, is_closed: bool, width: f32) {
    ops.push(Op::SetOutlineThickness {
        pt: Pt(width * PT_PER_MM),
    });
    ops.push(Op::DrawLine {
        line: Line {
            points: points
                .iter()
                .map(|(x, y)| LinePoint {
                    p: Point::new(Mm(*x), Mm(height - y)),
                    bezier: false,
                })
                .collect(),
            is_closed,
        },
    });
}

fn add_text(
    ops: &mut Vec<Op>,
    height: f32,
    x: f32,
    y: f32,
    text: &str,
    font: BuiltinFont,
    size: f32,
) {
    if text.is_empty() {
        return;
    }
    ops.extend([
        Op::StartTextSection,
        Op::SetFontSizeBuiltinFont {
            size: Pt(size),
            font,
        },
        Op::SetTextCursor {
            pos: Point::new(Mm(x), Mm(height - y)),
        },
        Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text(text.to_owned())],
            font,
        },
        Op::EndTextSection,
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Orientation, Page, SheetSize};

    #[test]
    fn exports_each_project_page_at_its_sheet_size() {
        let mut project = Project::new();
        let mut second_page = Page::new("Page 2".to_owned());
        second_page.sheet_size = SheetSize::A3;
        second_page.orientation = Orientation::Portrait;
        project.pages.push(second_page);

        let document = build_document(&project).expect("PDF document should be created");
        let mut warnings = Vec::new();
        let bytes = document.save(&PdfSaveOptions::default(), &mut warnings);

        assert_eq!(document.pages.len(), 2);
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn suggested_filename_replaces_invalid_characters() {
        assert_eq!(
            suggested_filename("Power: panel / north"),
            "Power_ panel _ north.pdf"
        );
        assert_eq!(suggested_filename("..."), "electrocraft.pdf");
    }
}
