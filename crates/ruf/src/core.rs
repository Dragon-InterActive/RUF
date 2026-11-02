//! Der plattformunabhängige Kern.
//!
//! Hier liegt alles, was auf allen Systemen gleich funktionieren muss:
//!
//! * **Registry** aller Widgets (`Id` → [`Node`]): Art, nativer Verweis,
//!   Eltern/Kinder, zwischengespeicherte Eigenschaften, Callbacks.
//! * **Lebensdauer:** tote Verweise (Widget zerstört) machen nichts und
//!   liefern Standardwerte – es gibt keine hängenden Zeiger.
//! * **Signale:** Nach jeder Änderung – per Code oder durch den Anwender –
//!   vergleicht [`sync`] den nativen Zustand mit dem zuletzt gemeldeten und
//!   ruft genau dann die Callbacks auf, wenn sich etwas geändert hat. Damit
//!   feuern Signale auf allen Backends gleich, auch bei Änderungen per Code.
//! * **Semantik**, die klassische Toolkits liefern, GTK aber anders
//!   handhabt: Optionsfelder je Elternteil exklusiv, Combobox wählt den
//!   ersten Eintrag, Wertebereiche begrenzen den Wert, Fenster werden beim
//!   Schließen nur ausgeblendet, nach dem letzten Fenster endet `exec()`.
//! * **Timer, Tastenkürzel, verzögertes Löschen** und Panik-Schutz.
//!
//! Regel für Backends: Während der Kern ein Backend aufruft
//! ([`setter`]), werden dessen Zustandsmeldungen ignoriert – der Kern
//! gleicht danach selbst ab. So entstehen keine doppelten Signale.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::backend::{Backend, Native, Platform};
use crate::canvas::{KeyEvent, MouseEvent, Shape};
use crate::code::LineMark;
use crate::geometry::Point;
use crate::geometry::{Rect, Size};
use crate::menu::{Menu, MenuBar};
use crate::style::Color;
use crate::text::{HighlightRule, highlight};
use crate::tree::{TreeItem, visible_rows};

/// Eindeutige Nummer eines Widgets (wird nie wiederverwendet).
pub(crate) type Id = u64;

/// Die Arten nativer Widgets, die RUF erzeugen kann.
///
/// Die Zahlenwerte sind stabil (Designer-Dateien) und werden nie umnummeriert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WidgetKind {
    /// Ein Top-Level-Fenster.
    Window = 0,
    /// Ein schlichter Container, z. B. für zusammengesetzte Komponenten.
    Container = 1,
    PushButton = 2,
    Label = 3,
    /// Einzeiliges Eingabefeld.
    LineEdit = 4,
    CheckBox = 5,
    RadioButton = 6,
    /// Container mit Rahmen und Titel.
    GroupBox = 7,
    /// Mehrzeiliger Text.
    TextArea = 8,
    ComboBox = 9,
    ListBox = 10,
    SpinBox = 11,
    /// Schieberegler (waagerecht).
    Slider = 12,
    ProgressBar = 13,
    /// Registerkarten.
    TabView = 14,
    /// Scrollbereich mit einem Container als Inhalt.
    ScrollArea = 15,
    /// Zeichenfläche: malt [`crate::Shape`]s, meldet Maus und Tastatur.
    Canvas = 16,
    /// Baumansicht mit auf- und zuklappbaren Knoten.
    TreeView = 17,
    /// Code-Editor: Hervorhebung, Zeilennummern, Marken am Rand.
    CodeEditor = 18,
}

impl WidgetKind {
    /// Kann diese Art Kinder aufnehmen?
    pub fn is_container(self) -> bool {
        matches!(
            self,
            WidgetKind::Window | WidgetKind::Container | WidgetKind::GroupBox
        )
    }

    fn is_checkable(self) -> bool {
        matches!(
            self,
            WidgetKind::PushButton | WidgetKind::CheckBox | WidgetKind::RadioButton
        )
    }

    fn has_range(self) -> bool {
        matches!(
            self,
            WidgetKind::SpinBox | WidgetKind::Slider | WidgetKind::ProgressBar
        )
    }

    /// Text wird im Steuerelement bearbeitet (sonst liegt er im Kern).
    fn has_native_text(self) -> bool {
        matches!(
            self,
            WidgetKind::LineEdit | WidgetKind::TextArea | WidgetKind::CodeEditor
        )
    }
}

/// Was ein Backend über eine Benutzeraktion meldet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Notify {
    /// Der Anwender hat einen Button, eine Checkbox oder ein Optionsfeld betätigt.
    Clicked,
    /// Text, Auswahl oder Wert wurden vom Anwender geändert.
    Changed,
    /// Enter in einem Eingabefeld.
    Activated,
    /// Neue Größe (Fenster: Innenbereich; Container: zugeteilter Platz).
    Resized(Size),
    /// Maus auf einer Zeichenfläche.
    Mouse(MouseEvent),
    /// Taste auf einer Zeichenfläche.
    Key(KeyEvent),
    /// Menüeintrag mit dieser Kennung gewählt (gemeldet für das Fenster).
    Menu(String),
    /// Text von der Eingabemethode (getippt, auch zusammengesetzt wie `ê`).
    Text(String),
    /// Knoten in Zeile `n` der Baumansicht auf-/zuklappen.
    TreeToggled(usize),
    /// Klick in die Randleiste des Code-Editors.
    Gutter(crate::code::GutterClick),
}

type Callback<F> = Rc<RefCell<F>>;

#[derive(Clone)]
enum Handler {
    Clicked(Callback<dyn FnMut()>),
    Toggled(Callback<dyn FnMut(bool)>),
    TextChanged(Callback<dyn FnMut(String)>),
    ReturnPressed(Callback<dyn FnMut()>),
    ValueChanged(Callback<dyn FnMut(i32)>),
    IndexChanged(Callback<dyn FnMut(i32)>),
    Resized(Callback<dyn FnMut(i32, i32)>),
    CloseRequested(Callback<dyn FnMut() -> bool>),
    Mouse(Callback<dyn FnMut(MouseEvent)>),
    Key(Callback<dyn FnMut(KeyEvent)>),
    Menu(Callback<dyn FnMut(String)>),
    TextInput(Callback<dyn FnMut(String)>),
    TreeSelected(Callback<dyn FnMut(String)>),
    TreeActivated(Callback<dyn FnMut(String)>),
    TreeToggled(Callback<dyn FnMut(String, bool)>),
    Gutter(Callback<dyn FnMut(crate::code::GutterClick)>),
}

/// Ein Signal, das ein Widget anbieten kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Signal {
    Clicked,
    Toggled,
    TextChanged,
    ReturnPressed,
    ValueChanged,
    IndexChanged,
    Resized,
    CloseRequested,
    Mouse,
    Key,
    TextInput,
    Menu,
    TreeSelected,
    TreeActivated,
    TreeToggled,
    Gutter,
}

impl Signal {
    /// Besitzt `kind` dieses Signal? (Sonst liefert `on_…` `false`.)
    pub(crate) fn supported_by(self, kind: WidgetKind) -> bool {
        use WidgetKind as K;
        match self {
            Signal::Clicked | Signal::Toggled => kind.is_checkable(),
            Signal::TextChanged => {
                matches!(
                    kind,
                    K::LineEdit | K::TextArea | K::ComboBox | K::CodeEditor
                )
            }
            Signal::ReturnPressed => kind == K::LineEdit,
            Signal::ValueChanged => kind.has_range(),
            Signal::IndexChanged => matches!(kind, K::ComboBox | K::ListBox | K::TabView),
            Signal::Resized | Signal::CloseRequested => true,
            Signal::Mouse => matches!(kind, K::Canvas | K::CodeEditor | K::TreeView),
            Signal::Key | Signal::TextInput => kind == K::Canvas,
            Signal::Menu => kind == K::Window,
            Signal::TreeSelected | Signal::TreeActivated | Signal::TreeToggled => {
                kind == K::TreeView
            }
            Signal::Gutter => kind == K::CodeEditor,
        }
    }
}

/// Zuletzt gemeldeter Zustand – Grundlage der Änderungserkennung.
#[derive(Debug, Clone, Default)]
struct Snapshot {
    text: String,
    checked: bool,
    index: i32,
    value: i32,
}

/// Ein Widget im Kern.
pub(crate) struct Node {
    pub kind: WidgetKind,
    pub native: Native,
    pub parent: Option<Id>,
    pub children: Vec<Id>,
    pub name: String,
    pub tool_tip: String,
    pub style_sheet: String,
    /// Text nicht bearbeitbarer Widgets (Titel, Beschriftung, Label).
    pub text: String,
    pub placeholder: String,
    pub geometry: Rect,
    /// Ausdrücklich ausgeblendet (Fenster starten ausgeblendet).
    pub hidden: bool,
    pub enabled: bool,
    pub password: bool,
    pub read_only: bool,
    pub editable: bool,
    pub word_wrap: bool,
    pub monospace: bool,
    pub vertical: bool,
    pub items: Vec<String>,
    /// Seiten einer Registerkarte.
    pub tabs: Vec<Id>,
    pub tab_titles: Vec<String>,
    /// Inhalt eines Scrollbereichs.
    pub content: Option<Id>,
    pub content_min: Size,
    pub minimum: i32,
    pub maximum: i32,
    pub rules: Vec<HighlightRule>,
    pub marked: Option<(usize, Color)>,
    /// Was eine Zeichenfläche malt.
    pub shapes: Vec<Shape>,
    /// Knoten einer Baumansicht und die Kennungen der sichtbaren Zeilen.
    pub tree: Vec<TreeItem>,
    pub tree_rows: Vec<String>,
    pub tree_selected: Option<String>,
    /// Menüleiste eines Fensters.
    pub menu_bar: MenuBar,
    /// Sprache und Zeilenmarken eines Code-Editors.
    pub language: String,
    pub line_marks: Vec<LineMark>,
    pub diff_marks: Vec<crate::code::DiffMark>,
    pub breakpoint_marks: Vec<crate::code::BreakpointMark>,
    /// Zeile (ab 1), an der der Debugger steht.
    pub execution_line: Option<usize>,
    /// Bild eines Knopfs.
    pub image: Option<crate::Image>,
    last: Snapshot,
    handlers: Vec<Handler>,
}

#[derive(Default)]
struct TimerEntry {
    single_shot: bool,
    once: bool,
    active: bool,
    interval: i32,
    handlers: Vec<Callback<dyn FnMut()>>,
}

#[derive(Default)]
struct Core {
    nodes: HashMap<Id, Node>,
    next_id: Id,
    app: bool,
    running: bool,
    deferred: Vec<Id>,
    timers: HashMap<u64, TimerEntry>,
    shortcuts: HashMap<u64, Callback<dyn FnMut()>>,
}

thread_local! {
    static CORE: RefCell<Core> = RefCell::new(Core { next_id: 1, ..Default::default() });
    static SETTER_DEPTH: Cell<u32> = const { Cell::new(0) };
}

fn with_core<R>(f: impl FnOnce(&mut Core) -> R) -> R {
    CORE.with(|c| f(&mut c.borrow_mut()))
}

fn next_id() -> Id {
    with_core(|c| {
        let id = c.next_id;
        c.next_id += 1;
        id
    })
}

// --- Panik-Schutz --------------------------------------------------------------

/// Eine Panik in einem Callback darf weder die Event-Schleife noch die
/// übrigen Handler mitreißen: sie wird abgefangen und gemeldet.
pub(crate) fn guarded(f: impl FnOnce()) {
    if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        let message = panic
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unbekannte Panik".to_owned());
        eprintln!("ruf: Panik in einem Callback abgefangen: {message}");
    }
}

// --- Anwendung -------------------------------------------------------------------

pub(crate) fn app_exists() -> bool {
    with_core(|c| c.app)
}

pub(crate) fn app_init() {
    Platform::init();
    with_core(|c| c.app = true);
}

pub(crate) fn app_shutdown() {
    with_core(|c| c.app = false);
}

/// Event-Schleife; endet mit [`quit`] oder wenn das letzte Fenster geschlossen wird.
pub(crate) fn run() {
    with_core(|c| c.running = true);
    flush_deferred();
    Platform::run();
    with_core(|c| c.running = false);
}

pub(crate) fn quit() {
    Platform::quit();
}

pub(crate) fn process_events() {
    Platform::process_events();
    flush_deferred();
}

// --- Registry --------------------------------------------------------------------

pub(crate) fn alive(id: Id) -> bool {
    with_core(|c| c.nodes.contains_key(&id))
}

/// Liest aus einem Widget; `None`, wenn es nicht (mehr) existiert.
pub(crate) fn get<R>(id: Id, f: impl FnOnce(&Node) -> R) -> Option<R> {
    with_core(|c| c.nodes.get(&id).map(f))
}

/// Ändert zwischengespeicherte Werte (ohne Backend-Aufruf).
pub(crate) fn update<R>(id: Id, f: impl FnOnce(&mut Node) -> R) -> Option<R> {
    with_core(|c| c.nodes.get_mut(&id).map(f))
}

pub(crate) fn native(id: Id) -> Option<(Native, WidgetKind)> {
    get(id, |n| (n.native.clone(), n.kind))
}

/// Ruft das Backend auf; Zustandsmeldungen währenddessen werden ignoriert.
pub(crate) fn setter<R>(f: impl FnOnce() -> R) -> R {
    struct Depth;
    impl Drop for Depth {
        fn drop(&mut self) {
            SETTER_DEPTH.with(|d| d.set(d.get() - 1));
        }
    }
    SETTER_DEPTH.with(|d| d.set(d.get() + 1));
    let _depth = Depth;
    f()
}

fn in_setter() -> bool {
    SETTER_DEPTH.with(|d| d.get() > 0)
}

/// Sucht den nächsten Vorfahren, der Kinder aufnehmen kann.
fn container_for(parent: Id) -> Option<(Id, Native)> {
    let mut current = Some(parent);
    while let Some(id) = current {
        let (kind, native, up) = get(id, |n| (n.kind, n.native.clone(), n.parent))?;
        if kind.is_container() {
            return Some((id, native));
        }
        current = up;
    }
    None
}

fn new_node(kind: WidgetKind, native: Native, parent: Option<Id>, geometry: Rect) -> Node {
    Node {
        kind,
        native,
        parent,
        children: Vec::new(),
        name: String::new(),
        tool_tip: String::new(),
        style_sheet: String::new(),
        text: String::new(),
        placeholder: String::new(),
        geometry,
        hidden: kind == WidgetKind::Window && parent.is_none(),
        enabled: true,
        password: false,
        read_only: false,
        editable: false,
        word_wrap: false,
        monospace: false,
        vertical: false,
        items: Vec::new(),
        tabs: Vec::new(),
        tab_titles: Vec::new(),
        content: None,
        content_min: Size::default(),
        minimum: 0,
        maximum: 0,
        rules: Vec::new(),
        marked: None,
        shapes: Vec::new(),
        tree: Vec::new(),
        tree_rows: Vec::new(),
        tree_selected: None,
        menu_bar: Vec::new(),
        language: String::new(),
        line_marks: Vec::new(),
        diff_marks: Vec::new(),
        breakpoint_marks: Vec::new(),
        execution_line: None,
        image: None,
        last: Snapshot {
            index: -1,
            ..Default::default()
        },
        handlers: Vec::new(),
    }
}

fn insert(id: Id, node: Node) {
    let parent = node.parent;
    with_core(|c| {
        c.nodes.insert(id, node);
        if let Some(p) = parent
            && let Some(parent) = c.nodes.get_mut(&p)
        {
            parent.children.push(id);
        }
    });
}

/// Erzeugt ein Widget samt nativem Steuerelement.
///
/// Ein Fenster mit Elternteil bleibt ein eigenes Fenster (gehört aber dem
/// Elternteil). Andere Widgets kommen in den nächsten Container-Vorfahren.
pub(crate) fn create(kind: WidgetKind, parent: Option<Id>) -> Id {
    let id = next_id();
    let host = match (kind, parent) {
        (WidgetKind::Window, _) | (_, None) => None,
        (_, Some(p)) => container_for(p).map(|(_, native)| native),
    };
    let native = setter(|| Platform::create(id, kind, host.as_ref()));
    let geometry = match kind {
        WidgetKind::Window => Rect::new(0, 0, 640, 480),
        _ => Rect::new(0, 0, 100, 30),
    };
    insert(id, new_node(kind, native.clone(), parent, geometry));
    setter(|| Platform::set_geometry(&native, kind, geometry));

    // Vorgaben wie bei klassischen Toolkits.
    match kind {
        WidgetKind::SpinBox | WidgetKind::Slider => init_range(id, 0, 99),
        WidgetKind::ProgressBar => init_range(id, 0, 100),
        WidgetKind::ScrollArea => {
            let content = next_id();
            let content_native = setter(|| Platform::scroll_content(&native, content));
            insert(
                content,
                new_node(
                    WidgetKind::Container,
                    content_native,
                    Some(id),
                    Rect::default(),
                ),
            );
            update(id, |n| n.content = Some(content));
        }
        _ => {}
    }
    id
}

fn init_range(id: Id, minimum: i32, maximum: i32) {
    if let Some((native, kind)) = native(id) {
        update(id, |n| {
            n.minimum = minimum;
            n.maximum = maximum;
        });
        setter(|| {
            Platform::set_range(&native, kind, minimum, maximum);
            Platform::set_value(&native, kind, minimum);
        });
        update(id, |n| n.last.value = minimum);
    }
}

/// Entfernt ein Widget samt Kindern aus der Registry – ohne natives Löschen.
fn forget(id: Id) -> Option<Node> {
    let node = with_core(|c| c.nodes.remove(&id))?;
    for child in &node.children {
        forget(*child);
    }
    if let Some(p) = node.parent {
        update(p, |parent| parent.children.retain(|c| *c != id));
    }
    Some(node)
}

/// Zerstört ein Widget samt Kindern (nativ und in der Registry).
pub(crate) fn destroy(id: Id) {
    if let Some(node) = forget(id) {
        setter(|| Platform::destroy(&node.native, node.kind));
        check_last_window();
    }
}

/// Nach dem Abarbeiten der Ereignisse löschen (siehe `Widget::delete_later`).
pub(crate) fn delete_later(id: Id) {
    set_visible(id, false);
    with_core(|c| c.deferred.push(id));
    Platform::post(Box::new(flush_deferred));
}

pub(crate) fn flush_deferred() {
    let ids = with_core(|c| std::mem::take(&mut c.deferred));
    for id in ids {
        destroy(id);
    }
}

// --- Sichtbarkeit und Fenster -------------------------------------------------------

pub(crate) fn set_visible(id: Id, visible: bool) {
    let Some((native, kind)) = native(id) else {
        return;
    };
    update(id, |n| n.hidden = !visible);
    setter(|| Platform::set_visible(&native, kind, visible));
    if !visible {
        check_last_window();
    }
}

/// Sichtbar = selbst nicht ausgeblendet und alle Vorfahren ebenso.
pub(crate) fn is_visible(id: Id) -> bool {
    let mut current = Some(id);
    let mut found = false;
    while let Some(i) = current {
        match get(i, |n| (n.hidden, n.parent)) {
            Some((false, parent)) => {
                found = true;
                current = parent;
            }
            _ => return false,
        }
    }
    found
}

/// Das Backend meldet "Fenster schließen" (Klick auf X) – oder `close()`.
/// Alle Handler müssen zustimmen; dann wird das Fenster ausgeblendet.
/// Liefert, ob geschlossen wurde.
pub(crate) fn close_requested(id: Id) -> bool {
    let handlers = handlers(id, |h| match h {
        Handler::CloseRequested(f) => Some(f.clone()),
        _ => None,
    });
    let mut accept = true;
    for handler in handlers {
        // Im Zweifel (Re-Entranz, Panik) schließen lassen – ein Fenster, das
        // sich nie mehr schließen lässt, wäre schlimmer.
        if let Ok(mut f) = handler.try_borrow_mut() {
            let mut answer = true;
            guarded(|| answer = f());
            accept &= answer;
        }
    }
    if accept {
        set_visible(id, false);
    }
    accept
}

/// Ist kein Fenster mehr sichtbar, endet die Event-Schleife.
fn check_last_window() {
    let (running, any_visible) = with_core(|c| {
        (
            c.running,
            c.nodes
                .values()
                .any(|n| n.kind == WidgetKind::Window && !n.hidden),
        )
    });
    if running && !any_visible {
        Platform::quit();
    }
}

// --- Callbacks ---------------------------------------------------------------------

fn handlers<T>(id: Id, pick: impl Fn(&Handler) -> Option<T>) -> Vec<T> {
    get(id, |n| n.handlers.iter().filter_map(&pick).collect()).unwrap_or_default()
}

pub(crate) fn connect_clicked(id: Id, f: impl FnMut() + 'static) -> bool {
    add_handler(
        id,
        Signal::Clicked,
        Handler::Clicked(Rc::new(RefCell::new(f))),
    )
}

pub(crate) fn connect_toggled(id: Id, f: impl FnMut(bool) + 'static) -> bool {
    add_handler(
        id,
        Signal::Toggled,
        Handler::Toggled(Rc::new(RefCell::new(f))),
    )
}

pub(crate) fn connect_text_changed(id: Id, f: impl FnMut(String) + 'static) -> bool {
    add_handler(
        id,
        Signal::TextChanged,
        Handler::TextChanged(Rc::new(RefCell::new(f))),
    )
}

pub(crate) fn connect_return_pressed(id: Id, f: impl FnMut() + 'static) -> bool {
    add_handler(
        id,
        Signal::ReturnPressed,
        Handler::ReturnPressed(Rc::new(RefCell::new(f))),
    )
}

pub(crate) fn connect_value_changed(id: Id, f: impl FnMut(i32) + 'static) -> bool {
    add_handler(
        id,
        Signal::ValueChanged,
        Handler::ValueChanged(Rc::new(RefCell::new(f))),
    )
}

pub(crate) fn connect_index_changed(id: Id, f: impl FnMut(i32) + 'static) -> bool {
    add_handler(
        id,
        Signal::IndexChanged,
        Handler::IndexChanged(Rc::new(RefCell::new(f))),
    )
}

pub(crate) fn connect_resized(id: Id, f: impl FnMut(i32, i32) + 'static) -> bool {
    add_handler(
        id,
        Signal::Resized,
        Handler::Resized(Rc::new(RefCell::new(f))),
    )
}

pub(crate) fn connect_close_requested(id: Id, f: impl FnMut() -> bool + 'static) -> bool {
    add_handler(
        id,
        Signal::CloseRequested,
        Handler::CloseRequested(Rc::new(RefCell::new(f))),
    )
}

fn add_handler(id: Id, signal: Signal, handler: Handler) -> bool {
    update(id, |n| {
        if signal.supported_by(n.kind) {
            n.handlers.push(handler);
            true
        } else {
            false
        }
    })
    .unwrap_or(false)
}

/// Ruft Handler auf – ohne die Registry zu sperren; ein Handler, der gerade
/// läuft (Re-Entranz), wird übersprungen.
fn fire<F: ?Sized>(callbacks: Vec<Callback<F>>, call: impl Fn(&mut F)) {
    for callback in callbacks {
        if let Ok(mut f) = callback.try_borrow_mut() {
            guarded(|| call(&mut f));
        }
    }
}

fn fire_clicked(id: Id) {
    fire(
        handlers(id, |h| match h {
            Handler::Clicked(f) => Some(f.clone()),
            _ => None,
        }),
        |f| f(),
    );
}

fn fire_toggled(id: Id, value: bool) {
    fire(
        handlers(id, |h| match h {
            Handler::Toggled(f) => Some(f.clone()),
            _ => None,
        }),
        |f| f(value),
    );
}

fn fire_text(id: Id, text: &str) {
    fire(
        handlers(id, |h| match h {
            Handler::TextChanged(f) => Some(f.clone()),
            _ => None,
        }),
        |f| f(text.to_owned()),
    );
}

fn fire_return(id: Id) {
    fire(
        handlers(id, |h| match h {
            Handler::ReturnPressed(f) => Some(f.clone()),
            _ => None,
        }),
        |f| f(),
    );
}

fn fire_value(id: Id, value: i32) {
    fire(
        handlers(id, |h| match h {
            Handler::ValueChanged(f) => Some(f.clone()),
            _ => None,
        }),
        |f| f(value),
    );
}

fn fire_index(id: Id, index: i32) {
    fire(
        handlers(id, |h| match h {
            Handler::IndexChanged(f) => Some(f.clone()),
            _ => None,
        }),
        |f| f(index),
    );
}

fn fire_resized(id: Id, size: Size) {
    fire(
        handlers(id, |h| match h {
            Handler::Resized(f) => Some(f.clone()),
            _ => None,
        }),
        |f| f(size.width, size.height),
    );
}

// --- Änderungserkennung ---------------------------------------------------------------

/// Text, den der Anwender sieht: bearbeitbarer Inhalt, Combobox-Text oder
/// der zwischengespeicherte Text.
pub(crate) fn current_text(id: Id) -> String {
    let Some((native, kind, editable, items, cached)) = get(id, |n| {
        (
            n.native.clone(),
            n.kind,
            n.editable,
            n.items.clone(),
            n.text.clone(),
        )
    }) else {
        return String::new();
    };
    match kind {
        k if k.has_native_text() => Platform::text(&native, kind),
        WidgetKind::ComboBox if editable => Platform::text(&native, kind),
        WidgetKind::ComboBox => {
            let index = Platform::current_index(&native, kind);
            usize::try_from(index)
                .ok()
                .and_then(|i| items.get(i).cloned())
                .unwrap_or_default()
        }
        _ => cached,
    }
}

/// Vergleicht den nativen Zustand mit dem zuletzt gemeldeten und feuert
/// die passenden Signale. Liefert, ob sich der Schaltzustand geändert hat.
pub(crate) fn sync(id: Id) -> bool {
    let Some((native, kind)) = native(id) else {
        return false;
    };
    let mut toggled = None;
    let mut text = None;
    let mut index = None;
    let mut value = None;

    match kind {
        k if k.is_checkable() => {
            let checked = Platform::is_checked(&native, kind);
            if update(id, |n| {
                std::mem::replace(&mut n.last.checked, checked) != checked
            }) == Some(true)
            {
                toggled = Some(checked);
            }
        }
        WidgetKind::LineEdit | WidgetKind::TextArea | WidgetKind::CodeEditor => {
            let now = Platform::text(&native, kind);
            if update(id, |n| n.last.text != now).unwrap_or(false) {
                update(id, |n| n.last.text = now.clone());
                if kind == WidgetKind::TextArea {
                    rehighlight(id, &now);
                }
                text = Some(now);
            }
        }
        WidgetKind::ComboBox | WidgetKind::ListBox | WidgetKind::TabView => {
            let now = Platform::current_index(&native, kind);
            if update(id, |n| std::mem::replace(&mut n.last.index, now) != now) == Some(true) {
                index = Some(now);
            }
            if kind == WidgetKind::ComboBox {
                let now = current_text(id);
                if update(id, |n| n.last.text != now).unwrap_or(false) {
                    update(id, |n| n.last.text = now.clone());
                    text = Some(now);
                }
            }
        }
        k if k.has_range() => {
            let now = Platform::value(&native, kind);
            if update(id, |n| std::mem::replace(&mut n.last.value, now) != now) == Some(true) {
                value = Some(now);
            }
        }
        _ => {}
    }

    if let Some(checked) = toggled {
        if checked && kind == WidgetKind::RadioButton {
            make_exclusive(id);
        }
        fire_toggled(id, checked);
    }
    if let Some(i) = index {
        fire_index(id, i);
    }
    if let Some(t) = text {
        fire_text(id, &t);
    }
    if let Some(v) = value {
        fire_value(id, v);
    }
    toggled.is_some()
}

/// Optionsfelder mit demselben Elternteil schließen sich gegenseitig aus.
fn make_exclusive(id: Id) {
    let Some(parent) = get(id, |n| n.parent).flatten() else {
        return;
    };
    let siblings: Vec<Id> = get(parent, |p| p.children.clone()).unwrap_or_default();
    for sibling in siblings.into_iter().filter(|s| *s != id) {
        if let Some((native, WidgetKind::RadioButton)) = native(sibling) {
            if Platform::is_checked(&native, WidgetKind::RadioButton) {
                setter(|| Platform::set_checked(&native, WidgetKind::RadioButton, false));
            }
            sync(sibling);
        }
    }
}

/// Meldung eines Backends über eine Benutzeraktion.
pub(crate) fn notify(id: Id, event: Notify) {
    if let Notify::Resized(size) = event {
        resized(id, size);
        return;
    }
    // Während der Kern selbst ändert, gleicht er danach selbst ab.
    if in_setter() || !alive(id) {
        return;
    }
    match event {
        Notify::Clicked => {
            sync(id);
            fire_clicked(id);
        }
        Notify::Changed => {
            if get(id, |n| n.kind) == Some(WidgetKind::TreeView) {
                tree_sync(id);
            } else {
                sync(id);
            }
        }
        Notify::Activated => {
            if get(id, |n| n.kind) == Some(WidgetKind::TreeView) {
                if let Some(item) = get(id, |n| n.tree_selected.clone()).flatten() {
                    fire_string(id, item, |h| match h {
                        Handler::TreeActivated(f) => Some(f.clone()),
                        _ => None,
                    });
                }
            } else {
                fire_return(id);
            }
        }
        Notify::Resized(_) => {}
        Notify::Mouse(event) => fire(
            handlers(id, |h| match h {
                Handler::Mouse(f) => Some(f.clone()),
                _ => None,
            }),
            |f| f(event),
        ),
        Notify::Key(event) => fire(
            handlers(id, |h| match h {
                Handler::Key(f) => Some(f.clone()),
                _ => None,
            }),
            |f| f(event),
        ),
        Notify::Text(text) => fire_string(id, text, |h| match h {
            Handler::TextInput(f) => Some(f.clone()),
            _ => None,
        }),
        Notify::Menu(item) => fire_string(id, item, |h| match h {
            Handler::Menu(f) => Some(f.clone()),
            _ => None,
        }),
        Notify::TreeToggled(row) => {
            if let Some(item) = get(id, |n| n.tree_rows.get(row).cloned()).flatten() {
                toggle_tree_item(id, &item);
            }
        }
        Notify::Gutter(click) => fire(
            handlers(id, |h| match h {
                Handler::Gutter(f) => Some(f.clone()),
                _ => None,
            }),
            |f| f(click),
        ),
    }
}

fn resized(id: Id, size: Size) {
    let changed = update(id, |n| {
        let changed = n.geometry.size() != size;
        n.geometry.width = size.width;
        n.geometry.height = size.height;
        changed
    });
    if changed == Some(true) {
        fire_resized(id, size);
    }
}

// --- Setter mit Kern-Semantik -----------------------------------------------------------

pub(crate) fn set_geometry(id: Id, rect: Rect) {
    let Some((native, kind)) = native(id) else {
        return;
    };
    let old = get(id, |n| n.geometry).unwrap_or_default();
    update(id, |n| n.geometry = rect);
    setter(|| Platform::set_geometry(&native, kind, rect));
    if old.size() != rect.size() {
        fire_resized(id, rect.size());
    }
}

/// Schaltzustand setzen (Optionsfelder werden exklusiv).
pub(crate) fn set_checked(id: Id, checked: bool) {
    let Some((native, kind)) = native(id) else {
        return;
    };
    setter(|| Platform::set_checked(&native, kind, checked));
    sync(id);
}

/// Klick per Code: schaltet um wie ein Mausklick und feuert die Signale.
pub(crate) fn click(id: Id) -> bool {
    let Some((native, kind, enabled)) = get(id, |n| (n.native.clone(), n.kind, n.enabled)) else {
        return false;
    };
    if !kind.is_checkable() {
        return false;
    }
    if !enabled {
        return true;
    }
    match kind {
        WidgetKind::CheckBox => {
            let checked = Platform::is_checked(&native, kind);
            set_checked(id, !checked);
        }
        WidgetKind::RadioButton => set_checked(id, true),
        _ => {}
    }
    fire_clicked(id);
    true
}

/// Setzt Text bzw. Titel und feuert ggf. `text_changed`.
pub(crate) fn set_text(id: Id, text: &str) {
    let Some((native, kind)) = native(id) else {
        return;
    };
    if kind == WidgetKind::ComboBox {
        set_combo_text(id, text);
        return;
    }
    update(id, |n| n.text = text.to_owned());
    setter(|| Platform::set_text(&native, kind, text));
    sync(id);
}

/// Combobox: editierbar → freier Text; sonst den passenden Eintrag wählen.
fn set_combo_text(id: Id, text: &str) {
    let Some((native, kind, editable, items)) = get(id, |n| {
        (n.native.clone(), n.kind, n.editable, n.items.clone())
    }) else {
        return;
    };
    if editable {
        setter(|| Platform::set_text(&native, kind, text));
        sync(id);
    } else if let Some(i) = items.iter().position(|item| item == text) {
        set_current_index(id, i as i32);
    }
}

pub(crate) fn set_current_index(id: Id, index: i32) {
    let Some((native, kind, count)) = get(id, |n| {
        (n.native.clone(), n.kind, n.items.len().max(n.tabs.len()))
    }) else {
        return;
    };
    let index = if index < 0 || index as usize >= count {
        -1
    } else {
        index
    };
    if kind == WidgetKind::TabView && index < 0 {
        return;
    }
    setter(|| Platform::set_current_index(&native, kind, index));
    sync(id);
}

/// Einträge ändern (Combobox, Liste) mit klassischer Semantik:
/// Combobox wählt beim ersten Eintrag automatisch, die Auswahl folgt
/// dem gewählten Eintrag beim Einfügen/Entfernen.
pub(crate) fn edit_items(id: Id, edit: impl FnOnce(&mut Vec<String>) -> ItemChange) {
    let Some((native, kind)) = native(id) else {
        return;
    };
    if !matches!(kind, WidgetKind::ComboBox | WidgetKind::ListBox) {
        return;
    }
    let current = setter(|| Platform::current_index(&native, kind));
    let edit_text = (kind == WidgetKind::ComboBox && get(id, |n| n.editable) == Some(true))
        .then(|| Platform::text(&native, kind));
    let Some((items, change)) = update(id, |n| {
        let change = edit(&mut n.items);
        (n.items.clone(), change)
    }) else {
        return;
    };
    let len = items.len() as i32;
    let mut index = match change {
        ItemChange::Inserted(at) if current >= 0 && at <= current => current + 1,
        ItemChange::Removed(at) if current > at => current - 1,
        ItemChange::Removed(at) if current == at => current.min(len - 1),
        ItemChange::Replaced => -1,
        _ => current,
    };
    if kind == WidgetKind::ComboBox && index < 0 && len > 0 {
        index = 0;
    }
    if index >= len {
        index = len - 1;
    }
    setter(|| {
        Platform::set_items(&native, kind, &items);
        Platform::set_current_index(&native, kind, index);
        if let Some(text) = &edit_text
            && !matches!(change, ItemChange::Replaced)
        {
            Platform::set_text(&native, kind, text);
        }
    });
    sync(id);
}

/// Art einer Änderung an der Eintragsliste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemChange {
    Inserted(i32),
    Removed(i32),
    Renamed,
    /// Alle Einträge ersetzt bzw. geleert.
    Replaced,
    None,
}

pub(crate) fn set_editable(id: Id, editable: bool) {
    let Some((native, kind, items)) = get(id, |n| (n.native.clone(), n.kind, n.items.clone()))
    else {
        return;
    };
    if kind != WidgetKind::ComboBox {
        return;
    }
    let index = Platform::current_index(&native, kind);
    let replacement = setter(|| Platform::set_editable(&native, editable));
    let native = replacement.unwrap_or(native);
    update(id, |n| {
        n.editable = editable;
        n.native = native.clone();
    });
    if let Some(geometry) = get(id, |n| n.geometry) {
        setter(|| {
            Platform::set_items(&native, kind, &items);
            Platform::set_current_index(&native, kind, index);
            Platform::set_geometry(&native, kind, geometry);
        });
    }
    sync(id);
}

pub(crate) fn set_range(id: Id, minimum: i32, maximum: i32) {
    let Some((native, kind)) = native(id) else {
        return;
    };
    if !kind.has_range() {
        return;
    }
    let maximum = maximum.max(minimum);
    update(id, |n| {
        n.minimum = minimum;
        n.maximum = maximum;
    });
    let value = Platform::value(&native, kind).clamp(minimum, maximum);
    setter(|| {
        Platform::set_range(&native, kind, minimum, maximum);
        Platform::set_value(&native, kind, value);
    });
    sync(id);
}

pub(crate) fn set_value(id: Id, value: i32) {
    let Some((native, kind, minimum, maximum)) =
        get(id, |n| (n.native.clone(), n.kind, n.minimum, n.maximum))
    else {
        return;
    };
    if !kind.has_range() {
        return;
    }
    setter(|| Platform::set_value(&native, kind, value.clamp(minimum, maximum)));
    sync(id);
}

// --- Registerkarten ------------------------------------------------------------------------

pub(crate) fn tab_add(id: Id, title: &str) -> Option<Id> {
    let (native, kind) = native(id)?;
    if kind != WidgetKind::TabView {
        return None;
    }
    let page = next_id();
    let page_native = setter(|| Platform::tab_add(&native, page, title));
    insert(
        page,
        new_node(
            WidgetKind::Container,
            page_native,
            Some(id),
            Rect::default(),
        ),
    );
    update(id, |n| {
        n.tabs.push(page);
        n.tab_titles.push(title.to_owned());
    });
    sync(id);
    Some(page)
}

pub(crate) fn tab_remove(id: Id, index: i32) {
    let Some((native, page)) = get(id, |n| {
        let page = usize::try_from(index)
            .ok()
            .and_then(|i| n.tabs.get(i).copied());
        (n.native.clone(), page)
    }) else {
        return;
    };
    let Some(page) = page else { return };
    update(id, |n| {
        n.tabs.remove(index as usize);
        n.tab_titles.remove(index as usize);
    });
    // Die Seite gehört der Registerkarte; das Backend gibt sie mit frei.
    forget(page);
    setter(|| Platform::tab_remove(&native, index));
    sync(id);
}

pub(crate) fn tab_set_title(id: Id, index: i32, title: &str) {
    let Some(native) = get(id, |n| n.native.clone()) else {
        return;
    };
    let ok = update(id, |n| {
        match usize::try_from(index)
            .ok()
            .and_then(|i| n.tab_titles.get_mut(i))
        {
            Some(t) => {
                *t = title.to_owned();
                true
            }
            None => false,
        }
    });
    if ok == Some(true) {
        setter(|| Platform::tab_set_title(&native, index, title));
    }
}

// --- Code-Editor ------------------------------------------------------------------------------

/// Färbt den Text eines Textbereichs nach dessen Regeln neu ein.
pub(crate) fn rehighlight(id: Id, text: &str) {
    let Some((native, rules, marked)) = get(id, |n| (n.native.clone(), n.rules.clone(), n.marked))
    else {
        return;
    };
    let spans = highlight(text, &rules);
    setter(|| {
        Platform::set_highlights(&native, &spans);
        Platform::set_marked_line(&native, marked);
    });
}

// --- Timer -------------------------------------------------------------------------------------

pub(crate) fn timer_new(once: bool) -> u64 {
    let id = next_id();
    with_core(|c| {
        c.timers.insert(
            id,
            TimerEntry {
                once,
                single_shot: once,
                ..Default::default()
            },
        )
    });
    id
}

pub(crate) fn timer_on_timeout(id: u64, f: impl FnMut() + 'static) {
    with_core(|c| {
        if let Some(t) = c.timers.get_mut(&id) {
            t.handlers.push(Rc::new(RefCell::new(f)));
        }
    });
}

pub(crate) fn timer_start(id: u64, msec: i32) {
    with_core(|c| {
        if let Some(t) = c.timers.get_mut(&id) {
            t.active = true;
            t.interval = msec;
        }
    });
    Platform::timer_stop(id);
    Platform::timer_start(id, msec.max(0) as u32);
}

pub(crate) fn timer_stop(id: u64) {
    with_core(|c| {
        if let Some(t) = c.timers.get_mut(&id) {
            t.active = false;
        }
    });
    Platform::timer_stop(id);
}

pub(crate) fn timer_set_single_shot(id: u64, single: bool) {
    with_core(|c| {
        if let Some(t) = c.timers.get_mut(&id) {
            t.single_shot = single;
        }
    });
}

/// (aktiv, einmalig, Intervall)
pub(crate) fn timer_state(id: u64) -> (bool, bool, i32) {
    with_core(|c| {
        c.timers
            .get(&id)
            .map(|t| (t.active, t.single_shot, t.interval))
            .unwrap_or_default()
    })
}

pub(crate) fn timer_drop(id: u64) {
    Platform::timer_stop(id);
    with_core(|c| c.timers.remove(&id));
}

/// Vom Backend aufgerufen, wenn ein Timer abläuft.
pub(crate) fn timer_fired(id: u64) {
    let Some((handlers, single, once)) = with_core(|c| {
        let t = c.timers.get_mut(&id)?;
        if !t.active {
            return None;
        }
        if t.single_shot {
            t.active = false;
        }
        Some((t.handlers.clone(), t.single_shot, t.once))
    }) else {
        Platform::timer_stop(id);
        return;
    };
    if single {
        Platform::timer_stop(id);
    }
    fire(handlers, |f| f());
    if once {
        with_core(|c| c.timers.remove(&id));
    }
}

// --- Tastenkürzel ----------------------------------------------------------------------------

pub(crate) fn add_shortcut(window: Id, sequence: &str, f: impl FnMut() + 'static) -> bool {
    match crate::text::KeySpec::parse(sequence) {
        Some(spec) => add_key_shortcut(window, &spec, f),
        None => false,
    }
}

pub(crate) fn add_key_shortcut(
    window: Id,
    spec: &crate::text::KeySpec,
    f: impl FnMut() + 'static,
) -> bool {
    let Some(native) = container_window(window) else {
        return false;
    };
    let id = next_id();
    with_core(|c| c.shortcuts.insert(id, Rc::new(RefCell::new(f))));
    Platform::add_shortcut(&native, spec, id)
}

/// Das Fenster, zu dem ein Widget gehört.
fn container_window(id: Id) -> Option<Native> {
    let mut current = Some(id);
    while let Some(i) = current {
        let (kind, native, parent) = get(i, |n| (n.kind, n.native.clone(), n.parent))?;
        if kind == WidgetKind::Window {
            return Some(native);
        }
        current = parent;
    }
    None
}

// --- Zeichenfläche, Baumansicht, Menüs, Fensterposition ---------------------------------

/// Feuert Handler mit einer Zeichenkette (Menü, Baumknoten).
fn fire_string(
    id: Id,
    value: String,
    pick: impl Fn(&Handler) -> Option<Callback<dyn FnMut(String)>>,
) {
    fire(handlers(id, pick), |f| f(value.clone()));
}

pub(crate) fn connect_mouse(id: Id, f: impl FnMut(MouseEvent) + 'static) -> bool {
    add_handler(id, Signal::Mouse, Handler::Mouse(Rc::new(RefCell::new(f))))
}

pub(crate) fn connect_key(id: Id, f: impl FnMut(KeyEvent) + 'static) -> bool {
    add_handler(id, Signal::Key, Handler::Key(Rc::new(RefCell::new(f))))
}

/// Texteingabe über die Eingabemethode; schaltet sie für das Widget ein.
pub(crate) fn connect_text_input(id: Id, f: impl FnMut(String) + 'static) -> bool {
    let added = add_handler(
        id,
        Signal::TextInput,
        Handler::TextInput(Rc::new(RefCell::new(f))),
    );
    if added && let Some((native, _)) = native(id) {
        setter(|| Platform::enable_text_input(&native));
    }
    added
}

pub(crate) fn connect_menu(id: Id, f: impl FnMut(String) + 'static) -> bool {
    add_handler(id, Signal::Menu, Handler::Menu(Rc::new(RefCell::new(f))))
}

pub(crate) fn connect_tree_selected(id: Id, f: impl FnMut(String) + 'static) -> bool {
    add_handler(
        id,
        Signal::TreeSelected,
        Handler::TreeSelected(Rc::new(RefCell::new(f))),
    )
}

pub(crate) fn connect_tree_activated(id: Id, f: impl FnMut(String) + 'static) -> bool {
    add_handler(
        id,
        Signal::TreeActivated,
        Handler::TreeActivated(Rc::new(RefCell::new(f))),
    )
}

pub(crate) fn connect_tree_toggled(id: Id, f: impl FnMut(String, bool) + 'static) -> bool {
    add_handler(
        id,
        Signal::TreeToggled,
        Handler::TreeToggled(Rc::new(RefCell::new(f))),
    )
}

pub(crate) fn connect_gutter(id: Id, f: impl FnMut(crate::code::GutterClick) + 'static) -> bool {
    add_handler(
        id,
        Signal::Gutter,
        Handler::Gutter(Rc::new(RefCell::new(f))),
    )
}

/// Ersetzt, was eine Zeichenfläche malt.
pub(crate) fn set_shapes(id: Id, shapes: Vec<Shape>) {
    let Some((native, WidgetKind::Canvas)) = native(id) else {
        return;
    };
    setter(|| Platform::set_shapes(&native, &shapes));
    update(id, |n| n.shapes = shapes);
}

/// Zeigt die sichtbaren Zeilen und stellt die Auswahl wieder her.
fn refresh_tree(id: Id) {
    let Some((native, WidgetKind::TreeView)) = native(id) else {
        return;
    };
    let (rows, selected) =
        get(id, |n| (visible_rows(&n.tree), n.tree_selected.clone())).unwrap_or_default();
    let index = selected
        .as_ref()
        .and_then(|s| rows.iter().position(|r| &r.id == s))
        .map_or(-1, |i| i as i32);
    update(id, |n| {
        n.tree_rows = rows.iter().map(|r| r.id.clone()).collect()
    });
    setter(|| {
        Platform::set_tree_rows(&native, &rows);
        Platform::set_current_index(&native, WidgetKind::TreeView, index);
    });
    // Ein zugeklappter Knoten verdeckt die Auswahl: sie geht auf "nichts".
    if selected.is_some() && index < 0 {
        update(id, |n| n.tree_selected = None);
    }
}

pub(crate) fn set_tree(id: Id, items: Vec<TreeItem>) {
    update(id, |n| n.tree = items);
    refresh_tree(id);
}

pub(crate) fn toggle_tree_item(id: Id, item: &str) -> bool {
    let expanded = update(id, |n| {
        crate::tree::toggle(&mut n.tree, item)
            .then(|| crate::tree::is_expanded(&n.tree, item))
            .flatten()
    })
    .flatten();
    if let Some(expanded) = expanded {
        refresh_tree(id);
        fire(
            handlers(id, |h| match h {
                Handler::TreeToggled(f) => Some(f.clone()),
                _ => None,
            }),
            |f| f(item.to_owned(), expanded),
        );
    }
    expanded.is_some()
}

/// Wählt einen Knoten aus (`None` = keine Auswahl); feuert die Auswahl.
pub(crate) fn select_tree_item(id: Id, item: Option<&str>) {
    let Some((native, WidgetKind::TreeView)) = native(id) else {
        return;
    };
    let index = item
        .and_then(|item| get(id, |n| n.tree_rows.iter().position(|r| r == item)).flatten())
        .map_or(-1, |i| i as i32);
    setter(|| Platform::set_current_index(&native, WidgetKind::TreeView, index));
    tree_sync(id);
}

/// Gleicht die Auswahl der Baumansicht ab und feuert bei Änderung.
fn tree_sync(id: Id) {
    let Some((native, kind)) = native(id) else {
        return;
    };
    let index = Platform::current_index(&native, kind);
    let now = get(id, |n| {
        usize::try_from(index)
            .ok()
            .and_then(|i| n.tree_rows.get(i).cloned())
    })
    .flatten();
    let changed = update(id, |n| {
        std::mem::replace(&mut n.tree_selected, now.clone()) != now
    });
    if changed == Some(true) {
        fire_string(id, now.unwrap_or_default(), |h| match h {
            Handler::TreeSelected(f) => Some(f.clone()),
            _ => None,
        });
    }
}

pub(crate) fn set_menu_bar(id: Id, bar: MenuBar) {
    let Some((native, WidgetKind::Window)) = native(id) else {
        return;
    };
    setter(|| Platform::set_menu_bar(&native, &bar));
    update(id, |n| n.menu_bar = bar);
}

/// Zeigt ein Kontextmenü an `x`/`y` (relativ zum Widget); gewählte
/// Einträge meldet das Fenster des Widgets über `on_menu`.
pub(crate) fn popup_menu(id: Id, menu: &Menu, x: i32, y: i32) -> bool {
    let Some((native, kind)) = native(id) else {
        return false;
    };
    let mut current = Some(id);
    while let Some(i) = current {
        let Some((window_kind, window, parent)) = get(i, |n| (n.kind, n.native.clone(), n.parent))
        else {
            return false;
        };
        if window_kind == WidgetKind::Window {
            setter(|| Platform::popup_menu(&native, kind, &window, menu, x, y));
            return true;
        }
        current = parent;
    }
    false
}

/// Sprache eines Code-Editors, z. B. `"rust"` (leer = ohne Hervorhebung).
pub(crate) fn set_language(id: Id, language: &str) {
    let Some((native, WidgetKind::CodeEditor)) = native(id) else {
        return;
    };
    setter(|| Platform::set_language(&native, language));
    update(id, |n| n.language = language.to_owned());
}

pub(crate) fn set_line_marks(id: Id, marks: Vec<LineMark>) {
    let Some((native, WidgetKind::CodeEditor)) = native(id) else {
        return;
    };
    setter(|| Platform::set_line_marks(&native, &marks));
    update(id, |n| n.line_marks = marks);
}

pub(crate) fn set_diff_marks(id: Id, marks: Vec<crate::code::DiffMark>) {
    let Some((native, WidgetKind::CodeEditor)) = native(id) else {
        return;
    };
    setter(|| Platform::set_diff_marks(&native, &marks));
    update(id, |n| n.diff_marks = marks);
}

pub(crate) fn set_breakpoint_marks(id: Id, marks: Vec<crate::code::BreakpointMark>) {
    let Some((native, WidgetKind::CodeEditor)) = native(id) else {
        return;
    };
    setter(|| Platform::set_breakpoint_marks(&native, &marks));
    update(id, |n| n.breakpoint_marks = marks);
}

pub(crate) fn set_image(id: Id, image: Option<crate::Image>) {
    let Some((native, WidgetKind::PushButton)) = native(id) else {
        return;
    };
    setter(|| Platform::set_button_image(&native, image.as_ref()));
    update(id, |n| n.image = image);
}

pub(crate) fn set_execution_line(id: Id, line: Option<usize>) {
    let Some((native, WidgetKind::CodeEditor)) = native(id) else {
        return;
    };
    setter(|| Platform::set_execution_line(&native, line));
    update(id, |n| n.execution_line = line);
}

/// Setzt die Bildschirmposition eines Fensters (`false`, wenn das System es
/// nicht erlaubt, z. B. unter Wayland).
pub(crate) fn set_window_position(id: Id, x: i32, y: i32) -> bool {
    match native(id) {
        Some((native, WidgetKind::Window)) => {
            update(id, |n| {
                n.geometry.x = x;
                n.geometry.y = y;
            });
            setter(|| Platform::set_window_position(&native, x, y))
        }
        _ => false,
    }
}

pub(crate) fn set_window_decorated(id: Id, decorated: bool) {
    if let Some((native, WidgetKind::Window)) = native(id) {
        setter(|| Platform::set_window_decorated(&native, decorated));
    }
}

pub(crate) fn window_position(id: Id) -> Option<Point> {
    match native(id)? {
        (native, WidgetKind::Window) => Platform::window_position(&native),
        _ => None,
    }
}

/// Vom Backend aufgerufen, wenn ein Tastenkürzel ausgelöst wurde.
pub(crate) fn shortcut_fired(id: u64) {
    let handler = with_core(|c| c.shortcuts.get(&id).cloned());
    if let Some(handler) = handler {
        fire(vec![handler], |f| f());
    }
}
