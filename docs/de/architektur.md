# Architektur

```
┌─────────────────────────────────────────────────────────────┐
│ 3. Form-Designer (Freya TD)                                 │
│    kennt nur: ComponentRegistry · Component · PropertyValue │
├─────────────────────────────────────────────────────────────┤
│ 2. Logik-Ebene – RNCL-Komponenten (pures Rust)              │
│    RnclWindow · RnclButton · … · eigene Komponenten         │
│    ruf::Component · ruf::Widget · Eigenschaftsmodell        │
├─────────────────────────────────────────────────────────────┤
│ 1. System-Ebene – die Brücke                                │
│    ruf::ffi (#[cxx_qt::bridge])  ⇄  cpp/ruf_widgets.{h,cpp} │
│    cxx · cxx-qt · cxx-qt-lib     ⇄  installiertes Qt ≥ 6    │
└─────────────────────────────────────────────────────────────┘
```

## 1. System-Ebene (`crates/ruf/cpp`, `crates/ruf/src/ffi.rs`)

- Eine **dünne C++-Schicht** über QtWidgets: erzeugen, Eigenschaften lesen/schreiben,
  Signale verbinden. Keine Anwendungslogik.
- Die Bridge wird mit `#[cxx_qt::bridge]` deklariert und von `cxx-qt-build`
  übersetzt; Qt-Typen (`QString`, `QRect`) kommen aus `cxx-qt-lib`.
- `build.rs` sucht das installierte Qt, linkt `Core`, `Gui`, `Widgets` und exportiert
  die Header, sodass Downstream-Crates eigene Bridges gegen `ruf` schreiben können.

### Signale → Rust-Closures

```
QPushButton::clicked ─► C++-Lambda ─► invoke_callback(&RufCallback) ─► Box<dyn FnMut()>
```

- Die Closure lebt in einem `RefCell`: Löst sie (indirekt) dasselbe Signal erneut aus,
  wird der verschachtelte Aufruf verworfen statt undefiniertes Verhalten zu erzeugen.
- **Panics** in Closures werden abgefangen und auf `stderr` gemeldet – sie laufen nie über
  die C++-Grenze, der Event-Loop läuft weiter.
- Verbindungen hängen am Widget: wird es zerstört, wird auch die Closure freigegeben.

## 2. Logik-Ebene (`crates/ruf/src`, `crates/rncl`)

### `Widget` – der sichere Verweis

`ruf::Widget` hält intern einen `QPointer`. Daraus folgt das **Besitzmodell**:

| Situation | Verhalten |
|---|---|
| Widget ohne Elternteil (Fenster) | wird beim Droppen des erzeugenden `Widget` gelöscht |
| Widget mit Elternteil | gehört dem Elternteil (Qt-Modell); Droppen in Rust entfernt es nicht |
| `widget.clone()` | nicht besitzender Verweis – ideal für Closures |
| Qt hat das Widget gelöscht | alle Aufrufe sind No-Ops, Getter liefern Standardwerte, `is_alive()` = `false` |

Es gibt also **keine hängenden Zeiger** auf Rust-Seite.

### `Component` – der Vertrag

Jede RNCL-Komponente implementiert `ruf::Component`. Pflicht sind nur `widget()` und
`type_name()`; Position, Größe, Sichtbarkeit und die Basis-Eigenschaften kommen als
Default-Methoden mit. Eigene Eigenschaften werden über `properties()`, `property()`
und `set_property()` ergänzt.

### Threads

`Widget` und alle Komponenten sind **nicht `Send`/`Sync`** – der Compiler verhindert,
dass Widgets aus anderen Threads benutzt werden (Qt-Regel).

## 3. Designer-Ebene

Freya TD erzeugt Komponenten nur über die `ComponentRegistry` (per Typname) und
bearbeitet sie über das Eigenschaftsmodell. Siehe [Designer-Anbindung](designer.md).

## Qt-Versionen

RUF bindet das jeweils installierte Qt – 6.x heute, Qt 7 später als eigenes Feature.
Neuere Funktionen werden in C++ per `QT_VERSION_CHECK` freigeschaltet und haben immer
einen Fallback. Beispiel Farbschema:

| Funktion | benötigt | ohne |
|---|---|---|
| `Application::color_scheme()` | Qt 6.5 | `ColorScheme::Unknown` |
| `Application::set_color_scheme()` | Qt 6.8 | liefert `false` |

Zur Laufzeit prüfen:

```rust
use ruf::QtVersion;

let qt = QtVersion::runtime();
if qt.at_least(6, 8) { /* neue Funktion nutzen */ }
println!("gebaut gegen {}, läuft mit {qt}", QtVersion::compile_time());
```

## Features

| Feature | Standard | Bedeutung |
|---|---|---|
| `qt6` | ja | Qt-≥-6-Backend (QtWidgets). Ohne Backend bricht der Build mit klarer Meldung ab. |
