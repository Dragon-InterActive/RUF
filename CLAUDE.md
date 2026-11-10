# RUF – Hinweise für KI-Assistenten und Mitwirkende

*Notes for AI assistants and contributors: what RUF is, its decisions and conventions.
Details on structure and commands: `AGENTS.md`; developer docs: `docs/de`, `docs/en`.*

## Was RUF ist

**RUF** (Rust native UI Framework) ist ein UI-Framework auf Basis von GTK 4 (gtk4-rs)
für Linux, Windows und macOS. Anwendungen schreiben denselben Code für alle Systeme;
Releases für Windows und macOS liefern die GTK-Laufzeit mit (`tools/bundle-windows.sh`,
`tools/bundle-macos.sh`, `docs/de/release.md`).

Die UI-Elemente (Buttons, Textfelder, Checkboxen …) sind einzelne Komponenten der
**RNCL** (Rust native Component Library). Andere Entwickler können eigene Komponenten
nach demselben Muster schreiben und im Framework registrieren.

RUF ist die Grundlage der Editor-Oberfläche des RAD-Tools
[Freya TDE](https://github.com/Dragon-InterActive/Freya-TDE) und des
[CT-Editors](https://github.com/Dragon-InterActive/CT-E).

## Entscheidungen

- Nur GTK 4 auf allen Systemen (seit 23.10.2026): Win32 und Cocoa wurden entfernt,
  Qt wird seit 22.10.2026 nicht mehr verwendet. Das Backend bleibt hinter einem
  Rust-Trait, damit ein anderes Toolkit später möglich wäre.
- Abhängigkeiten: `gtk4` (Backend), `regex` (Syntax-Hervorhebung im Code-Editor).
  Weitere Crates nur in aktuellen, stabilen Versionen und mit freier Lizenz.
- Einbindung in Projekte, solange RUF nicht auf crates.io veröffentlicht ist:

  ```toml
  [dependencies]
  ruf  = { git = "https://github.com/Dragon-InterActive/RUF" }   # das Fundament
  rncl = { git = "https://github.com/Dragon-InterActive/RUF" }   # die Komponenten
  ```

  Zielbild nach einer Veröffentlichung: `ruf = "0.2"`, `rncl = "0.2"`.

## Architektur

1. **System-Ebene (Backend):** GTK 4 hinter dem Trait `Backend`; darüber ein
   plattformunabhängiger Kern (Signale, Lebensdauer, Semantik).
2. **Logik-Ebene (RNCL-Komponenten):** reines Rust beschreibt, was ein Fenster oder
   Button kann (`RnclButton` mit Methoden wie `set_position(x, y)`).
3. **Designer (Front-End):** ein visueller Form-Designer spricht nur die
   RNCL-Strukturen an.

## Konventionen

- Rust ≥ 1.99, Edition 2024; `cargo clippy` warnungsfrei.
- Entwickler-Dokumentation mit Beispielen auf Deutsch und Englisch, bei
  API-Änderungen beide pflegen.
- Die Nutzung durch KI-Assistenten mitdenken: Skills in `.claude/skills/`.
- Lizenz: MIT OR Apache-2.0 (nach Wahl des Nutzers).
