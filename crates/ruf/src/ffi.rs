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
        /// `QRadioButton`
        RadioButton = 6,
        /// `QGroupBox`
        GroupBox = 7,
        /// `QPlainTextEdit`
        TextArea = 8,
        /// `QComboBox`
        ComboBox = 9,
        /// `QListWidget`
        ListBox = 10,
        /// `QSpinBox`
        SpinBox = 11,
        /// `QSlider` (waagerecht)
        Slider = 12,
        /// `QProgressBar`
        ProgressBar = 13,
        /// `QTabWidget`
        TabView = 14,
        /// `QScrollArea` mit einem Container als Inhalt
        ScrollArea = 15,
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
        type RufIntCallback;
        type RufSizeCallback;
        type RufSendCallback;
        type RufCloseCallback;

        fn invoke_callback(callback: &RufCallback);
        fn invoke_bool_callback(callback: &RufBoolCallback, value: bool);
        fn invoke_string_callback(callback: &RufStringCallback, value: &QString);
        fn invoke_int_callback(callback: &RufIntCallback, value: i32);
        fn invoke_size_callback(callback: &RufSizeCallback, width: i32, height: i32);
        fn invoke_send_callback(callback: &RufSendCallback);
        fn invoke_close_callback(callback: &RufCloseCallback) -> bool;
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
        type TimerHandle;

        fn application_new(args: &Vec<String>) -> UniquePtr<Application>;
        fn application_exists() -> bool;
        fn application_exec() -> i32;
        fn application_quit();
        fn application_process_events();
        fn application_set_name(name: &QString);
        fn application_name() -> QString;
        fn application_color_scheme() -> ColorScheme;
        fn application_set_color_scheme(scheme: ColorScheme) -> bool;
        fn post_to_gui(callback: Box<RufSendCallback>) -> bool;
        fn dialog_message(parent: &WidgetHandle, kind: i32, title: &QString, text: &QString);
        fn dialog_confirm(parent: &WidgetHandle, title: &QString, text: &QString) -> bool;
        fn qt_compile_version() -> i32;
        fn qt_runtime_version() -> QString;

        fn timer_new() -> UniquePtr<TimerHandle>;
        fn timer_connect_timeout(timer: &TimerHandle, callback: Box<RufCallback>);
        fn timer_start(timer: &TimerHandle, msec: i32);
        fn timer_stop(timer: &TimerHandle);
        fn timer_is_active(timer: &TimerHandle) -> bool;
        fn timer_set_single_shot(timer: &TimerHandle, single_shot: bool);
        fn timer_is_single_shot(timer: &TimerHandle) -> bool;
        fn timer_interval(timer: &TimerHandle) -> i32;
        fn timer_single_shot(msec: i32, callback: Box<RufCallback>);

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
        fn widget_delete_later(handle: &WidgetHandle);
        fn widget_add_shortcut(
            handle: &WidgetHandle,
            sequence: &QString,
            callback: Box<RufCallback>,
        ) -> bool;
        fn widget_scroll_content(handle: &WidgetHandle) -> UniquePtr<WidgetHandle>;

        fn widget_set_monospace(handle: &WidgetHandle, monospace: bool);
        fn widget_is_monospace(handle: &WidgetHandle) -> bool;
        fn widget_add_highlight_rule(
            handle: &WidgetHandle,
            pattern: &QString,
            color: &QString,
            bold: bool,
        ) -> bool;
        fn widget_clear_highlight_rules(handle: &WidgetHandle);
        fn widget_set_marked_line(handle: &WidgetHandle, line: i32, color: &QString);
        fn widget_goto_line(handle: &WidgetHandle, line: i32);

        fn widget_set_text(handle: &WidgetHandle, text: &QString);
        fn widget_text(handle: &WidgetHandle) -> QString;
        fn widget_set_word_wrap(handle: &WidgetHandle, wrap: bool);
        fn widget_is_word_wrap(handle: &WidgetHandle) -> bool;
        fn widget_set_placeholder(handle: &WidgetHandle, text: &QString);
        fn widget_placeholder(handle: &WidgetHandle) -> QString;
        fn widget_set_password_mode(handle: &WidgetHandle, enabled: bool);
        fn widget_is_password_mode(handle: &WidgetHandle) -> bool;
        fn widget_set_checked(handle: &WidgetHandle, checked: bool);
        fn widget_is_checked(handle: &WidgetHandle) -> bool;
        fn widget_click(handle: &WidgetHandle) -> bool;
        fn widget_set_read_only(handle: &WidgetHandle, read_only: bool);
        fn widget_is_read_only(handle: &WidgetHandle) -> bool;
        fn widget_set_editable(handle: &WidgetHandle, editable: bool);
        fn widget_is_editable(handle: &WidgetHandle) -> bool;

        fn widget_add_item(handle: &WidgetHandle, text: &QString);
        fn widget_insert_item(handle: &WidgetHandle, index: i32, text: &QString);
        fn widget_remove_item(handle: &WidgetHandle, index: i32);
        fn widget_clear_items(handle: &WidgetHandle);
        fn widget_item_count(handle: &WidgetHandle) -> i32;
        fn widget_item_text(handle: &WidgetHandle, index: i32) -> QString;
        fn widget_set_item_text(handle: &WidgetHandle, index: i32, text: &QString);
        fn widget_set_current_index(handle: &WidgetHandle, index: i32);
        fn widget_current_index(handle: &WidgetHandle) -> i32;
        fn widget_tab_add(handle: &WidgetHandle, title: &QString) -> UniquePtr<WidgetHandle>;

        fn widget_set_range(handle: &WidgetHandle, minimum: i32, maximum: i32);
        fn widget_minimum(handle: &WidgetHandle) -> i32;
        fn widget_maximum(handle: &WidgetHandle) -> i32;
        fn widget_set_value(handle: &WidgetHandle, value: i32);
        fn widget_value(handle: &WidgetHandle) -> i32;
        fn widget_set_vertical(handle: &WidgetHandle, vertical: bool);
        fn widget_is_vertical(handle: &WidgetHandle) -> bool;

        fn widget_connect_clicked(handle: &WidgetHandle, callback: Box<RufCallback>) -> bool;
        fn widget_connect_toggled(handle: &WidgetHandle, callback: Box<RufBoolCallback>) -> bool;
        fn widget_connect_text_changed(
            handle: &WidgetHandle,
            callback: Box<RufStringCallback>,
        ) -> bool;
        fn widget_connect_return_pressed(handle: &WidgetHandle, callback: Box<RufCallback>)
        -> bool;
        fn widget_connect_value_changed(
            handle: &WidgetHandle,
            callback: Box<RufIntCallback>,
        ) -> bool;
        fn widget_connect_current_index_changed(
            handle: &WidgetHandle,
            callback: Box<RufIntCallback>,
        ) -> bool;
        fn widget_connect_resized(handle: &WidgetHandle, callback: Box<RufSizeCallback>) -> bool;
        fn widget_connect_close_requested(
            handle: &WidgetHandle,
            callback: Box<RufCloseCallback>,
        ) -> bool;
    }
}

use cxx_qt_lib::QString;

// Die Callbacks liegen in einem RefCell: Löst eine Closure (direkt oder indirekt)
// dasselbe Signal erneut aus, wird der verschachtelte Aufruf verworfen, statt zwei
// `&mut` auf dieselbe Closure zu erzeugen.

pub struct RufCallback(RefCell<Box<dyn FnMut()>>);
pub struct RufBoolCallback(RefCell<Box<dyn FnMut(bool)>>);
pub struct RufStringCallback(RefCell<Box<dyn FnMut(String)>>);
pub struct RufIntCallback(RefCell<Box<dyn FnMut(i32)>>);
pub struct RufSizeCallback(RefCell<Box<dyn FnMut(i32, i32)>>);
pub struct RufCloseCallback(RefCell<Box<dyn FnMut() -> bool>>);
// Wird in einem beliebigen Thread erzeugt und genau einmal im GUI-Thread ausgeführt.
pub struct RufSendCallback(RefCell<Option<Box<dyn FnOnce() + Send>>>);

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

impl RufIntCallback {
    pub fn new(f: impl FnMut(i32) + 'static) -> Box<Self> {
        Box::new(Self(RefCell::new(Box::new(f))))
    }
}

impl RufSizeCallback {
    pub fn new(f: impl FnMut(i32, i32) + 'static) -> Box<Self> {
        Box::new(Self(RefCell::new(Box::new(f))))
    }
}

impl RufCloseCallback {
    pub fn new(f: impl FnMut() -> bool + 'static) -> Box<Self> {
        Box::new(Self(RefCell::new(Box::new(f))))
    }
}

impl RufSendCallback {
    pub fn new(f: impl FnOnce() + Send + 'static) -> Box<Self> {
        Box::new(Self(RefCell::new(Some(Box::new(f)))))
    }
}

// Eine Panik darf nicht über die C++-Grenze laufen (das bräche den Prozess ab).
// Sie wird hier abgefangen und gemeldet; der Event-Loop läuft weiter.
pub(crate) fn guarded(f: impl FnOnce()) {
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

fn invoke_int_callback(callback: &RufIntCallback, value: i32) {
    if let Ok(mut f) = callback.0.try_borrow_mut() {
        guarded(|| f(value));
    }
}

fn invoke_size_callback(callback: &RufSizeCallback, width: i32, height: i32) {
    if let Ok(mut f) = callback.0.try_borrow_mut() {
        guarded(|| f(width, height));
    }
}

fn invoke_send_callback(callback: &RufSendCallback) {
    let f = callback.0.borrow_mut().take();
    if let Some(f) = f {
        guarded(f);
    }
}

fn invoke_close_callback(callback: &RufCloseCallback) -> bool {
    // Im Zweifel (Re-Entranz, Panik) schließen lassen – ein Fenster, das sich
    // nie mehr schließen lässt, wäre schlimmer.
    let Ok(mut f) = callback.0.try_borrow_mut() else {
        return true;
    };
    let mut accept = true;
    guarded(|| accept = f());
    accept
}
