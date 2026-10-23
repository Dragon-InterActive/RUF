# AGENTS.md – Hinweise für KI-Assistenten

Projektziel und Vorgaben: siehe `CLAUDE.md`. Ausführliche Anleitungen:
`.claude/skills/{ruf-app,rncl-component,ruf-backend}/SKILL.md`.

## Aufbau

- `crates/ruf` – Fundament.
  - `src/backend/mod.rs` – Trait `Backend` (Vertrag des Backends).
  - `src/backend/gtk.rs` – GTK 4 auf Linux, Windows und macOS; einziger Ort mit
    Toolkit-Code und `unsafe`. (Win32/Cocoa am 23.10.2026 entfernt, Git-Historie.)
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
xvfb-run -a cargo run -p rncl --example designer -- --screenshot /tmp/ui.png
```

## Regeln

- Widgets nur im Hauptthread; `Application` vor allen Widgets, genau eine pro Prozess.
- Plattformlogik gehört in den Kern, nicht in ein Backend: Backends erzeugen, setzen,
  lesen und melden (`core::notify`). Während `core::setter` ignoriert der Kern Meldungen.
- Windows/macOS werden nativ auf dem jeweiligen System gebaut (MSYS2 bzw. Homebrew)
  und mit `tools/bundle-windows.sh` / `tools/bundle-macos.sh` samt GTK-Laufzeit
  ausgeliefert (`docs/de/release.md`).
- `WidgetKind`-Werte nie umnummerieren; Eigenschaftsnamen sind stabile API.
- Neue Komponente ⇒ `lib.rs` (mod, pub use, `register_all`, prelude), Test, Doku DE+EN.
- Neuer Integrationstest ⇒ Funktion **und** Eintrag in der `tests`-Liste in `main`.
- Code-Kommentare und Fehlermeldungen auf Deutsch.
