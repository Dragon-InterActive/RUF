---
name: ruf-backend
description: Die System-Ebene von RUF erweitern – neue native Steuerelemente (WidgetKind), neue Widget-Fähigkeiten oder Plattformfunktionen in den Backends GTK 4 (Linux), Win32 (Windows) und Cocoa (macOS). Verwenden bei Änderungen an crates/ruf/src/backend/*, crates/ruf/src/core.rs oder bei Build-/Laufzeitfehlern, die nur ein Betriebssystem betreffen.
---

# RUF-Backends erweitern

| Datei | Inhalt |
|---|---|
| `crates/ruf/src/backend/mod.rs` | Trait `Backend` – der Vertrag aller Backends |
| `crates/ruf/src/backend/gtk.rs` | Linux/BSD: `gtk4-rs`, Container `RufFixed` |
| `crates/ruf/src/backend/win32.rs` | Windows: `windows-rs`, Fensterklasse `RufContainer` |
| `crates/ruf/src/backend/cocoa.rs` | macOS: `objc2`, `RufView` (gespiegelt), `RufTarget` (Aktionen/Delegate) |
| `crates/ruf/src/core.rs` | plattformunabhängige Logik: Registry, `sync`, Semantik |
| `crates/ruf/src/widget.rs` | öffentliche API, ruft Kern bzw. `Platform::…` |

## Grundregel: Was gehört wohin?

- **Backend:** erzeugen, setzen, lesen, Benutzeraktionen melden
  (`core::notify(id, Notify::Clicked | Changed | Activated | Resized(size))`,
  `core::close_requested`, `core::timer_fired`, `core::shortcut_fired`).
- **Kern:** alles, was überall gleich sein muss – wann Signale feuern, Exklusivität,
  Begrenzungen, Auswahl-Semantik. Nie in ein einzelnes Backend verlagern.
- Während `core::setter(|| Platform::…)` ignoriert der Kern Meldungen; danach ruft er
  `sync(id)` und feuert genau die geänderten Signale.

## Rezept: neue Widget-Fähigkeit

1. Methode im Trait `Backend` ergänzen (deutscher Doc-Kommentar).
2. In **allen drei** Backends umsetzen. Fehlt es einem System nativ, sinnvoll
   nachbilden oder im Kern speichern und dokumentieren (z. B. Platzhalter in
   mehrzeiligen Feldern unter Win32/Cocoa).
3. Öffentliche Methode in `widget.rs`: Zustand ggf. im Kern-Knoten (`core::update`)
   ablegen, dann `core::setter(|| Platform::…)`; bei Zustandsänderungen `core::sync`.
4. Test in `crates/rncl/tests/components.rs` (Funktion + Eintrag in der `tests`-Liste).

## Rezept: neues Steuerelement

1. `WidgetKind` in `core.rs` um die **nächste freie Zahl** ergänzen.
2. `Signal::supported_by`, ggf. `is_container`/`has_range` anpassen.
3. In jedem Backend: Fall in `create` (Signal-Verbindungen → `core::notify`) und in
   den betroffenen Getter/Settern.
4. RNCL-Komponente → Skill `rncl-component`.

## Plattform-Hinweise

- **GTK:** Steuerelemente nie kleiner als ihre natürliche Größe. Layout ist
  asynchron (Frame-Clock); `RufFixed::size_allocate` meldet Größen per Idle.
  `glib::MainContext::invoke` **nicht** für `post` benutzen (läuft u. U. im fremden
  Thread) – `glib::idle_add_once`.
- **Win32:** Benachrichtigungen kommen beim *Elternteil* an (`container_proc`).
  Koordinaten mit `px()`/`lx()` skalieren. Stile, die nur beim Erzeugen wirken
  (Combobox editierbar, Regler senkrecht) → `recreate()`. `STATE` nie verschachtelt
  veränderbar ausleihen (z. B. kein `px()` innerhalb von `borrow_mut`).
- **Cocoa:** nur im Hauptthread (`MainThreadMarker`). Aktionen/Delegates laufen über
  das gemeinsame `RufTarget`; Absender per Zeiger in `STATE.objects`. Upcasts nach
  `NSView` mit `up(&x)` (über `retain()`), nicht mit `.into()`.

## Prüfen

```bash
cargo clippy --workspace --all-targets                                   # Linux/GTK
cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu    # Win32
cargo clippy --workspace --all-targets --target aarch64-apple-darwin     # Cocoa
env -u WAYLAND_DISPLAY GDK_BACKEND=x11 xvfb-run -a cargo test --workspace
cargo build -p rncl --tests --target x86_64-pc-windows-gnu
WINEPREFIX=/tmp/ruf-wine xvfb-run -a wine target/x86_64-pc-windows-gnu/debug/deps/components-*.exe
```

macOS lässt sich unter Linux nur prüfen, nicht ausführen – Tests dann auf einem Mac:
`cargo test --workspace`.
