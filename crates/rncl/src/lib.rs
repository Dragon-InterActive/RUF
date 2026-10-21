//! # RNCL – Rust native Component Library
//!
//! Die nativen System-Widgets für [RUF](ruf): pure Rust-Strukturen, die jeweils
//! ein echtes Qt-Widget steuern.
//!
//! | Komponente        | Qt-Widget     | Container |
//! |-------------------|---------------|-----------|
//! | [`RnclWindow`]    | `QWidget`     | ja        |
//! | [`RnclPanel`]     | `QWidget`     | ja        |
//! | [`RnclButton`]    | `QPushButton` | nein      |
//! | [`RnclLabel`]     | `QLabel`      | nein      |
//! | [`RnclTextField`] | `QLineEdit`   | nein      |
//! | [`RnclCheckBox`]  | `QCheckBox`   | nein      |
//!
//! ```no_run
//! use rncl::prelude::*;
//!
//! let app = Application::new();
//! let window = RnclWindow::new("Hallo RNCL");
//! window.set_size(300, 120);
//!
//! let label = RnclLabel::new(&window, "Noch nicht geklickt");
//! label.set_geometry(20, 20, 260, 24);
//!
//! let button = RnclButton::new(&window, "Klick mich");
//! button.set_position(20, 60);
//! let l = label.clone();
//! button.on_clicked(move || l.set_text("Geklickt!"));
//!
//! window.show();
//! std::process::exit(app.exec());
//! ```
//!
//! Eigene Komponenten implementieren [`ruf::Component`] und werden über
//! [`ruf::ComponentRegistry`] angemeldet – siehe `docs/` und das Beispiel
//! `custom_component`.

mod button;
mod checkbox;
mod label;
mod panel;
mod text_field;
mod window;

pub use button::RnclButton;
pub use checkbox::RnclCheckBox;
pub use label::RnclLabel;
pub use panel::RnclPanel;
pub use text_field::RnclTextField;
pub use window::RnclWindow;

pub use ruf;

use ruf::{ComponentRegistry, RegistryError};

/// Meldet alle Standardkomponenten in `registry` an.
pub fn register_all(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(RnclWindow::info())?;
    registry.register(RnclPanel::info())?;
    registry.register(RnclButton::info())?;
    registry.register(RnclLabel::info())?;
    registry.register(RnclTextField::info())?;
    registry.register(RnclCheckBox::info())?;
    Ok(())
}

/// Eine neue Registry mit allen Standardkomponenten.
pub fn registry() -> ComponentRegistry {
    let mut registry = ComponentRegistry::new();
    register_all(&mut registry).expect("Standardkomponenten sind eindeutig");
    registry
}

/// `use rncl::prelude::*;` – alle Komponenten plus [`ruf::prelude`].
pub mod prelude {
    pub use crate::{RnclButton, RnclCheckBox, RnclLabel, RnclPanel, RnclTextField, RnclWindow};
    pub use ruf::prelude::*;
}

/// Gemeinsame Hilfen für Komponenten mit einer `text`-Eigenschaft.
pub(crate) mod text {
    use ruf::{PropertyDescriptor, PropertyType, PropertyValue, Widget};

    pub const TEXT: &str = "text";

    pub fn descriptor(description: &'static str) -> PropertyDescriptor {
        PropertyDescriptor::new(TEXT, PropertyType::Text, description)
    }

    pub fn get(widget: &Widget, name: &str) -> Option<PropertyValue> {
        (name == TEXT).then(|| widget.text().into())
    }
}
