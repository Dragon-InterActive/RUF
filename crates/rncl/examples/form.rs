//! Ein Anmeldeformular mit allen Standardkomponenten, Validierung und
//! Farbschema-Umschaltung (nutzt Qt >= 6.8, falls vorhanden).
//!
//! cargo run -p rncl --example form

use rncl::prelude::*;
use ruf::{ColorScheme, QtVersion};

fn main() {
    let app = Application::new();
    app.set_name("RNCL Formular");

    let window = RnclWindow::new("Anmelden");
    window.set_size(360, 250);

    let user_label = RnclLabel::new(&window, "Benutzer:");
    user_label.set_position(20, 24);
    let user = RnclTextField::new(&window);
    user.set_geometry(110, 20, 220, 28);
    user.set_placeholder("max.mustermann");
    user.set_name("txtUser");

    let pass_label = RnclLabel::new(&window, "Passwort:");
    pass_label.set_position(20, 64);
    let pass = RnclTextField::new(&window);
    pass.set_geometry(110, 60, 220, 28);
    pass.set_placeholder("mindestens 4 Zeichen");
    pass.set_password_mode(true);

    let remember = RnclCheckBox::new(&window, "Angemeldet bleiben");
    remember.set_position(110, 100);

    let dark = RnclCheckBox::new(&window, "Dunkles Design");
    dark.set_position(110, 128);
    let qt = QtVersion::runtime();
    if qt.at_least(6, 8) {
        dark.set_checked(Application::color_scheme() == ColorScheme::Dark);
        dark.on_toggled(|on| {
            Application::set_color_scheme(if on {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            });
        });
    } else {
        dark.set_enabled(false);
        dark.set_tool_tip(&format!("Benötigt Qt >= 6.8 (gefunden: {qt})"));
    }

    let login = RnclButton::new(&window, "Anmelden");
    login.set_geometry(110, 170, 100, 32);
    login.set_enabled(false);

    let status = RnclLabel::new(&window, "");
    status.set_geometry(20, 212, 320, 24);

    // Button nur aktiv, wenn beide Felder sinnvoll befüllt sind.
    let validate = {
        let (user, pass, login) = (user.clone(), pass.clone(), login.clone());
        move || login.set_enabled(!user.text().is_empty() && pass.text().len() >= 4)
    };
    let v = validate.clone();
    user.on_text_changed(move |_| v());
    pass.on_text_changed(move |_| validate());

    {
        let (user, remember, status) = (user.clone(), remember.clone(), status.clone());
        login.on_clicked(move || {
            let extra = if remember.is_checked() {
                " (gemerkt)"
            } else {
                ""
            };
            status.set_text(&format!("Willkommen, <b>{}</b>{extra}", user.text()));
        });
    }
    let l = login.clone();
    pass.on_return_pressed(move || l.click());

    window.show();
    std::process::exit(app.exec());
}
