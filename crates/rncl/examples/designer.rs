//! So spricht ein Form-Designer (Freya TD) mit RNCL: nur über Typnamen,
//! die `ComponentRegistry` und das Eigenschaftsmodell – ohne die konkreten
//! Rust-Typen zu kennen.
//!
//! Das Formular wird aus einer einfachen Textbeschreibung aufgebaut:
//!
//! ```text
//! <Typname> <name> [eigenschaft=wert ...]
//! ```
//!
//! cargo run -p rncl --example designer            # Fenster anzeigen
//! cargo run -p rncl --example designer -- --dump  # Eigenschaften ausgeben
//! cargo run -p rncl --example designer -- --screenshot form.png

use rncl::prelude::*;
use ruf::{PropertyType, PropertyValue};

const FORM: &str = r#"
RnclWindow    frmMain   title="Designer-Demo" width=360 height=200
RnclLabel     lblName   text="Name:" x=20 y=24
RnclTextField txtName   x=100 y=20 width=230 placeholder="Vor- und Nachname"
RnclCheckBox  chkNews   text="Newsletter" x=100 y=60 checked=true
RnclButton    btnOk     text="OK" x=100 y=110 width=100
RnclButton    btnCancel text="Abbrechen" x=210 y=110 width=100 enabled=false
"#;

fn main() {
    let app = Application::new();
    let registry = rncl::registry();

    let mut root: Option<Box<dyn Component>> = None;
    let mut children: Vec<Box<dyn Component>> = Vec::new();

    for (line_no, line) in FORM.lines().enumerate() {
        let tokens = tokenize(line);
        let [type_name, name, props @ ..] = tokens.as_slice() else {
            continue;
        };
        let parent = root.as_deref().map(|r| r.widget());
        let mut component = registry
            .create(type_name, parent)
            .unwrap_or_else(|e| panic!("Zeile {}: {e}", line_no + 1));
        component.set_name(name);

        for prop in props {
            let (key, raw) = prop.split_once('=').expect("eigenschaft=wert");
            let ty = component
                .properties()
                .into_iter()
                .find(|d| d.name == key)
                .map(|d| d.ty);
            let value = match ty {
                Some(PropertyType::Int) => PropertyValue::Int(raw.parse().expect("Zahl")),
                Some(PropertyType::Bool) => PropertyValue::Bool(raw == "true"),
                _ => PropertyValue::Text(raw.to_owned()),
            };
            if let Err(e) = component.set_property(key, value) {
                eprintln!("Zeile {}: {name}: {e}", line_no + 1);
            }
        }

        if root.is_none() {
            root = Some(component);
        } else {
            children.push(component);
        }
    }

    let root = root.expect("Formular braucht ein Fenster");

    if std::env::args().any(|a| a == "--dump") {
        for c in std::iter::once(&root).chain(&children) {
            println!("{} ({})", c.name(), c.type_name());
            for d in c.properties() {
                let v = c
                    .property(d.name)
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                println!("    {:<12} {:<5} = {v}", d.name, d.ty);
            }
        }
        return;
    }

    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--screenshot") {
        let path = args.get(pos + 1).map_or("designer.png", String::as_str);
        root.show();
        Application::process_events();
        let ok = root.widget().save_screenshot(path);
        println!(
            "Screenshot {path}: {}",
            if ok { "gespeichert" } else { "fehlgeschlagen" }
        );
        return;
    }

    root.show();
    std::process::exit(app.exec());
}

/// Zerlegt eine Zeile an Leerzeichen; `"…"` hält Text mit Leerzeichen zusammen.
fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for ch in line.chars() {
        match ch {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}
