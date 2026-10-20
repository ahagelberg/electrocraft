use eframe::egui;
use uuid::Uuid;

use crate::document::{Page, PinRef, PlacedSymbol, PointMm, SymbolDefinition, SymbolGlyph};

const SHEET_MARGIN_MM: f32 = 10.0;
const FRAME_STROKE_WIDTH: f32 = 1.0;
const SYMBOL_HALF_HEIGHT_MM: f32 = 16.0;
const HIT_TARGET_POINTS: f32 = 20.0;
const QUARTER_TURNS_PER_REVOLUTION: u8 = 4;
const MIN_SYMBOL_SCALE: f32 = 0.75;
const SYMBOL_SIZE_FACTOR: f32 = 0.25;
const MIN_SYMBOL_STROKE_WIDTH: f32 = 0.65;
const MIN_MOTOR_LABEL_SIZE: f32 = 4.0;
const GRID_SPACING_MM: f32 = 10.0;
const MIN_GRID_SPACING_POINTS: f32 = 4.0;
const CANVAS_ZOOM_STEP: f32 = 1.2;
const MIN_CANVAS_ZOOM: f32 = 0.35;
const MAX_CANVAS_ZOOM: f32 = 4.0;
const SCROLL_ZOOM_DIVISOR: f32 = 120.0;
const PIN_HIT_RADIUS_POINTS: f32 = 9.0;
const PIN_MARKER_RADIUS_POINTS: f32 = 3.0;

#[derive(Clone, Copy)]
pub struct CanvasView {
    pub zoom: f32,
    pub pan: egui::Vec2,
    dragged_symbol: Option<Uuid>,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            dragged_symbol: None,
        }
    }
}

impl CanvasView {
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * CANVAS_ZOOM_STEP).min(MAX_CANVAS_ZOOM);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / CANVAS_ZOOM_STEP).max(MIN_CANVAS_ZOOM);
    }

    pub fn fit(&mut self) {
        *self = Self::default();
    }
}

pub enum CanvasAction {
    None,
    Select(Option<Uuid>),
    Place(Uuid, PointMm),
    Move(Uuid, PointMm),
    PinClicked(PinRef),
}

pub fn show(
    ui: &mut egui::Ui,
    page: &Page,
    definitions: &[SymbolDefinition],
    selected_symbol: Option<Uuid>,
    pending_definition: Option<Uuid>,
    view: &mut CanvasView,
    wire_tool: bool,
) -> CanvasAction {
    let available = ui.available_size();
    let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
    let (space_down, pointer_delta, scroll_delta) = ui.ctx().input(|input| {
        (
            input.key_down(egui::Key::Space),
            input.pointer.delta(),
            input.smooth_scroll_delta.y,
        )
    });
    if response.dragged() && space_down {
        view.pan += pointer_delta;
        view.dragged_symbol = None;
        return CanvasAction::None;
    }
    if response.hovered() && scroll_delta != 0.0 {
        view.zoom = (view.zoom * CANVAS_ZOOM_STEP.powf(scroll_delta / SCROLL_ZOOM_DIVISOR))
            .clamp(MIN_CANVAS_ZOOM, MAX_CANVAS_ZOOM);
    }
    let (page_width, page_height) = page.dimensions_mm();
    let scale = (available.x / page_width).min(available.y / page_height) * 0.9 * view.zoom;
    let sheet_size = egui::vec2(page_width * scale, page_height * scale);
    let sheet = egui::Rect::from_center_size(response.rect.center() + view.pan, sheet_size);
    let margin = SHEET_MARGIN_MM * scale;
    let frame = sheet.shrink(margin);

    painter.rect_filled(response.rect, 0, ui.visuals().faint_bg_color);
    painter.rect_filled(sheet, 0, egui::Color32::WHITE);
    draw_grid(&painter, frame, scale);
    painter.rect_stroke(
        sheet,
        0,
        egui::Stroke::new(FRAME_STROKE_WIDTH, egui::Color32::from_gray(65)),
        egui::StrokeKind::Inside,
    );
    painter.rect_stroke(
        frame,
        0,
        egui::Stroke::new(FRAME_STROKE_WIDTH, egui::Color32::from_gray(105)),
        egui::StrokeKind::Inside,
    );
    draw_title_block(&painter, frame, page, scale);
    draw_connections(&painter, sheet, scale, page, definitions);
    for symbol in &page.symbols {
        let Some(definition) = definitions
            .iter()
            .find(|definition| definition.id == symbol.definition_id)
        else {
            continue;
        };
        draw_symbol(
            &painter,
            sheet,
            scale,
            symbol,
            definition.glyph,
            selected_symbol == Some(symbol.id),
        );
        if wire_tool {
            draw_pin_markers(&painter, sheet, scale, symbol, definition);
        }
    }

    if response.drag_started() && !wire_tool && pending_definition.is_none() {
        view.dragged_symbol = response
            .interact_pointer_pos()
            .and_then(|pointer| find_symbol_at(pointer, sheet, scale, page));
    }
    if response.dragged()
        && !wire_tool
        && let (Some(symbol_id), Some(pointer)) =
            (view.dragged_symbol, response.interact_pointer_pos())
    {
        return CanvasAction::Move(
            symbol_id,
            PointMm {
                x: (pointer.x - sheet.left()) / scale,
                y: (pointer.y - sheet.top()) / scale,
            },
        );
    }
    if response.drag_stopped() {
        view.dragged_symbol = None;
    }

    if response.clicked() {
        let Some(pointer) = response.interact_pointer_pos() else {
            return CanvasAction::None;
        };
        if !sheet.contains(pointer) {
            return CanvasAction::Select(None);
        }
        if wire_tool {
            return find_pin_at(pointer, sheet, scale, page, definitions)
                .map_or(CanvasAction::None, CanvasAction::PinClicked);
        }
        if let Some(definition_id) = pending_definition {
            let position = PointMm {
                x: (pointer.x - sheet.left()) / scale,
                y: (pointer.y - sheet.top()) / scale,
            };
            return CanvasAction::Place(definition_id, position);
        }
        let symbol_id = find_symbol_at(pointer, sheet, scale, page);
        return CanvasAction::Select(symbol_id);
    }
    CanvasAction::None
}

fn find_symbol_at(pointer: egui::Pos2, sheet: egui::Rect, scale: f32, page: &Page) -> Option<Uuid> {
    page.symbols.iter().rev().find_map(|symbol| {
        let center = symbol_screen_position(sheet, scale, symbol.position.x, symbol.position.y);
        let hit_rect = egui::Rect::from_center_size(
            center,
            egui::vec2(HIT_TARGET_POINTS * 2.0, HIT_TARGET_POINTS * 2.0),
        );
        hit_rect.contains(pointer).then_some(symbol.id)
    })
}

fn draw_connections(
    painter: &egui::Painter,
    sheet: egui::Rect,
    scale: f32,
    page: &Page,
    definitions: &[SymbolDefinition],
) {
    let stroke = egui::Stroke::new(1.2, egui::Color32::from_gray(35));
    for connection in &page.connections {
        let Some(mut previous) = pin_screen_position(
            sheet,
            scale,
            page,
            definitions,
            connection.from.symbol_id,
            &connection.from.pin_id,
        ) else {
            continue;
        };
        for vertex in &connection.vertices {
            let point = symbol_screen_position(sheet, scale, vertex.x, vertex.y);
            painter.line_segment([previous, point], stroke);
            previous = point;
        }
        if let Some(end) = pin_screen_position(
            sheet,
            scale,
            page,
            definitions,
            connection.to.symbol_id,
            &connection.to.pin_id,
        ) {
            painter.line_segment([previous, end], stroke);
        }
    }
}

fn draw_pin_markers(
    painter: &egui::Painter,
    sheet: egui::Rect,
    scale: f32,
    symbol: &PlacedSymbol,
    definition: &SymbolDefinition,
) {
    let color = egui::Color32::from_rgb(0, 112, 105);
    for pin in &definition.pins {
        let position = pin_screen_position_for_symbol(sheet, scale, symbol, pin);
        painter.circle_filled(position, PIN_MARKER_RADIUS_POINTS, color);
    }
}

fn find_pin_at(
    pointer: egui::Pos2,
    sheet: egui::Rect,
    scale: f32,
    page: &Page,
    definitions: &[SymbolDefinition],
) -> Option<PinRef> {
    let mut closest: Option<(PinRef, f32)> = None;
    for symbol in &page.symbols {
        let Some(definition) = definitions
            .iter()
            .find(|definition| definition.id == symbol.definition_id)
        else {
            continue;
        };
        for pin in &definition.pins {
            let position = pin_screen_position_for_symbol(sheet, scale, symbol, pin);
            let distance = pointer.distance(position);
            if distance <= PIN_HIT_RADIUS_POINTS
                && closest
                    .as_ref()
                    .is_none_or(|(_, closest_distance)| distance < *closest_distance)
            {
                closest = Some((
                    PinRef {
                        symbol_id: symbol.id,
                        pin_id: pin.id.clone(),
                    },
                    distance,
                ));
            }
        }
    }
    closest.map(|(pin_ref, _)| pin_ref)
}

fn pin_screen_position(
    sheet: egui::Rect,
    scale: f32,
    page: &Page,
    definitions: &[SymbolDefinition],
    symbol_id: Uuid,
    pin_id: &str,
) -> Option<egui::Pos2> {
    let symbol = page.symbols.iter().find(|symbol| symbol.id == symbol_id)?;
    let definition = definitions
        .iter()
        .find(|definition| definition.id == symbol.definition_id)?;
    let pin = definition.pins.iter().find(|pin| pin.id == pin_id)?;
    Some(pin_screen_position_for_symbol(sheet, scale, symbol, pin))
}

fn pin_screen_position_for_symbol(
    sheet: egui::Rect,
    scale: f32,
    symbol: &PlacedSymbol,
    pin: &crate::document::PinDefinition,
) -> egui::Pos2 {
    let center = symbol_screen_position(sheet, scale, symbol.position.x, symbol.position.y);
    let symbol_scale = scale.max(MIN_SYMBOL_SCALE) * SYMBOL_SIZE_FACTOR;
    rotated_point(
        center,
        pin.position.x * symbol_scale,
        pin.position.y * symbol_scale,
        symbol.rotation_quarters,
    )
}

fn draw_grid(painter: &egui::Painter, frame: egui::Rect, scale: f32) {
    let spacing = GRID_SPACING_MM * scale;
    if spacing < MIN_GRID_SPACING_POINTS {
        return;
    }
    let stroke = egui::Stroke::new(0.5, egui::Color32::from_gray(238));
    let mut x = frame.left();
    while x < frame.right() {
        painter.line_segment(
            [egui::pos2(x, frame.top()), egui::pos2(x, frame.bottom())],
            stroke,
        );
        x += spacing;
    }
    let mut y = frame.top();
    while y < frame.bottom() {
        painter.line_segment(
            [egui::pos2(frame.left(), y), egui::pos2(frame.right(), y)],
            stroke,
        );
        y += spacing;
    }
}

fn draw_title_block(painter: &egui::Painter, frame: egui::Rect, page: &Page, scale: f32) {
    let block_width = (180.0 * scale).min(frame.width() * 0.58);
    let block_height = (34.0 * scale).min(frame.height() * 0.2);
    let block = egui::Rect::from_min_size(
        egui::pos2(frame.right() - block_width, frame.bottom() - block_height),
        egui::vec2(block_width, block_height),
    );
    let stroke = egui::Stroke::new(FRAME_STROKE_WIDTH, egui::Color32::from_gray(105));
    painter.rect_stroke(block, 0, stroke, egui::StrokeKind::Inside);
    let split_x = block.left() + block.width() * 0.62;
    let split_y = block.top() + block.height() * 0.56;
    painter.line_segment(
        [
            egui::pos2(split_x, block.top()),
            egui::pos2(split_x, block.bottom()),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(block.left(), split_y),
            egui::pos2(block.right(), split_y),
        ],
        stroke,
    );
    let text_color = egui::Color32::from_gray(35);
    let font = egui::FontId::proportional((9.0 * scale).clamp(6.0, 12.0));
    painter.text(
        block.left_top() + egui::vec2(4.0 * scale, 3.0 * scale),
        egui::Align2::LEFT_TOP,
        &page.title_block.project_title,
        font.clone(),
        text_color,
    );
    painter.text(
        egui::pos2(block.left() + 4.0 * scale, split_y + 3.0 * scale),
        egui::Align2::LEFT_TOP,
        &page.title_block.page_title,
        font.clone(),
        text_color,
    );
    painter.text(
        egui::pos2(split_x + 4.0 * scale, split_y + 3.0 * scale),
        egui::Align2::LEFT_TOP,
        format!(
            "{}  Rev {}",
            page.title_block.page_number, page.title_block.revision
        ),
        font,
        text_color,
    );
}

fn draw_symbol(
    painter: &egui::Painter,
    sheet: egui::Rect,
    scale: f32,
    symbol: &PlacedSymbol,
    glyph: SymbolGlyph,
    selected: bool,
) {
    let center = symbol_screen_position(sheet, scale, symbol.position.x, symbol.position.y);
    let stroke_color = if selected {
        egui::Color32::from_rgb(0, 112, 105)
    } else {
        egui::Color32::from_gray(25)
    };
    let symbol_scale = scale.max(MIN_SYMBOL_SCALE) * SYMBOL_SIZE_FACTOR;
    let stroke = egui::Stroke::new(
        (1.6 * SYMBOL_SIZE_FACTOR).max(MIN_SYMBOL_STROKE_WIDTH),
        stroke_color,
    );
    let half_height = SYMBOL_HALF_HEIGHT_MM;
    let point = |x: f32, y: f32| {
        rotated_point(
            center,
            x * symbol_scale,
            y * symbol_scale,
            symbol.rotation_quarters,
        )
    };
    let top = point(0.0, -half_height);
    let bottom = point(0.0, half_height);

    painter.line_segment([top, point(0.0, -7.0)], stroke);
    painter.line_segment([point(0.0, 7.0), bottom], stroke);
    match glyph {
        SymbolGlyph::Breaker => {
            painter.line_segment([point(-5.0, 5.0), point(6.0, -6.0)], stroke);
        }
        SymbolGlyph::Contactor => {
            painter.line_segment([point(0.0, -7.0), point(8.0, -2.0)], stroke);
            draw_rotated_rect(painter, &point, 16.0, 0.0, 12.0, 18.0, stroke);
        }
        SymbolGlyph::PushButton => {
            painter.line_segment([point(-8.0, 0.0), point(8.0, 0.0)], stroke);
            painter.line_segment([point(0.0, -7.0), point(5.0, -12.0)], stroke);
        }
        SymbolGlyph::IndicatorLamp => {
            painter.circle_stroke(point(0.0, 0.0), 7.0 * symbol_scale, stroke);
            painter.line_segment([point(-4.0, -4.0), point(4.0, 4.0)], stroke);
            painter.line_segment([point(-4.0, 4.0), point(4.0, -4.0)], stroke);
        }
        SymbolGlyph::Motor => {
            painter.circle_stroke(point(0.0, 0.0), 8.0 * symbol_scale, stroke);
            painter.text(
                point(0.0, 0.0),
                egui::Align2::CENTER_CENTER,
                "M",
                egui::FontId::proportional((10.0 * symbol_scale).max(MIN_MOTOR_LABEL_SIZE)),
                stroke_color,
            );
        }
        SymbolGlyph::Transformer => {
            painter.circle_stroke(point(-4.0, -4.0), 6.0 * symbol_scale, stroke);
            painter.circle_stroke(point(4.0, 4.0), 6.0 * symbol_scale, stroke);
        }
        SymbolGlyph::Terminal => {
            painter.circle_stroke(point(0.0, 0.0), 5.0 * symbol_scale, stroke);
            painter.line_segment([point(-8.0, 0.0), point(8.0, 0.0)], stroke);
        }
    }
    painter.text(
        egui::pos2(
            center.x,
            center.y + half_height * symbol_scale + 4.0 * symbol_scale,
        ),
        egui::Align2::CENTER_TOP,
        format!("{}  {}", symbol.designation, symbol.label),
        egui::FontId::proportional(10.0),
        stroke_color,
    );
}

fn draw_rotated_rect(
    painter: &egui::Painter,
    point: &impl Fn(f32, f32) -> egui::Pos2,
    center_x: f32,
    center_y: f32,
    width: f32,
    height: f32,
    stroke: egui::Stroke,
) {
    let left = center_x - width / 2.0;
    let right = center_x + width / 2.0;
    let top = center_y - height / 2.0;
    let bottom = center_y + height / 2.0;
    let corners = [
        point(left, top),
        point(right, top),
        point(right, bottom),
        point(left, bottom),
        point(left, top),
    ];
    for edge in corners.windows(2) {
        painter.line_segment([edge[0], edge[1]], stroke);
    }
}

fn rotated_point(center: egui::Pos2, x: f32, y: f32, quarter_turns: u8) -> egui::Pos2 {
    let (x, y) = match quarter_turns % QUARTER_TURNS_PER_REVOLUTION {
        0 => (x, y),
        1 => (-y, x),
        2 => (-x, -y),
        _ => (y, -x),
    };
    center + egui::vec2(x, y)
}

fn symbol_screen_position(sheet: egui::Rect, scale: f32, x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(sheet.left() + x * scale, sheet.top() + y * scale)
}
