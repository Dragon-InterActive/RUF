//! Die rohe Brücke zwischen Rust und C++/Qt.
//!
//! Nur über [`crate::sys`] öffentlich und für Fortgeschrittene gedacht.
//! Normale Anwender und RNCL-Komponenten nutzen [`crate::Widget`] und
//! [`crate::Application`].

use std::cell::RefCell;

#[cxx_qt::bridge(namespace = "ruf")]
pub mod ffi {
    /// Die Arten nativer Widgets, die RUF direkt erzeugen kann.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum WidgetKind {
        /// Ein Top-Level-Fenster (`QWidget` mit `Qt::Window`).
        Window = 0,
        /// Ein schlichter Container (`QWidget`), z. B. für zusammengesetzte Komponenten.
        Container = 1,
        /// `QPushButton`
        PushButton = 2,
        /// `QLabel`
        Label = 3,
        /// `QLineEdit`
        LineEdit = 4,
        /// `QCheckBox`
        CheckBox = 5,
    }

    /// Farbschema der Anwendung (Qt >= 6.5 liest, Qt >= 6.8 setzt).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum ColorScheme {
        Unknown = 0,
        Light = 1,
        Dark = 2,
    }

    extern "Rust" {
        type RufCallback;
        type RufBoolCallback;
        type RufStringCallback;

        fn invoke_callback(callback: &RufCallback);
        fn invoke_bool_callback(callback: &RufBoolCallback, value: bool);
        fn invoke_string_callback(callback: &RufStringCallback, value: &QString);
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        #[namespace = ""]
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qrect.h");
        #[namespace = ""]
        type QRect = cxx_qt_lib::QRect;

        include!("ruf/cpp/ruf_widgets.h");

        /// Das native Qt-Widget. Downstream-Bridges können es per
        /// `type QWidget = ruf::sys::QWidget;` wiederverwenden.
        #[namespace = ""]
        type QWidget;

        type Application;
        type WidgetHandle;

        fn application_new(args: &Vec<String>) -> UniquePtr<Application>;
        fn application_exists() -> bool;
        fn application_exec() -> i32;
        fn application_quit();
        fn application_process_events();
        fn application_set_name(name: &QString);
        fn application_name() -> QString;
        fn application_color_scheme() -> ColorScheme;
        fn application_set_color_scheme(scheme: ColorScheme) -> bool;
        fn qt_compile_version() -> i32;
        fn qt_runtime_version() -> QString;

        fn widget_create(kind: WidgetKind) -> UniquePtr<WidgetHandle>;
        fn widget_create_child(kind: WidgetKind, parent: &WidgetHandle) -> UniquePtr<WidgetHandle>;
        unsafe fn widget_wrap(widget: *mut QWidget) -> UniquePtr<WidgetHandle>;

        fn widget_raw(handle: &WidgetHandle) -> *mut QWidget;
        fn widget_is_alive(handle: &WidgetHandle) -> bool;
        fn widget_kind(handle: &WidgetHandle) -> WidgetKind;
        fn widget_class_name(handle: &WidgetHandle) -> QString;
        fn widget_set_parent(handle: &WidgetHandle, parent: &WidgetHandle);

        fn widget_set_geometry(handle: &WidgetHandle, rect: &QRect);
        fn widget_geometry(handle: &WidgetHandle) -> QRect;
        fn widget_set_minimum_size(handle: &WidgetHandle, width: i32, height: i32);
        fn widget_adjust_size(handle: &WidgetHandle);
        fn widget_set_visible(handle: &WidgetHandle, visible: bool);
        fn widget_is_visible(handle: &WidgetHandle) -> bool;
        fn widget_is_hidden(handle: &WidgetHandle) -> bool;
        fn widget_set_enabled(handle: &WidgetHandle, enabled: bool);
        fn widget_is_enabled(handle: &WidgetHandle) -> bool;
        fn widget_set_tool_tip(handle: &WidgetHandle, text: &QString);
        fn widget_tool_tip(handle: &WidgetHandle) -> QString;
        fn widget_set_style_sheet(handle: &WidgetHandle, css: &QString);
        fn widget_style_sheet(handle: &WidgetHandle) -> QString;
        fn widget_set_object_name(handle: &WidgetHandle, name: &QString);
        fn widget_object_name(handle: &WidgetHandle) -> QString;
        fn widget_close(handle: &WidgetHandle);
        fn widget_save_screenshot(handle: &WidgetHandle, path: &QString) -> bool;

        fn widget_set_text(handle: &WidgetHandle, text: &QString);
        fn widget_text(handle: &WidgetHandle) -> QString;
        fn widget_set_placeholder(handle: &WidgetHandle, text: &QString);
        fn widget_placeholder(handle: &WidgetHandle) -> QString;
        fn widget_set_password_mode(handle: &WidgetHandle, enabled: bool);
        fn widget_is_password_mode(handle: &WidgetHandle) -> bool;
        fn widget_set_checked(handle: &WidgetHandle, checked: bool);
        fn widget_is_checked(handle: &WidgetHandle) -> bool;
        fn widget_click(handle: &WidgetHandle) -> bool;

        fn widget_connect_clicked(handle: &WidgetHandle, callback: Box<RufCallback>) -> bool;
        fn widget_connect_toggled(handle: &WidgetHandle, callback: Box<RufBoolCallback>) -> bool;
        fn widget_connect_text_changed(
            handle: &WidgetHandle,
            callback: Box<RufStringCallback>,
        ) -> bool;
        fn widget_connect_return_pressed(handle: &WidgetHandle, callback: Box<RufCallback>)
        -> bool;
    }
}

use cxx_qt_lib::QString;

// Die Callbacks liegen in einem RefCell: Löst eine Closure (direkt oder indirekt)
// dasselbe Signal erneut aus, wird der verschachtelte Aufruf verworfen, statt zwei
// `&mut` auf dieselbe Closure zu erzeugen.

pub struct RufCallback(RefCell<Box<dyn FnMut()>>);
pub struct RufBoolCallback(RefCell<Box<dyn FnMut(bool)>>);
pub struct RufStringCallback(RefCell<Box<dyn FnMut(String)>>);

impl RufCallback {
    pub fn new(f: impl FnMut() + 'static) -> Box<Self> {
        Box::new(Self(RefCell::new(Box::new(f))))
    }
}

impl RufBoolCallback {
    pub fn new(f: impl FnMut(bool) + 'static) -> Box<Self> {
        Box::new(Self(RefCell::new(Box::new(f))))
    }
}

impl RufStringCallback {
    pub fn new(f: impl FnMut(String) + 'static) -> Box<Self> {
        Box::new(Self(RefCell::new(Box::new(f))))
    }
}

// Eine Panik darf nicht über die C++-Grenze laufen (das bräche den Prozess ab).
// Sie wird hier abgefangen und gemeldet; der Event-Loop läuft weiter.
fn guarded(f: impl FnOnce()) {
    if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        let message = panic
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unbekannte Panik".to_owned());
        eprintln!("ruf: Panik in einem Signal-Callback abgefangen: {message}");
    }
}

fn invoke_callback(callback: &RufCallback) {
    if let Ok(mut f) = callback.0.try_borrow_mut() {
        guarded(&mut *f);
    }
}

fn invoke_bool_callback(callback: &RufBoolCallback, value: bool) {
    if let Ok(mut f) = callback.0.try_borrow_mut() {
        guarded(|| f(value));
    }
}

fn invoke_string_callback(callback: &RufStringCallback, value: &QString) {
    if let Ok(mut f) = callback.0.try_borrow_mut() {
        let value = value.to_string();
        guarded(|| f(value));
    }
}
