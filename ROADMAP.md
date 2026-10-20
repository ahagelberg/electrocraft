# Electrocraft Roadmap

This roadmap tracks delivery order, not calendar dates. A milestone is complete when its listed outcomes are implemented and verified; IEC-oriented geometry and a preliminary title block are not claims of standards compliance.

## Baseline: Core Editor — Complete

- Native Rust desktop application for Windows and Linux.
- A4/A3 portrait and landscape pages, with control, power single-line, and power multi-line diagram types.
- Symbol placement, selection, movement, rotation, deletion, basic designations, and pin-to-pin connections.
- Project files stored as versioned JSON, with backward-compatible schema migrations.
- Light, dark, and system themes; native window geometry persistence.
- Multipage vector PDF export at the project's sheet sizes.

## 1. Reliable Project and PDF Workflows — In Progress

- Verify PDF page count, dimensions, orientation, and drawing placement for every supported sheet configuration.
- Exercise export with populated diagrams, title-block fields, connections, annotations, and symbols.
- Handle unsupported text characters predictably and report save failures clearly.
- Add regression coverage for project migrations and export output.

## 2. Structured Wiring

- Support explicit wire segments and junctions rather than only direct pin-to-pin links.
- Add wire labels and electrical properties while preserving stable connectivity in the project format.
- Provide editing operations for routing, splitting, joining, and removing wires.
- Add cross-page references for connected signals.

## 3. Symbol Libraries and Authoring

- Define a stable, documented symbol-library format and improve local-library management.
- Add symbol creation and editing for graphics, pins, metadata, and designations.
- Validate starter symbols against authorized IEC 60617 references before making compliance claims.
- Add manufacturer and component catalogs that can reference reusable symbols.

## 4. Electrical Validation and Reports

- Add opt-in electrical rule checks for connectivity, pin compatibility, and common schematic errors.
- Explain each finding with a location and a direct path to the affected object.
- Generate practical project reports from the document model, including connection and component lists.

## 5. Publishing and Release Readiness

- Complete PDF sheet frames and title blocks for the supported documentation workflows.
- Test build, project round-trip, and export workflows on Windows and Linux.
- Audit dependency licenses and document installation and project-format compatibility.
- Establish release packaging and regression checks for supported platforms.

## Principles

- Keep the project format portable and evolve it through explicit migrations.
- Prefer explicit connectivity and editable vector geometry over presentation-only approximations.
- Validate standards-related symbols and documentation against authorized references; do not infer compliance from visual similarity.
- Keep electrical checks transparent and actionable, and avoid silently rewriting project data.