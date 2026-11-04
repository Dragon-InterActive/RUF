//! Menüleiste, Baumansicht und Zeichenfläche: die Bausteine einer
//! Entwicklungsumgebung. Auf der Zeichenfläche lassen sich Rechtecke mit der
//! Maus aufziehen; Entf löscht das letzte, Rechtsklick öffnet ein Kontextmenü.
//!
//! ```bash
//! cargo run -p rncl --example workbench
//! cargo run -p rncl --example workbench -- --screenshot workbench.png
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use rncl::prelude::*;
use ruf::{Color, Key};

fn draw(boxes: &[Rect], drag: Option<Rect>) -> Vec<Shape> {
    let mut shapes = vec![Shape::Rect {
        rect: Rect::new(0, 0, 2000, 2000),
        fill: Some(Color::rgb(228, 228, 228)),
        stroke: None,
        radius: 0,
        dashed: false,
    }];
    for (i, r) in boxes.iter().enumerate() {
        shapes.push(Shape::Rect {
            rect: *r,
            fill: Some(Color::rgb(250, 250, 250)),
            stroke: Some(Color::rgb(160, 160, 160)),
            radius: 6,
            dashed: false,
        });
        shapes.push(Shape::Text {
            x: r.x + 8,
            y: r.y + 6,
            text: format!("Box {}", i + 1),
            color: Color::rgb(40, 40, 40),
            size: 13,
            bold: i == boxes.len() - 1,
            italic: false,
            family: None,
        });
    }
    if let Some(r) = drag {
        shapes.push(Shape::Rect {
            rect: r,
            fill: None,
            stroke: Some(Color::rgb(53, 132, 228)),
            radius: 0,
            dashed: true,
        });
    }
    shapes
}

fn normalized(a: (i32, i32), b: (i32, i32)) -> Rect {
    Rect::new(
        a.0.min(b.0),
        a.1.min(b.1),
        (a.0 - b.0).abs(),
        (a.1 - b.1).abs(),
    )
}

fn main() {
    let app = Application::new();
    let window = RnclWindow::new("Werkbank");
    window.set_size(640, 400);
    window.set_menu_bar(vec![
        (
            "Datei".into(),
            Menu::new()
                .item("file.new", "Neu", Some("Ctrl+N"))
                .item("file.open", "Öffnen …", Some("Ctrl+O"))
                .separator()
                .item("file.quit", "Beenden", Some("Ctrl+Q")),
        ),
        (
            "Bearbeiten".into(),
            Menu::new()
                .item("edit.delete", "Letzte Box löschen", Some("Delete"))
                .disabled_item("edit.redo", "Wiederholen"),
        ),
    ]);

    let tree = RnclTreeView::new(&window);
    tree.set_geometry(8, 8, 180, 340);
    tree.set_items(vec![
        TreeItem::new("main", "main (Formular)")
            .with_child(TreeItem::new("main.window", "window"))
            .with_child(TreeItem::new("main.ok", "ok_button")),
        TreeItem::new("services", "services (Datenmodul)")
            .with_child(TreeItem::new("services.pool", "db_pool"))
            .collapsed(),
    ]);
    tree.select(Some("main.ok"));

    let canvas = RnclCanvas::new(&window);
    canvas.set_geometry(196, 8, 436, 340);
    let boxes = Rc::new(RefCell::new(vec![
        Rect::new(30, 30, 160, 60),
        Rect::new(60, 120, 220, 90),
    ]));
    let start: Rc<RefCell<Option<(i32, i32)>>> = Rc::default();
    canvas.set_shapes(draw(&boxes.borrow(), None));

    let (c, b, s) = (canvas.clone(), boxes.clone(), start.clone());
    canvas.on_mouse(move |e| match (e.action, e.button) {
        (MouseAction::Pressed, 3) => {
            c.widget().popup_menu(
                &Menu::new().item("edit.delete", "Letzte Box löschen", None),
                e.x,
                e.y,
            );
        }
        (MouseAction::Pressed, 1) => *s.borrow_mut() = Some((e.x, e.y)),
        (MouseAction::Moved, _) => {
            if let Some(a) = *s.borrow() {
                c.set_shapes(draw(&b.borrow(), Some(normalized(a, (e.x, e.y)))));
            }
        }
        (MouseAction::Released, 1) => {
            if let Some(a) = s.borrow_mut().take() {
                let r = normalized(a, (e.x, e.y));
                if r.width > 4 && r.height > 4 {
                    b.borrow_mut().push(r);
                }
                c.set_shapes(draw(&b.borrow(), None));
            }
        }
        _ => {}
    });
    let (c, b) = (canvas.clone(), boxes.clone());
    let delete = Rc::new(move || {
        b.borrow_mut().pop();
        c.set_shapes(draw(&b.borrow(), None));
    });
    let d = delete.clone();
    canvas.on_key(move |e| {
        if e.key == Key::Delete || e.key == Key::Backspace {
            d();
        }
    });
    let w = window.clone();
    window.on_menu(move |id| match id.as_str() {
        "edit.delete" => delete(),
        "file.quit" => w.close(),
        _ => println!("Menü: {id}"),
    });

    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--screenshot") {
        let path = args.get(pos + 1).map_or("workbench.png", String::as_str);
        window.show();
        for _ in 0..20 {
            Application::process_events();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let ok = window.widget().save_screenshot(path);
        println!(
            "Screenshot {path}: {}",
            if ok { "gespeichert" } else { "fehlgeschlagen" }
        );
        return;
    }
    window.show();
    std::process::exit(app.exec());
}
