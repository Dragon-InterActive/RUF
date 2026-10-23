//! Backend für Linux/BSD: GTK 4 über `gtk4-rs`.
//!
//! Absolute Positionierung über einen eigenen Container `RufFixed`
//! (Unterklasse von `GtkFixed`), der seine zugeteilte Größe an den Kern
//! meldet – damit funktionieren `on_resized` und Layouts in Fenstern,
//! Registerkarten-Seiten und Scrollbereichen.
//!
//! Hinweis: GTK-Steuerelemente werden nie kleiner als ihre natürliche
//! Größe; `set_size` wirkt als Mindestgröße. Fensterpositionen kann GTK 4
//! nicht setzen – das Modul `position` erledigt es nativ (X11, Win32,
//! macOS); unter Wayland ist es nicht möglich.

// `GtkComboBoxText` ist seit GTK 4.10 als veraltet markiert, hat aber noch
// keinen Ersatz mit Eingabefeld (`GtkDropDown` kann keinen freien Text).
// Es bleibt bis GTK 5 verfügbar.
#![allow(deprecated)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib, pango};
use gtk4 as gtk;

use super::{Backend, MessageKind};
use crate::app::ColorScheme;
use crate::canvas::{KeyEvent, Modifiers, MouseAction, MouseEvent, Shape};
use crate::core::{self, Id, Notify, WidgetKind};
use crate::dialog::FileFilter;
use crate::geometry::Point;
use crate::geometry::{Rect, Size};
use crate::menu::{Menu, MenuBar, MenuEntry};
use crate::style::{Color, Style};
use crate::text::{Key, KeySpec, Span, looks_like_markup, markup_to_pango};
use crate::tree::TreeRow;

// --- RufFixed: Container mit absoluter Platzierung und Größenmeldung -----------------

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct RufFixed {
        pub id: Cell<Id>,
        pub last: Cell<(i32, i32)>,
        /// Kinder mit Position, in Zeichenreihenfolge.
        pub children: RefCell<Vec<(gtk::Widget, i32, i32)>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RufFixed {
        const NAME: &'static str = "RufFixed";
        type Type = super::RufFixed;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for RufFixed {
        fn dispose(&self) {
            for (child, _, _) in self.children.take() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for RufFixed {
        // Der Inhalt bestimmt nicht die Mindestgröße – Fenster lassen sich
        // kleiner ziehen als ihr Inhalt (wie bei klassischen Toolkits).
        // Eine Mindestgröße per `set_size_request` gilt trotzdem.
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (0, 0, -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            for (child, x, y) in self.children.borrow().iter() {
                if !child.should_layout() {
                    continue;
                }
                // Angeforderte Größe, aber nie kleiner als das Minimum des Toolkits.
                let (request_w, request_h) = child.size_request();
                let (min_w, nat_w, _, _) = child.measure(gtk::Orientation::Horizontal, -1);
                let w = if request_w > 0 {
                    min_w.max(request_w)
                } else {
                    nat_w
                };
                let (min_h, nat_h, _, _) = child.measure(gtk::Orientation::Vertical, w);
                let h = if request_h > 0 {
                    min_h.max(request_h)
                } else {
                    nat_h
                };
                child.size_allocate(&gtk::Allocation::new(*x, *y, w, h), -1);
            }
            if self.last.replace((width, height)) != (width, height) {
                // Nicht während der Größenverteilung umbauen: erst danach melden.
                let id = self.id.get();
                glib::idle_add_local_once(move || {
                    core::notify(id, Notify::Resized(Size::new(width, height)));
                });
            }
        }
    }
}

glib::wrapper! {
    pub struct RufFixed(ObjectSubclass<imp::RufFixed>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RufFixed {
    fn new(id: Id) -> Self {
        let fixed: Self = glib::Object::new();
        fixed.imp().id.set(id);
        fixed
    }

    fn put(&self, child: &impl IsA<gtk::Widget>, x: i32, y: i32) {
        child.set_parent(self);
        self.imp()
            .children
            .borrow_mut()
            .push((child.clone().upcast(), x, y));
        self.queue_allocate();
    }

    fn move_(&self, child: &impl IsA<gtk::Widget>, x: i32, y: i32) {
        let child = child.as_ref();
        if let Some(entry) = self
            .imp()
            .children
            .borrow_mut()
            .iter_mut()
            .find(|(c, _, _)| c == child)
        {
            entry.1 = x;
            entry.2 = y;
        }
        self.queue_allocate();
    }

    fn remove(&self, child: &impl IsA<gtk::Widget>) {
        let child = child.as_ref();
        self.imp()
            .children
            .borrow_mut()
            .retain(|(c, _, _)| c != child);
        child.unparent();
        self.queue_allocate();
    }

    fn child_position(&self, child: &impl IsA<gtk::Widget>) -> (i32, i32) {
        let child = child.as_ref();
        self.imp()
            .children
            .borrow()
            .iter()
            .find(|(c, _, _)| c == child)
            .map(|(_, x, y)| (*x, *y))
            .unwrap_or_default()
    }
}

/// Der `RufFixed`, in dem ein Widget liegt.
fn parent_fixed(widget: &gtk::Widget) -> Option<RufFixed> {
    widget.parent().and_then(|p| p.downcast::<RufFixed>().ok())
}

// --- Native Verweise --------------------------------------------------------------------

/// Typisierter Zugriff auf das eigentliche Steuerelement.
#[derive(Clone)]
pub enum Inner {
    Window(gtk::Window),
    Container,
    Group(gtk::Frame),
    Button(gtk::Button),
    Label(gtk::Label),
    Entry(gtk::Entry),
    Check(gtk::CheckButton),
    Text {
        view: gtk::TextView,
        placeholder: gtk::Label,
    },
    Combo(gtk::ComboBoxText),
    List(gtk::ListBox),
    Spin(gtk::SpinButton),
    Scale(gtk::Scale),
    Progress {
        bar: gtk::ProgressBar,
        state: Rc<ProgressState>,
    },
    Tabs(gtk::Notebook),
    Scroll {
        content: RufFixed,
    },
    Canvas {
        area: gtk::DrawingArea,
        shapes: Rc<RefCell<Vec<Shape>>>,
    },
    Tree(gtk::ListBox),
}

#[derive(Default)]
pub struct ProgressState {
    range: Cell<(i32, i32)>,
    value: Cell<i32>,
    pulse: RefCell<Option<glib::SourceId>>,
}

/// Ein GTK-Steuerelement samt Container für Kinder.
#[derive(Clone)]
pub struct GtkNative {
    id: Id,
    /// Das, was im Elternteil platziert wird (bzw. das Fenster).
    outer: gtk::Widget,
    /// Hier kommen Kinder hinein (Fenster, Panel, Gruppe, Seite, Scroll-Inhalt).
    fixed: Option<RufFixed>,
    inner: Inner,
}

// --- Globaler Zustand des Backends -----------------------------------------------------

thread_local! {
    static LOOPS: RefCell<Vec<glib::MainLoop>> = const { RefCell::new(Vec::new()) };
    static TIMERS: RefCell<HashMap<u64, glib::SourceId>> = RefCell::new(HashMap::new());
    static STYLES: RefCell<(Option<gtk::CssProvider>, HashMap<Id, String>)> = RefCell::new((None, HashMap::new()));
    /// Erstes Optionsfeld je Container – GTK gruppiert Optionsfelder ausdrücklich.
    /// Tastenkürzel der Menüleiste je Fenster (werden beim Ersetzen entfernt).
    static MENU_SHORTCUTS: RefCell<HashMap<Id, gtk::ShortcutController>> = RefCell::new(HashMap::new());
    /// Offene Kontextmenüs – sie hängen an einem Widget und müssen vor ihm gehen.
    static POPOVERS: RefCell<Vec<glib::WeakRef<gtk::PopoverMenu>>> = const { RefCell::new(Vec::new()) };
    /// Vorgemerkte Fensterpositionen (angewendet, sobald das Fenster erscheint).
    static PENDING_POSITIONS: RefCell<HashMap<Id, (i32, i32)>> = RefCell::new(HashMap::new());
    static RADIO_GROUPS: RefCell<Vec<(glib::WeakRef<gtk::Widget>, glib::WeakRef<gtk::CheckButton>)>> = const { RefCell::new(Vec::new()) };
}

pub struct Gtk;

fn notify(id: Id, event: Notify) {
    core::notify(id, event);
}

fn place(child: &gtk::Widget, parent: Option<&GtkNative>) {
    if let Some(fixed) = parent.and_then(|p| p.fixed.as_ref()) {
        fixed.put(child, 0, 0);
    }
}

fn window_of(native: &GtkNative) -> Option<gtk::Window> {
    match &native.inner {
        Inner::Window(w) => Some(w.clone()),
        _ => native
            .outer
            .root()
            .and_then(|r| r.downcast::<gtk::Window>().ok()),
    }
}

/// Lässt GTK arbeiten, bis `done()` gilt (höchstens `limit`).
fn pump_until(limit: Duration, done: impl Fn() -> bool) {
    let start = std::time::Instant::now();
    let context = glib::MainContext::default();
    while !done() && start.elapsed() < limit {
        context.iteration(false);
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn radio_group_for(parent: &gtk::Widget, radio: &gtk::CheckButton) {
    RADIO_GROUPS.with(|groups| {
        let mut groups = groups.borrow_mut();
        groups.retain(|(k, v)| k.upgrade().is_some() && v.upgrade().is_some());
        let first = groups
            .iter()
            .find(|(k, _)| k.upgrade().as_ref() == Some(parent))
            .and_then(|(_, v)| v.upgrade());
        match first {
            Some(first) => radio.set_group(Some(&first)),
            None => groups.push((parent.downgrade(), radio.downgrade())),
        }
    });
}

fn accelerator(key: &KeySpec) -> String {
    let mut accel = String::new();
    // <Primary> ist Strg unter Linux/Windows und die Befehlstaste unter
    // macOS – so gilt `Ctrl+…` auf dem Mac als `Cmd+…`.
    if key.ctrl {
        accel.push_str("<Primary>");
    }
    if key.shift {
        accel.push_str("<Shift>");
    }
    if key.alt {
        accel.push_str("<Alt>");
    }
    if key.meta {
        // Befehlstaste (macOS) bzw. Super-/Windows-Taste.
        accel.push_str(if cfg!(target_os = "macos") {
            "<Meta>"
        } else {
            "<Super>"
        });
    }
    accel.push_str(&match &key.key {
        Key::Char(c) => c.to_string(),
        Key::F(n) => format!("F{n}"),
        Key::Enter => "Return".into(),
        Key::Escape => "Escape".into(),
        Key::Tab => "Tab".into(),
        Key::Delete => "Delete".into(),
        Key::Insert => "Insert".into(),
        Key::Home => "Home".into(),
        Key::End => "End".into(),
        Key::PageUp => "Page_Up".into(),
        Key::PageDown => "Page_Down".into(),
        Key::Left => "Left".into(),
        Key::Right => "Right".into(),
        Key::Up => "Up".into(),
        Key::Down => "Down".into(),
        Key::Backspace => "BackSpace".into(),
        Key::Space => "space".into(),
    });
    accel
}

fn update_progress(bar: &gtk::ProgressBar, state: &ProgressState) {
    let (min, max) = state.range.get();
    let busy = min == 0 && max == 0;
    let mut pulse = state.pulse.borrow_mut();
    if busy {
        if pulse.is_none() {
            let weak = bar.downgrade();
            *pulse = Some(glib::timeout_add_local(
                Duration::from_millis(100),
                move || match weak.upgrade() {
                    Some(bar) => {
                        bar.pulse();
                        glib::ControlFlow::Continue
                    }
                    None => glib::ControlFlow::Break,
                },
            ));
        }
    } else {
        if let Some(id) = pulse.take() {
            id.remove();
        }
        let span = f64::from(max - min).max(1.0);
        bar.set_fraction(f64::from(state.value.get() - min) / span);
    }
}

fn text_buffer(native: &GtkNative) -> Option<(gtk::TextView, gtk::TextBuffer)> {
    match &native.inner {
        Inner::Text { view, .. } => Some((view.clone(), view.buffer())),
        _ => None,
    }
}

fn combo_entry(combo: &gtk::ComboBoxText) -> Option<gtk::Entry> {
    combo.child().and_then(|c| c.downcast::<gtk::Entry>().ok())
}

fn new_combo(id: Id, editable: bool) -> gtk::ComboBoxText {
    let combo = if editable {
        gtk::ComboBoxText::with_entry()
    } else {
        gtk::ComboBoxText::new()
    };
    combo.connect_changed(move |_| notify(id, Notify::Changed));
    combo
}

fn color_css(color: Color) -> String {
    color.to_string()
}

fn set_tag(
    buffer: &gtk::TextBuffer,
    name: &str,
    setup: impl FnOnce(&gtk::TextTag),
) -> gtk::TextTag {
    let table = buffer.tag_table();
    if let Some(tag) = table.lookup(name) {
        return tag;
    }
    let tag = gtk::TextTag::new(Some(name));
    setup(&tag);
    table.add(&tag);
    tag
}

impl Backend for Gtk {
    type Native = GtkNative;
    const NAME: &'static str = "GTK";

    fn version() -> String {
        format!(
            "{}.{}.{}",
            gtk::major_version(),
            gtk::minor_version(),
            gtk::micro_version()
        )
    }

    fn init() {
        if let Err(e) = gtk::init() {
            panic!(
                "ruf: GTK konnte nicht initialisiert werden ({e}). Läuft eine grafische \
                 Sitzung? Ohne Bildschirm z. B. mit `xvfb-run` starten."
            );
        }
    }

    fn run() {
        let main_loop = glib::MainLoop::new(None, false);
        LOOPS.with(|l| l.borrow_mut().push(main_loop.clone()));
        main_loop.run();
        LOOPS.with(|l| l.borrow_mut().pop());
    }

    fn quit() {
        LOOPS.with(|l| {
            if let Some(main_loop) = l.borrow().last() {
                main_loop.quit();
            }
        });
    }

    fn process_events() {
        // GTK verteilt Größen über seine Frame-Clock (~16 ms) – asynchron.
        // Deshalb so lange abarbeiten, bis eine Weile nichts mehr ansteht,
        // damit Layout und Größenmeldungen danach aktuell sind.
        let context = glib::MainContext::default();
        let start = std::time::Instant::now();
        let mut quiet = 0;
        while quiet < 3 && start.elapsed() < Duration::from_millis(250) {
            let mut busy = false;
            while context.iteration(false) {
                busy = true;
            }
            quiet = if busy { 0 } else { quiet + 1 };
            if quiet > 0 {
                std::thread::sleep(Duration::from_millis(8));
            }
        }
    }

    fn post(f: Box<dyn FnOnce() + Send>) -> bool {
        // Nicht `MainContext::invoke`: das führt `f` unter Umständen direkt im
        // aufrufenden Thread aus. Eine Idle-Quelle im Standardkontext läuft
        // garantiert im GUI-Thread.
        glib::idle_add_once(f);
        true
    }

    fn timer_start(id: u64, msec: u32) {
        let source =
            glib::timeout_add_local(Duration::from_millis(u64::from(msec.max(1))), move || {
                core::timer_fired(id);
                // Wurde der Timer im Callback gestoppt oder neu gestartet, gehört
                // diese Quelle nicht mehr dazu.
                glib::ControlFlow::Continue
            });
        TIMERS.with(|t| {
            if let Some(old) = t.borrow_mut().insert(id, source) {
                old.remove();
            }
        });
    }

    fn timer_stop(id: u64) {
        if let Some(source) = TIMERS.with(|t| t.borrow_mut().remove(&id)) {
            source.remove();
        }
    }

    fn color_scheme() -> ColorScheme {
        let Some(settings) = gtk::Settings::default() else {
            return ColorScheme::Unknown;
        };
        let theme = settings
            .gtk_theme_name()
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        if settings.is_gtk_application_prefer_dark_theme() || theme.contains("dark") {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        }
    }

    fn set_color_scheme(scheme: ColorScheme) -> bool {
        match gtk::Settings::default() {
            Some(settings) => {
                settings.set_gtk_application_prefer_dark_theme(scheme == ColorScheme::Dark);
                true
            }
            None => false,
        }
    }

    fn message(parent: Option<&GtkNative>, kind: MessageKind, title: &str, text: &str) {
        let _ = kind; // GTK-Dialoge kennen keine Symbolarten.
        let dialog = gtk::AlertDialog::builder()
            .modal(true)
            .message(title)
            .detail(text)
            .buttons(["OK"])
            .build();
        let main_loop = glib::MainLoop::new(None, false);
        let l = main_loop.clone();
        let window = parent.and_then(window_of);
        dialog.choose(window.as_ref(), None::<&gtk::gio::Cancellable>, move |_| {
            l.quit()
        });
        main_loop.run();
    }

    fn confirm(parent: Option<&GtkNative>, title: &str, text: &str) -> bool {
        let (no, yes) = if crate::dialog::german() {
            ("Nein", "Ja")
        } else {
            ("No", "Yes")
        };
        let dialog = gtk::AlertDialog::builder()
            .modal(true)
            .message(title)
            .detail(text)
            .buttons([no, yes])
            .cancel_button(0)
            .default_button(1)
            .build();
        let main_loop = glib::MainLoop::new(None, false);
        let answer = Rc::new(Cell::new(false));
        let (l, a) = (main_loop.clone(), answer.clone());
        let window = parent.and_then(window_of);
        dialog.choose(
            window.as_ref(),
            None::<&gtk::gio::Cancellable>,
            move |result| {
                a.set(result == Ok(1));
                l.quit();
            },
        );
        main_loop.run();
        answer.get()
    }

    fn create(id: Id, kind: WidgetKind, parent: Option<&GtkNative>) -> GtkNative {
        let (outer, fixed, inner): (gtk::Widget, Option<RufFixed>, Inner) = match kind {
            WidgetKind::Window => {
                let window = gtk::Window::new();
                let content = RufFixed::new(id);
                window.set_child(Some(&content));
                window.connect_close_request(move |_| {
                    core::close_requested(id);
                    // Der Kern blendet das Fenster selbst aus (statt es zu zerstören).
                    glib::Propagation::Stop
                });
                window.connect_map(move |w| {
                    if let Some((x, y)) = PENDING_POSITIONS.with(|p| p.borrow_mut().remove(&id)) {
                        let w = w.clone();
                        glib::idle_add_local_once(move || {
                            position::set(&w, x, y);
                        });
                    }
                });
                (
                    window.clone().upcast(),
                    Some(content),
                    Inner::Window(window),
                )
            }
            WidgetKind::Container => {
                let fixed = RufFixed::new(id);
                (fixed.clone().upcast(), Some(fixed), Inner::Container)
            }
            WidgetKind::GroupBox => {
                let fixed = RufFixed::new(id);
                let frame = gtk::Frame::new(None);
                frame.set_can_target(false);
                fixed.put(&frame, 0, 0);
                (fixed.clone().upcast(), Some(fixed), Inner::Group(frame))
            }
            WidgetKind::PushButton => {
                let button = gtk::Button::new();
                button.connect_clicked(move |_| notify(id, Notify::Clicked));
                (button.clone().upcast(), None, Inner::Button(button))
            }
            WidgetKind::Label => {
                let label = gtk::Label::new(None);
                label.set_xalign(0.0);
                (label.clone().upcast(), None, Inner::Label(label))
            }
            WidgetKind::LineEdit => {
                let entry = gtk::Entry::new();
                entry.connect_changed(move |_| notify(id, Notify::Changed));
                entry.connect_activate(move |_| notify(id, Notify::Activated));
                (entry.clone().upcast(), None, Inner::Entry(entry))
            }
            WidgetKind::CheckBox | WidgetKind::RadioButton => {
                let check = gtk::CheckButton::new();
                if kind == WidgetKind::RadioButton
                    && let Some(f) = parent.and_then(|p| p.fixed.as_ref())
                {
                    radio_group_for(f.upcast_ref(), &check);
                }
                check.connect_toggled(move |_| notify(id, Notify::Clicked));
                (check.clone().upcast(), None, Inner::Check(check))
            }
            WidgetKind::TextArea => {
                let view = gtk::TextView::new();
                view.set_wrap_mode(gtk::WrapMode::None);
                let scroll = gtk::ScrolledWindow::new();
                scroll.set_child(Some(&view));
                scroll.set_has_frame(true);
                let placeholder = gtk::Label::new(None);
                placeholder.set_halign(gtk::Align::Start);
                placeholder.set_valign(gtk::Align::Start);
                placeholder.set_margin_start(6);
                placeholder.set_margin_top(4);
                placeholder.set_can_target(false);
                placeholder.add_css_class("dim-label");
                let overlay = gtk::Overlay::new();
                overlay.set_child(Some(&scroll));
                overlay.add_overlay(&placeholder);
                let p = placeholder.clone();
                view.buffer().connect_changed(move |buffer| {
                    p.set_visible(buffer.char_count() == 0);
                    notify(id, Notify::Changed);
                });
                (overlay.upcast(), None, Inner::Text { view, placeholder })
            }
            WidgetKind::ComboBox => {
                let combo = new_combo(id, false);
                (combo.clone().upcast(), None, Inner::Combo(combo))
            }
            WidgetKind::Canvas => {
                let area = gtk::DrawingArea::new();
                let shapes: Rc<RefCell<Vec<Shape>>> = Rc::default();
                let s = shapes.clone();
                area.set_draw_func(move |area, cr, _, _| draw_shapes(area, cr, &s.borrow()));
                area.set_focusable(true);
                area.set_can_focus(true);
                connect_canvas_input(&area, id);
                (area.clone().upcast(), None, Inner::Canvas { area, shapes })
            }
            WidgetKind::TreeView => {
                let list = gtk::ListBox::new();
                list.set_selection_mode(gtk::SelectionMode::Single);
                list.connect_row_selected(move |_, _| notify(id, Notify::Changed));
                list.connect_row_activated(move |_, _| notify(id, Notify::Activated));
                let scroll = gtk::ScrolledWindow::new();
                scroll.set_child(Some(&list));
                scroll.set_has_frame(true);
                (scroll.upcast(), None, Inner::Tree(list))
            }
            WidgetKind::ListBox => {
                let list = gtk::ListBox::new();
                list.set_selection_mode(gtk::SelectionMode::Single);
                list.connect_row_selected(move |_, _| notify(id, Notify::Changed));
                let scroll = gtk::ScrolledWindow::new();
                scroll.set_child(Some(&list));
                scroll.set_has_frame(true);
                (scroll.upcast(), None, Inner::List(list))
            }
            WidgetKind::SpinBox => {
                let spin = gtk::SpinButton::with_range(0.0, 99.0, 1.0);
                spin.set_digits(0);
                spin.connect_value_changed(move |_| notify(id, Notify::Changed));
                (spin.clone().upcast(), None, Inner::Spin(spin))
            }
            WidgetKind::Slider => {
                let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 99.0, 1.0);
                scale.set_draw_value(false);
                scale.set_round_digits(0);
                scale.connect_value_changed(move |_| notify(id, Notify::Changed));
                (scale.clone().upcast(), None, Inner::Scale(scale))
            }
            WidgetKind::ProgressBar => {
                let bar = gtk::ProgressBar::new();
                let state = Rc::new(ProgressState::default());
                state.range.set((0, 100));
                (bar.clone().upcast(), None, Inner::Progress { bar, state })
            }
            WidgetKind::TabView => {
                let notebook = gtk::Notebook::new();
                notebook.set_scrollable(true);
                notebook
                    .connect_notify_local(Some("page"), move |_, _| notify(id, Notify::Changed));
                (notebook.clone().upcast(), None, Inner::Tabs(notebook))
            }
            WidgetKind::ScrollArea => {
                let window = gtk::ScrolledWindow::new();
                window.set_has_frame(true);
                // Die Nummer des Inhalts setzt `scroll_content`.
                let content = RufFixed::new(0);
                window.set_child(Some(&content));
                (window.upcast(), None, Inner::Scroll { content })
            }
        };
        if kind != WidgetKind::Window {
            place(&outer, parent);
        }
        GtkNative {
            id,
            outer,
            fixed,
            inner,
        }
    }

    fn destroy(native: &GtkNative, kind: WidgetKind) {
        STYLES.with(|s| s.borrow_mut().1.remove(&native.id));
        // Kontextmenüs an diesem Widget oder seinen Kindern zuerst lösen.
        POPOVERS.with(|p| {
            p.borrow_mut().retain(|weak| {
                let Some(popover) = weak.upgrade() else {
                    return false;
                };
                let inside = popover
                    .parent()
                    .is_some_and(|a| a == native.outer || a.is_ancestor(&native.outer));
                if inside {
                    popover.unparent();
                }
                !inside
            });
        });
        if let Inner::Progress { state, .. } = &native.inner
            && let Some(pulse) = state.pulse.borrow_mut().take()
        {
            pulse.remove();
        }
        match &native.inner {
            Inner::Window(window) => window.destroy(),
            _ => {
                let _ = kind;
                match parent_fixed(&native.outer) {
                    Some(fixed) => fixed.remove(&native.outer),
                    None if native.outer.parent().is_some() => native.outer.unparent(),
                    None => {}
                }
            }
        }
    }

    fn set_parent(native: &GtkNative, kind: WidgetKind, parent: &GtkNative) {
        if kind == WidgetKind::Window {
            if let (Inner::Window(w), Some(p)) = (&native.inner, window_of(parent)) {
                w.set_transient_for(Some(&p));
            }
            return;
        }
        if let Some(fixed) = parent_fixed(&native.outer) {
            fixed.remove(&native.outer);
        }
        place(&native.outer, Some(parent));
    }

    fn class_name(native: &GtkNative, _kind: WidgetKind) -> String {
        let widget: gtk::Widget = match &native.inner {
            Inner::Window(w) => w.clone().upcast(),
            Inner::Group(f) => f.clone().upcast(),
            Inner::Text { view, .. } => view.clone().upcast(),
            Inner::List(l) | Inner::Tree(l) => l.clone().upcast(),
            _ => native.outer.clone(),
        };
        widget.type_().name().to_owned()
    }

    fn set_geometry(native: &GtkNative, kind: WidgetKind, rect: Rect) {
        let (w, h) = (rect.width.max(0), rect.height.max(0));
        match &native.inner {
            Inner::Window(window) => window.set_default_size(w.max(1), h.max(1)),
            _ => {
                native.outer.set_size_request(w, h);
                if let Some(fixed) = parent_fixed(&native.outer) {
                    fixed.move_(&native.outer, rect.x, rect.y);
                }
                if let Inner::Group(frame) = &native.inner {
                    frame.set_size_request(w, h);
                }
            }
        }
        let _ = kind;
    }

    fn preferred_size(native: &GtkNative, _kind: WidgetKind) -> Size {
        let widget = &native.outer;
        let (_, width, _, _) = widget.measure(gtk::Orientation::Horizontal, -1);
        let (_, height, _, _) = widget.measure(gtk::Orientation::Vertical, width);
        Size::new(width, height)
    }

    fn set_visible(native: &GtkNative, _kind: WidgetKind, visible: bool) {
        match &native.inner {
            Inner::Window(window) if visible => window.present(),
            _ => native.outer.set_visible(visible),
        }
    }

    fn set_enabled(native: &GtkNative, _kind: WidgetKind, enabled: bool) {
        native.outer.set_sensitive(enabled);
    }

    fn set_tool_tip(native: &GtkNative, _kind: WidgetKind, text: &str) {
        native
            .outer
            .set_tooltip_text((!text.is_empty()).then_some(text));
    }

    fn set_style(native: &GtkNative, _kind: WidgetKind, style: &Style) {
        let class = format!("ruf-{}", native.id);
        let mut css = String::new();
        if let Some(c) = style.color {
            css.push_str(&format!("color: {};", color_css(c)));
        }
        if let Some(c) = style.background {
            css.push_str(&format!(
                "background-color: {}; background-image: none;",
                color_css(c)
            ));
        }
        if style.bold {
            css.push_str("font-weight: bold;");
        }
        let target: gtk::Widget = match &native.inner {
            Inner::Window(_) | Inner::Group(_) => native
                .fixed
                .clone()
                .map(|f| f.upcast())
                .unwrap_or(native.outer.clone()),
            Inner::Text { view, .. } => view.clone().upcast(),
            _ => native.outer.clone(),
        };
        target.add_css_class(&class);
        STYLES.with(|s| {
            let mut s = s.borrow_mut();
            if css.is_empty() {
                s.1.remove(&native.id);
            } else {
                s.1.insert(native.id, css);
            }
            let all: String =
                s.1.iter()
                    .map(|(id, css)| format!(".ruf-{id}, .ruf-{id} text {{ {css} }}\n"))
                    .collect();
            let provider = s.0.get_or_insert_with(|| {
                let provider = gtk::CssProvider::new();
                if let Some(display) = gtk::gdk::Display::default() {
                    gtk::style_context_add_provider_for_display(
                        &display,
                        &provider,
                        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                    );
                }
                provider
            });
            provider.load_from_string(&all);
        });
    }

    fn screenshot(native: &GtkNative, _kind: WidgetKind, path: &Path) -> bool {
        // Unsichtbare Fenster kurz anzeigen – GTK zeichnet nur, was abgebildet ist.
        let window = window_of(native);
        let temporarily = window.as_ref().is_some_and(|w| !w.is_visible());
        if let Some(w) = &window {
            if temporarily {
                w.present();
            }
            pump_until(Duration::from_secs(3), || {
                w.is_mapped() && native.outer.width() > 0
            });
            Gtk::process_events();
        }
        let widget = &native.outer;
        let (width, height) = (widget.width(), widget.height());
        let ok = (|| {
            if width <= 0 || height <= 0 {
                return false;
            }
            let paintable = gtk::WidgetPaintable::new(Some(widget));
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, f64::from(width), f64::from(height));
            let Some(node) = snapshot.to_node() else {
                return false;
            };
            let Some(renderer) = widget.native().and_then(|n| n.renderer()) else {
                return false;
            };
            let texture = renderer.render_texture(&node, None);
            texture.save_to_png(path).is_ok()
        })();
        if temporarily && let Some(w) = &window {
            w.set_visible(false);
        }
        ok
    }

    fn add_shortcut(window: &GtkNative, key: &KeySpec, shortcut: u64) -> bool {
        let Inner::Window(w) = &window.inner else {
            return false;
        };
        let Some(trigger) = gtk::ShortcutTrigger::parse_string(&accelerator(key)) else {
            return false;
        };
        let action = gtk::CallbackAction::new(move |_, _| {
            core::shortcut_fired(shortcut);
            glib::Propagation::Stop
        });
        let controller = gtk::ShortcutController::new();
        controller.set_scope(gtk::ShortcutScope::Global);
        controller.add_shortcut(gtk::Shortcut::new(Some(trigger), Some(action)));
        w.add_controller(controller);
        true
    }

    fn set_text(native: &GtkNative, _kind: WidgetKind, text: &str) {
        match &native.inner {
            Inner::Window(w) => w.set_title(Some(text)),
            Inner::Group(f) => f.set_label((!text.is_empty()).then_some(text)),
            Inner::Button(b) => b.set_label(text),
            Inner::Check(c) => c.set_label(Some(text)),
            Inner::Label(l) => {
                if looks_like_markup(text) {
                    l.set_markup(&markup_to_pango(text));
                } else {
                    l.set_text(text);
                }
            }
            Inner::Entry(e) => e.set_text(text),
            Inner::Text { view, .. } => view.buffer().set_text(text),
            Inner::Combo(c) => {
                if let Some(entry) = combo_entry(c) {
                    entry.set_text(text);
                }
            }
            _ => {}
        }
    }

    fn text(native: &GtkNative, _kind: WidgetKind) -> String {
        match &native.inner {
            Inner::Entry(e) => e.text().to_string(),
            Inner::Text { view, .. } => {
                let buffer = view.buffer();
                buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), false)
                    .to_string()
            }
            Inner::Combo(c) => match combo_entry(c) {
                Some(entry) => entry.text().to_string(),
                None => c.active_text().map(|t| t.to_string()).unwrap_or_default(),
            },
            _ => String::new(),
        }
    }

    fn set_placeholder(native: &GtkNative, _kind: WidgetKind, text: &str) {
        match &native.inner {
            Inner::Entry(e) => e.set_placeholder_text(Some(text)),
            Inner::Text { placeholder, view } => {
                placeholder.set_text(text);
                placeholder.set_visible(view.buffer().char_count() == 0);
            }
            Inner::Combo(c) => {
                if let Some(entry) = combo_entry(c) {
                    entry.set_placeholder_text(Some(text));
                }
            }
            _ => {}
        }
    }

    fn set_password(native: &GtkNative, enabled: bool) {
        if let Inner::Entry(e) = &native.inner {
            e.set_visibility(!enabled);
        }
    }

    fn set_read_only(native: &GtkNative, _kind: WidgetKind, read_only: bool) {
        match &native.inner {
            Inner::Entry(e) => e.set_editable(!read_only),
            Inner::Text { view, .. } => view.set_editable(!read_only),
            Inner::Spin(s) => s.set_editable(!read_only),
            _ => {}
        }
    }

    fn set_word_wrap(native: &GtkNative, wrap: bool) {
        if let Inner::Label(l) = &native.inner {
            l.set_wrap(wrap);
            l.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            l.set_yalign(if wrap { 0.0 } else { 0.5 });
        }
    }

    fn set_checked(native: &GtkNative, _kind: WidgetKind, checked: bool) {
        if let Inner::Check(c) = &native.inner {
            c.set_active(checked);
        }
    }

    fn is_checked(native: &GtkNative, _kind: WidgetKind) -> bool {
        match &native.inner {
            Inner::Check(c) => c.is_active(),
            _ => false,
        }
    }

    fn set_items(native: &GtkNative, _kind: WidgetKind, items: &[String]) {
        match &native.inner {
            Inner::Combo(c) => {
                c.remove_all();
                for item in items {
                    c.append_text(item);
                }
            }
            Inner::List(list) => {
                list.remove_all();
                for item in items {
                    let label = gtk::Label::new(Some(item));
                    label.set_xalign(0.0);
                    label.set_margin_start(4);
                    label.set_margin_end(4);
                    list.append(&label);
                }
            }
            _ => {}
        }
    }

    fn set_current_index(native: &GtkNative, _kind: WidgetKind, index: i32) {
        match &native.inner {
            Inner::Combo(c) => c.set_active(u32::try_from(index).ok()),
            Inner::List(list) | Inner::Tree(list) => match list.row_at_index(index) {
                Some(row) if index >= 0 => list.select_row(Some(&row)),
                _ => list.unselect_all(),
            },
            Inner::Tabs(n) => n.set_current_page(u32::try_from(index).ok()),
            _ => {}
        }
    }

    fn current_index(native: &GtkNative, _kind: WidgetKind) -> i32 {
        match &native.inner {
            Inner::Combo(c) => c.active().map(|i| i as i32).unwrap_or(-1),
            Inner::List(list) | Inner::Tree(list) => {
                list.selected_row().map(|r| r.index()).unwrap_or(-1)
            }
            Inner::Tabs(n) => n.current_page().map(|i| i as i32).unwrap_or(-1),
            _ => -1,
        }
    }

    fn set_editable(native: &GtkNative, editable: bool) -> Option<GtkNative> {
        let Inner::Combo(old) = &native.inner else {
            return None;
        };
        if combo_entry(old).is_some() == editable {
            return None;
        }
        // GTK kann eine Combobox nicht nachträglich editierbar machen: ersetzen.
        let combo = new_combo(native.id, editable);
        if let Some(fixed) = parent_fixed(old.upcast_ref()) {
            let (x, y) = fixed.child_position(old);
            fixed.remove(old);
            fixed.put(&combo, x, y);
        }
        combo.set_tooltip_text(old.tooltip_text().as_deref());
        combo.set_sensitive(old.is_sensitive());
        combo.set_visible(old.is_visible());
        for class in old.css_classes() {
            combo.add_css_class(&class);
        }
        Some(GtkNative {
            id: native.id,
            outer: combo.clone().upcast(),
            fixed: None,
            inner: Inner::Combo(combo),
        })
    }

    fn tab_add(native: &GtkNative, page_id: Id, title: &str) -> GtkNative {
        let page = RufFixed::new(page_id);
        if let Inner::Tabs(n) = &native.inner {
            n.append_page(&page, Some(&gtk::Label::new(Some(title))));
        }
        GtkNative {
            id: page_id,
            outer: page.clone().upcast(),
            fixed: Some(page),
            inner: Inner::Container,
        }
    }

    fn tab_remove(native: &GtkNative, index: i32) {
        if let Inner::Tabs(n) = &native.inner {
            n.remove_page(u32::try_from(index).ok());
        }
    }

    fn tab_set_title(native: &GtkNative, index: i32, title: &str) {
        if let Inner::Tabs(n) = &native.inner
            && let Some(page) = n.nth_page(u32::try_from(index).ok())
        {
            n.set_tab_label_text(&page, title);
        }
    }

    fn set_range(native: &GtkNative, _kind: WidgetKind, minimum: i32, maximum: i32) {
        match &native.inner {
            Inner::Spin(s) => s.set_range(f64::from(minimum), f64::from(maximum)),
            Inner::Scale(s) => s.set_range(f64::from(minimum), f64::from(maximum.max(minimum + 1))),
            Inner::Progress { bar, state } => {
                state.range.set((minimum, maximum));
                update_progress(bar, state);
            }
            _ => {}
        }
    }

    fn set_value(native: &GtkNative, _kind: WidgetKind, value: i32) {
        match &native.inner {
            Inner::Spin(s) => s.set_value(f64::from(value)),
            Inner::Scale(s) => s.set_value(f64::from(value)),
            Inner::Progress { bar, state } => {
                state.value.set(value);
                update_progress(bar, state);
            }
            _ => {}
        }
    }

    fn value(native: &GtkNative, _kind: WidgetKind) -> i32 {
        match &native.inner {
            Inner::Spin(s) => s.value_as_int(),
            Inner::Scale(s) => s.value().round() as i32,
            Inner::Progress { state, .. } => state.value.get(),
            _ => 0,
        }
    }

    fn set_vertical(native: &GtkNative, _kind: WidgetKind, vertical: bool) {
        let orientation = if vertical {
            gtk::Orientation::Vertical
        } else {
            gtk::Orientation::Horizontal
        };
        match &native.inner {
            Inner::Scale(s) => {
                s.set_orientation(orientation);
                s.set_inverted(vertical);
            }
            Inner::Progress { bar, .. } => {
                bar.set_orientation(orientation);
                bar.set_inverted(vertical);
            }
            _ => {}
        }
    }

    fn scroll_content(native: &GtkNative, content_id: Id) -> GtkNative {
        let Inner::Scroll { content, .. } = &native.inner else {
            let fixed = RufFixed::new(content_id);
            return GtkNative {
                id: content_id,
                outer: fixed.clone().upcast(),
                fixed: Some(fixed),
                inner: Inner::Container,
            };
        };
        content.imp().id.set(content_id);
        GtkNative {
            id: content_id,
            outer: content.clone().upcast(),
            fixed: Some(content.clone()),
            inner: Inner::Container,
        }
    }

    fn set_content_min_size(native: &GtkNative, size: Size) {
        if let Inner::Scroll { content, .. } = &native.inner {
            let w = if size.width > 0 { size.width } else { -1 };
            let h = if size.height > 0 { size.height } else { -1 };
            content.set_size_request(w, h);
        }
    }

    fn set_monospace(native: &GtkNative, _kind: WidgetKind, monospace: bool) {
        match &native.inner {
            Inner::Text { view, .. } => view.set_monospace(monospace),
            Inner::Entry(e) => {
                if monospace {
                    e.add_css_class("monospace");
                } else {
                    e.remove_css_class("monospace");
                }
            }
            _ => {}
        }
    }

    fn set_highlights(native: &GtkNative, spans: &[Span]) {
        let Some((_, buffer)) = text_buffer(native) else {
            return;
        };
        let (start, end) = (buffer.start_iter(), buffer.end_iter());
        let mut old = Vec::new();
        buffer.tag_table().foreach(|tag| {
            if tag.name().is_some_and(|n| n.starts_with("ruf-hl-")) {
                old.push(tag.clone());
            }
        });
        for tag in old {
            buffer.remove_tag(&tag, &start, &end);
        }
        for span in spans {
            let name = format!("ruf-hl-{}-{}", span.color, span.bold);
            let tag = set_tag(&buffer, &name, |tag| {
                tag.set_foreground(Some(&span.color.to_string()));
                if span.bold {
                    tag.set_weight(700);
                }
            });
            let from = buffer.iter_at_offset(span.start as i32);
            let to = buffer.iter_at_offset((span.start + span.len) as i32);
            buffer.apply_tag(&tag, &from, &to);
        }
    }

    fn set_marked_line(native: &GtkNative, line: Option<(usize, Color)>) {
        let Some((_, buffer)) = text_buffer(native) else {
            return;
        };
        let tag = set_tag(&buffer, "ruf-mark", |_| {});
        buffer.remove_tag(&tag, &buffer.start_iter(), &buffer.end_iter());
        if let Some((line, color)) = line
            && let Some(from) = buffer.iter_at_line(line as i32)
        {
            tag.set_paragraph_background(Some(&color.to_string()));
            let mut to = from;
            to.forward_line();
            buffer.apply_tag(&tag, &from, &to);
        }
    }

    fn goto_line(native: &GtkNative, line: usize) {
        if let Some((view, buffer)) = text_buffer(native)
            && let Some(iter) = buffer.iter_at_line(line as i32)
        {
            buffer.place_cursor(&iter);
            view.scroll_to_mark(&buffer.get_insert(), 0.1, false, 0.0, 0.0);
        }
    }

    fn set_shapes(native: &GtkNative, shapes: &[Shape]) {
        if let Inner::Canvas {
            area,
            shapes: store,
        } = &native.inner
        {
            *store.borrow_mut() = shapes.to_vec();
            area.queue_draw();
        }
    }

    fn set_tree_rows(native: &GtkNative, rows: &[TreeRow]) {
        let Inner::Tree(list) = &native.inner else {
            return;
        };
        let id = native.id;
        list.remove_all();
        for (index, row) in rows.iter().enumerate() {
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            line.set_margin_start(4 + row.depth as i32 * 16);
            line.set_margin_top(2);
            line.set_margin_bottom(2);
            // Symbol und Platzhalter gleich breit: Kinder sind sauber eingerückt.
            let toggle = gtk::Label::new(Some(match (row.has_children, row.expanded) {
                (false, _) => "",
                (true, true) => "▾",
                (true, false) => "▸",
            }));
            toggle.set_size_request(14, -1);
            if row.has_children {
                let click = gtk::GestureClick::new();
                click.connect_pressed(move |g, _, _, _| {
                    g.set_state(gtk::EventSequenceState::Claimed);
                    notify(id, Notify::TreeToggled(index));
                });
                toggle.add_controller(click);
            }
            line.append(&toggle);
            let label = gtk::Label::new(Some(&row.text));
            label.set_xalign(0.0);
            line.append(&label);
            list.append(&line);
        }
    }

    fn set_menu_bar(window: &GtkNative, bar: &MenuBar) {
        let (Inner::Window(w), Some(fixed)) = (&window.inner, &window.fixed) else {
            return;
        };
        let group = gio::SimpleActionGroup::new();
        let shortcuts = gtk::ShortcutController::new();
        shortcuts.set_scope(gtk::ShortcutScope::Global);
        let model = gio::Menu::new();
        let mut counter = 0;
        for (title, menu) in bar {
            let sub = build_menu(
                menu,
                "rufbar",
                &group,
                &mut counter,
                window.id,
                Some(&shortcuts),
            );
            model.append_submenu(Some(title), &sub);
        }
        w.insert_action_group("rufbar", Some(&group));
        let old = MENU_SHORTCUTS.with(|m| m.borrow_mut().insert(window.id, shortcuts.clone()));
        if let Some(old) = old {
            w.remove_controller(&old);
        }
        w.add_controller(shortcuts);

        // Fensterinhalt: Menüleiste über dem Container der Kinder.
        let column = match w.child().and_then(|c| c.downcast::<gtk::Box>().ok()) {
            Some(column) => column,
            None => {
                w.set_child(None::<&gtk::Widget>);
                let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
                fixed.set_vexpand(true);
                fixed.set_hexpand(true);
                column.append(fixed);
                w.set_child(Some(&column));
                column
            }
        };
        if let Some(old) = column
            .first_child()
            .filter(|c| c.is::<gtk::PopoverMenuBar>())
        {
            column.remove(&old);
        }
        if !bar.is_empty() {
            column.prepend(&gtk::PopoverMenuBar::from_model(Some(&model)));
        }
    }

    fn popup_menu(
        native: &GtkNative,
        _kind: WidgetKind,
        window: &GtkNative,
        menu: &Menu,
        x: i32,
        y: i32,
    ) {
        let Inner::Window(w) = &window.inner else {
            return;
        };
        let group = gio::SimpleActionGroup::new();
        let mut counter = 0;
        let model = build_menu(menu, "rufpop", &group, &mut counter, window.id, None);
        w.insert_action_group("rufpop", Some(&group));
        let anchor = match &native.inner {
            Inner::Window(_) => native
                .fixed
                .clone()
                .map(|f| f.upcast())
                .unwrap_or(native.outer.clone()),
            _ => native.outer.clone(),
        };
        let popover = gtk::PopoverMenu::from_model(Some(&model));
        popover.set_parent(&anchor);
        popover.set_has_arrow(false);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x, y, 1, 1)));
        popover.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || {
                if p.parent().is_some() {
                    p.unparent();
                }
            });
        });
        POPOVERS.with(|p| p.borrow_mut().push(popover.downgrade()));
        popover.popup();
    }

    fn open_file(
        parent: Option<&GtkNative>,
        title: &str,
        filters: &[FileFilter],
    ) -> Option<PathBuf> {
        let dialog = file_dialog(title, filters);
        run_file_dialog(|done| {
            dialog.open(
                parent.and_then(window_of).as_ref(),
                None::<&gio::Cancellable>,
                move |r| done(r.ok().and_then(|f| f.path())),
            )
        })
    }

    fn save_file(
        parent: Option<&GtkNative>,
        title: &str,
        name: &str,
        filters: &[FileFilter],
    ) -> Option<PathBuf> {
        let dialog = file_dialog(title, filters);
        dialog.set_initial_name(Some(name));
        run_file_dialog(|done| {
            dialog.save(
                parent.and_then(window_of).as_ref(),
                None::<&gio::Cancellable>,
                move |r| done(r.ok().and_then(|f| f.path())),
            )
        })
    }

    fn choose_folder(parent: Option<&GtkNative>, title: &str) -> Option<PathBuf> {
        let dialog = file_dialog(title, &[]);
        run_file_dialog(|done| {
            dialog.select_folder(
                parent.and_then(window_of).as_ref(),
                None::<&gio::Cancellable>,
                move |r| done(r.ok().and_then(|f| f.path())),
            )
        })
    }

    fn set_window_position(window: &GtkNative, x: i32, y: i32) -> bool {
        let Inner::Window(w) = &window.inner else {
            return false;
        };
        if !position::supported(&WidgetExt::display(w)) {
            return false;
        }
        if w.is_mapped() {
            position::set(w, x, y)
        } else {
            PENDING_POSITIONS.with(|p| p.borrow_mut().insert(window.id, (x, y)));
            true
        }
    }

    fn window_position(window: &GtkNative) -> Option<Point> {
        match &window.inner {
            Inner::Window(w) if w.is_mapped() => position::get(w),
            _ => None,
        }
    }
}

// --- Zeichenfläche ---------------------------------------------------------------------

fn set_color(cr: &gtk::cairo::Context, color: Color) {
    let (r, g, b) = color.to_f32();
    cr.set_source_rgb(f64::from(r), f64::from(g), f64::from(b));
}

fn rounded_rect(cr: &gtk::cairo::Context, x: f64, y: f64, w: f64, h: f64, radius: f64) {
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    if r <= 0.0 {
        cr.rectangle(x, y, w, h);
        return;
    }
    use std::f64::consts::{FRAC_PI_2, PI};
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, FRAC_PI_2);
    cr.arc(x + r, y + h - r, r, FRAC_PI_2, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * FRAC_PI_2);
    cr.close_path();
}

fn draw_shapes(area: &gtk::DrawingArea, cr: &gtk::cairo::Context, shapes: &[Shape]) {
    let base = area.pango_context().font_description().unwrap_or_default();
    cr.set_line_width(1.0);
    for shape in shapes {
        match shape {
            Shape::Rect {
                rect,
                fill,
                stroke,
                radius,
                dashed,
            } => {
                // Halbe Pixel: scharfe 1-px-Linien.
                let (x, y) = (f64::from(rect.x) + 0.5, f64::from(rect.y) + 0.5);
                let (w, h) = (
                    f64::from(rect.width - 1).max(0.0),
                    f64::from(rect.height - 1).max(0.0),
                );
                rounded_rect(cr, x, y, w, h, f64::from(*radius));
                if let Some(color) = fill {
                    set_color(cr, *color);
                    let _ = cr.fill_preserve();
                }
                if let Some(color) = stroke {
                    set_color(cr, *color);
                    if *dashed {
                        cr.set_dash(&[4.0, 3.0], 0.0);
                    }
                    let _ = cr.stroke_preserve();
                    cr.set_dash(&[], 0.0);
                }
                cr.new_path();
            }
            Shape::Line { from, to, color } => {
                set_color(cr, *color);
                cr.move_to(f64::from(from.x) + 0.5, f64::from(from.y) + 0.5);
                cr.line_to(f64::from(to.x) + 0.5, f64::from(to.y) + 0.5);
                let _ = cr.stroke();
            }
            Shape::Text {
                x,
                y,
                text,
                color,
                size,
                bold,
            } => {
                let layout = pangocairo::functions::create_layout(cr);
                let mut font = base.clone();
                font.set_absolute_size(f64::from(*size) * f64::from(pango::SCALE));
                font.set_weight(if *bold {
                    pango::Weight::Bold
                } else {
                    pango::Weight::Normal
                });
                layout.set_font_description(Some(&font));
                layout.set_text(text);
                set_color(cr, *color);
                cr.move_to(f64::from(*x), f64::from(*y));
                pangocairo::functions::show_layout(cr, &layout);
            }
        }
    }
}

fn modifiers(state: gdk::ModifierType) -> Modifiers {
    Modifiers {
        ctrl: state.contains(gdk::ModifierType::CONTROL_MASK),
        shift: state.contains(gdk::ModifierType::SHIFT_MASK),
        alt: state.contains(gdk::ModifierType::ALT_MASK),
        meta: state.intersects(gdk::ModifierType::META_MASK | gdk::ModifierType::SUPER_MASK),
    }
}

fn key_from(keyval: gdk::Key) -> Option<Key> {
    use gdk::Key as K;
    Some(match keyval {
        K::Return | K::KP_Enter => Key::Enter,
        K::Escape => Key::Escape,
        K::Tab | K::ISO_Left_Tab => Key::Tab,
        K::Delete | K::KP_Delete => Key::Delete,
        K::Insert => Key::Insert,
        K::Home => Key::Home,
        K::End => Key::End,
        K::Page_Up => Key::PageUp,
        K::Page_Down => Key::PageDown,
        K::Left => Key::Left,
        K::Right => Key::Right,
        K::Up => Key::Up,
        K::Down => Key::Down,
        K::BackSpace => Key::Backspace,
        K::space => Key::Space,
        other => {
            let name = other.name().map(|n| n.to_string()).unwrap_or_default();
            if let Some(n) = name.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()) {
                Key::F(n)
            } else {
                let c = other.to_unicode()?;
                if c.is_control() {
                    return None;
                }
                Key::Char(c.to_ascii_lowercase())
            }
        }
    })
}

fn connect_canvas_input(area: &gtk::DrawingArea, id: Id) {
    let click = gtk::GestureClick::new();
    click.set_button(0);
    click.connect_pressed(move |g, n, x, y| {
        if let Some(w) = g.widget() {
            w.grab_focus();
        }
        notify(
            id,
            Notify::Mouse(MouseEvent {
                action: if n == 2 {
                    MouseAction::DoubleClicked
                } else {
                    MouseAction::Pressed
                },
                x: x as i32,
                y: y as i32,
                button: g.current_button() as u8,
                modifiers: modifiers(g.current_event_state()),
            }),
        );
    });
    click.connect_released(move |g, _, x, y| {
        notify(
            id,
            Notify::Mouse(MouseEvent {
                action: MouseAction::Released,
                x: x as i32,
                y: y as i32,
                button: g.current_button() as u8,
                modifiers: modifiers(g.current_event_state()),
            }),
        );
    });
    area.add_controller(click);

    let motion = gtk::EventControllerMotion::new();
    motion.connect_motion(move |c, x, y| {
        notify(
            id,
            Notify::Mouse(MouseEvent {
                action: MouseAction::Moved,
                x: x as i32,
                y: y as i32,
                button: 0,
                modifiers: modifiers(c.current_event_state()),
            }),
        );
    });
    area.add_controller(motion);

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, keyval, _, state| match key_from(keyval) {
        Some(key) => {
            notify(
                id,
                Notify::Key(KeyEvent {
                    key,
                    modifiers: modifiers(state),
                }),
            );
            glib::Propagation::Stop
        }
        None => glib::Propagation::Proceed,
    });
    area.add_controller(keys);
}

// --- Menüs -------------------------------------------------------------------------------

/// Baut ein GIO-Menü; jeder Eintrag bekommt eine eigene Aktion in `group`
/// (`<prefix>.<prefix><n>`), die `Notify::Menu(id)` für `window` meldet.
fn build_menu(
    menu: &Menu,
    prefix: &str,
    group: &gio::SimpleActionGroup,
    counter: &mut usize,
    window: Id,
    shortcuts: Option<&gtk::ShortcutController>,
) -> gio::Menu {
    let model = gio::Menu::new();
    let mut section = gio::Menu::new();
    for entry in &menu.entries {
        match entry {
            MenuEntry::Item {
                id,
                label,
                shortcut,
                enabled,
            } => {
                let name = format!("{prefix}{counter}");
                *counter += 1;
                let action = gio::SimpleAction::new(&name, None);
                action.set_enabled(*enabled);
                let item = id.clone();
                action.connect_activate(move |_, _| {
                    // Nach dem Schließen des Menüs melden (Handler dürfen Dialoge öffnen).
                    let item = item.clone();
                    glib::idle_add_local_once(move || notify(window, Notify::Menu(item)));
                });
                group.add_action(&action);
                let detailed = format!("{prefix}.{name}");
                let menu_item = gio::MenuItem::new(Some(label), Some(&detailed));
                if let Some(spec) = shortcut.as_deref().and_then(KeySpec::parse) {
                    let accel = accelerator(&spec);
                    menu_item.set_attribute_value("accel", Some(&accel.to_variant()));
                    if let (Some(controller), Some(trigger)) =
                        (shortcuts, gtk::ShortcutTrigger::parse_string(&accel))
                    {
                        controller.add_shortcut(gtk::Shortcut::new(
                            Some(trigger),
                            Some(gtk::NamedAction::new(&detailed)),
                        ));
                    }
                }
                section.append_item(&menu_item);
            }
            MenuEntry::Separator => {
                model.append_section(None, &section);
                section = gio::Menu::new();
            }
            MenuEntry::Submenu { label, menu } => {
                let sub = build_menu(menu, prefix, group, counter, window, shortcuts);
                section.append_submenu(Some(label), &sub);
            }
        }
    }
    model.append_section(None, &section);
    model
}

// --- Dateidialoge ----------------------------------------------------------------------

fn file_dialog(title: &str, filters: &[FileFilter]) -> gtk::FileDialog {
    let dialog = gtk::FileDialog::builder().title(title).modal(true).build();
    if !filters.is_empty() {
        let store = gio::ListStore::new::<gtk::FileFilter>();
        for filter in filters {
            let f = gtk::FileFilter::new();
            f.set_name(Some(&filter.name));
            for pattern in &filter.patterns {
                f.add_pattern(pattern);
            }
            store.append(&f);
        }
        dialog.set_filters(Some(&store));
    }
    dialog
}

/// Startet einen asynchronen Dateidialog und wartet in einer eigenen
/// Ereignisschleife auf das Ergebnis.
fn run_file_dialog(start: impl FnOnce(Box<dyn FnOnce(Option<PathBuf>)>)) -> Option<PathBuf> {
    let main_loop = glib::MainLoop::new(None, false);
    let result: Rc<RefCell<Option<PathBuf>>> = Rc::default();
    let (l, r) = (main_loop.clone(), result.clone());
    start(Box::new(move |path| {
        *r.borrow_mut() = path;
        l.quit();
    }));
    main_loop.run();
    result.take()
}

// --- Fensterposition (nativ) -----------------------------------------------------------

#[cfg(all(unix, not(target_os = "macos")))]
mod position {
    use std::sync::OnceLock;

    use gdk4_x11::x11::xlib;
    use gdk4_x11::{X11Display, X11Surface};
    use gtk::prelude::*;
    use gtk4 as gtk;

    use crate::geometry::Point;

    /// Xlib, zur Laufzeit geladen (keine feste Abhängigkeit von libX11).
    fn xlib() -> Option<&'static xlib::Xlib> {
        static XLIB: OnceLock<Option<xlib::Xlib>> = OnceLock::new();
        XLIB.get_or_init(|| xlib::Xlib::open().ok()).as_ref()
    }

    /// Nur unter X11 (bzw. XWayland); Wayland verbietet das Positionieren.
    pub fn supported(display: &gtk::gdk::Display) -> bool {
        display.is::<X11Display>() && xlib().is_some()
    }

    fn handles(window: &gtk::Window) -> Option<(*mut xlib::Display, xlib::Window)> {
        let surface = window.surface()?.downcast::<X11Surface>().ok()?;
        let display = WidgetExt::display(window).downcast::<X11Display>().ok()?;
        // SAFETY: die Display-Verbindung gehört GDK und lebt so lange wie das Display.
        let xdisplay = unsafe { display.xdisplay() };
        Some((xdisplay, surface.xid()))
    }

    pub fn set(window: &gtk::Window, x: i32, y: i32) -> bool {
        let (Some(x11), Some((display, xid))) = (xlib(), handles(window)) else {
            return false;
        };
        // SAFETY: gültige Verbindung und Fensterkennung von GDK.
        unsafe {
            (x11.XMoveWindow)(display, xid, x, y);
            (x11.XFlush)(display);
        }
        true
    }

    pub fn get(window: &gtk::Window) -> Option<Point> {
        let x11 = xlib()?;
        let (display, xid) = handles(window)?;
        let (mut x, mut y, mut child) = (0, 0, 0);
        // SAFETY: gültige Verbindung und Fensterkennung von GDK.
        let ok = unsafe {
            let root = (x11.XDefaultRootWindow)(display);
            (x11.XTranslateCoordinates)(display, xid, root, 0, 0, &mut x, &mut y, &mut child)
        };
        (ok != 0).then_some(Point::new(x, y))
    }
}

#[cfg(windows)]
mod position {
    use gdk4_win32::Win32Surface;
    use gtk::prelude::*;
    use gtk4 as gtk;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowRect, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
    };

    use crate::geometry::Point;

    pub fn supported(_display: &gtk::gdk::Display) -> bool {
        true
    }

    fn hwnd(window: &gtk::Window) -> Option<*mut std::ffi::c_void> {
        let surface = window.surface()?.downcast::<Win32Surface>().ok()?;
        Some(surface.handle().0)
    }

    pub fn set(window: &gtk::Window, x: i32, y: i32) -> bool {
        let Some(hwnd) = hwnd(window) else {
            return false;
        };
        // SAFETY: gültiges Fenster-Handle von GDK.
        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            ) != 0
        }
    }

    pub fn get(window: &gtk::Window) -> Option<Point> {
        let hwnd = hwnd(window)?;
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: gültiges Fenster-Handle von GDK.
        let ok = unsafe { GetWindowRect(hwnd, &mut rect) } != 0;
        ok.then_some(Point::new(rect.left, rect.top))
    }
}

#[cfg(target_os = "macos")]
mod position {
    use gdk4_macos::MacosSurface;
    use gtk::prelude::*;
    use gtk4 as gtk;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use objc2_foundation::{NSPoint, NSRect};

    use crate::geometry::Point;

    pub fn supported(_display: &gtk::gdk::Display) -> bool {
        true
    }

    fn ns_window(window: &gtk::Window) -> Option<*mut AnyObject> {
        let surface = window.surface()?.downcast::<MacosSurface>().ok()?;
        let ns = surface.native() as *mut AnyObject;
        (!ns.is_null()).then_some(ns)
    }

    /// Höhe des Hauptbildschirms: Cocoa zählt y von unten.
    fn screen_height() -> f64 {
        // SAFETY: Klassenmethoden von AppKit im Hauptthread.
        unsafe {
            let screens: *mut AnyObject = msg_send![class!(NSScreen), screens];
            let first: *mut AnyObject = msg_send![screens, firstObject];
            if first.is_null() {
                return 0.0;
            }
            let frame: NSRect = msg_send![first, frame];
            frame.size.height
        }
    }

    pub fn set(window: &gtk::Window, x: i32, y: i32) -> bool {
        let Some(ns) = ns_window(window) else {
            return false;
        };
        let point = NSPoint::new(f64::from(x), screen_height() - f64::from(y));
        // SAFETY: gültiges NSWindow von GDK, im Hauptthread.
        unsafe {
            let _: () = msg_send![ns, setFrameTopLeftPoint: point];
        }
        true
    }

    pub fn get(window: &gtk::Window) -> Option<Point> {
        let ns = ns_window(window)?;
        // SAFETY: gültiges NSWindow von GDK, im Hauptthread.
        let frame: NSRect = unsafe { msg_send![ns, frame] };
        let top = screen_height() - (frame.origin.y + frame.size.height);
        Some(Point::new(frame.origin.x as i32, top as i32))
    }
}
