//! Das kleinste RNCL-Programm: ein Fenster, ein Label, ein Button.
//!
//! cargo run -p rncl --example hello

use rncl::prelude::*;

fn main() {
    // 1. Immer zuerst die Anwendung.
    let app = Application::new();

    // 2. Ein Fenster als Container.
    let window = RnclWindow::new("Hallo RUF");
    window.set_size(320, 140);

    // 3. Komponenten im Fenster – Positionen relativ zum Fenster.
    let label = RnclLabel::new(&window, "Noch nicht geklickt");
    label.set_geometry(20, 20, 280, 24);

    let button = RnclButton::new(&window, "Klick mich");
    button.set_position(20, 70);

    // 4. Verhalten: Ein Klon ist ein leichter Verweis für die Closure.
    let l = label.clone();
    let mut clicks = 0;
    button.on_clicked(move || {
        clicks += 1;
        l.set_text(&format!("{clicks}× geklickt"));
    });

    // 5. Anzeigen und Event-Loop starten.
    window.show();
    std::process::exit(app.exec());
}
