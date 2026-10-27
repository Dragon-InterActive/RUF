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

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib, pango};
use gtk4 as gtk;
use sourceview5::prelude::*;

use super::{Backend, MessageKind};
use crate::app::ColorScheme;
use crate::canvas::{KeyEvent, Modifiers, MouseAction, MouseEvent, Shape};
use crate::code::{Completion, Fold, LineMark, MarkKind, fold_regions, folds_language};
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
    /// Auswahlliste mit freier Eingabe (Eingabefeld + Vorschlagsliste).
    Combo(Suggest),
    /// Auswahlliste ohne freie Eingabe. (`GtkComboBox` ist seit GTK 4.10
    /// veraltet und öffnet sich unter macOS nach der Auswahl erneut.)
    Drop {
        drop: gtk::DropDown,
        model: gtk::StringList,
    },
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
    Code(sourceview5::View),
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
    /// Hell/Dunkel folgt dem System, bis das Programm es festlegt.
    static FOLLOW_SYSTEM: Cell<bool> = const { Cell::new(true) };
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
        Inner::Code(view) => Some((view.clone().upcast(), view.buffer())),
        _ => None,
    }
}

/// Eingabefeld mit Vorschlagsliste – Ersatz für das veraltete
/// `GtkComboBoxText` mit Eingabe (nach dem Vorbild aus `gtk4-demo`,
/// „Lists → Selections“). Wie dort: Tippen setzt die Auswahl auf -1, die
/// Wahl aus der Liste übernimmt Text und Index.
#[derive(Clone)]
pub struct Suggest {
    root: gtk::Box,
    entry: gtk::Entry,
    list: gtk::ListBox,
    popover: gtk::Popover,
    items: Rc<RefCell<Vec<String>>>,
    current: Rc<Cell<i32>>,
    /// Während der Text aus der Liste übernommen wird: Index nicht zurücksetzen.
    choosing: Rc<Cell<bool>>,
}

impl Suggest {
    fn new(id: Id) -> Self {
        let entry = gtk::Entry::new();
        entry.set_hexpand(true);
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.set_activate_on_single_click(true);
        let scroll = gtk::ScrolledWindow::new();
        scroll.set_child(Some(&list));
        scroll.set_propagate_natural_height(true);
        scroll.set_max_content_height(260);
        scroll.set_min_content_width(160);
        let popover = gtk::Popover::new();
        popover.set_child(Some(&scroll));
        popover.set_has_arrow(false);
        let button = gtk::MenuButton::new();
        button.set_popover(Some(&popover));
        button.set_focusable(false);
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        root.add_css_class("linked");
        root.append(&entry);
        root.append(&button);

        let suggest = Self {
            root,
            entry,
            list,
            popover,
            items: Rc::default(),
            current: Rc::new(Cell::new(-1)),
            choosing: Rc::new(Cell::new(false)),
        };
        let s = suggest.clone();
        suggest.list.connect_row_activated(move |_, row| {
            s.select(row.index());
            s.popover.popdown();
        });
        let s = suggest.clone();
        suggest.entry.connect_changed(move |_| {
            if !s.choosing.get() {
                s.current.set(-1);
            }
            glib::idle_add_local_once(move || notify(id, Notify::Changed));
        });
        suggest
    }

    fn set_items(&self, items: &[String]) {
        self.list.remove_all();
        for item in items {
            let label = gtk::Label::new(Some(item));
            label.set_xalign(0.0);
            label.set_margin_start(6);
            label.set_margin_end(6);
            label.set_margin_top(3);
            label.set_margin_bottom(3);
            self.list.append(&label);
        }
        *self.items.borrow_mut() = items.to_vec();
        self.current.set(-1);
    }

    /// Übernimmt Eintrag `index` (−1 = keine Auswahl, Text bleibt).
    fn select(&self, index: i32) {
        let item = usize::try_from(index)
            .ok()
            .and_then(|i| self.items.borrow().get(i).cloned());
        match item {
            Some(text) => {
                self.choosing.set(true);
                self.entry.set_text(&text);
                self.choosing.set(false);
                self.current.set(index);
            }
            None => self.current.set(-1),
        }
    }
}

/// Eine Auswahlliste: mit freier Eingabe [`Suggest`], sonst `GtkDropDown`.
fn new_choice(id: Id, editable: bool) -> (gtk::Widget, Inner) {
    if editable {
        let suggest = Suggest::new(id);
        (suggest.root.clone().upcast(), Inner::Combo(suggest))
    } else {
        let model = gtk::StringList::new(&[]);
        let drop = gtk::DropDown::new(Some(model.clone()), None::<gtk::Expression>);
        // Erst melden, wenn GTK die Liste geschlossen hat; Änderungen per Code
        // gleicht der Kern ohnehin selbst ab.
        drop.connect_selected_notify(move |_| {
            glib::idle_add_local_once(move || notify(id, Notify::Changed));
        });
        (drop.clone().upcast(), Inner::Drop { drop, model })
    }
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
        quiet_frame_timing_warnings();
        // Reines GTK folgt dem Hell/Dunkel des Systems unter macOS und Windows
        // nicht (und unter Linux nicht überall) – RUF übernimmt das.
        system_scheme::watch(apply_system_scheme);
        apply_system_scheme();
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
        if FOLLOW_SYSTEM.get()
            && let Some(dark) = system_scheme::dark()
        {
            return if dark {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            };
        }
        if let Some(dark) = APPLIED.get() {
            return if dark {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            };
        }
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
        let Some(settings) = gtk::Settings::default() else {
            return false;
        };
        if scheme == ColorScheme::Unknown {
            // Wieder dem System folgen.
            FOLLOW_SYSTEM.set(true);
            match system_scheme::dark() {
                Some(_) => apply_system_scheme(),
                None => set_dark(&settings, None),
            }
        } else {
            FOLLOW_SYSTEM.set(false);
            set_dark(&settings, Some(scheme == ColorScheme::Dark));
        }
        true
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

    fn input(
        parent: Option<&GtkNative>,
        title: &str,
        label: &str,
        text: &str,
        secret: bool,
    ) -> Option<String> {
        let (cancel, ok) = if crate::dialog::german() {
            ("Abbrechen", "OK")
        } else {
            ("Cancel", "OK")
        };
        let dialog = gtk::Window::builder()
            .title(title)
            .modal(true)
            .resizable(false)
            .default_width(360)
            .build();
        if let Some(window) = parent.and_then(window_of) {
            dialog.set_transient_for(Some(&window));
        }
        let column = gtk::Box::new(gtk::Orientation::Vertical, 8);
        column.set_margin_top(12);
        column.set_margin_bottom(12);
        column.set_margin_start(12);
        column.set_margin_end(12);
        let caption = gtk::Label::new(Some(label));
        caption.set_xalign(0.0);
        let entry = gtk::Entry::new();
        entry.set_text(text);
        entry.set_activates_default(true);
        if secret {
            entry.set_visibility(false);
            entry.set_input_purpose(gtk::InputPurpose::Password);
        }
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        buttons.set_halign(gtk::Align::End);
        let cancel = gtk::Button::with_label(cancel);
        let ok = gtk::Button::with_label(ok);
        ok.add_css_class("suggested-action");
        buttons.append(&cancel);
        buttons.append(&ok);
        column.append(&caption);
        column.append(&entry);
        column.append(&buttons);
        dialog.set_child(Some(&column));
        dialog.set_default_widget(Some(&ok));

        let main_loop = glib::MainLoop::new(None, false);
        let answer: Rc<RefCell<Option<String>>> = Rc::default();
        let finish = {
            let (l, d) = (main_loop.clone(), dialog.downgrade());
            move || {
                if let Some(d) = d.upgrade() {
                    d.destroy();
                }
                l.quit();
            }
        };
        {
            let (a, e, f) = (answer.clone(), entry.clone(), finish.clone());
            ok.connect_clicked(move |_| {
                *a.borrow_mut() = Some(e.text().to_string());
                f();
            });
        }
        {
            let f = finish.clone();
            cancel.connect_clicked(move |_| f());
        }
        {
            let f = finish.clone();
            dialog.connect_close_request(move |_| {
                f();
                glib::Propagation::Proceed
            });
        }
        let keys = gtk::EventControllerKey::new();
        {
            let f = finish;
            keys.connect_key_pressed(move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    f();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            });
        }
        dialog.add_controller(keys);
        dialog.present();
        entry.grab_focus();
        // Dateinamen: nur den Namen ohne Endung auswählen.
        let end = text
            .rfind('.')
            .filter(|&i| i > 0)
            .map_or(-1, |i| text[..i].chars().count() as i32);
        entry.select_region(0, end);
        main_loop.run();
        answer.take()
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
                // Vor dem Erscheinen: Position samt Hinweis setzen, damit der
                // Fenstermanager das Fenster gleich dort platziert …
                window.connect_realize(move |w| {
                    if let Some((x, y)) = PENDING_POSITIONS.with(|p| p.borrow().get(&id).copied()) {
                        position::set(w, x, y);
                    }
                });
                // … und danach noch einmal, falls er den Hinweis übergeht.
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
            WidgetKind::CodeEditor => {
                let view = new_code_view();
                view.buffer()
                    .connect_changed(move |_| notify(id, Notify::Changed));
                // Maus melden (z. B. Strg+Klick = Gehe zu Definition). Beim
                // Loslassen hat GTK den Cursor schon gesetzt.
                let click = gtk::GestureClick::new();
                click.set_button(0);
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
                view.add_controller(click);
                let scroll = gtk::ScrolledWindow::new();
                scroll.set_child(Some(&view));
                scroll.set_has_frame(true);
                (scroll.upcast(), None, Inner::Code(view))
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
                let (widget, inner) = new_choice(id, false);
                (widget, None, inner)
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
                // Rechtsklick wählt die Zeile und meldet sich als Mausereignis
                // (Koordinaten relativ zur Baumansicht) – z. B. für ein Kontextmenü.
                let click = gtk::GestureClick::new();
                click.set_button(gdk::BUTTON_SECONDARY);
                let outer = scroll.downgrade();
                click.connect_pressed(move |g, _, x, y| {
                    let Some(list) = g.widget().and_downcast::<gtk::ListBox>() else {
                        return;
                    };
                    if let Some(row) = list.row_at_y(y as i32) {
                        list.select_row(Some(&row));
                    }
                    let at = outer.upgrade().and_then(|outer| {
                        list.compute_point(&outer, &gtk::graphene::Point::new(x as f32, y as f32))
                    });
                    let (x, y) = at.map_or((x, y), |p| (f64::from(p.x()), f64::from(p.y())));
                    notify(
                        id,
                        Notify::Mouse(MouseEvent {
                            action: MouseAction::Pressed,
                            x: x as i32,
                            y: y as i32,
                            button: 3,
                            modifiers: modifiers(g.current_event_state()),
                        }),
                    );
                });
                list.add_controller(click);
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
            Inner::Code(view) => view.clone().upcast(),
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
        // GTK rechnet eine gesetzte Größe (`set_size_request`) in die Messung
        // ein. Sonst wüchse ein Widget, das nach seiner bevorzugten Größe
        // plus Rand gesetzt wird, bei jeder Neuanordnung weiter.
        let request = widget.size_request();
        widget.set_size_request(-1, -1);
        let (_, width, _, _) = widget.measure(gtk::Orientation::Horizontal, -1);
        let (_, height, _, _) = widget.measure(gtk::Orientation::Vertical, width);
        widget.set_size_request(request.0, request.1);
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

    fn measure_text(text: &str, size: i32, bold: bool, monospace: bool) -> Size {
        let context = pangocairo::FontMap::default().create_context();
        let layout = pango::Layout::new(&context);
        layout.set_font_description(Some(&text_font(None, size, bold, false, monospace)));
        layout.set_text(text);
        let (width, height) = layout.pixel_size();
        Size::new(width, height)
    }

    fn clipboard_set(text: &str) {
        if let Some(display) = gdk::Display::default() {
            display.clipboard().set_text(text);
        }
    }

    fn clipboard_read(f: Box<dyn FnOnce(Option<String>)>) {
        let Some(display) = gdk::Display::default() else {
            f(None);
            return;
        };
        display
            .clipboard()
            .read_text_async(None::<&gio::Cancellable>, move |result| {
                f(result.ok().flatten().map(|t| t.to_string()));
            });
    }

    fn focus(native: &GtkNative) {
        match &native.inner {
            Inner::Canvas { area, .. } => {
                area.grab_focus();
            }
            Inner::Text { view, .. } => {
                view.grab_focus();
            }
            Inner::Code(view) => {
                view.grab_focus();
            }
            _ => {
                native.outer.grab_focus();
            }
        }
    }

    fn set_mouse_cursor(native: &GtkNative, name: &str) {
        let target = match &native.inner {
            Inner::Canvas { area, .. } => area.clone().upcast(),
            _ => native.outer.clone(),
        };
        target.set_cursor_from_name((!name.is_empty()).then_some(name));
    }

    fn text_height(native: &GtkNative, width: i32) -> Option<(i32, i32)> {
        let view: gtk::TextView = match &native.inner {
            Inner::Text { view, .. } => view.clone(),
            Inner::Code(view) => view.clone().upcast(),
            _ => return None,
        };
        let buffer = view.buffer();
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        // Rand des Rahmens und Innenabstände der Textansicht abziehen.
        let inner = width - view.left_margin() - view.right_margin() - 4;
        let measure = |text: &str| {
            let layout = view.create_pango_layout(Some(text));
            if view.wrap_mode() != gtk::WrapMode::None {
                layout.set_width(inner.max(1) * pango::SCALE);
                layout.set_wrap(pango::WrapMode::WordChar);
            }
            layout.pixel_size().1
        };
        Some((measure(&text), measure("X")))
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
            Inner::Code(view) => view.clone().upcast(),
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
            Inner::Code(view) => view.buffer().set_text(text),
            Inner::Combo(c) => c.entry.set_text(text),
            _ => {}
        }
    }

    fn text(native: &GtkNative, _kind: WidgetKind) -> String {
        match &native.inner {
            Inner::Entry(e) => e.text().to_string(),
            Inner::Code(view) => {
                let buffer = view.buffer();
                buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), false)
                    .to_string()
            }
            Inner::Text { view, .. } => {
                let buffer = view.buffer();
                buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), false)
                    .to_string()
            }
            Inner::Combo(c) => c.entry.text().to_string(),
            Inner::Drop { drop, model } => model
                .string(drop.selected())
                .map(|t| t.to_string())
                .unwrap_or_default(),
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
            Inner::Combo(c) => c.entry.set_placeholder_text(Some(text)),
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
            Inner::Code(view) => view.set_editable(!read_only),
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
        if let Inner::Text { view, .. } = &native.inner {
            view.set_wrap_mode(if wrap {
                gtk::WrapMode::WordChar
            } else {
                gtk::WrapMode::None
            });
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
            Inner::Combo(c) => c.set_items(items),
            Inner::Drop { model, .. } => {
                let items: Vec<&str> = items.iter().map(String::as_str).collect();
                model.splice(0, model.n_items(), &items);
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
            Inner::Combo(c) => c.select(index),
            Inner::Drop { drop, .. } => {
                drop.set_selected(u32::try_from(index).unwrap_or(gtk::INVALID_LIST_POSITION))
            }
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
            Inner::Combo(c) => c.current.get(),
            Inner::Drop { drop, .. } => match drop.selected() {
                gtk::INVALID_LIST_POSITION => -1,
                i => i as i32,
            },
            Inner::List(list) | Inner::Tree(list) => {
                list.selected_row().map(|r| r.index()).unwrap_or(-1)
            }
            Inner::Tabs(n) => n.current_page().map(|i| i as i32).unwrap_or(-1),
            _ => -1,
        }
    }

    fn set_editable(native: &GtkNative, editable: bool) -> Option<GtkNative> {
        let is_editable = match &native.inner {
            Inner::Combo(_) => true,
            Inner::Drop { .. } => false,
            _ => return None,
        };
        if is_editable == editable {
            return None;
        }
        // Eine Liste mit und eine ohne freie Eingabe sind verschiedene
        // GTK-Steuerelemente: ersetzen (Einträge und Auswahl setzt der Kern).
        let old = &native.outer;
        let (widget, inner) = new_choice(native.id, editable);
        if let Some(fixed) = parent_fixed(old) {
            let (x, y) = fixed.child_position(old);
            fixed.remove(old);
            fixed.put(&widget, x, y);
        }
        widget.set_tooltip_text(old.tooltip_text().as_deref());
        widget.set_sensitive(old.is_sensitive());
        widget.set_visible(old.is_visible());
        for class in old.css_classes() {
            widget.add_css_class(&class);
        }
        Some(GtkNative {
            id: native.id,
            outer: widget,
            fixed: None,
            inner,
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
        let Inner::Tabs(n) = &native.inner else {
            return;
        };
        let Some(page) = n.nth_page(u32::try_from(index).ok()) else {
            return;
        };
        // Den Text des vorhandenen Labels ändern: `set_tab_label_text` ersetzt
        // das Label-Widget. Liegt der Mauszeiger gerade darauf, verschickt GTK
        // danach Ereignisse an das freigegebene Label (Absturz unter macOS in
        // `gtk_synthesize_crossing_events`), und der Reiter springt beim Klicken.
        match n.tab_label(&page).and_downcast::<gtk::Label>() {
            Some(label) => {
                if label.text() != title {
                    label.set_text(title);
                }
            }
            None => n.set_tab_label_text(&page, title),
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

    fn scroll_to(native: &GtkNative, x: i32, y: i32) {
        if let Some(window) = native.outer.downcast_ref::<gtk::ScrolledWindow>() {
            window.hadjustment().set_value(f64::from(x));
            window.vadjustment().set_value(f64::from(y));
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
            Inner::Code(view) => view.set_monospace(monospace),
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

    fn set_language(native: &GtkNative, language: &str) {
        let Inner::Code(view) = &native.inner else {
            return;
        };
        let Ok(buffer) = view.buffer().downcast::<sourceview5::Buffer>() else {
            return;
        };
        let language = (!language.is_empty())
            .then(|| sourceview5::LanguageManager::default().language(language))
            .flatten();
        buffer.set_language(language.as_ref());
        buffer.set_highlight_syntax(language.is_some());
    }

    fn set_line_marks(native: &GtkNative, marks: &[LineMark]) {
        let Inner::Code(view) = &native.inner else {
            return;
        };
        let Ok(buffer) = view.buffer().downcast::<sourceview5::Buffer>() else {
            return;
        };
        if let Some(state) = fold_state(&buffer) {
            let mut state = state.borrow_mut();
            state.line_marks = marks.to_vec();
            place_line_marks(&buffer, &state);
        }
    }

    fn folds(native: &GtkNative) -> Vec<Fold> {
        let Inner::Code(view) = &native.inner else {
            return Vec::new();
        };
        view.buffer()
            .downcast::<sourceview5::Buffer>()
            .ok()
            .and_then(|buffer| fold_state(&buffer))
            .map(|state| state.borrow().regions.clone())
            .unwrap_or_default()
    }

    fn set_folded(native: &GtkNative, line: usize, folded: bool) {
        if let Inner::Code(view) = &native.inner
            && let Ok(buffer) = view.buffer().downcast::<sourceview5::Buffer>()
        {
            set_folded(&buffer, line, folded);
        }
    }

    fn cursor(native: &GtkNative) -> (usize, usize) {
        let Inner::Code(view) = &native.inner else {
            return (1, 0);
        };
        let buffer = view.buffer();
        let iter = buffer.iter_at_mark(&buffer.get_insert());
        (iter.line() as usize + 1, iter.line_offset() as usize)
    }

    fn set_cursor(native: &GtkNative, line: usize, column: usize) {
        let Inner::Code(view) = &native.inner else {
            return;
        };
        let buffer = view.buffer();
        let line = line.saturating_sub(1) as i32;
        if let Some(mut iter) = buffer.iter_at_line(line) {
            let length = iter.chars_in_line();
            iter.set_line_offset((column as i32).min((length - 1).max(0)));
            buffer.place_cursor(&iter);
            view.scroll_to_mark(&buffer.get_insert(), 0.1, false, 0.0, 0.0);
        }
    }

    fn apply_completion(native: &GtkNative, text: &str) {
        if let Inner::Code(view) = &native.inner {
            replace_word_before_cursor(view, text);
        }
    }

    fn show_completions(native: &GtkNative, items: &[Completion]) {
        let Inner::Code(view) = &native.inner else {
            return;
        };
        close_completions();
        if items.is_empty() {
            return;
        }
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Browse);
        for item in items {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            row.set_margin_start(6);
            row.set_margin_end(6);
            let label = gtk::Label::new(Some(&item.label));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            row.append(&label);
            if let Some(detail) = &item.detail {
                let detail = gtk::Label::new(Some(detail));
                detail.add_css_class("dim-label");
                detail.set_ellipsize(pango::EllipsizeMode::End);
                detail.set_max_width_chars(40);
                row.append(&detail);
            }
            list.append(&row);
        }
        let scroll = gtk::ScrolledWindow::new();
        scroll.set_child(Some(&list));
        scroll.set_propagate_natural_height(true);
        scroll.set_propagate_natural_width(true);
        scroll.set_max_content_height(260);
        scroll.set_min_content_width(260);
        let popover = gtk::Popover::new();
        popover.set_child(Some(&scroll));
        popover.set_has_arrow(false);
        popover.set_position(gtk::PositionType::Bottom);
        popover.set_parent(view);
        // Am Cursor öffnen.
        let buffer = view.buffer();
        let cursor = buffer.iter_at_mark(&buffer.get_insert());
        let location = view.iter_location(&cursor);
        let (x, y) =
            view.buffer_to_window_coords(gtk::TextWindowType::Widget, location.x(), location.y());
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x, y, 1, location.height())));

        let inserts: Vec<String> = items.iter().map(|i| i.insert.clone()).collect();
        let (v, p) = (view.clone(), popover.clone());
        list.connect_row_activated(move |_, row| {
            if let Some(text) = usize::try_from(row.index())
                .ok()
                .and_then(|i| inserts.get(i))
            {
                replace_word_before_cursor(&v, text);
            }
            p.popdown();
            v.grab_focus();
        });
        let v = view.clone();
        popover.connect_closed(move |p| {
            let p = p.clone();
            glib::idle_add_local_once(move || {
                if p.parent().is_some() {
                    p.unparent();
                }
            });
            v.grab_focus();
        });
        if let Some(first) = list.row_at_index(0) {
            list.select_row(Some(&first));
        }
        COMPLETION.with(|c| *c.borrow_mut() = Some(popover.downgrade()));
        popover.popup();
        list.grab_focus();
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
            label.set_hexpand(true);
            label.set_ellipsize(pango::EllipsizeMode::Middle);
            let colored = |label: &gtk::Label, text: &str| {
                if let Some(c) = row.color {
                    label.set_markup(&format!(
                        "<span foreground=\"#{:02x}{:02x}{:02x}\">{}</span>",
                        c.r,
                        c.g,
                        c.b,
                        glib::markup_escape_text(text)
                    ));
                }
            };
            colored(&label, &row.text);
            line.append(&label);
            if let Some(badge) = &row.badge {
                let badge_label = gtk::Label::new(Some(badge));
                badge_label.set_margin_end(6);
                colored(&badge_label, badge);
                line.append(&badge_label);
            }
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
        // Menükürzel vor dem fokussierten Widget: Strg+Enter in einem
        // Textfeld löst den Menüeintrag aus statt einen Zeilenumbruch.
        shortcuts.set_propagation_phase(gtk::PropagationPhase::Capture);
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

/// Schrift für Text auf der Zeichenfläche (`base` = Schrift des Widgets).
fn text_font(
    base: Option<&pango::FontDescription>,
    size: i32,
    bold: bool,
    italic: bool,
    monospace: bool,
) -> pango::FontDescription {
    let mut font = base.cloned().unwrap_or_default();
    if monospace {
        font.set_family("Monospace");
    }
    font.set_absolute_size(f64::from(size) * f64::from(pango::SCALE));
    font.set_weight(if bold {
        pango::Weight::Bold
    } else {
        pango::Weight::Normal
    });
    if italic {
        font.set_style(pango::Style::Italic);
    }
    font
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
            Shape::MonoText {
                x,
                y,
                text,
                color,
                background,
                size,
                bold,
                italic,
                underline,
            } => {
                let layout = pangocairo::functions::create_layout(cr);
                layout.set_font_description(Some(&text_font(
                    Some(&base),
                    *size,
                    *bold,
                    *italic,
                    true,
                )));
                layout.set_text(text);
                if *underline {
                    let attrs = pango::AttrList::new();
                    attrs.insert(pango::AttrInt::new_underline(pango::Underline::Single));
                    layout.set_attributes(Some(&attrs));
                }
                if let Some(bg) = background {
                    let (w, h) = layout.pixel_size();
                    set_color(cr, *bg);
                    cr.rectangle(f64::from(*x), f64::from(*y), f64::from(w), f64::from(h));
                    let _ = cr.fill();
                }
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

    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    scroll.connect_scroll(move |c, _, dy| {
        if dy == 0.0 {
            return glib::Propagation::Proceed;
        }
        notify(
            id,
            Notify::Mouse(MouseEvent {
                action: if dy < 0.0 {
                    MouseAction::ScrollUp
                } else {
                    MouseAction::ScrollDown
                },
                x: 0,
                y: 0,
                button: 0,
                modifiers: modifiers(c.current_event_state()),
            }),
        );
        glib::Propagation::Stop
    });
    area.add_controller(scroll);

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, keyval, _, state| match key_from(keyval) {
        Some(key) => {
            notify(
                id,
                Notify::Key(KeyEvent {
                    key,
                    modifiers: modifiers(state),
                    text: keyval.to_unicode().filter(|c| !c.is_control()),
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
        // SAFETY: gültige Verbindung und Fensterkennung von GDK; die Hinweis-
        // Struktur kommt von Xlib und wird mit XFree freigegeben.
        unsafe {
            // "Vom Programm festgelegte Position" (ICCCM): Fenstermanager wie
            // KWin platzieren das Fenster dann nicht nach eigener Regel
            // (unter der Maus, auf dem aktiven Bildschirm …).
            let hints = (x11.XAllocSizeHints)();
            if !hints.is_null() {
                let mut supplied = 0;
                (x11.XGetWMNormalHints)(display, xid, hints, &mut supplied);
                (*hints).flags |= xlib::USPosition | xlib::PPosition;
                (*hints).x = x;
                (*hints).y = y;
                (x11.XSetWMNormalHints)(display, xid, hints);
                (x11.XFree)(hints.cast());
            }
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

// --- Hell/Dunkel des Systems -----------------------------------------------------------

/// Übernimmt das Hell/Dunkel des Systems, solange das Programm es nicht
/// selbst festgelegt hat.
fn apply_system_scheme() {
    if !FOLLOW_SYSTEM.get() {
        return;
    }
    if let (Some(dark), Some(settings)) = (system_scheme::dark(), gtk::Settings::default())
        && APPLIED.get() != Some(dark)
    {
        set_dark(&settings, Some(dark));
    }
}

thread_local! {
    /// Zuletzt gesetztes Hell/Dunkel (nur bei Änderung neu setzen).
    static APPLIED: Cell<Option<bool>> = const { Cell::new(None) };
}

/// Setzt Hell/Dunkel (`None` = Vorgabe des Themas). GTK ab 4.20 nutzt
/// `gtk-interface-color-scheme` (über den Namen gesetzt, damit RUF auch mit
/// GTK 4.12 läuft), ältere Versionen `gtk-application-prefer-dark-theme`.
fn set_dark(settings: &gtk::Settings, dark: Option<bool>) {
    APPLIED.set(dark);
    update_code_schemes(dark == Some(true));
    let Some(spec) = settings.find_property("gtk-interface-color-scheme") else {
        settings.set_gtk_application_prefer_dark_theme(dark == Some(true));
        return;
    };
    let nick = match dark {
        Some(true) => "dark",
        Some(false) => "light",
        None => "default",
    };
    if let Some(value) =
        glib::EnumClass::with_type(spec.value_type()).and_then(|class| class.to_value_by_nick(nick))
    {
        settings.set_property_from_value("gtk-interface-color-scheme", &value);
    }
}

/// Linux/BSD: Desktop-Portal `org.freedesktop.appearance color-scheme`
/// (GNOME, KDE, …), Änderungen kommen als D-Bus-Signal.
#[cfg(all(unix, not(target_os = "macos")))]
mod system_scheme {
    use std::cell::{Cell, RefCell};

    use gtk::glib::prelude::*;
    use gtk::{gio, glib};
    use gtk4 as gtk;

    const DEST: &str = "org.freedesktop.portal.Desktop";
    const PATH: &str = "/org/freedesktop/portal/desktop";
    const IFACE: &str = "org.freedesktop.portal.Settings";
    const NAMESPACE: &str = "org.freedesktop.appearance";
    const KEY: &str = "color-scheme";

    thread_local! {
        static DARK: Cell<Option<bool>> = const { Cell::new(None) };
        static SUBSCRIPTION: RefCell<Option<(gio::DBusConnection, gio::SignalSubscriptionId)>> =
            const { RefCell::new(None) };
    }

    /// 1 = dunkel bevorzugt, 2 = hell, 0 = keine Vorgabe (auch verschachtelt in `v`).
    fn parse(value: &glib::Variant) -> Option<bool> {
        let mut value = value.clone();
        while value.is_type(glib::VariantTy::VARIANT) {
            value = value.as_variant()?;
        }
        match value.get::<u32>()? {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        }
    }

    fn read(connection: &gio::DBusConnection) -> Option<bool> {
        let args = (NAMESPACE, KEY).to_variant();
        // `ReadOne` (neuere Portale), sonst `Read`.
        ["ReadOne", "Read"].iter().find_map(|method| {
            let reply = connection
                .call_sync(
                    Some(DEST),
                    PATH,
                    IFACE,
                    method,
                    Some(&args),
                    None,
                    gio::DBusCallFlags::NONE,
                    1000,
                    None::<&gio::Cancellable>,
                )
                .ok()?;
            parse(&reply.child_value(0))
        })
    }

    pub fn dark() -> Option<bool> {
        DARK.get()
    }

    pub fn watch(changed: fn()) {
        let Ok(connection) = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>)
        else {
            return;
        };
        DARK.set(read(&connection));
        #[allow(deprecated)]
        let id = connection.signal_subscribe(
            Some(DEST),
            Some(IFACE),
            Some("SettingChanged"),
            Some(PATH),
            None,
            gio::DBusSignalFlags::NONE,
            move |_, _, _, _, _, params| {
                let namespace = params.child_value(0);
                let key = params.child_value(1);
                if namespace.str() == Some(NAMESPACE) && key.str() == Some(KEY) {
                    DARK.set(parse(&params.child_value(2)));
                    changed();
                }
            },
        );
        SUBSCRIPTION.with(|s| *s.borrow_mut() = Some((connection, id)));
    }
}

/// Windows: `AppsUseLightTheme` in der Registry, alle 2 s abgefragt.
#[cfg(windows)]
mod system_scheme {
    use std::time::Duration;

    use gtk::glib;
    use gtk4 as gtk;
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn dark() -> Option<bool> {
        let key = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let name = wide("AppsUseLightTheme");
        let mut value: u32 = 1;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: gültige, nullterminierte Zeichenketten und Puffer passender Größe.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                (&mut value as *mut u32).cast(),
                &mut size,
            )
        };
        (status == 0).then_some(value == 0)
    }

    pub fn watch(changed: fn()) {
        glib::timeout_add_local(Duration::from_secs(2), move || {
            changed();
            glib::ControlFlow::Continue
        });
    }
}

/// macOS: `AppleInterfaceStyle` (nur bei dunkler Darstellung gesetzt), alle 2 s abgefragt.
#[cfg(target_os = "macos")]
mod system_scheme {
    use std::time::Duration;

    use gtk::glib;
    use gtk4 as gtk;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    use objc2_foundation::NSString;

    pub fn dark() -> Option<bool> {
        let key = NSString::from_str("AppleInterfaceStyle");
        // SAFETY: Klassenmethode und Abfrage von Foundation, im Hauptthread.
        let value: *mut AnyObject = unsafe {
            let defaults: *mut AnyObject = msg_send![class!(NSUserDefaults), standardUserDefaults];
            msg_send![defaults, stringForKey: &*key]
        };
        Some(!value.is_null())
    }

    pub fn watch(changed: fn()) {
        glib::timeout_add_local(Duration::from_secs(2), move || {
            changed();
            glib::ControlFlow::Continue
        });
    }
}

// --- Meldungen -------------------------------------------------------------------------

/// GDK warnt über Unstimmigkeiten seines eigenen Bildtakts (z. B. auf
/// 120-Hz-Displays unter macOS oder unter Xvfb): `gdk_frame_timings_…`,
/// `gdk_frame_presented()`. Unter macOS meldet GTK außerdem beim Öffnen von
/// Auswahllisten `gdk_surface_thaw_updates: assertion … failed` – ein Fehler
/// in GTK selbst (tritt auch in `gtk4-demo` auf), ohne sichtbare Folgen.
/// Das Breeze-Theme von KDE gibt den Schiebern der Scrollbalken negative
/// Abstände, GTK meldet dann `GtkGizmo … (slider) reported min width -2` –
/// ein Fehler des Themes, ebenfalls ohne Folgen.
/// Diese Meldungen werden ausgefiltert, alle anderen bleiben.
///
/// GLib erlaubt nur einen Log-Writer pro Prozess; RUF setzt ihn einmalig.
fn quiet_frame_timing_warnings() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(install_log_filter);
}

fn install_log_filter() {
    glib::log_set_writer_func(|level, fields| {
        let field = |key: &str| {
            fields
                .iter()
                .find(|f| f.key() == key)
                .and_then(|f| f.value_str())
        };
        let frame_noise = field("GLIB_DOMAIN") == Some("Gdk")
            && field("MESSAGE").is_some_and(|m| {
                m.contains("gdk_frame_timings_")
                    || m.contains("gdk_frame_presented()")
                    || (cfg!(target_os = "macos")
                        && m.contains(
                            "gdk_surface_thaw_updates: assertion 'surface->update_freeze_count > 0' failed",
                        ))
            });
        let theme_noise = field("GLIB_DOMAIN") == Some("Gtk")
            && field("MESSAGE").is_some_and(|m| {
                m.starts_with("GtkGizmo")
                    && m.contains("(slider) reported min")
                    && m.contains("but sizes must be >= 0")
            });
        if frame_noise || theme_noise {
            glib::LogWriterOutput::Handled
        } else {
            glib::log_writer_default(level, fields)
        }
    });
}

// --- Code-Editor ---------------------------------------------------------------------------

const MARK_CATEGORIES: [&str; 3] = ["ruf-error", "ruf-warning", "ruf-info"];

fn mark_category(kind: MarkKind) -> &'static str {
    match kind {
        MarkKind::Error => MARK_CATEGORIES[0],
        MarkKind::Warning => MARK_CATEGORIES[1],
        MarkKind::Info => MARK_CATEGORIES[2],
    }
}

thread_local! {
    /// Texte der Marken (Tooltip), zum Mark-Objekt.
    static MARK_MESSAGES: RefCell<Vec<(glib::WeakRef<sourceview5::Mark>, String)>> =
        const { RefCell::new(Vec::new()) };
    /// Alle Code-Editoren: ihr Farbschema folgt Hell/Dunkel.
    static CODE_BUFFERS: RefCell<Vec<glib::WeakRef<sourceview5::Buffer>>> =
        const { RefCell::new(Vec::new()) };
}

/// Farbschema von GtkSourceView passend zu Hell/Dunkel.
fn code_scheme(dark: bool) -> Option<sourceview5::StyleScheme> {
    let manager = sourceview5::StyleSchemeManager::default();
    let ids: &[&str] = if dark {
        &["Adwaita-dark", "classic-dark", "oblivion"]
    } else {
        &["Adwaita", "classic"]
    };
    ids.iter().find_map(|id| manager.scheme(id))
}

/// Setzt das Farbschema aller Code-Editoren (nach einem Hell/Dunkel-Wechsel).
fn update_code_schemes(dark: bool) {
    let scheme = code_scheme(dark);
    CODE_BUFFERS.with(|buffers| {
        buffers.borrow_mut().retain(|weak| match weak.upgrade() {
            Some(buffer) => {
                buffer.set_style_scheme(scheme.as_ref());
                true
            }
            None => false,
        });
    });
}

fn new_code_view() -> sourceview5::View {
    let buffer = sourceview5::Buffer::new(None::<&gtk::TextTagTable>);
    buffer.set_highlight_matching_brackets(true);
    buffer.set_style_scheme(code_scheme(APPLIED.get() == Some(true)).as_ref());
    CODE_BUFFERS.with(|b| b.borrow_mut().push(buffer.downgrade()));
    let view = sourceview5::View::with_buffer(&buffer);
    view.set_monospace(true);
    view.set_highlight_current_line(true);
    view.set_auto_indent(true);
    view.set_indent_on_tab(true);
    view.set_insert_spaces_instead_of_tabs(true);
    view.set_tab_width(4);
    view.set_smart_backspace(true);
    view.set_show_line_marks(true);
    view.set_left_margin(4);
    for (category, icon, color) in [
        (
            MARK_CATEGORIES[0],
            "dialog-error-symbolic",
            (0.88, 0.2, 0.2),
        ),
        (
            MARK_CATEGORIES[1],
            "dialog-warning-symbolic",
            (0.9, 0.6, 0.1),
        ),
        (
            MARK_CATEGORIES[2],
            "dialog-information-symbolic",
            (0.2, 0.5, 0.9),
        ),
    ] {
        let attributes = sourceview5::MarkAttributes::new();
        attributes.set_icon_name(icon);
        attributes.set_background(&gdk::RGBA::new(color.0, color.1, color.2, 0.18));
        attributes.connect_query_tooltip_text(|_, mark| {
            MARK_MESSAGES.with(|m| {
                let mut m = m.borrow_mut();
                m.retain(|(weak, _)| weak.upgrade().is_some());
                m.iter()
                    .find(|(weak, _)| weak.upgrade().as_ref() == Some(mark))
                    .map(|(_, text)| text.clone())
                    .unwrap_or_default()
            })
        });
        view.set_mark_attributes(category, &attributes, 10);
    }
    setup_folding(&view, &buffer);
    view
}

// --- Faltung ------------------------------------------------------------------------
//
// GtkSourceView 5 faltet nicht selbst. Die Bereiche kommen aus
// `code::fold_regions`; Pfeile stehen als Marken in der Markenleiste (Klick
// faltet), gefaltete Zeilen verbirgt eine unsichtbare Textmarke. Welche
// Bereiche gefaltet sind, merken sich Textmarken an der Startzeile – so
// überlebt der Zustand Bearbeitungen darüber.

const FOLD_OPEN: &str = "ruf-fold-open";
const FOLD_CLOSED: &str = "ruf-fold-closed";
const FOLDED_TAG: &str = "ruf-folded";

#[derive(Default)]
struct FoldState {
    regions: Vec<Fold>,
    /// Marken am Rand (Fehler usw.); in gefalteten Bereichen stehen sie an
    /// der Startzeile.
    line_marks: Vec<LineMark>,
    /// Anker der gefalteten Bereiche (Anfang der Startzeile).
    anchors: Vec<gtk::TextMark>,
}

type FoldStates = Vec<(glib::WeakRef<sourceview5::Buffer>, Rc<RefCell<FoldState>>)>;

thread_local! {
    static FOLDS: RefCell<FoldStates> =
        const { RefCell::new(Vec::new()) };
}

fn fold_state(buffer: &sourceview5::Buffer) -> Option<Rc<RefCell<FoldState>>> {
    FOLDS.with(|f| {
        let mut f = f.borrow_mut();
        f.retain(|(weak, _)| weak.upgrade().is_some());
        f.iter()
            .find(|(weak, _)| weak.upgrade().as_ref() == Some(buffer))
            .map(|(_, state)| state.clone())
    })
}

fn setup_folding(view: &sourceview5::View, buffer: &sourceview5::Buffer) {
    let tag = gtk::TextTag::builder()
        .name(FOLDED_TAG)
        .invisible(true)
        .build();
    buffer.tag_table().add(&tag);
    for (category, icon) in [
        (FOLD_OPEN, "pan-down-symbolic"),
        (FOLD_CLOSED, "pan-end-symbolic"),
    ] {
        let attributes = sourceview5::MarkAttributes::new();
        attributes.set_icon_name(icon);
        view.set_mark_attributes(category, &attributes, 0);
    }
    FOLDS.with(|f| f.borrow_mut().push((buffer.downgrade(), Rc::default())));
    buffer.connect_changed(refresh_folds);
    buffer.connect_language_notify(refresh_folds);
    // Cursor in verborgenen Zeilen (Sprung, Suche): Bereich aufklappen.
    buffer.connect_cursor_position_notify(|buffer| {
        let line = buffer.iter_at_mark(&buffer.get_insert()).line() as usize + 1;
        let Some(state) = fold_state(buffer) else {
            return;
        };
        let hidden: Vec<usize> = state
            .borrow()
            .regions
            .iter()
            .filter(|r| r.folded && r.start < line && line < r.end)
            .map(|r| r.start)
            .collect();
        for start in hidden {
            set_folded(buffer, start, false);
        }
    });
    // Eigene Zeilennummern: die eingebauten zeichnen verborgene Zeilen
    // übereinander. Diese lassen sie leer; die Cursorzeile ist fett.
    let numbers = sourceview5::GutterRendererText::new();
    numbers.set_xalign(1.0);
    numbers.set_xpad(6);
    numbers.set_alignment_mode(sourceview5::GutterRendererAlignmentMode::First);
    let weak = buffer.downgrade();
    numbers.connect_query_data(move |renderer, lines, line| {
        let Some(buffer) = weak.upgrade() else {
            return;
        };
        let number = line as usize + 1;
        let hidden = fold_state(&buffer).is_some_and(|state| {
            state
                .borrow()
                .regions
                .iter()
                .any(|r| r.folded && r.start < number && number < r.end)
        });
        let cursor = lines
            .downcast_ref::<sourceview5::GutterLines>()
            .is_some_and(|l| l.is_cursor(line));
        if hidden {
            renderer.set_text("");
        } else if cursor {
            renderer.set_markup(&format!("<b>{number}</b>"));
        } else {
            renderer.set_text(&number.to_string());
        }
    });
    // Die Bindung liefert die schwebende Referenz; ohne sie zu übernehmen
    // nähme die Randleiste sie an sich und Rust gäbe sie ein zweites Mal frei.
    // SAFETY: `numbers` ist ein gültiges GObject.
    unsafe {
        let object = numbers.as_ptr() as *mut glib::gobject_ffi::GObject;
        if glib::gobject_ffi::g_object_is_floating(object as *mut _) != 0 {
            glib::gobject_ffi::g_object_ref_sink(object as *mut _);
        }
    }
    sourceview5::prelude::ViewExt::gutter(view, gtk::TextWindowType::Left).insert(&numbers, -30);
    let fit = {
        let numbers = numbers.clone();
        move |buffer: &sourceview5::Buffer| {
            let digits = buffer.line_count().max(1).to_string().len().max(2);
            let (width, _) = numbers.measure_markup(&format!("<b>{}</b>", "9".repeat(digits)));
            numbers.set_width_request(width + 12);
        }
    };
    fit(buffer);
    buffer.connect_changed(fit);
    view.connect_line_mark_activated(|view, iter, button, _, _| {
        if button != 1 {
            return;
        }
        let Ok(buffer) = view.buffer().downcast::<sourceview5::Buffer>() else {
            return;
        };
        let line = iter.line() as usize + 1;
        let folded = fold_state(&buffer).and_then(|state| {
            state
                .borrow()
                .regions
                .iter()
                .find(|r| r.start == line)
                .map(|r| r.folded)
        });
        if let Some(folded) = folded {
            set_folded(&buffer, line, !folded);
        }
    });
}

/// Bereiche neu bestimmen, Pfeile und verborgene Zeilen neu setzen.
fn refresh_folds(buffer: &sourceview5::Buffer) {
    let Some(state) = fold_state(buffer) else {
        return;
    };
    let mut state = state.borrow_mut();
    let enabled = buffer
        .language()
        .is_some_and(|l| folds_language(l.id().as_str()));
    let (start, end) = buffer.bounds();
    let mut regions = if enabled {
        fold_regions(&buffer.text(&start, &end, true))
    } else {
        Vec::new()
    };
    // Anker, deren Zeile keinen Bereich mehr beginnt, verwerfen.
    let mut folded = Vec::new();
    state.anchors.retain(|mark| {
        let line = buffer.iter_at_mark(mark).line() as usize + 1;
        let keep = !folded.contains(&line) && regions.iter().any(|r| r.start == line);
        if keep {
            folded.push(line);
        } else {
            buffer.delete_mark(mark);
        }
        keep
    });
    buffer.remove_tag_by_name(FOLDED_TAG, &start, &end);
    buffer.remove_source_marks(&start, &end, Some(FOLD_OPEN));
    buffer.remove_source_marks(&start, &end, Some(FOLD_CLOSED));
    for region in &mut regions {
        region.folded = folded.contains(&region.start);
    }
    let folded_ranges: Vec<(usize, usize)> = regions
        .iter()
        .filter(|r| r.folded)
        .map(|r| (r.start, r.end))
        .collect();
    for region in &regions {
        let Some(at) = buffer.iter_at_line(region.start as i32 - 1) else {
            continue;
        };
        let category = if region.folded {
            FOLD_CLOSED
        } else {
            FOLD_OPEN
        };
        // Pfeile verborgener Zeilen lägen übereinander.
        let hidden = folded_ranges
            .iter()
            .any(|(s, e)| *s < region.start && region.start < *e);
        if !hidden {
            buffer.create_source_mark(None, category, &at);
        }
        if region.folded
            && let (Some(from), Some(to)) = (
                buffer.iter_at_line(region.start as i32),
                buffer.iter_at_line(region.end as i32 - 1),
            )
        {
            buffer.apply_tag_by_name(FOLDED_TAG, &from, &to);
        }
    }
    state.regions = regions;
    place_line_marks(buffer, &state);
}

/// Setzt die Marken am Rand; Marken verborgener Zeilen an die Startzeile des
/// äußersten gefalteten Bereichs.
fn place_line_marks(buffer: &sourceview5::Buffer, state: &FoldState) {
    let (start, end) = buffer.bounds();
    for category in MARK_CATEGORIES {
        buffer.remove_source_marks(&start, &end, Some(category));
    }
    for mark in &state.line_marks {
        let line = state
            .regions
            .iter()
            .find(|r| r.folded && r.start < mark.line && mark.line < r.end)
            .map_or(mark.line, |r| r.start);
        let Some(iter) = buffer.iter_at_line(line.saturating_sub(1) as i32) else {
            continue;
        };
        let source_mark = buffer.create_source_mark(None, mark_category(mark.kind), &iter);
        MARK_MESSAGES.with(|m| {
            m.borrow_mut()
                .push((source_mark.downgrade(), mark.message.clone()))
        });
    }
}

fn set_folded(buffer: &sourceview5::Buffer, line: usize, folded: bool) {
    let Some(state) = fold_state(buffer) else {
        return;
    };
    // Cursor aus dem Bereich, der gleich verschwindet (sonst klappte er
    // sofort wieder auf): ans Ende der Startzeile.
    let end = state
        .borrow()
        .regions
        .iter()
        .find(|r| r.start == line)
        .map(|r| r.end);
    let cursor = buffer.iter_at_mark(&buffer.get_insert()).line() as usize + 1;
    if folded
        && let Some(end) = end
        && line < cursor
        && cursor < end
        && let Some(mut at) = buffer.iter_at_line(line as i32 - 1)
    {
        at.forward_to_line_end();
        buffer.place_cursor(&at);
    }
    {
        let mut state = state.borrow_mut();
        let Some(region) = state.regions.iter().find(|r| r.start == line).copied() else {
            return;
        };
        if region.folded == folded {
            return;
        }
        if folded {
            if let Some(at) = buffer.iter_at_line(line as i32 - 1) {
                state.anchors.push(buffer.create_mark(None, &at, true));
            }
        } else {
            state.anchors.retain(|mark| {
                let at = buffer.iter_at_mark(mark).line() as usize + 1;
                if at == line {
                    buffer.delete_mark(mark);
                }
                at != line
            });
        }
    }
    refresh_folds(buffer);
}

thread_local! {
    /// Die offene Vorschlagsliste (höchstens eine).
    static COMPLETION: RefCell<Option<glib::WeakRef<gtk::Popover>>> = const { RefCell::new(None) };
}

fn close_completions() {
    if let Some(popover) = COMPLETION
        .with(|c| c.borrow_mut().take())
        .and_then(|w| w.upgrade())
    {
        popover.popdown();
    }
}

/// Ersetzt das Bezeichner-Stück vor dem Cursor durch `text`.
fn replace_word_before_cursor(view: &sourceview5::View, text: &str) {
    let buffer = view.buffer();
    let end = buffer.iter_at_mark(&buffer.get_insert());
    let mut start = end;
    while start.backward_char() {
        let c = start.char();
        if !(c.is_alphanumeric() || c == '_') {
            start.forward_char();
            break;
        }
    }
    let (mut start, mut end) = (start, end);
    buffer.begin_user_action();
    buffer.delete(&mut start, &mut end);
    buffer.insert(&mut start, text);
    buffer.end_user_action();
}
