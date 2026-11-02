//! # RNCL – Rust native Component Library
//!
//! Die nativen System-Widgets für [RUF](ruf): pure Rust-Strukturen, die jeweils
//! ein echtes GTK-4-Steuerelement steuern – auf Linux, Windows und macOS.
//!
//! | Komponente | GTK | Container |
//! |---|---|---|
//! | [`RnclWindow`] | `GtkWindow` | ja |
//! | [`RnclPanel`] | `RufFixed` | ja |
//! | [`RnclButton`] | `GtkButton` | nein |
//! | [`RnclLabel`] | `GtkLabel` | nein |
//! | [`RnclTextField`] | `GtkEntry` | nein |
//! | [`RnclCheckBox`] | `GtkCheckButton` | nein |
//! | [`RnclRadioButton`] | `GtkCheckButton` | nein |
//! | [`RnclGroupBox`] | `GtkFrame` | ja |
//! | [`RnclTextArea`] | `GtkTextView` | nein |
//! | [`RnclComboBox`] | `GtkDropDown` (mit Eingabe: `GtkEntry` + Vorschlagsliste) | nein |
//! | [`RnclListBox`] | `GtkListBox` | nein |
//! | [`RnclSpinBox`] | `GtkSpinButton` | nein |
//! | [`RnclSlider`] | `GtkScale` | nein |
//! | [`RnclProgressBar`] | `GtkProgressBar` | nein |
//! | [`RnclTabView`] | `GtkNotebook` | ja (Seiten) |
//! | [`RnclScrollView`] | `GtkScrolledWindow` | ja (Inhalt) |
//! | [`RnclCanvas`] | `GtkDrawingArea` | nein |
//! | [`RnclTreeView`] | `GtkListBox` (eingerückt) | nein |
//! | [`RnclCodeEditor`] | `GtkSourceView` | nein |
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
mod canvas;
mod checkbox;
mod code_editor;
mod combo_box;
mod group_box;
mod label;
mod list_box;
mod panel;
mod progress_bar;
mod props;
mod radio_button;
mod scroll_view;
mod slider;
mod spin_box;
mod splitter;
mod tab_view;
mod text_area;
mod text_field;
mod tree_view;
mod window;

pub use button::RnclButton;
pub use canvas::RnclCanvas;
pub use checkbox::RnclCheckBox;
pub use code_editor::RnclCodeEditor;
pub use combo_box::RnclComboBox;
pub use group_box::RnclGroupBox;
pub use label::RnclLabel;
pub use list_box::RnclListBox;
pub use panel::RnclPanel;
pub use progress_bar::RnclProgressBar;
pub use radio_button::RnclRadioButton;
pub use scroll_view::RnclScrollView;
pub use slider::RnclSlider;
pub use spin_box::RnclSpinBox;
pub use splitter::{RnclSplitter, SPLITTER_SIZE};
pub use tab_view::RnclTabView;
pub use text_area::RnclTextArea;
pub use text_field::RnclTextField;
pub use tree_view::RnclTreeView;
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
    registry.register(RnclRadioButton::info())?;
    registry.register(RnclGroupBox::info())?;
    registry.register(RnclTextArea::info())?;
    registry.register(RnclComboBox::info())?;
    registry.register(RnclListBox::info())?;
    registry.register(RnclSpinBox::info())?;
    registry.register(RnclSlider::info())?;
    registry.register(RnclProgressBar::info())?;
    registry.register(RnclTabView::info())?;
    registry.register(RnclScrollView::info())?;
    registry.register(RnclCanvas::info())?;
    registry.register(RnclTreeView::info())?;
    registry.register(RnclCodeEditor::info())?;
    registry.register(RnclSplitter::info())?;
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
    pub use crate::{
        RnclButton, RnclCanvas, RnclCheckBox, RnclCodeEditor, RnclComboBox, RnclGroupBox,
        RnclLabel, RnclListBox, RnclPanel, RnclProgressBar, RnclRadioButton, RnclScrollView,
        RnclSlider, RnclSpinBox, RnclSplitter, RnclTabView, RnclTextArea, RnclTextField,
        RnclTreeView, RnclWindow,
    };
    pub use ruf::prelude::*;
    pub use ruf::{
        Completion, DiffKind, DiffMark, Fold, KeyEvent, LineMark, MarkKind, Menu, MenuBar,
        Modifiers, MouseAction, MouseEvent, Shape, TreeItem,
    };
}
