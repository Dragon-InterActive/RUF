//! # RUF – Rust native UI Framework
//!
//! Das native Fundament für [RNCL](https://docs.rs/rncl)-Komponenten und das
//! RAD-Tool *Freya TD*. RUF bindet das auf dem System installierte Qt (>= 6)
//! über [`cxx`] / [`cxx_qt`] und stellt eine sichere, pure Rust-API bereit.
//!
//! ## Ebenen
//!
//! 1. **System-Ebene (Brücke)** – [`sys`]: C++/Qt-Anbindung via cxx-qt.
//! 2. **Logik-Ebene** – [`Widget`], [`Component`], [`ComponentRegistry`] und
//!    das Eigenschaftsmodell; darauf bauen die RNCL-Komponenten auf.
//! 3. **Designer** – spricht nur [`Component`] und [`ComponentRegistry`] an.
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

#[cfg(not(feature = "qt6"))]
compile_error!("ruf benötigt ein Backend-Feature, z. B. `features = [\"qt6\"]`");

mod app;
mod component;
pub mod dialog;
mod ffi;
mod geometry;
mod gui_sender;
mod property;
mod registry;
mod timer;
mod version;
mod widget;

pub use app::{Application, ColorScheme};
pub use component::{Component, base};
pub use geometry::{Point, Rect, Size};
pub use gui_sender::{GuiSender, post_to_gui};
pub use property::{PropertyDescriptor, PropertyError, PropertyType, PropertyValue};
pub use registry::{ComponentFactory, ComponentInfo, ComponentRegistry, RegistryError};
pub use timer::Timer;
pub use version::QtVersion;
pub use widget::{AsWidget, Widget, WidgetKind};

/// Die rohe System-Ebene – nur für eigene cxx-Bridges (z. B. Komponenten mit
/// eigenen Qt-Widgets). In einer eigenen Bridge:
///
/// ```ignore
/// unsafe extern "C++" {
///     include!("ruf/cpp/ruf_widgets.h");
///     type QWidget = ruf::sys::QWidget;
/// }
/// ```
pub mod sys {
    pub use crate::ffi::ffi::QWidget;
}

/// Re-Export von `cxx-qt-lib` (QString, QColor, QPoint, …) für eigene Bridges.
pub use cxx_qt_lib as qt;

/// `use ruf::prelude::*;` holt die wichtigsten Typen und den
/// [`Component`]-Trait (für `.set_position()` & Co.) in den Scope.
pub mod prelude {
    pub use crate::{
        Application, AsWidget, Component, ComponentInfo, ComponentRegistry, GuiSender, Point,
        PropertyValue, Rect, Size, Timer, Widget, WidgetKind,
    };
}
