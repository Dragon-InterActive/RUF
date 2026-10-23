---
name: ruf-backend
description: Die System-Ebene von RUF erweitern – neue Steuerelemente (WidgetKind), neue Widget-Fähigkeiten oder Toolkit-Funktionen im GTK-4-Backend (läuft auf Linux, Windows und macOS). Verwenden bei Änderungen an crates/ruf/src/backend/*, crates/ruf/src/core.rs oder bei Build-/Laufzeitfehlern, die nur ein Betriebssystem betreffen (z. B. fehlende GTK-DLLs unter Windows).
---

# RUF-Backend erweitern

RUF hat ein Backend: GTK 4 (`gtk4-rs`) auf allen Systemen. Win32 und Cocoa wurden am
23.10.2026 entfernt (Git-Historie). Der Trait `Backend` bleibt als Schicht bestehen.

| Datei | Inhalt |
|---|---|
| `crates/ruf/src/backend/mod.rs` | Trait `Backend` – der Vertrag des Backends |
| `crates/ruf/src/backend/gtk.rs` | GTK 4, Container `RufFixed` |
| `crates/ruf/src/core.rs` | plattformunabhängige Logik: Registry, `sync`, Semantik |
| `crates/ruf/src/widget.rs` | öffentliche API, ruft Kern bzw. `Platform::…` |

## Grundregel: Was gehört wohin?

- **Backend:** erzeugen, setzen, lesen, Benutzeraktionen melden
  (`core::notify(id, Notify::Clicked | Changed | Activated | Resized(size))`,
  `core::close_requested`, `core::timer_fired`, `core::shortcut_fired`).
- **Kern:** Semantik – wann Signale feuern, Exklusivität, Begrenzungen,
  Auswahl-Semantik. Nicht ins Backend verlagern.
- Während `core::setter(|| Platform::…)` ignoriert der Kern Meldungen; danach ruft er
  `sync(id)` und feuert genau die geänderten Signale.

## Rezept: neue Widget-Fähigkeit

1. Methode im Trait `Backend` ergänzen (deutscher Doc-Kommentar).
2. Im GTK-Backend umsetzen.
3. Öffentliche Methode in `widget.rs`: Zustand ggf. im Kern-Knoten (`core::update`)
   ablegen, dann `core::setter(|| Platform::…)`; bei Zustandsänderungen `core::sync`.
4. Test in `crates/rncl/tests/components.rs` (Funktion + Eintrag in der `tests`-Liste).

## Rezept: neues Steuerelement

1. `WidgetKind` in `core.rs` um die **nächste freie Zahl** ergänzen.
2. `Signal::supported_by`, ggf. `is_container`/`has_range` anpassen.
3. Im Backend: Fall in `create` (Signal-Verbindungen → `core::notify`) und in den
   betroffenen Gettern/Settern.
4. RNCL-Komponente → Skill `rncl-component`.

## GTK-Hinweise

- Steuerelemente nie kleiner als ihre natürliche Größe. Layout ist asynchron
  (Frame-Clock); `RufFixed::size_allocate` meldet Größen per Idle.
- `glib::MainContext::invoke` **nicht** für `post` benutzen (läuft u. U. im fremden
  Thread) – `glib::idle_add_once`.
- Tastenkürzel: `<Primary>` statt `<Control>` – unter macOS ist das die Befehlstaste.
- Keine Linux-only-APIs (X11/Wayland-spezifisch); der Code läuft auch unter Windows
  und macOS. Plattformunterschiede höchstens per `cfg!(target_os = …)`.

## Prüfen

```bash
cargo clippy --workspace --all-targets
env -u WAYLAND_DISPLAY GDK_BACKEND=x11 xvfb-run -a cargo test --workspace
```

In einer Sandbox, die `/tmp/.X11-unix` abschottet, startet Xvfb nicht – Tests dann
außerhalb der Sandbox. Windows/macOS: nativ bauen (MSYS2 UCRT64 bzw. Homebrew) und
`cargo test --workspace`; Ausliefern mit `tools/bundle-*.sh` (`docs/de/release.md`).
