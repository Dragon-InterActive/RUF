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
    assert_eq!(registry.len(), 6);
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
