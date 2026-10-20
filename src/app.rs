use std::path::PathBuf;

use eframe::egui;
use uuid::Uuid;

use crate::canvas::{self, CanvasAction, CanvasView};
use crate::document::{
    Connection, DiagramType, Orientation, PinRef, PlacedSymbol, Project, SheetSize, SymbolCategory,
    SymbolGlyph,
};
use crate::pdf_export;
use crate::project_io;

const LEFT_PANEL_WIDTH: f32 = 224.0;
const RIGHT_PANEL_WIDTH: f32 = 260.0;
const DESIGNATION_START: usize = 1;
const QUARTER_TURNS_PER_REVOLUTION: u8 = 4;
const ZOOM_PERCENT_MULTIPLIER: f32 = 100.0;

pub struct ElectrocraftApp {
    project: Project,
    active_page: usize,
    symbol_search: String,
    selected_symbol: Option<Uuid>,
    pending_definition: Option<Uuid>,
    canvas_view: CanvasView,
    wire_tool: bool,
    wire_start: Option<PinRef>,
    theme: egui::ThemePreference,
    project_path: Option<PathBuf>,
    status: String,
}

impl Default for ElectrocraftApp {
    fn default() -> Self {
        Self {
            project: Project::new(),
            active_page: 0,
            symbol_search: String::new(),
            selected_symbol: None,
            pending_definition: None,
            canvas_view: CanvasView::default(),
            wire_tool: false,
            wire_start: None,
            theme: egui::ThemePreference::System,
            project_path: None,
            status: "Ready".to_owned(),
        }
    }
}

impl eframe::App for ElectrocraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.set_theme(self.theme);

        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(page) = self.project.pages.get(self.active_page) {
                        let (width, height) = page.dimensions_mm();
                        ui.label(format!("{} x {} mm", width as u32, height as u32));
                    }
                });
            });
        });

        egui::Panel::left("symbol_palette")
            .resizable(true)
            .default_size(LEFT_PANEL_WIDTH)
            .show(ui, |ui| self.palette(ui));
        egui::Panel::right("properties")
            .resizable(true)
            .default_size(RIGHT_PANEL_WIDTH)
            .show(ui, |ui| self.properties(ui));

        egui::CentralPanel::default().show(ui, |ui| {
            let Some(page) = self.project.pages.get(self.active_page) else {
                ui.centered_and_justified(|ui| ui.label("This project has no pages."));
                return;
            };
            let action = canvas::show(
                ui,
                page,
                &self.project.symbol_definitions,
                self.selected_symbol,
                self.pending_definition,
                &mut self.canvas_view,
                self.wire_tool,
            );
            match action {
                CanvasAction::None => {}
                CanvasAction::Select(symbol_id) => {
                    self.selected_symbol = symbol_id;
                    self.pending_definition = None;
                }
                CanvasAction::Place(definition_id, position) => {
                    self.place_symbol(definition_id, position.x, position.y);
                }
                CanvasAction::Move(symbol_id, position) => {
                    self.move_symbol(symbol_id, position);
                }
                CanvasAction::PinClicked(pin_ref) => self.handle_pin_click(pin_ref),
            }
        });
    }
}

impl ElectrocraftApp {
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New project").clicked() {
                    self.project = Project::new();
                    self.project_path = None;
                    self.active_page = 0;
                    self.selected_symbol = None;
                    self.wire_start = None;
                    self.wire_tool = false;
                    self.status = "New project".to_owned();
                    ui.close();
                }
                if ui.button("Open project...").clicked() {
                    self.open_project();
                    ui.close();
                }
                if ui.button("Save").clicked() {
                    self.save_project(false);
                    ui.close();
                }
                if ui.button("Save as...").clicked() {
                    self.save_project(true);
                    ui.close();
                }
                if ui.button("Export PDF...").clicked() {
                    self.export_pdf();
                    ui.close();
                }
            });

            ui.separator();
            if ui.button("New").clicked() {
                self.project = Project::new();
                self.project_path = None;
                self.active_page = 0;
                self.selected_symbol = None;
                self.wire_start = None;
                self.wire_tool = false;
                self.status = "New project".to_owned();
            }
            if ui.button("Open").clicked() {
                self.open_project();
            }
            if ui.button("Save").clicked() {
                self.save_project(false);
            }

            ui.separator();
            ui.label("Theme");
            ui.selectable_value(&mut self.theme, egui::ThemePreference::Light, "Light");
            ui.selectable_value(&mut self.theme, egui::ThemePreference::Dark, "Dark");
            ui.selectable_value(&mut self.theme, egui::ThemePreference::System, "System");

            ui.separator();
            if ui.button("-").on_hover_text("Zoom out").clicked() {
                self.canvas_view.zoom_out();
            }
            ui.label(format!(
                "{:.0}%",
                self.canvas_view.zoom * ZOOM_PERCENT_MULTIPLIER
            ));
            if ui.button("+").on_hover_text("Zoom in").clicked() {
                self.canvas_view.zoom_in();
            }
            if ui
                .button("Fit")
                .on_hover_text("Fit sheet to view")
                .clicked()
            {
                self.canvas_view.fit();
            }

            if ui.selectable_label(self.wire_tool, "Wire").clicked() {
                self.wire_tool = !self.wire_tool;
                self.wire_start = None;
                self.pending_definition = None;
                self.status = if self.wire_tool {
                    "Select a start pin".to_owned()
                } else {
                    "Wire tool off".to_owned()
                };
            }
            if self.wire_tool
                && let Some(start) = &self.wire_start
            {
                let designation = self
                    .project
                    .pages
                    .get(self.active_page)
                    .and_then(|page| {
                        page.symbols
                            .iter()
                            .find(|symbol| symbol.id == start.symbol_id)
                    })
                    .map_or("?", |symbol| symbol.designation.as_str());
                ui.label(format!("From {designation}:{}", start.pin_id));
                if ui.button("Cancel wire").clicked() {
                    self.wire_start = None;
                    self.status = "Select a start pin".to_owned();
                }
            }

            ui.separator();
            let previous_page = self.active_page;
            let selected_page = self
                .project
                .pages
                .get(self.active_page)
                .map_or("No page", |page| page.name.as_str());
            egui::ComboBox::from_id_salt("active_page")
                .selected_text(selected_page)
                .show_ui(ui, |ui| {
                    for (index, page) in self.project.pages.iter().enumerate() {
                        ui.selectable_value(&mut self.active_page, index, &page.name);
                    }
                });
            if self.active_page != previous_page {
                self.selected_symbol = None;
                self.wire_start = None;
            }
            if ui.button("Add page").clicked() {
                let page_number = self.project.pages.len() + DESIGNATION_START;
                self.project
                    .pages
                    .push(crate::document::Page::new(format!("Page {page_number}")));
                self.active_page = self.project.pages.len() - DESIGNATION_START;
                self.selected_symbol = None;
                self.wire_start = None;
                self.status = "Page added".to_owned();
            }

            if self.pending_definition.is_some() {
                ui.separator();
                ui.label("Click the sheet to place");
                if ui.button("Cancel").clicked() {
                    self.pending_definition = None;
                }
            }
        });
    }

    fn palette(&mut self, ui: &mut egui::Ui) {
        ui.heading("Symbols");
        ui.label("Generic starter set");
        ui.add(egui::TextEdit::singleline(&mut self.symbol_search).hint_text("Search symbols"));
        ui.separator();
        let query = self.symbol_search.trim().to_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for category in [
                SymbolCategory::ProtectionSwitching,
                SymbolCategory::Control,
                SymbolCategory::PowerEquipment,
                SymbolCategory::WiringInfrastructure,
            ] {
                let entries: Vec<(Uuid, String)> = self
                    .project
                    .symbol_definitions
                    .iter()
                    .filter(|definition| {
                        definition.category == category
                            && definition.name.to_lowercase().contains(&query)
                    })
                    .map(|definition| (definition.id, definition.name.clone()))
                    .collect();
                if entries.is_empty() {
                    continue;
                }
                egui::CollapsingHeader::new(category.label())
                    .default_open(true)
                    .show(ui, |ui| {
                        for (definition_id, name) in entries {
                            let selected = self.pending_definition == Some(definition_id);
                            if ui.selectable_label(selected, name).clicked() {
                                self.pending_definition = Some(definition_id);
                                self.selected_symbol = None;
                                self.status = "Click the sheet to place the symbol".to_owned();
                            }
                        }
                    });
            }
        });
    }

    fn properties(&mut self, ui: &mut egui::Ui) {
        ui.heading("Page");
        let Some(page) = self.project.pages.get_mut(self.active_page) else {
            ui.label("No page selected");
            return;
        };

        egui::Grid::new("page_properties")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Diagram");
                egui::ComboBox::from_id_salt("diagram_type")
                    .selected_text(page.diagram_type.label())
                    .show_ui(ui, |ui| {
                        for diagram_type in [
                            DiagramType::Control,
                            DiagramType::PowerSingleLine,
                            DiagramType::PowerMultiLine,
                        ] {
                            ui.selectable_value(
                                &mut page.diagram_type,
                                diagram_type,
                                diagram_type.label(),
                            );
                        }
                    });
                ui.end_row();

                ui.label("Sheet");
                egui::ComboBox::from_id_salt("sheet_size")
                    .selected_text(match page.sheet_size {
                        SheetSize::A4 => "A4",
                        SheetSize::A3 => "A3",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut page.sheet_size, SheetSize::A4, "A4");
                        ui.selectable_value(&mut page.sheet_size, SheetSize::A3, "A3");
                    });
                ui.end_row();

                ui.label("Orientation");
                egui::ComboBox::from_id_salt("orientation")
                    .selected_text(match page.orientation {
                        Orientation::Portrait => "Portrait",
                        Orientation::Landscape => "Landscape",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut page.orientation,
                            Orientation::Portrait,
                            "Portrait",
                        );
                        ui.selectable_value(
                            &mut page.orientation,
                            Orientation::Landscape,
                            "Landscape",
                        );
                    });
                ui.end_row();
            });

        ui.separator();
        ui.heading("Title block");
        ui.label("Project");
        ui.text_edit_singleline(&mut self.project.title);
        page.title_block.project_title = self.project.title.clone();
        ui.label("Page title");
        ui.text_edit_singleline(&mut page.title_block.page_title);
        ui.label("Prepared by");
        ui.text_edit_singleline(&mut page.title_block.prepared_by);

        ui.separator();
        ui.heading("Selection");
        let Some(selected_id) = self.selected_symbol else {
            ui.label("Select a placed symbol to edit its properties.");
            return;
        };
        let mut rotate = false;
        let mut delete = false;
        {
            let Some(symbol) = page
                .symbols
                .iter_mut()
                .find(|symbol| symbol.id == selected_id)
            else {
                self.selected_symbol = None;
                ui.label("No symbol selected.");
                return;
            };
            ui.label("Designation");
            ui.text_edit_singleline(&mut symbol.designation);
            ui.label("Description");
            ui.text_edit_singleline(&mut symbol.label);
            ui.horizontal(|ui| {
                rotate = ui.button("Rotate").clicked();
                delete = ui.button("Delete").clicked();
            });
        }
        if rotate
            && let Some(symbol) = page
                .symbols
                .iter_mut()
                .find(|symbol| symbol.id == selected_id)
        {
            symbol.rotation_quarters =
                (symbol.rotation_quarters + 1) % QUARTER_TURNS_PER_REVOLUTION;
        }
        if delete {
            page.symbols.retain(|placed| placed.id != selected_id);
            page.connections.retain(|connection| {
                connection.from.symbol_id != selected_id && connection.to.symbol_id != selected_id
            });
            self.selected_symbol = None;
        }
    }

    fn export_pdf(&mut self) {
        let filename = pdf_export::suggested_filename(&self.project.title);
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name(filename)
            .save_file()
        else {
            return;
        };
        match pdf_export::write_project(&self.project, &path) {
            Ok(()) => self.status = format!("PDF exported to {}", path.display()),
            Err(error) => self.status = format!("PDF export failed: {error}"),
        }
    }

    fn place_symbol(&mut self, definition_id: Uuid, x: f32, y: f32) {
        let Some(definition) = self
            .project
            .symbol_definitions
            .iter()
            .find(|definition| definition.id == definition_id)
        else {
            self.pending_definition = None;
            return;
        };
        let prefix = designation_prefix(definition.glyph);
        let Some(page) = self.project.pages.get_mut(self.active_page) else {
            return;
        };
        let next_number = page
            .symbols
            .iter()
            .filter(|symbol| symbol.designation.starts_with(prefix))
            .count()
            + DESIGNATION_START;
        let placed = PlacedSymbol {
            id: Uuid::new_v4(),
            definition_id,
            designation: format!("{prefix}{next_number}"),
            label: definition.name.clone(),
            position: crate::document::PointMm { x, y },
            rotation_quarters: 0,
        };
        self.selected_symbol = Some(placed.id);
        page.symbols.push(placed);
        self.pending_definition = None;
        self.status = "Symbol placed".to_owned();
    }

    fn handle_pin_click(&mut self, pin_ref: PinRef) {
        let Some(start) = self.wire_start.take() else {
            self.status = format!(
                "Start pin {}:{} selected",
                pin_ref.symbol_id, pin_ref.pin_id
            );
            self.wire_start = Some(pin_ref);
            return;
        };
        if start.symbol_id == pin_ref.symbol_id && start.pin_id == pin_ref.pin_id {
            self.status = "Choose a different end pin".to_owned();
            self.wire_start = Some(start);
            return;
        }
        let Some(page) = self.project.pages.get_mut(self.active_page) else {
            self.wire_start = Some(start);
            return;
        };
        let duplicate = page.connections.iter().any(|connection| {
            (connection.from.symbol_id == start.symbol_id
                && connection.from.pin_id == start.pin_id
                && connection.to.symbol_id == pin_ref.symbol_id
                && connection.to.pin_id == pin_ref.pin_id)
                || (connection.to.symbol_id == start.symbol_id
                    && connection.to.pin_id == start.pin_id
                    && connection.from.symbol_id == pin_ref.symbol_id
                    && connection.from.pin_id == pin_ref.pin_id)
        });
        if duplicate {
            self.status = "Connection already exists".to_owned();
        } else {
            page.connections.push(Connection {
                id: Uuid::new_v4(),
                from: start,
                to: pin_ref,
                vertices: Vec::new(),
            });
            self.status = "Connection added".to_owned();
        }
    }

    fn move_symbol(&mut self, symbol_id: Uuid, position: crate::document::PointMm) {
        let Some(symbol) = self
            .project
            .pages
            .get_mut(self.active_page)
            .and_then(|page| {
                page.symbols
                    .iter_mut()
                    .find(|symbol| symbol.id == symbol_id)
            })
        else {
            return;
        };
        symbol.position = position;
        self.selected_symbol = Some(symbol_id);
    }

    fn open_project(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Electrocraft project", &["json"])
            .pick_file()
        else {
            return;
        };
        match project_io::load(&path) {
            Ok(project) => {
                self.project = project;
                self.project_path = Some(path);
                self.active_page = 0;
                self.selected_symbol = None;
                self.pending_definition = None;
                self.wire_start = None;
                self.status = "Project opened".to_owned();
            }
            Err(error) => self.status = format!("Open failed: {error}"),
        }
    }

    fn save_project(&mut self, force_dialog: bool) {
        let path = if force_dialog {
            None
        } else {
            self.project_path.clone()
        };
        let path = path.or_else(|| {
            rfd::FileDialog::new()
                .add_filter("Electrocraft project", &["json"])
                .set_file_name("project.json")
                .save_file()
        });
        let Some(path) = path else {
            return;
        };
        match project_io::save(&path, &self.project) {
            Ok(()) => {
                self.project_path = Some(path);
                self.status = "Project saved".to_owned();
            }
            Err(error) => self.status = format!("Save failed: {error}"),
        }
    }
}

fn designation_prefix(glyph: SymbolGlyph) -> &'static str {
    match glyph {
        SymbolGlyph::Breaker => "Q",
        SymbolGlyph::Contactor => "K",
        SymbolGlyph::PushButton => "S",
        SymbolGlyph::IndicatorLamp => "H",
        SymbolGlyph::Motor => "M",
        SymbolGlyph::Transformer => "T",
        SymbolGlyph::Terminal => "X",
    }
}
