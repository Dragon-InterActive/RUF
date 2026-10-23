# Projekt Bezeichnung und Funktion

`RUF` ist die kurzform für Rust native UI Framework. Es ist ein Framework auf Basis von GTK 4 (gtk4-rs) – auf Linux, Windows und macOS. Releases für Windows und macOS liefern die GTK-Laufzeit mit (`tools/bundle-windows.sh`, `tools/bundle-macos.sh`, `docs/de/release.md`).

Die einzelenen UI-Elemente wie Buttons, Textfelder oder Checkboxen etc. sind einzelene Componenten und heißen `RNCL` (Rust native Component Library).
Diese Festlegung soll es anderen Entwicklern ermöglichen, eigene UI-Elemente zu entwickeln und an das Framework zuübergeben und zu nutzen.

Ruf wird die Bais des RAD-Tools (Rapid Application Development) "Freya TD" werden.

Qt wird nicht mehr verwendet (Entscheidung vom 22.10.2026). Win32 und Cocoa wurden entfernt, alles läuft über GTK 4 (Entscheidung vom 23.10.2026, um doppelte Arbeit zu sparen). Anwendungen schreiben denselben Code für alle Systeme.

Die spätere Verwendung in Rust-Projekten bzw. in den toml-Dateien sollte diesem Muster folgen:

```toml
[dependencies]
ruf = { version = "0.2.0" }  # Das native Fundament (Backend automatisch)
rncl = { version = "0.2.0" } # Die nativen System-Widgets

```

## Crates

- `gtk4` (gtk4-rs) – Backend für Linux, Windows und macOS
- `regex` – Syntax-Hervorhebung im Code-Editor

Weitere crates, wenn sie sinnvoll erscheinen, können selbstverständlich mitgenutzt und installiert werden. Die Entscheidung hierfür liegt bei dir.

## Architektur

1. Die System-Ebene (Das Backend): GTK 4 hinter einem Rust-Trait (bleibt, damit ein anderes Toolkit später möglich wäre); darüber ein plattformunabhängiger Kern (Signale, Lebensdauer, Semantik)
2. Logik-Ebene (Die RNCL-Komponenten): Der Entwickler definiert in sauberen, puren Rust, was ein Fenster oder ein Button tun kann (z.B. ein `struct RnclButton` mit Methoden wie `.set_psoition(x, y)`)
3. Der visuelle Form-Designer (Das Front-End): Die geplante Oberfläche spricht am Ende nur die RNCL-Rust-Strukturen an.

## Entwickler und AI

- Denke immer die Möglichkeit mit, eine AI zu nutzen. Erstelle dafür Skills.
- Erstelle auch eine Entwickler-Dokumentation mit Beispielen in deutsch und in englisch.
