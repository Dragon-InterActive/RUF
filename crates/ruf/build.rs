use cxx_qt_build::CxxQtBuilder;

fn main() {
    if std::env::var_os("CARGO_FEATURE_QT6").is_none() {
        // Ohne Backend gibt es nichts zu bauen; lib.rs meldet einen verständlichen Fehler.
        return;
    }

    // Die Header unter `cpp/` sind für Downstream-Crates (eigene RNCL-Komponenten mit
    // eigener Bridge) als `#include "ruf/cpp/ruf_widgets.h"` verfügbar.
    CxxQtBuilder::new()
        .qt_module("Gui")
        .qt_module("Widgets")
        .file("src/ffi.rs")
        .cpp_file("cpp/ruf_widgets.cpp")
        .build()
        .reexport_dependency("cxx-qt-lib")
        .export();

    println!("cargo::rerun-if-changed=cpp/ruf_widgets.h");
}
