//! Der Code-Editor: Rust-Hervorhebung, Zeilennummern, Marken am Rand,
//! Haltepunkte (Klick in die linke Spalte schaltet um) und die Zeile des
//! Debuggers.
//!
//! ```bash
//! cargo run -p rncl --example code_editor
//! cargo run -p rncl --example code_editor -- --screenshot editor.png
//! cargo run -p rncl --example code_editor -- --folded   # Funktion gefaltet
//! ```

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use rncl::prelude::*;

const SOURCE: &str = r#"//! Form `main_form` – your code.

include!("main_form.gen.rs");

impl MainForm {
    /// Handles `ok_button.on_click`.
    pub fn ok_button_click(&mut self) {
        let name = self.name_edit.text();
        println!("Hallo {name}!");
        undefined_function();
    }
}
"#;

fn main() {
    let app = Application::new();
    let window = RnclWindow::new("Code-Editor");
    window.set_size(640, 320);
    let editor = RnclCodeEditor::new(&window);
    editor.set_text(SOURCE);
    editor.set_line_marks(vec![
        LineMark::new(
            10,
            MarkKind::Error,
            "cannot find function `undefined_function`",
        ),
        LineMark::new(8, MarkKind::Warning, "unused variable"),
    ]);
    // Haltepunkte: Linksklick setzt/entfernt, Rechtsklick schaltet ab/an.
    let breakpoints: Rc<RefCell<BTreeMap<usize, BreakpointKind>>> = Rc::new(RefCell::new(
        [
            (8, BreakpointKind::Normal),
            (9, BreakpointKind::Conditional),
        ]
        .into(),
    ));
    let show = {
        let editor = editor.clone();
        let breakpoints = breakpoints.clone();
        move || {
            editor.set_breakpoint_marks(
                breakpoints
                    .borrow()
                    .iter()
                    .map(|(&line, &kind)| BreakpointMark { line, kind })
                    .collect(),
            );
        }
    };
    show();
    editor.set_execution_line(Some(8));
    editor.on_gutter_clicked(move |line, button| {
        println!("Randleiste: Zeile {line}, Taste {button}");
        {
            let mut b = breakpoints.borrow_mut();
            match (button, b.get(&line).copied()) {
                (3, Some(BreakpointKind::Disabled)) => {
                    b.insert(line, BreakpointKind::Normal);
                }
                (3, Some(_)) => {
                    b.insert(line, BreakpointKind::Disabled);
                }
                (_, Some(_)) => {
                    b.remove(&line);
                }
                (_, None) => {
                    b.insert(line, BreakpointKind::Normal);
                }
            }
        }
        show();
    });
    let e = editor.clone();
    window.on_resized(move |w, h| e.set_geometry(0, 0, w, h));
    editor.goto_line(9);

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--folded") {
        editor.goto_line(1);
        editor.set_folded(7, true);
    }
    if let Some(pos) = args.iter().position(|a| a == "--screenshot") {
        let path = args.get(pos + 1).map_or("editor.png", String::as_str);
        window.show();
        for _ in 0..20 {
            Application::process_events();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let ok = window.widget().save_screenshot(path);
        println!("Screenshot {path}: {ok}");
        if !args.iter().any(|a| a == "--stay") {
            return;
        }
    }
    window.show();
    std::process::exit(app.exec());
}
