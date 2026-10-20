# Electrocraft

Electrocraft is a cross-platform Rust desktop application for electrical control and power diagrams. The project file is versioned JSON; the application is licensed under MIT.

## Build

Install Rust 1.92 or newer, then run:

```powershell
cargo run
```

The GUI uses `eframe`/`egui` for Windows and Linux. `serde` and `serde_json` define the portable project format, `uuid` provides stable object identifiers, and `rfd` provides native file dialogs. The direct dependencies use MIT or MIT/Apache-2.0 licenses.

## Current scope

The editor currently provides fixed A4/A3 portrait and landscape sheets; control, power single-line, and power multi-line page types; searchable generic starter symbols; basic designations; symbol placement, drag-movement, selection, rotation, and deletion; a sheet grid with zoom and pan; a two-click pin-to-pin wire tool; project open/save as JSON; multipage vector PDF export; and Light, Dark, and System themes. The document format stores symbol pins, explicit pin-to-pin connections, annotations, title-block fields, and optional local-library references.

The starter geometry is independently authored and IEC 60617-oriented, not yet validated against an authorized standards reference. The title block and frame are preliminary and are not a compliance claim. Structured wiring tools, symbol authoring, manufacturer catalogs, and electrical rule checks remain future work.

Project schema changes must preserve backward compatibility through explicit migrations; files with unsupported schema versions are rejected.