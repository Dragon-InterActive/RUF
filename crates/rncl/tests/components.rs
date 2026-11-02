//! Integrationstests gegen das echte Toolkit (GTK 4).
//!
//! Native Toolkits erlauben nur eine Anwendung pro Prozess, im Hauptthread.
//! Darum läuft dieser Test mit `harness = false` und eigenem `main()`.
//! Linux ohne Bildschirm: `xvfb-run cargo test`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rncl::prelude::*;
use ruf::{Color, PropertyError};

fn main() {
    let app = Application::new();

    let tests: &[(&str, fn())] = &[
        ("toolkit", toolkit),
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
        ("scroll_view", scroll_view),
        ("dynamic_children", dynamic_children),
        ("resize_and_shortcut", resize_and_shortcut),
        ("timer", timer),
        ("gui_sender", gui_sender),
        ("text_area_code_editor", text_area_code_editor),
        ("canvas_shapes_and_input", canvas_shapes_and_input),
        ("tree_view", tree_view),
        ("splitter", splitter),
        ("menus", menus),
        ("window_position", window_position),
        ("adjust_size_is_stable", adjust_size_is_stable),
        ("code_editor", code_editor),
    ];

    for (name, test) in tests {
        test();
        println!("test {name} ... ok");
    }
    Application::process_events();
    println!("\ntest result: ok. {} passed", tests.len());
    drop(app);
}

fn toolkit() {
    let toolkit = ruf::toolkit();
    assert!(toolkit.name == "GTK", "{toolkit}");
    assert!(!toolkit.version.is_empty());
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
    assert!(!window.widget().class_name().is_empty());
    assert_eq!(window.widget().kind(), WidgetKind::Window);
}

fn button_click() {
    let window = RnclWindow::new("t");
    let button = RnclButton::new(&window, "OK");
    assert_eq!(button.text(), "OK");
    assert!(!button.widget().class_name().is_empty());
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
    assert_eq!(registry.len(), 20);
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
    // Fenster samt Kindern sind gelöscht – der Verweis bleibt sicher benutzbar.
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
    assert!(combo.is_editable());
    // Einträge bleiben; Wahl aus der Liste übernimmt Text und Index.
    assert_eq!(combo.items(), ["Bremen", "Berlin", "Hamburg"]);
    combo.set_current_index(1);
    assert_eq!(combo.current_text(), "Berlin");
    assert_eq!(combo.current_index(), 1);
    // Freier Text: keine Auswahl mehr.
    combo.set_current_text("Freitext");
    assert_eq!(combo.current_text(), "Freitext");
    assert_eq!(combo.current_index(), -1);
    // Zurück ohne Eingabe: wieder eine Liste.
    combo.set_editable(false);
    assert!(!combo.is_editable());
    combo.set_editable(true);

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
    // Zusammengesetzte Steuerelemente (Textfeld + Pfeile) brauchen eine echte Größe.
    assert!(
        spin.size().height >= 16 && spin.size().width >= 40,
        "{:?}",
        spin.size()
    );
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
        ("RnclRadioButton", WidgetKind::RadioButton),
        ("RnclGroupBox", WidgetKind::GroupBox),
        ("RnclTextArea", WidgetKind::TextArea),
        ("RnclComboBox", WidgetKind::ComboBox),
        ("RnclListBox", WidgetKind::ListBox),
        ("RnclSpinBox", WidgetKind::SpinBox),
        ("RnclSlider", WidgetKind::Slider),
        ("RnclProgressBar", WidgetKind::ProgressBar),
        ("RnclTabView", WidgetKind::TabView),
        ("RnclScrollView", WidgetKind::ScrollArea),
    ];
    for (type_name, kind) in expected {
        let c = registry.create(type_name, Some(window.widget())).unwrap();
        assert_eq!(c.widget().kind(), kind, "{type_name}");
        assert!(!c.widget().class_name().is_empty(), "{type_name}");
        // Jede Eigenschaft muss lesbar sein.
        for d in c.properties() {
            assert!(c.property(d.name).is_some(), "{type_name}.{}", d.name);
        }
    }
}

/// Arbeitet Events ab, bis `done()` gilt (höchstens zwei Sekunden).
fn wait_until(done: impl Fn() -> bool) -> bool {
    let start = std::time::Instant::now();
    while !done() {
        if start.elapsed() > std::time::Duration::from_secs(2) {
            return false;
        }
        Application::process_events();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    true
}

fn scroll_view() {
    let window = RnclWindow::new("t");
    let mut scroll = RnclScrollView::new(&window);
    scroll.set_geometry(0, 0, 200, 100);
    let check = RnclCheckBox::new(scroll.content(), "unten");
    check.set_position(4, 400);
    scroll.set_content_size(0, 430);
    window.show();
    Application::process_events();

    assert!(scroll.content().widget().is_alive());
    assert!(scroll.content_size().height >= 430);
    // Breite 0 = Inhalt folgt dem sichtbaren Bereich (abzüglich Bildlaufleiste).
    assert!(scroll.content_size().width > 100);
    scroll.set_property("content_height", 500.into()).unwrap();
    Application::process_events();
    assert_eq!(scroll.property("content_height"), Some(500.into()));
    // Auf anderen Widgets ist der Inhalt tot statt einer Panik.
    assert!(!window.widget().scroll_content().is_alive());
}

fn dynamic_children() {
    let window = RnclWindow::new("t");
    window.show();
    Application::process_events();
    // Nach dem Anzeigen erzeugte Kinder müssen sofort sichtbar sein.
    let late = RnclLabel::new(&window, "später");
    assert!(late.is_visible());

    late.widget().delete_later();
    assert!(!late.is_visible());
    assert!(wait_until(|| !late.widget().is_alive()));
}

fn resize_and_shortcut() {
    let window = RnclWindow::new("t");
    let size = Rc::new(Cell::new((0, 0)));
    let s = size.clone();
    window.on_resized(move |w, h| s.set((w, h)));
    window.show();
    window.set_size(333, 222);
    Application::process_events();
    assert_eq!(size.get(), (333, 222));

    let allow = Rc::new(Cell::new(false));
    let a = allow.clone();
    window.on_close_requested(move || a.get());
    window.close();
    Application::process_events();
    assert!(window.is_visible(), "abgelehntes Schließen");
    allow.set(true);
    window.close();
    Application::process_events();
    assert!(!window.is_visible());

    assert!(window.add_shortcut("Ctrl+S", || {}));
    assert!(!window.add_shortcut("", || {}));
}

fn timer() {
    let fired = Rc::new(Cell::new(0));
    let timer = ruf::Timer::new();
    timer.set_single_shot(true);
    let f = fired.clone();
    timer.on_timeout(move || f.set(f.get() + 1));
    timer.start(5);
    assert!(timer.is_active());
    assert_eq!(timer.interval(), 5);
    assert!(wait_until(|| fired.get() == 1));
    assert!(!timer.is_active());

    let once = Rc::new(Cell::new(false));
    let o = once.clone();
    ruf::Timer::single_shot(1, move || o.set(true));
    assert!(wait_until(|| once.get()));
}

fn gui_sender() {
    let window = RnclWindow::new("t");
    let label = RnclLabel::new(&window, "");
    let received = Rc::new(RefCell::new(Vec::new()));
    let (l, r) = (label.clone(), received.clone());
    let sender = GuiSender::new(move |n: u32| {
        // Läuft im GUI-Thread: Widgets dürfen angefasst werden.
        l.set_text(&n.to_string());
        r.borrow_mut().push(n);
    });

    let workers: Vec<_> = (0..3)
        .map(|_| {
            let tx = sender.clone();
            std::thread::spawn(move || {
                for n in 0..10 {
                    assert!(tx.send(n));
                }
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }
    drop(sender);
    assert!(wait_until(|| received.borrow().len() == 30));
    // Reihenfolge je Thread bleibt erhalten; der letzte Wert ist eine 9.
    assert_eq!(label.text(), "9");

    // Panik im Handler wird abgefangen, der Sender bleibt benutzbar.
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    let sender = GuiSender::new(move |fail: bool| {
        c.set(c.get() + 1);
        assert!(!fail, "Testpanik");
    });
    sender.send(true);
    sender.send(false);
    assert!(wait_until(|| count.get() == 2));
}

fn text_area_code_editor() {
    let window = RnclWindow::new("t");
    let mut code = RnclTextArea::new(&window);
    code.set_text("[package]\nname = \"x\"\n# Kommentar");
    code.set_monospace(true);
    assert!(code.is_monospace());
    assert_eq!(code.property("monospace"), Some(true.into()));
    code.set_property("monospace", false.into()).unwrap();

    assert!(code.add_highlight_rule(r"^\s*\[.*\]", "#2e86c1", true));
    assert!(code.add_highlight_rule("#.*$", "gray", false));
    assert!(!code.add_highlight_rule("(", "red", false)); // ungültiges Muster
    assert!(!code.add_highlight_rule("x", "keine-farbe", false));
    code.clear_highlight_rules();

    let mut label = RnclLabel::new(&window, "lang");
    assert!(!label.is_word_wrap());
    label.set_property("word_wrap", true.into()).unwrap();
    assert!(label.is_word_wrap());

    code.set_marked_line(Some(1), "#ffdddd");
    code.goto_line(2);
    code.set_marked_line(None, "");
    code.set_marked_line(Some(99), "red"); // außerhalb: kein Absturz
    assert_eq!(code.text().lines().count(), 3);
}

fn canvas_shapes_and_input() {
    let window = RnclWindow::new("Zeichenfläche");
    let canvas = RnclCanvas::new(&window);
    canvas.set_geometry(0, 0, 200, 100);
    let shapes = vec![
        Shape::Rect {
            rect: Rect::new(10, 10, 80, 30),
            fill: Some(Color::rgb(240, 240, 240)),
            stroke: Some(Color::rgb(0, 0, 0)),
            radius: 4,
            dashed: false,
        },
        Shape::Text {
            x: 20,
            y: 16,
            text: "OK".into(),
            color: Color::rgb(0, 0, 0),
            size: 13,
            bold: false,
        },
    ];
    canvas.set_shapes(shapes.clone());
    assert_eq!(canvas.shapes(), shapes);
    assert_eq!(canvas.widget().kind(), WidgetKind::Canvas);

    let seen: Rc<RefCell<Vec<MouseEvent>>> = Rc::default();
    let s = seen.clone();
    canvas.on_mouse(move |e| s.borrow_mut().push(e));
    let keys: Rc<RefCell<Vec<KeyEvent>>> = Rc::default();
    let k = keys.clone();
    canvas.on_key(move |e| k.borrow_mut().push(e));
    let click = MouseEvent {
        action: MouseAction::Pressed,
        x: 30,
        y: 20,
        button: 1,
        modifiers: ruf::Modifiers::default(),
    };
    canvas.widget().simulate_mouse(click);
    canvas.widget().simulate_key(KeyEvent {
        key: ruf::Key::Delete,
        modifiers: ruf::Modifiers::default(),
        text: None,
    });
    assert_eq!(seen.borrow().as_slice(), &[click]);
    assert_eq!(keys.borrow()[0].key, ruf::Key::Delete);

    // Texteingabe über die Eingabemethode (z. B. ^ + e = ê) – nur auf der
    // Zeichenfläche, nicht auf anderen Widgets.
    let typed: Rc<RefCell<String>> = Rc::default();
    let t = typed.clone();
    canvas.on_text_input(move |text| t.borrow_mut().push_str(&text));
    canvas.widget().simulate_text_input("ê");
    canvas.widget().simulate_text_input("@");
    assert_eq!(typed.borrow().as_str(), "ê@");
    canvas
        .widget()
        .set_input_cursor(ruf::Rect::new(10, 10, 8, 16));
    assert!(!RnclButton::new(&window, "x").widget().on_text_input(|_| {}));

    // Gemalt wird wirklich (Screenshot ist nicht leer).
    window.show();
    Application::process_events();
    let shot = std::env::temp_dir().join(format!("ruf-canvas-{}.png", std::process::id()));
    if canvas.widget().save_screenshot(&shot) {
        assert!(std::fs::metadata(&shot).unwrap().len() > 0);
        let _ = std::fs::remove_file(&shot);
    }
}

fn splitter() {
    let window = RnclWindow::new("Teiler");
    let splitter = RnclSplitter::new(&window, true);
    assert!(splitter.is_horizontal());
    let moves: Rc<RefCell<Vec<i32>>> = Rc::default();
    let m = moves.clone();
    splitter.on_dragged(move |delta| m.borrow_mut().push(delta));
    let event = |action, y, button| MouseEvent {
        action,
        x: 50,
        y,
        button,
        modifiers: ruf::Modifiers::default(),
    };
    let w = splitter.widget();
    // Bewegen ohne Drücken meldet nichts; gezogen wird relativ zum Druckpunkt.
    w.simulate_mouse(event(MouseAction::Moved, 10, 0));
    w.simulate_mouse(event(MouseAction::Pressed, 3, 1));
    w.simulate_mouse(event(MouseAction::Moved, 23, 0));
    w.simulate_mouse(event(MouseAction::Moved, -2, 0));
    w.simulate_mouse(event(MouseAction::Released, -2, 1));
    w.simulate_mouse(event(MouseAction::Moved, 40, 0));
    assert_eq!(moves.borrow().as_slice(), [20, -5]);
}

fn tree_view() {
    let window = RnclWindow::new("Baum");
    let tree = RnclTreeView::new(&window);
    tree.set_items(vec![
        TreeItem::new("main", "main")
            .with_child(TreeItem::new("main.ok", "ok_button"))
            .with_child(TreeItem::new("main.name", "name_edit")),
        TreeItem::new("services", "services")
            .with_child(TreeItem::new("services.pool", "db_pool"))
            .collapsed(),
    ]);
    assert_eq!(
        tree.visible_items(),
        ["main", "main.ok", "main.name", "services"]
    );

    let selections: Rc<RefCell<Vec<String>>> = Rc::default();
    let s = selections.clone();
    tree.on_selection_changed(move |id| s.borrow_mut().push(id));
    let activated: Rc<RefCell<Vec<String>>> = Rc::default();
    let a = activated.clone();
    tree.on_item_activated(move |id| a.borrow_mut().push(id));

    tree.select(Some("main.name"));
    assert_eq!(tree.selected().as_deref(), Some("main.name"));
    assert_eq!(selections.borrow().as_slice(), ["main.name"]);
    tree.widget().activate_tree_item();
    assert_eq!(activated.borrow().as_slice(), ["main.name"]);

    // Aufklappen zeigt Kinder, die Auswahl bleibt.
    assert!(tree.toggle("services"));
    assert_eq!(tree.visible_items().len(), 5);
    assert_eq!(tree.selected().as_deref(), Some("main.name"));
    // Zuklappen des Elternknotens verbirgt die Auswahl: keine Auswahl mehr.
    assert!(tree.toggle("main"));
    assert_eq!(tree.visible_items(), ["main", "services", "services.pool"]);
    assert_eq!(tree.selected(), None);

    // Rechtsklick (die Zeile ist dann gewählt) meldet Kennung und Position.
    let context: Rc<RefCell<Vec<(String, i32, i32)>>> = Rc::default();
    let c = context.clone();
    tree.on_context_menu(move |id, x, y| c.borrow_mut().push((id, x, y)));
    tree.select(Some("services.pool"));
    let click = |button| MouseEvent {
        action: MouseAction::Pressed,
        x: 20,
        y: 40,
        button,
        modifiers: ruf::Modifiers::default(),
    };
    tree.widget().simulate_mouse(click(1));
    tree.widget().simulate_mouse(click(3));
    assert_eq!(
        context.borrow().as_slice(),
        [("services.pool".to_owned(), 20, 40)]
    );
}

fn menus() {
    let window = RnclWindow::new("Menüs");
    window.set_menu_bar(vec![
        (
            "Datei".into(),
            Menu::new()
                .item("file.open", "Öffnen …", Some("Ctrl+O"))
                .separator()
                .item("file.quit", "Beenden", Some("Ctrl+Q")),
        ),
        (
            "Bearbeiten".into(),
            Menu::new()
                .disabled_item("edit.undo", "Rückgängig")
                .submenu("Ausrichten", Menu::new().item("align.left", "Links", None)),
        ),
    ]);
    assert_eq!(window.widget().menu_bar().len(), 2);
    let chosen: Rc<RefCell<Vec<String>>> = Rc::default();
    let c = chosen.clone();
    window.on_menu(move |id| c.borrow_mut().push(id));
    window.widget().activate_menu_item("file.open");

    // Kürzel eines anderen Fensters übernehmen – außer den eigenen.
    let main = RnclWindow::new("Haupt");
    main.set_menu_bar(vec![(
        "Projekt".into(),
        Menu::new()
            .item("project.generate", "Code erzeugen", Some("Ctrl+G"))
            .item("file.save", "Speichern", Some("Ctrl+S"))
            .item("plain", "Ohne Kürzel", None),
    )]);
    let code = RnclWindow::new("Code");
    code.set_menu_bar(vec![(
        "Datei".into(),
        Menu::new().item("code.save", "Speichern", Some("Ctrl+S")),
    )]);
    assert_eq!(code.inherit_menu_shortcuts(&main), 1);
    assert_eq!(chosen.borrow().as_slice(), ["file.open"]);

    // Menüleiste ersetzen; der Inhalt bekommt weiter seine Größe.
    window.set_menu_bar(vec![(
        "Hilfe".into(),
        Menu::new().item("help.about", "Über", None),
    )]);
    let label = RnclLabel::new(&window, "Inhalt");
    label.set_position(10, 10);
    window.show();
    Application::process_events();
    assert!(label.widget().is_alive());
    // Kontextmenü an einem Kind öffnen und wieder schließen lassen.
    assert!(
        label
            .widget()
            .popup_menu(&Menu::new().item("ctx.copy", "Kopieren", None), 5, 5)
    );
    Application::process_events();
}

fn window_position() {
    let window = RnclWindow::new("Position");
    window.set_size(200, 100);
    // Vor dem Anzeigen: vorgemerkt (unter Wayland nicht möglich).
    if !window.set_screen_position(120, 80) {
        return;
    }
    window.show();
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(2) {
        Application::process_events();
        if window.screen_position() == Some(Point::new(120, 80)) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(window.screen_position(), Some(Point::new(120, 80)));
    assert!(window.set_screen_position(300, 200));
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(2)
        && window.screen_position() != Some(Point::new(300, 200))
    {
        Application::process_events();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(window.screen_position(), Some(Point::new(300, 200)));
}

/// Die bevorzugte Größe hängt nicht von der zuletzt gesetzten Größe ab –
/// sonst wachsen Knöpfe, die "bevorzugt + Rand" gesetzt werden, bei jeder
/// Neuanordnung weiter.
fn adjust_size_is_stable() {
    let window = RnclWindow::new("Größe");
    let button = RnclButton::new(&window, "Mit Cargo prüfen");
    button.widget().adjust_size();
    let natural = button.size();
    for _ in 0..5 {
        button.set_size(natural.width + 200, natural.height);
        button.widget().adjust_size();
        assert_eq!(button.size(), natural);
    }
}

fn code_editor() {
    let window = RnclWindow::new("Code");
    let editor = RnclCodeEditor::new(&window);
    assert_eq!(editor.language(), "rust");
    let changes = Rc::new(Cell::new(0));
    let c = changes.clone();
    editor.on_text_changed(move |_| c.set(c.get() + 1));
    editor.set_text("fn main() {\n    println!(\"hi\");\n}\n");
    assert_eq!(editor.text(), "fn main() {\n    println!(\"hi\");\n}\n");
    assert!(changes.get() >= 1);

    editor.set_line_marks(vec![
        LineMark::new(2, MarkKind::Error, "cannot find macro"),
        LineMark::new(1, MarkKind::Warning, "unused"),
    ]);
    assert_eq!(editor.line_marks().len(), 2);
    editor.set_line_marks(Vec::new());
    assert!(editor.line_marks().is_empty());

    // Cursor und Vervollständigung: "pri" wird zu "println".
    editor.set_text("fn main() {\n    pri\n}\n");
    editor.set_cursor(2, 7);
    assert_eq!(editor.cursor(), (2, 7));
    editor.apply_completion("println");
    assert_eq!(editor.text(), "fn main() {\n    println\n}\n");
    assert_eq!(editor.cursor(), (2, 11));
    editor.show_completions(&[Completion {
        label: "len".into(),
        detail: Some("fn(&self) -> usize".into()),
        insert: "len".into(),
    }]);
    editor.show_completions(&[]);

    // Faltung: Bereiche aus den Klammern, Zustand folgt Bearbeitungen.
    editor.set_text("fn main() {\n    let v = [\n        1,\n    ];\n}\n");
    let starts: Vec<(usize, usize)> = editor.folds().iter().map(|f| (f.start, f.end)).collect();
    assert_eq!(starts, [(1, 5), (2, 4)]);
    editor.set_cursor(3, 4);
    editor.set_folded(2, true);
    assert!(editor.folds()[1].folded);
    assert_eq!(
        editor.cursor().0,
        2,
        "Cursor verlässt den gefalteten Bereich"
    );
    editor.set_cursor(1, 0);
    editor.apply_completion("// oben\n");
    assert_eq!(
        editor
            .folds()
            .iter()
            .map(|f| (f.start, f.folded))
            .collect::<Vec<_>>(),
        [(2, false), (3, true)],
        "Faltung wandert mit"
    );
    editor.goto_line(4);
    assert!(
        editor.folds().iter().all(|f| !f.folded),
        "Sprung klappt auf"
    );
    editor.set_all_folded(true);
    assert!(editor.folds().iter().all(|f| f.folded));
    editor.set_all_folded(false);
    assert!(editor.folds().iter().all(|f| !f.folded));

    editor.set_read_only(true);
    assert!(editor.is_read_only());
    editor.goto_line(3);
    editor.set_language("toml");
    assert_eq!(editor.language(), "toml");
    assert_eq!(editor.widget().kind(), WidgetKind::CodeEditor);
}
