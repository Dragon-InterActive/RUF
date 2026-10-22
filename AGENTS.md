# AGENTS.md – Hinweise für KI-Assistenten

Projektziel und Vorgaben: siehe `CLAUDE.md`. Ausführliche Anleitungen:
`.claude/skills/{ruf-app,rncl-component,ruf-backend}/SKILL.md`.

## Aufbau

- `crates/ruf` – Fundament.
  - `src/backend/mod.rs` – Trait `Backend` (Vertrag aller Backends).
  - `src/backend/gtk.rs` (Linux), `win32.rs` (Windows), `cocoa.rs` (macOS) – einzige
    Orte mit Toolkit-Code und `unsafe`.
  - `src/core.rs` – plattformunabhängiger Kern: Registry, Signale (`sync`), Semantik,
    Timer, Tastenkürzel, verzögertes Löschen.
  - `src/widget.rs` – sicherer `Widget`-Verweis (Nummer im Kern).
  - `src/component.rs` – Trait `Component` + `base`-Eigenschaften.
  - `src/registry.rs`, `src/property.rs` – Designer-Schnittstelle.
  - `src/style.rs` (CSS-Teilmenge, Farben), `src/text.rs` (Tastenkürzel, Markup,
    Syntax-Hervorhebung).
- `crates/rncl` – Komponenten (`RnclButton` in `src/button.rs` ist die Referenz),
  Beispiele in `examples/`, Integrationstests in `tests/components.rs`.
- `docs/de`, `docs/en` – Entwickler-Doku; bei API-Änderungen **beide** pflegen.

## Befehle

```bash
cargo clippy --workspace --all-targets                       # muss warnungsfrei sein
xvfb-run -a cargo test --workspace                           # Linux/GTK (Wayland: GDK_BACKEND=x11)
cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu   # Win32
cargo clippy --workspace --all-targets --target aarch64-apple-darwin    # Cocoa (nur prüfen)
cargo build -p rncl --tests --target x86_64-pc-windows-gnu
WINEPREFIX=/tmp/ruf-wine xvfb-run -a wine target/x86_64-pc-windows-gnu/debug/deps/components-*.exe
xvfb-run -a cargo run -p rncl --example designer -- --screenshot /tmp/ui.png
```

## Regeln

- Widgets nur im Hauptthread; `Application` vor allen Widgets, genau eine pro Prozess.
- Plattformlogik gehört in den Kern, nicht in ein Backend: Backends erzeugen, setzen,
  lesen und melden (`core::notify`). Während `core::setter` ignoriert der Kern Meldungen.
- Änderung am Trait `Backend` ⇒ **alle drei** Backends umsetzen und für alle Ziele
  `cargo clippy` laufen lassen.
- `WidgetKind`-Werte nie umnummerieren; Eigenschaftsnamen sind stabile API.
- Neue Komponente ⇒ `lib.rs` (mod, pub use, `register_all`, prelude), Test, Doku DE+EN.
- Neuer Integrationstest ⇒ Funktion **und** Eintrag in der `tests`-Liste in `main`.
- Code-Kommentare und Fehlermeldungen auf Deutsch.
