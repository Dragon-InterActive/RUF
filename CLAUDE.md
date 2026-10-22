# Projekt Bezeichnung und Funktion

`RUF` ist die kurzform für Rust native UI Framework. Es ist ein Framework auf Basis der nativen Steuerelemente des jeweiligen Betriebssystems: Win32 via windows-rs (Windows), GTK4 (Linux) und Cocoa via objc2 (macOS).

Die einzelenen UI-Elemente wie Buttons, Textfelder oder Checkboxen etc. sind einzelene Componenten und heißen `RNCL` (Rust native Component Library).
Diese Festlegung soll es anderen Entwicklern ermöglichen, eigene UI-Elemente zu entwickeln und an das Framework zuübergeben und zu nutzen.

Ruf wird die Bais des RAD-Tools (Rapid Application Development) "Freya TD" werden.

Qt wird nicht mehr verwendet (Entscheidung vom 22.10.2026). Das passende Backend wählt der Compiler automatisch; Anwendungen schreiben denselben Code für alle Systeme.

Die spätere Verwendung in Rust-Projekten bzw. in den toml-Dateien sollte diesem Muster folgen:

```toml
[dependencies]
ruf = { version = "0.2.0" }  # Das native Fundament (Backend automatisch)
rncl = { version = "0.2.0" } # Die nativen System-Widgets

```

## Crates

- `gtk4` (gtk4-rs) – Backend für Linux
- `windows` (windows-rs) – Backend für Windows (Win32 + Common Controls)
- `objc2`, `objc2-app-kit`, `objc2-foundation`, `block2`, `dispatch2` – Backend für macOS (Cocoa)
- `regex` – Syntax-Hervorhebung im Code-Editor; `png` – Screenshots unter Windows

Weitere crates, wenn sie sinnvoll erscheinen, können selbstverständlich mitgenutzt und installiert werden. Die Entscheidung hierfür liegt bei dir.

## Architektur

1. Die System-Ebene (Die Backends): je ein Backend für Win32, GTK4 und Cocoa hinter einem gemeinsamen Rust-Trait; darüber ein plattformunabhängiger Kern (Signale, Lebensdauer, Semantik)
2. Logik-Ebene (Die RNCL-Komponenten): Der Entwickler definiert in sauberen, puren Rust, was ein Fenster oder ein Button tun kann (z.B. ein `struct RnclButton` mit Methoden wie `.set_psoition(x, y)`)
3. Der visuelle Form-Designer (Das Front-End): Die geplante Oberfläche spricht am Ende nur die RNCL-Rust-Strukturen an.

## Entwickler und AI

- Denke immer die Möglichkeit mit, eine AI zu nutzen. Erstelle dafür Skills.
- Erstelle auch eine Entwickler-Dokumentation mit Beispielen in deutsch und in englisch.
