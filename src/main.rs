mod app;
mod canvas;
mod document;
mod pdf_export;
mod project_io;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native(
        "Electrocraft",
        options,
        Box::new(|_| Ok(Box::<app::ElectrocraftApp>::default())),
    )
}
