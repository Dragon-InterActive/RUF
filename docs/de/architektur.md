# Architektur

RUF ist in vier Ebenen aufgebaut. Nur die unterste kennt das Toolkit (GTK 4).

```
 ┌──────────────────────────────────────────────────────────────────────┐
 │ 4  Designer / Freya TD     ComponentRegistry · Eigenschaftsmodell     │
 ├──────────────────────────────────────────────────────────────────────┤
 │ 3  Logik (rncl + ruf)      RnclButton … · Component · Widget          │
 ├──────────────────────────────────────────────────────────────────────┤
 │ 2  Kern (ruf/src/core.rs)  Registry · Signale · Geometrie · Timer     │
 ├──────────────────────────────────────────────────────────────────────┤
 │ 1  System-Ebene            trait Backend                              │
 │    (ruf/src/backend/)      gtk.rs                                     │
 └──────────────────────────────────────────────────────────────────────┘
               GTK 4 – Linux, Windows, macOS
```

## 1. System-Ebene (`crates/ruf/src/backend/`)

Ein Backend für alle Betriebssysteme: GTK 4. Unter Windows und macOS liefert das
Release die GTK-Laufzeit mit (siehe [Release](release.md)).

| Datei | System | Bindung | Container-Klasse |
|---|---|---|---|
| `gtk.rs` | Linux, BSD, Windows, macOS | `gtk4-rs` | `RufFixed` (Unterklasse von `GtkWidget`) |

Es implementiert den Trait `Backend` (`backend/mod.rs`). Die Schicht bleibt, obwohl es
nur ein Backend gibt: ein anderes Toolkit ließe sich später einsetzen, ohne Kern und
Komponenten anzufassen. Win32 und Cocoa wurden am 23.10.2026 entfernt (Git-Historie). Backends sind bewusst
**„dumm“**: Sie erzeugen Steuerelemente, setzen und lesen deren Zustand und melden
Benutzeraktionen an den Kern (`core::notify`, `core::close_requested`,
`core::timer_fired`, `core::shortcut_fired`).

Jedes Backend hat einen eigenen Container, der Kinder absolut positioniert und seine
Größe meldet – damit funktionieren `on_resized` und eigene Layouts in Fenstern,
Registerkarten-Seiten und Scrollbereichen auf allen Systemen gleich.

## 2. Kern (`crates/ruf/src/core.rs`)

Alles, was auf allen Systemen gleich funktionieren muss, gibt es nur einmal:

- **Registry:** Jedes Widget hat eine Nummer (`Id`) und einen Knoten mit Art,
  nativem Verweis, Eltern/Kindern, zwischengespeicherten Eigenschaften und Callbacks.
- **Lebensdauer:** Zerstörte Widgets verschwinden aus der Registry. Verweise darauf
  werden „tot“: Aufrufe sind No-Ops, Getter liefern Standardwerte – keine hängenden
  Zeiger, keine Abstürze.
- **Signale:** Nach jeder Änderung – per Code oder durch den Anwender – vergleicht
  `sync()` den nativen Zustand mit dem zuletzt gemeldeten und ruft genau dann die
  Callbacks auf, wenn sich etwas geändert hat. Signale feuern deshalb überall gleich,
  auch bei Änderungen per Code, und nie doppelt.
- **Gemeinsame Semantik:** Optionsfelder je Elternteil exklusiv; Combobox wählt den
  ersten Eintrag; Auswahl folgt beim Einfügen/Entfernen; Wertebereiche begrenzen den
  Wert; Fenster werden beim Schließen nur ausgeblendet; nach dem letzten sichtbaren
  Fenster endet `exec()`.
- **Timer, Tastenkürzel, verzögertes Löschen, Panik-Schutz** (eine Panik in einem
  Callback wird abgefangen, die übrigen Handler laufen weiter).

Regel zwischen Kern und Backend: Während der Kern ein Backend aufruft (`setter`),
ignoriert er dessen Zustandsmeldungen und gleicht danach selbst ab.

### Signale → Rust-Closures

```
Benutzer klickt ─► natives Signal ─► Backend: core::notify(id, Clicked)
                                         └─► Kern: sync(id) ─► Toggled? ─► Callbacks
                                                         └──► fire_clicked ─► Callbacks
Code: check.set_checked(true) ─► Kern ─► Backend (Meldungen ignoriert) ─► sync(id) ─► Callbacks
```

## 3. Logik-Ebene (`crates/ruf/src`, `crates/rncl`)

### `Widget` – der sichere Verweis

`Widget` ist nur eine Nummer plus Besitz-Kennzeichen:

- Ein Widget **ohne** Elternteil (Fenster) wird freigegeben, wenn sein erzeugender
  `Widget`-Wert gedroppt wird.
- Ein Widget **mit** Elternteil gehört dem Elternteil.
- `clone()` erzeugt einen nicht besitzenden Verweis – ideal für Closures.

### `Component` – der Vertrag

Jede RNCL-Komponente implementiert `ruf::Component`. Pflicht sind nur `widget()` und
`type_name()`; Position, Größe, Sichtbarkeit und die Basis-Eigenschaften kommen als
Default-Methoden mit. Eigene Eigenschaften werden über `properties()`, `property()`
und `set_property()` ergänzt.

### Threads

`Widget` und alle Komponenten sind **nicht `Send`/`Sync`** – der Compiler verhindert,
dass Widgets aus anderen Threads benutzt werden. Für Hintergrundarbeit gibt es
`ruf::GuiSender`: Der Sender ist `Send` und reicht Werte an einen Handler im
GUI-Thread weiter (GTK-Idle-Quelle). Siehe [Komponenten → Dienste](komponenten.md#dienste-für-dynamische-oberflächen).

## 4. Designer-Ebene

Freya TD erzeugt Komponenten nur über die `ComponentRegistry` (per Typname) und
bearbeitet sie über das Eigenschaftsmodell. Siehe [Designer-Anbindung](designer.md).

## Toolkit-Version zur Laufzeit

```rust
let toolkit = ruf::toolkit();      // z. B. "GTK 4.24.1"
println!("{} {}", toolkit.name, toolkit.version);
```

## Mindestversionen

| System | Minimum | Grund |
|---|---|---|
| GTK | 4.12 | `CssProvider::load_from_string`, `AlertDialog` |
| Windows | 10 | GTK 4 aus MSYS2 |
| macOS | 11 | GTK 4 aus Homebrew |
