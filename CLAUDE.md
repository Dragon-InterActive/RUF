# Projekt Bezeichnung und Funktion

`RUF` ist die kurzform für Rust native UI Framework. Es soll ein Framework auf der Basis von Qt >= 6 sein.

Die einzelenen UI-Elemente wie Buttons, Textfelder oder Checkboxen etc. sind einzelene Componenten und heißen `RNCL` (Rust native Component Library).
Diese Festlegung soll es anderen Entwicklern ermöglichen, eigene UI-Elemente zu entwickeln und an das Framework zuübergeben und zu nutzen.

Ruf wird die Bais des RAD-Tools (Rapid Application Development) "Freya TD" werden.

Wichtig ist die Nutzung von jüngeren Entwicklungen der Qt-Library über die Version 6 hinaus.

Die spätere Verwendung in Rust-Projekten bzw. in den toml-Dateien sollte diesem Muster folgen:

```toml
[dependencies]
ruf = { version = "0.1.0", features = ["qt6"] } # Das native Fundament
rncl = { version = "0.1.0" }                  # Die nativen System-Widgets

```

## Crates

- `cxx` Die stabilisierte Schnittstelle für bidirektionale C++ ⇄ Rust Kommunikation
- `cxx-qt` Bietet die QObject-Subklassen direkt in Rust
- `cxx-qt-lib` Bringt Standardtypen wie QString, QPoint, QColor mit

Weitere crates, wenn sie sinnvoll erscheinen, können selbstverständlich mitgenutzt und installiert werden. Die Entscheidung hierfür liegt bei dir.

## Architektur

1. Die System-Ebene (Die Brücke): cxx-qt bindet das jeweils auf dem System installierte Qt (6.12, 6.15 oder später Qt 7)
2. Logik-Ebene (Die RNCL-Komponenten): Der Entwickler definiert in sauberen, puren Rust, was ein Fenster oder ein Button tun kann (z.B. ein `struct RnclButton` mit Methoden wie `.set_psoition(x, y)`)
3. Der visuelle Form-Designer (Das Front-End): Die geplante Oberfläche spricht am Ende nur die RNCL-Rust-Strukturen an.

## Entwickler und AI

- Denke immer die Möglichkeit mit, eine AI zu nutzen. Erstelle dafür Skills.
- Erstelle auch eine Entwickler-Dokumentation mit Beispielen in deutsch und in englisch.
