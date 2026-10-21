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
| `ruf-bridge` | C++/Qt-Brücke erweitern: neue Widgets, Signale, Qt-Versionsweichen |

Andere KI-Werkzeuge finden dieselben Regeln kompakt in [`AGENTS.md`](../../AGENTS.md).

### Beispiel-Prompts

- „Baue mit RNCL ein Fenster zur Kundenerfassung mit Name, E-Mail und Speichern-Button."
- „Erstelle eine RNCL-Komponente `RnclSlider` auf Basis von `QSlider` mit Eigenschaft `value`."
- „Nutze in RUF das neue Qt-6.9-Feature X, mit Fallback für ältere Versionen."

## Warum das für KIs gut funktioniert

- **Selbstbeschreibende Komponenten**: `properties()` liefert Name, Typ und
  Beschreibung jeder Eigenschaft – eine KI kann Formulare über das Eigenschaftsmodell
  bauen, ohne den Quelltext jeder Komponente zu kennen.
- **Sichtprüfung ohne Bildschirm**: `QT_QPA_PLATFORM=offscreen` +
  `Widget::save_screenshot()` erlauben es einem Agenten, das Ergebnis als Bild zu
  prüfen (Überlappungen, abgeschnittene Texte).
- **Robuste API**: keine hängenden Zeiger, abgefangene Panics in Callbacks, klare
  Fehlermeldungen (`Application` fehlt, falscher Eigenschaftstyp …).
- **Schnelles Feedback**: `cargo test --workspace` testet gegen echtes Qt in Sekunden.

## Eigene Skills ergänzen

Neue Skills als `.claude/skills/<name>/SKILL.md` mit YAML-Kopf (`name`, `description`)
anlegen. Die `description` entscheidet, wann der Skill geladen wird – also konkret
beschreiben, *wann* er gebraucht wird.
