//! Zähler mit gemeinsamem Zustand für mehrere Closures (aus „Erste Schritte").
//!
//! cargo run -p rncl --example counter

use rncl::prelude::*;
use std::{cell::Cell, rc::Rc};

fn main() {
    let app = Application::new();

    let window = RnclWindow::new("Zähler");
    window.set_size(260, 120);

    let label = RnclLabel::new(&window, "0");
    label.set_geometry(20, 20, 220, 24);

    let plus = RnclButton::new(&window, "+1");
    plus.set_position(20, 60);

    let reset = RnclButton::new(&window, "Zurücksetzen");
    reset.set_position(120, 60);

    // Gemeinsamer Zustand für mehrere Closures
    let count = Rc::new(Cell::new(0));

    let (c, l) = (count.clone(), label.clone());
    plus.on_clicked(move || {
        c.set(c.get() + 1);
        l.set_text(&c.get().to_string());
    });

    let (c, l) = (count.clone(), label.clone());
    reset.on_clicked(move || {
        c.set(0);
        l.set_text("0");
    });

    window.show();
    std::process::exit(app.exec());
}
