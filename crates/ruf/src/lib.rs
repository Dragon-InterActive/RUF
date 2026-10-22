//! # RUF – Rust native UI Framework
//!
//! Das native Fundament für [RNCL](https://docs.rs/rncl)-Komponenten und das
//! RAD-Tool *Freya TD*. RUF steuert die **nativen Steuerelemente des
//! Betriebssystems** über eine sichere, pure Rust-API:
//!
//! | System | Toolkit | Bindung |
//! |---|---|---|
//! | Linux / BSD | GTK 4 | `gtk4-rs` |
//! | Windows | Win32 (Common Controls) | `windows-rs` |
//! | macOS | Cocoa (AppKit) | `objc2` |
//!
//! Das Backend wird beim Kompilieren automatisch gewählt – Anwendungen und
//! Komponenten schreiben denselben Code für alle drei Systeme.
//!
//! ## Ebenen
//!
//! 1. **System-Ebene** – je ein Backend pro Betriebssystem (intern, `backend/`).
//! 2. **Kern** – plattformunabhängige Logik: Lebensdauer, Signale,
//!    Geometrie, Timer (intern, siehe `core.rs`).
//! 3. **Logik-Ebene** – [`Widget`], [`Component`], [`ComponentRegistry`] und
//!    das Eigenschaftsmodell; darauf bauen die RNCL-Komponenten auf.
//! 4. **Designer** – spricht nur [`Component`] und [`ComponentRegistry`] an.
//!
//! ## Minimalbeispiel
//!
//! ```no_run
//! use ruf::{Application, Widget, WidgetKind};
//!
//! let app = Application::new();
//! let window = Widget::new(WidgetKind::Window);
//! window.set_text("Hallo RUF");
//! window.set_size(320, 200);
//! window.set_visible(true);
//! std::process::exit(app.exec());
//! ```

mod app;
mod backend;
mod component;
mod core;
pub mod dialog;
mod geometry;
mod gui_sender;
mod property;
mod registry;
mod style;
mod text;
mod timer;
mod toolkit;
mod widget;

pub use app::{Application, ColorScheme};
pub use component::{Component, base};
pub use geometry::{Point, Rect, Size};
pub use gui_sender::{GuiSender, post_to_gui};
pub use property::{PropertyDescriptor, PropertyError, PropertyType, PropertyValue};
pub use registry::{ComponentFactory, ComponentInfo, ComponentRegistry, RegistryError};
pub use style::{Color, Style};
pub use text::{Key, KeySpec};
pub use timer::Timer;
pub use toolkit::{Toolkit, toolkit};
pub use widget::{AsWidget, Widget, WidgetKind};

/// `use ruf::prelude::*;` holt die wichtigsten Typen und den
/// [`Component`]-Trait (für `.set_position()` & Co.) in den Scope.
pub mod prelude {
    pub use crate::{
        Application, AsWidget, Component, ComponentInfo, ComponentRegistry, GuiSender, Point,
        PropertyValue, Rect, Size, Timer, Widget, WidgetKind,
    };
}
