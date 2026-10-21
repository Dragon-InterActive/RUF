//! Integrationstests gegen echtes Qt (offscreen, ohne Bildschirm).
//!
//! Qt erlaubt nur eine QApplication pro Prozess, im Hauptthread. Darum läuft
//! dieser Test mit `harness = false` und eigenem `main()`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rncl::prelude::*;
use ruf::{PropertyError, QtVersion};

fn main() {
    let app = Application::with_args(["rncl-test", "-platform", "offscreen"]);

    let tests: &[(&str, fn())] = &[
        ("qt_version", qt_version),
        ("window_basics", window_basics),
        ("button_click", button_click),
        ("text_field_signals", text_field_signals),
        ("checkbox_toggle", checkbox_toggle),
        ("properties", properties),
        ("registry_create", registry_create),
        ("dropped_window_is_safe", dropped_window_is_safe),
        ("callback_panic_is_caught", callback_panic_is_caught),
        ("radio_buttons_are_exclusive", radio_buttons_are_exclusive),
        ("group_box_title", group_box_title),
        ("text_area", text_area),
        ("combo_box", combo_box),
        ("list_box", list_box),
        ("range_components", range_components),
        ("tab_view", tab_view),
        ("text_list_properties", text_list_properties),
        ("kinds_and_classes", kinds_and_classes),
    ];

    for (name, test) in tests {
        test();
        println!("test {name} ... ok");
    }
    Application::process_events();
    println!("\ntest result: ok. {} passed", tests.len());
    drop(app);
}

fn qt_version() {
    let rt = QtVersion::runtime();
    assert!(rt.at_least(6, 0), "Qt-Version {rt}");
    assert!(QtVersion::compile_time().major >= 6);
}

fn window_basics() {
    let window = RnclWindow::new("Titel");
    assert_eq!(window.title(), "Titel");
    assert_eq!(window.size(), Size::new(640, 480));
    window.set_position(10, 20);
    window.set_size(300, 200);
    assert_eq!(window.geometry(), Rect::new(10, 20, 300, 200));
    window.set_name("mainWindow");
    assert_eq!(window.name(), "mainWindow");
    assert_eq!(window.widget().class_name(), "QWidget");
    assert_eq!(window.widget().kind(), WidgetKind::Window);
}

fn button_click() {
    let window = RnclWindow::new("t");
    let button = RnclButton::new(&window, "OK");
    assert_eq!(button.text(), "OK");
    assert_eq!(button.widget().class_name(), "QPushButton");
    assert!(
        button.size().width > 0,
        "adjust_size sollte eine Breite liefern"
    );

    let clicks = Rc::new(Cell::new(0));
    let c = clicks.clone();
    button.on_clicked(move || c.set(c.get() + 1));

    // Ein Klon ist ein nicht besitzender Verweis auf dasselbe Widget.
    let label = RnclLabel::new(&window, "vorher");
    let l = label.clone();
    button.on_clicked(move || l.set_text("nachher"));

    button.click();
    button.click();
    assert_eq!(clicks.get(), 2);
    assert_eq!(label.text(), "nachher");

    button.set_position(5, 6);
    assert_eq!(button.position(), Point::new(5, 6));

    let png = std::env::temp_dir().join(format!("rncl-test-{}.png", std::process::id()));
    assert!(window.widget().save_screenshot(&png));
    assert!(
        std::fs::metadata(&png)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    );
    let _ = std::fs::remove_file(png);
}

fn text_field_signals() {
    let window = RnclWindow::new("t");
    let field = RnclTextField::new(&window);
    field.set_placeholder("Name");
    assert_eq!(field.placeholder(), "Name");
    field.set_password_mode(true);
    assert!(field.is_password_mode());

    let seen = Rc::new(RefCell::new(Vec::new()));
    let s = seen.clone();
    field.on_text_changed(move |t| s.borrow_mut().push(t));
    field.set_text("Hallo");
    field.set_text("Hallo Welt");
    assert_eq!(field.text(), "Hallo Welt");
    assert_eq!(*seen.borrow(), ["Hallo", "Hallo Welt"]);
}

fn checkbox_toggle() {
    let window = RnclWindow::new("t");
    let check = RnclCheckBox::new(&window, "Aktiv");
    let state = Rc::new(Cell::new(None));
    let s = state.clone();
    check.on_toggled(move |on| s.set(Some(on)));

    assert!(!check.is_checked());
    check.click();
    assert!(check.is_checked());
    assert_eq!(state.get(), Some(true));
    check.set_checked(false);
    assert_eq!(state.get(), Some(false));
}

fn properties() {
    let window = RnclWindow::new("t");
    let mut button = RnclButton::new(&window, "A");

    let names: Vec<_> = button.properties().iter().map(|p| p.name).collect();
    assert!(names.contains(&"text"));
    assert!(names.contains(&"x"));

    button.set_property("text", "B".into()).unwrap();
    button.set_property("x", 42.into()).unwrap();
    button.set_property("enabled", false.into()).unwrap();
    assert_eq!(button.property("text"), Some("B".into()));
    assert_eq!(button.property("x"), Some(42.into()));
    assert_eq!(button.property("enabled"), Some(false.into()));
    assert_eq!(button.property("gibt_es_nicht"), None);
    assert_eq!(button.property("visible"), Some(true.into()));
    button.set_property("visible", false.into()).unwrap();
    assert_eq!(button.property("visible"), Some(false.into()));

    assert!(matches!(
        button.set_property("text", 1.into()),
        Err(PropertyError::TypeMismatch { .. })
    ));
    assert!(matches!(
        button.set_property("x", "a".into()),
        Err(PropertyError::TypeMismatch { .. })
    ));
    assert!(matches!(
        button.set_property("gibt_es_nicht", 1.into()),
        Err(PropertyError::Unknown { .. })
    ));

    let mut check = RnclCheckBox::new(&window, "c");
    check.set_property("checked", true.into()).unwrap();
    assert!(check.is_checked());
}

fn registry_create() {
    let registry = rncl::registry();
    assert_eq!(registry.len(), 15);
    assert!(registry.categories().contains(&"Container"));

    let window = registry.create("RnclWindow", None).unwrap();
    for info in registry.iter().filter(|i| !i.container) {
        let mut c = registry
            .create(info.type_name, Some(window.widget()))
            .unwrap();
        assert_eq!(c.type_name(), info.type_name);
        c.set_property("name", info.type_name.into()).unwrap();
        assert_eq!(c.name(), info.type_name);
    }
    assert!(registry.create("Unbekannt", None).is_err());
}

fn dropped_window_is_safe() {
    let window = RnclWindow::new("t");
    let button = RnclButton::new(&window, "x");
    drop(window);
    // Qt hat Fenster samt Kindern gelöscht – der Verweis bleibt sicher benutzbar.
    assert!(!button.widget().is_alive());
    button.set_text("egal");
    assert_eq!(button.text(), "");
}

fn callback_panic_is_caught() {
    let window = RnclWindow::new("t");
    let button = RnclButton::new(&window, "x");
    let after = Rc::new(Cell::new(false));
    button.on_clicked(|| panic!("absichtliche Test-Panik"));
    let a = after.clone();
    button.on_clicked(move || a.set(true));
    button.click();
    assert!(
        after.get(),
        "nach einer Panik laufen weitere Handler weiter"
    );
}

fn radio_buttons_are_exclusive() {
    let window = RnclWindow::new("t");
    let group = RnclGroupBox::new(&window, "Farbe");
    let red = RnclRadioButton::new(&group, "Rot");
    let green = RnclRadioButton::new(&group, "Grün");
    // Eigene Gruppe: beeinflusst die Auswahl in `group` nicht.
    let other = RnclRadioButton::new(&window, "Andere Gruppe");

    let toggles = Rc::new(RefCell::new(Vec::new()));
    let t = toggles.clone();
    red.on_toggled(move |on| t.borrow_mut().push(on));

    red.click();
    other.click();
    assert!(red.is_checked());
    assert!(other.is_checked());
    green.set_checked(true);
    assert!(green.is_checked());
    assert!(
        !red.is_checked(),
        "Optionsfelder einer Gruppe schließen sich aus"
    );
    assert!(other.is_checked());
    assert_eq!(*toggles.borrow(), [true, false]);
}

fn group_box_title() {
    let window = RnclWindow::new("t");
    let mut group = RnclGroupBox::new(&window, "Adresse");
    assert_eq!(group.title(), "Adresse");
    group.set_property("title", "Anschrift".into()).unwrap();
    assert_eq!(group.property("title"), Some("Anschrift".into()));
    assert_eq!(group.widget().kind(), WidgetKind::GroupBox);
}

fn text_area() {
    let window = RnclWindow::new("t");
    let area = RnclTextArea::new(&window);
    let last = Rc::new(RefCell::new(String::new()));
    let l = last.clone();
    area.on_text_changed(move |t| *l.borrow_mut() = t);

    area.set_text("Zeile 1");
    area.append_line("Zeile 2");
    assert_eq!(area.text(), "Zeile 1\nZeile 2");
    assert_eq!(*last.borrow(), "Zeile 1\nZeile 2");

    area.set_placeholder("Notizen");
    assert_eq!(area.placeholder(), "Notizen");
    area.set_read_only(true);
    assert!(area.is_read_only());
}

fn combo_box() {
    let window = RnclWindow::new("t");
    let combo = RnclComboBox::new(&window);
    let indices = Rc::new(RefCell::new(Vec::new()));
    let i = indices.clone();
    combo.on_current_index_changed(move |idx| i.borrow_mut().push(idx));

    combo.set_items(&["Berlin", "Hamburg", "München"]);
    assert_eq!(combo.count(), 3);
    assert_eq!(
        combo.current_index(),
        0,
        "erster Eintrag wird automatisch gewählt"
    );
    combo.set_current_index(2);
    assert_eq!(combo.current_text(), "München");
    combo.set_current_text("Hamburg");
    assert_eq!(combo.current_index(), 1);

    combo.insert_item(0, "Bremen");
    combo.remove_item(3);
    assert_eq!(combo.items(), ["Bremen", "Berlin", "Hamburg"]);
    assert!(indices.borrow().contains(&2));

    combo.set_editable(true);
    combo.set_current_text("Freitext");
    assert!(combo.is_editable());
    assert_eq!(combo.current_text(), "Freitext");

    combo.clear();
    assert_eq!(combo.count(), 0);
    assert_eq!(combo.current_index(), -1);
}

fn list_box() {
    let window = RnclWindow::new("t");
    let list = RnclListBox::new(&window);
    let selected = Rc::new(Cell::new(-2));
    let s = selected.clone();
    list.on_current_index_changed(move |idx| s.set(idx));

    list.set_items(&["A", "B", "C"]);
    assert_eq!(list.current_index(), -1);
    assert_eq!(list.current_text(), None);
    list.set_current_index(1);
    assert_eq!(selected.get(), 1);
    assert_eq!(list.current_text().as_deref(), Some("B"));

    list.set_item_text(1, "Beta");
    list.add_item("D");
    list.remove_item(0);
    assert_eq!(list.items(), ["Beta", "C", "D"]);
    assert_eq!(list.item_text(99), "");
}

fn range_components() {
    let window = RnclWindow::new("t");

    let spin = RnclSpinBox::new(&window);
    let seen = Rc::new(Cell::new(0));
    let s = seen.clone();
    spin.on_value_changed(move |v| s.set(v));
    spin.set_range(1, 10);
    spin.set_value(42);
    assert_eq!(spin.value(), 10, "Wert wird auf den Bereich begrenzt");
    assert_eq!(seen.get(), 10);
    assert_eq!((spin.minimum(), spin.maximum()), (1, 10));

    let slider = RnclSlider::new(&window);
    assert!(!slider.is_vertical(), "Schieberegler startet waagerecht");
    slider.set_range(0, 255);
    slider.set_value(128);
    assert_eq!(slider.value(), 128);
    slider.set_vertical(true);
    assert!(slider.is_vertical());

    let bar = RnclProgressBar::new(&window);
    assert_eq!(bar.value(), 0);
    assert_eq!((bar.minimum(), bar.maximum()), (0, 100));
    bar.set_value(75);
    assert_eq!(bar.value(), 75);
    bar.set_busy(true);
    assert_eq!((bar.minimum(), bar.maximum()), (0, 0));

    // Eigenschaftsmodell über alle drei
    let mut spin = spin;
    spin.set_property("maximum", 50.into()).unwrap();
    spin.set_property("value", 33.into()).unwrap();
    assert_eq!(spin.property("value"), Some(33.into()));
    let mut slider = slider;
    slider.set_property("vertical", false.into()).unwrap();
    assert_eq!(slider.property("vertical"), Some(false.into()));
}

fn tab_view() {
    let window = RnclWindow::new("t");
    let tabs = RnclTabView::new(&window);
    let page1 = tabs.add_tab("Allgemein");
    let page2 = tabs.add_tab("Erweitert");
    let check = RnclCheckBox::new(&page2, "Experte");

    assert_eq!(tabs.tab_count(), 2);
    assert_eq!(tabs.tab_titles(), ["Allgemein", "Erweitert"]);
    assert!(page1.widget().is_alive());

    let changed = Rc::new(Cell::new(-1));
    let c = changed.clone();
    tabs.on_current_index_changed(move |i| c.set(i));
    tabs.set_current_index(1);
    assert_eq!(changed.get(), 1);

    tabs.set_tab_title(0, "Basis");
    assert_eq!(tabs.tab_title(0), "Basis");

    // Umbenennen behält den Inhalt, Entfernen löscht ihn mit.
    tabs.set_tab_titles(&["Basis", "Profi", "Neu"]);
    assert_eq!(tabs.tab_count(), 3);
    assert!(check.widget().is_alive());
    tabs.set_tab_titles(&["Basis"]);
    assert_eq!(tabs.tab_count(), 1);
    assert!(!check.widget().is_alive());
    assert!(!page2.widget().is_alive());

    // add_tab auf einem Nicht-Tab-Widget liefert ein totes Widget statt einer Panik.
    assert!(!window.widget().add_tab("x").is_alive());
}

fn text_list_properties() {
    let window = RnclWindow::new("t");
    let mut combo = RnclComboBox::new(&window);
    combo.set_property("items", ["x", "y"].into()).unwrap();
    combo.set_property("current_index", 1.into()).unwrap();
    assert_eq!(combo.property("items"), Some(["x", "y"].into()));
    assert_eq!(combo.property("text"), Some("y".into()));
    assert!(matches!(
        combo.set_property("items", "x".into()),
        Err(PropertyError::TypeMismatch { .. })
    ));

    let mut tabs = RnclTabView::new(&window);
    tabs.set_property("tabs", ["Eins", "Zwei"].into()).unwrap();
    assert_eq!(tabs.tab_count(), 2);
}

fn kinds_and_classes() {
    let registry = rncl::registry();
    let window = RnclWindow::new("t");
    let expected = [
        ("RnclRadioButton", WidgetKind::RadioButton, "QRadioButton"),
        ("RnclGroupBox", WidgetKind::GroupBox, "QGroupBox"),
        ("RnclTextArea", WidgetKind::TextArea, "QPlainTextEdit"),
        ("RnclComboBox", WidgetKind::ComboBox, "QComboBox"),
        ("RnclListBox", WidgetKind::ListBox, "QListWidget"),
        ("RnclSpinBox", WidgetKind::SpinBox, "QSpinBox"),
        ("RnclSlider", WidgetKind::Slider, "QSlider"),
        ("RnclProgressBar", WidgetKind::ProgressBar, "QProgressBar"),
        ("RnclTabView", WidgetKind::TabView, "QTabWidget"),
    ];
    for (type_name, kind, class) in expected {
        let c = registry.create(type_name, Some(window.widget())).unwrap();
        assert_eq!(c.widget().kind(), kind, "{type_name}");
        assert_eq!(c.widget().class_name(), class, "{type_name}");
        // Jede Eigenschaft muss lesbar sein.
        for d in c.properties() {
            assert!(c.property(d.name).is_some(), "{type_name}.{}", d.name);
        }
    }
}
