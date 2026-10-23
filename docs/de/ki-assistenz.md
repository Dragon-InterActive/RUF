# Arbeiten mit KI-Assistenten

RUF ist von Anfang an darauf ausgelegt, dass KI-Assistenten (z. B. Claude Code)
zuverlässig damit arbeiten können.

## Skills

Unter `.claude/skills/` liegen Anleitungen, die Claude Code automatisch lädt, sobald
eine passende Aufgabe ansteht:

| Skill | Wann |
|---|---|
| `ruf-app` | Anwendung/Formular mit RUF und RNCL bauen, Sichtprüfung per Screenshot |
| `rncl-component` | neue RNCL-Komponente schreiben oder erweitern (Checkliste + Gerüst) |
| `ruf-backend` | System-Ebene erweitern: neue Steuerelemente oder Fähigkeiten im GTK-Backend |

Andere KI-Werkzeuge finden dieselben Regeln kompakt in [`AGENTS.md`](../../AGENTS.md).

### Beispiel-Prompts

- „Baue mit RNCL ein Fenster zur Kundenerfassung mit Name, E-Mail und Speichern-Button."
- „Erstelle eine RNCL-Komponente `RnclDatePicker` mit Eigenschaft `date` für alle drei Backends."
- „Ergänze im GTK-Backend eine Fortschrittsanzeige mit Text."

## Warum das für KIs gut funktioniert

- **Selbstbeschreibende Komponenten**: `properties()` liefert Name, Typ und
  Beschreibung jeder Eigenschaft – eine KI kann Formulare über das Eigenschaftsmodell
  bauen, ohne den Quelltext jeder Komponente zu kennen.
- **Sichtprüfung ohne Bildschirm**: `xvfb-run` (Linux) +
  `Widget::save_screenshot()` erlauben es einem Agenten, das Ergebnis als Bild zu
  prüfen (Überlappungen, abgeschnittene Texte).
- **Robuste API**: keine hängenden Zeiger, abgefangene Panics in Callbacks, klare
  Fehlermeldungen (`Application` fehlt, falscher Eigenschaftstyp …).
- **Schnelles Feedback**: `cargo test --workspace` testet gegen das echte Toolkit in
  Sekunden – dasselbe Toolkit auf allen Systemen.
- **Ein Kern für alle Systeme**: Signale und Semantik liegen einmalig in `core.rs`;
  ein Backend-Fehler zeigt sich in denselben Tests auf jeder Plattform.

## Eigene Skills ergänzen

Neue Skills als `.claude/skills/<name>/SKILL.md` mit YAML-Kopf (`name`, `description`)
anlegen. Die `description` entscheidet, wann der Skill geladen wird – also konkret
beschreiben, *wann* er gebraucht wird.
