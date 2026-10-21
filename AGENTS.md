# AGENTS.md – Hinweise für KI-Assistenten

Projektziel und Vorgaben: siehe `CLAUDE.md`. Ausführliche Anleitungen:
`.claude/skills/{ruf-app,rncl-component,ruf-bridge}/SKILL.md`.

## Aufbau

- `crates/ruf` – System-Ebene + Logik-Basis.
  - `cpp/ruf_widgets.{h,cpp}` – C++-Shim über QtWidgets (einziger Ort für Qt-C++).
  - `src/ffi.rs` – `#[cxx_qt::bridge]` (einziger Ort für FFI/`unsafe`).
  - `src/widget.rs` – sicherer `Widget`-Verweis (QPointer-basiert).
  - `src/component.rs` – Trait `Component` + `base`-Eigenschaften.
  - `src/registry.rs`, `src/property.rs` – Designer-Schnittstelle.
- `crates/rncl` – Komponenten (`RnclButton` in `src/button.rs` ist die Referenz),
  Beispiele in `examples/`, Qt-Integrationstests in `tests/components.rs`.
- `docs/de`, `docs/en` – Entwickler-Doku; bei API-Änderungen **beide** pflegen.

## Befehle

```bash
cargo clippy --workspace --all-targets   # muss warnungsfrei sein
cargo test --workspace                   # läuft offscreen
QT_QPA_PLATFORM=offscreen cargo run -p rncl --example designer -- --screenshot /tmp/ui.png
```

## Regeln

- Qt-Widgets nur im Hauptthread; `Application` vor allen Widgets, genau eine pro Prozess.
- Neue Qt-Funktionen in C++ immer null-sicher (`handle.get()` kann null sein) und
  versionsabhängige APIs mit `#if QT_VERSION >= QT_VERSION_CHECK(..)` + Fallback.
- `WidgetKind`-Werte nie umnummerieren; Eigenschaftsnamen sind stabile API.
- Neue Komponente ⇒ `lib.rs` (mod, pub use, `register_all`, prelude), Test, Doku DE+EN.
- Neuer Integrationstest ⇒ Funktion **und** Eintrag in der `tests`-Liste in `main`.
- Code-Kommentare und Fehlermeldungen auf Deutsch.
