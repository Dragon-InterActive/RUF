---
name: ruf-bridge
description: Die C++/Qt-Brücke von RUF erweitern – neue Qt-Widgets (WidgetKind), neue Widget-Funktionen, Signale oder versionsabhängige Qt-Features (Qt 6.x/7) anbinden. Verwenden bei Änderungen an crates/ruf/cpp/*, crates/ruf/src/ffi.rs oder bei Build-/Link-Fehlern rund um cxx/cxx-qt.
---

# RUF-Brücke erweitern

Die System-Ebene besteht aus vier Dateien, die **immer gemeinsam** geändert werden:

| Datei | Inhalt |
|---|---|
| `crates/ruf/cpp/ruf_widgets.h` | C++-Deklaration (`namespace ruf`) |
| `crates/ruf/cpp/ruf_widgets.cpp` | Implementierung gegen QtWidgets |
| `crates/ruf/src/ffi.rs` | `#[cxx_qt::bridge]`-Deklaration |
| `crates/ruf/src/widget.rs` (oder `app.rs`) | sichere Rust-Methode darüber |

## Rezept: neue Widget-Funktion

1. **Header**: `ReturnType widget_xxx(WidgetHandle const& handle, …);`
2. **cpp**: immer null-sicher – Widget kann von Qt gelöscht worden sein:
   ```cpp
   void widget_set_xxx(WidgetHandle const& handle, std::int32_t v)
   {
       if (auto* s = qobject_cast<QSlider*>(handle.get())) {
           s->setValue(v);
       }
   }
   ```
   Getter liefern bei null/falschem Typ einen Standardwert.
3. **ffi.rs** im `unsafe extern "C++"`-Block:
   `fn widget_set_xxx(handle: &WidgetHandle, v: i32);`
   Qt-Typen aus cxx-qt-lib brauchen `#[namespace = ""]` (wie `QString`, `QRect`).
4. **widget.rs**: `pub fn set_xxx(&self, v: i32) { ffi::widget_set_xxx(&self.handle, v) }`
   mit deutschem Doc-Kommentar. Strings: `&str` ↔ `QString::from(..)` / `.to_string()`.

## Rezept: neues Qt-Widget

1. `WidgetKind` in `ffi.rs` um eine Variante mit **nächster freier Zahl** erweitern
   (Werte nie umnummerieren – sie werden als Qt-Property gespeichert).
2. In `ruf_widgets.cpp`: `#include <QtWidgets/QXxx>`, Fall in `create()` und
   Erkennung in `widget_kind()` ergänzen.
3. Text/Checked/Signale: bestehende Dispatch-Funktionen (`widget_set_text`, …)
   per `qobject_cast` erweitern, statt neue Varianten zu erfinden.
4. Danach Komponente bauen → Skill `rncl-component`.

## Rezept: neues Signal

Muster `widget_connect_clicked`: Rust-Callback-Typ (`RufCallback`, `RufBoolCallback`,
`RufStringCallback` – bei Bedarf `RufIntCallback` analog anlegen: Typ + `invoke_*`
im `extern "Rust"`-Block + `guarded`), in C++ per `share(std::move(cb))` kopierbar
machen und mit dem Widget als Kontextobjekt verbinden:

```cpp
QObject::connect(s, &QSlider::valueChanged, s, [cb](int v) { invoke_int_callback(**cb, v); });
```

Panics werden in `guarded` abgefangen, Re-Entranz per `RefCell::try_borrow_mut`.

## Neuere Qt-Versionen

RUF soll Features jenseits von Qt 6.0 nutzen. Immer mit Fallback kapseln:

```cpp
#if QT_VERSION >= QT_VERSION_CHECK(6, 8, 0)
    /* neue API */  return true;
#else
    return false;   // Rust-Seite meldet "nicht unterstützt"
#endif
```

Rust-seitig zur Laufzeit: `ruf::QtVersion::runtime().at_least(6, 8)`.
Vorbild: `application_set_color_scheme`.

## Fallstricke

- **Linkerfehler `undefined symbol: ruf$cxxbridge1$NNN$…`**: `cxx` und `cxx-gen`
  haben verschiedene Versionen im `Cargo.lock`. Lösen mit
  `cargo update -p cxx-gen --precise 0.7.<patch von cxx>`.
- **„cannot parse string into token stream"** beim Build: Syntaxfehler in `ffi.rs`
  (cxx-qt parst die Datei selbst, bevor rustc sie sieht).
- Rohzeiger nur über `unsafe fn` (`widget_wrap`); öffentliche API bleibt sicher.
- Qt-Module: neue Module (z. B. `Svg`) in `crates/ruf/build.rs` per `.qt_module("Svg")`.

## Prüfen

```bash
cargo clippy --workspace --all-targets && cargo test --workspace
```
