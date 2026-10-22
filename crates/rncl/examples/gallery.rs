//! Alle RNCL-Standardkomponenten in einem Fenster – miteinander verdrahtet.
//!
//! cargo run -p rncl --example gallery
//! cargo run -p rncl --example gallery -- --screenshot gallery.png

use rncl::prelude::*;

fn main() {
    let app = Application::new();

    let window = RnclWindow::new("RNCL Galerie");
    window.set_size(560, 430);

    let tabs = RnclTabView::new(&window);
    tabs.set_geometry(10, 10, 540, 370);

    // --- Seite 1: Eingabe -------------------------------------------------
    let input = tabs.add_tab("Eingabe");

    RnclLabel::new(&input, "Name:").set_position(10, 16);
    let name = RnclTextField::new(&input);
    name.set_geometry(110, 12, 200, 26);
    name.set_placeholder("Vor- und Nachname");

    RnclLabel::new(&input, "Stadt:").set_position(10, 52);
    let city = RnclComboBox::new(&input);
    city.set_geometry(110, 48, 200, 26);
    city.set_items(&["Berlin", "Hamburg", "München", "Köln"]);

    RnclLabel::new(&input, "Alter:").set_position(10, 88);
    let age = RnclSpinBox::new(&input);
    age.set_position(110, 84);
    age.set_range(0, 120);
    age.set_value(30);

    let contact = RnclGroupBox::new(&input, "Kontakt per");
    contact.set_geometry(330, 8, 190, 110);
    let mail = RnclRadioButton::new(&contact, "E-Mail");
    mail.set_position(12, 26);
    mail.set_checked(true);
    let phone = RnclRadioButton::new(&contact, "Telefon");
    phone.set_position(12, 52);
    let post = RnclRadioButton::new(&contact, "Post");
    post.set_position(12, 78);

    let news = RnclCheckBox::new(&input, "Newsletter abonnieren");
    news.set_position(110, 124);

    RnclLabel::new(&input, "Notizen:").set_position(10, 160);
    let notes = RnclTextArea::new(&input);
    notes.set_geometry(110, 156, 410, 130);
    notes.set_placeholder("Freitext …");

    let save = RnclButton::new(&input, "Speichern");
    save.set_geometry(110, 296, 120, 30);

    // --- Seite 2: Werte ---------------------------------------------------
    let values = tabs.add_tab("Werte");

    RnclLabel::new(&values, "Lautstärke:").set_position(10, 16);
    let volume = RnclSlider::new(&values);
    volume.set_geometry(110, 14, 260, 24);
    volume.set_range(0, 100);
    let volume_label = RnclLabel::new(&values, "0 %");
    volume_label.set_geometry(380, 16, 60, 20);

    RnclLabel::new(&values, "Pegel:").set_position(10, 56);
    let level = RnclProgressBar::new(&values);
    level.set_geometry(110, 52, 330, 24);

    RnclLabel::new(&values, "Verlauf:").set_position(10, 96);
    let history = RnclListBox::new(&values);
    history.set_geometry(110, 92, 330, 200);

    // --- Statuszeile ------------------------------------------------------
    let status = RnclLabel::new(&window, "Bereit.");
    status.set_geometry(14, 392, 530, 24);

    // --- Verhalten --------------------------------------------------------
    {
        let (vl, lv) = (volume_label.clone(), level.clone());
        volume.on_value_changed(move |v| {
            vl.set_text(&format!("{v} %"));
            lv.set_value(v);
        });
    }
    {
        let h = history.clone();
        level.on_value_changed(move |v| {
            if v % 25 == 0 {
                h.add_item(&format!("Pegel erreicht {v} %"));
            }
        });
    }
    {
        let s = status.clone();
        city.on_text_changed(move |c| s.set_text(&format!("Stadt gewählt: {c}")));
    }
    {
        let (name, city, age, news, mail, phone, status, history) = (
            name.clone(),
            city.clone(),
            age.clone(),
            news.clone(),
            mail.clone(),
            phone.clone(),
            status.clone(),
            history.clone(),
        );
        save.on_clicked(move || {
            let channel = if mail.is_checked() {
                "E-Mail"
            } else if phone.is_checked() {
                "Telefon"
            } else {
                "Post"
            };
            let summary = format!(
                "{} ({}), {} – Kontakt per {channel}{}",
                if name.text().is_empty() {
                    "Unbekannt".to_owned()
                } else {
                    name.text()
                },
                age.value(),
                city.current_text(),
                if news.is_checked() {
                    ", Newsletter"
                } else {
                    ""
                },
            );
            history.add_item(&summary);
            status.set_text(&format!("Gespeichert: {summary}"));
        });
    }
    volume.set_value(40);

    window.show();

    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--screenshot") {
        let path = args.get(pos + 1).map_or("gallery.png", String::as_str);
        Application::process_events();
        let ok = window.widget().save_screenshot(path);
        tabs.set_current_index(1);
        Application::process_events();
        let second = path.replace(".png", "-werte.png");
        let ok2 = window.widget().save_screenshot(&second);
        println!("Screenshots {path}, {second}: {}", ok && ok2);
        return;
    }

    std::process::exit(app.exec());
}
