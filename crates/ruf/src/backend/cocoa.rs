//! Backend für macOS: Cocoa (AppKit) über `objc2`.
//!
//! * Container sind Instanzen der eigenen Klasse `RufView` (Unterklasse von
//!   `NSView`) mit gespiegelter Y-Achse – Koordinaten zählen wie auf den
//!   anderen Systemen von oben links. `RufView` meldet Größenänderungen an
//!   den Kern.
//! * Ein gemeinsames Objekt `RufTarget` ist Ziel aller Aktionen und Delegate
//!   aller Steuerelemente; es ordnet den Absender über seinen Zeiger der
//!   Kern-Nummer zu.
//! * Tastenkürzel: `Ctrl+…` gilt unter macOS auch als `Cmd+…`.

#![allow(unused_unsafe)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::Path;
use std::ptr::NonNull;
use std::rc::Rc;

use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, Message, define_class, msg_send, sel};
use objc2_app_kit::*;
use objc2_foundation::*;

use super::{Backend, MessageKind};
use crate::app::ColorScheme;
use crate::core::{self, Id, Notify, WidgetKind};
use crate::geometry::{Rect, Size};
use crate::style::{Color, Style};
use crate::text::{Key, KeySpec, Span, markup_to_plain};

fn mtm() -> MainThreadMarker {
    MainThreadMarker::new().expect("ruf: Cocoa darf nur im Hauptthread benutzt werden")
}

fn ns(text: &str) -> Retained<NSString> {
    NSString::from_str(text)
}

fn rect(r: Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(f64::from(r.x), f64::from(r.y)),
        NSSize::new(f64::from(r.width.max(0)), f64::from(r.height.max(0))),
    )
}

fn ns_color(c: Color) -> Retained<NSColor> {
    let (r, g, b) = c.to_f32();
    NSColor::colorWithSRGBRed_green_blue_alpha(f64::from(r), f64::from(g), f64::from(b), 1.0)
}

fn key_of<T: ?Sized>(object: &T) -> usize {
    object as *const T as *const () as usize
}

// --- Zustand -----------------------------------------------------------------------------

#[derive(Default)]
struct State {
    /// Zeiger eines nativen Objekts → Kern-Nummer und Art.
    objects: HashMap<usize, (Id, WidgetKind)>,
    tables: HashMap<usize, Vec<String>>,
    timers: HashMap<u64, Retained<NSTimer>>,
    shortcuts: HashMap<usize, Vec<(KeySpec, u64)>>,
    scrolls: HashMap<usize, ScrollState>,
    monitor: Option<Retained<AnyObject>>,
    target: Option<Retained<RufTarget>>,
    loops: u32,
}

struct ScrollState {
    scroll: Retained<NSScrollView>,
    document: Retained<RufView>,
    min: Size,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn register(object: &AnyObject, id: Id, kind: WidgetKind) {
    STATE.with(|s| s.borrow_mut().objects.insert(key_of(object), (id, kind)));
}

fn lookup(object: &AnyObject) -> Option<(Id, WidgetKind)> {
    STATE.with(|s| s.borrow().objects.get(&key_of(object)).copied())
}

fn target() -> Retained<RufTarget> {
    STATE
        .with(|s| s.borrow().target.clone())
        .expect("ruf: Cocoa nicht initialisiert")
}

// --- RufView: Container mit gespiegelter Y-Achse -----------------------------------------------

#[derive(Default)]
pub struct RufViewIvars {
    id: Cell<Id>,
    last: Cell<(i32, i32)>,
}

define_class!(
    // SAFETY: NSView hat keine besonderen Anforderungen an Unterklassen.
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = RufViewIvars]
    #[name = "RufView"]
    pub struct RufView;

    impl RufView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        #[unsafe(method(setFrameSize:))]
        fn set_frame_size(&self, size: NSSize) {
            let _: () = unsafe { msg_send![super(self), setFrameSize: size] };
            let (w, h) = (size.width.round() as i32, size.height.round() as i32);
            if self.ivars().last.replace((w, h)) != (w, h) {
                layout_scroll(key_of(self));
                let id = self.ivars().id.get();
                if id != 0 {
                    // Erst nach dem Layout-Durchlauf melden.
                    DispatchQueue::main().exec_async(move || core::notify(id, Notify::Resized(Size::new(w, h))));
                }
            }
        }
    }
);

impl RufView {
    fn new(id: Id) -> Retained<Self> {
        let this = Self::alloc(mtm()).set_ivars(RufViewIvars {
            id: Cell::new(id),
            last: Cell::new((-1, -1)),
        });
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }
}

// --- RufTarget: Ziel aller Aktionen und Delegate ------------------------------------------------

define_class!(
    // SAFETY: NSObject hat keine besonderen Anforderungen an Unterklassen.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "RufTarget"]
    pub struct RufTarget;

    unsafe impl NSObjectProtocol for RufTarget {}

    impl RufTarget {
        #[unsafe(method(action:))]
        fn action(&self, sender: &AnyObject) {
            on_action(sender);
        }

        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, notification: &NSNotification) {
            if let Some(object) = unsafe { notification.object() } {
                on_text_changed(&object);
            }
        }

        #[unsafe(method(textDidChange:))]
        fn text_did_change(&self, notification: &NSNotification) {
            if let Some(object) = unsafe { notification.object() }
                && let Some((id, _)) = lookup(&object)
            {
                core::notify(id, Notify::Changed);
            }
        }

        #[unsafe(method(comboBoxSelectionDidChange:))]
        fn combo_selection_did_change(&self, notification: &NSNotification) {
            if let Some(object) = unsafe { notification.object() }
                && let Some(combo) = object.downcast_ref::<NSComboBox>()
                && let Some((id, _)) = lookup(&object)
            {
                // Der Text steht erst danach im Feld – vorher übernehmen.
                let index = combo.indexOfSelectedItem();
                if index >= 0
                    && let Some(value) = unsafe { combo.itemObjectValueAtIndex(index) }.downcast::<NSString>().ok()
                {
                    combo.setStringValue(&value);
                }
                core::notify(id, Notify::Changed);
            }
        }

        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, sender: &NSWindow) -> bool {
            if let Some((id, _)) = lookup(sender) {
                core::close_requested(id);
            }
            // Der Kern blendet das Fenster selbst aus.
            false
        }

        #[unsafe(method(numberOfRowsInTableView:))]
        fn number_of_rows(&self, table: &NSTableView) -> NSInteger {
            STATE.with(|s| s.borrow().tables.get(&key_of(table)).map_or(0, Vec::len)) as NSInteger
        }

        #[unsafe(method_id(tableView:objectValueForTableColumn:row:))]
        fn object_value(&self, table: &NSTableView, _column: Option<&NSTableColumn>, row: NSInteger) -> Option<Retained<AnyObject>> {
            let text = STATE.with(|s| {
                s.borrow()
                    .tables
                    .get(&key_of(table))
                    .and_then(|items| items.get(row as usize).cloned())
            });
            text.map(|t| {
                let string = ns(&t);
                let object: &AnyObject = &string;
                object.retain()
            })
        }

        #[unsafe(method(tableViewSelectionDidChange:))]
        fn table_selection_did_change(&self, notification: &NSNotification) {
            if let Some(object) = unsafe { notification.object() }
                && let Some((id, _)) = lookup(&object)
            {
                core::notify(id, Notify::Changed);
            }
        }

        #[unsafe(method(tabView:didSelectTabViewItem:))]
        fn tab_did_select(&self, tab: &NSTabView, _item: Option<&NSTabViewItem>) {
            if let Some((id, _)) = lookup(tab) {
                core::notify(id, Notify::Changed);
            }
        }
    }
);

impl RufTarget {
    fn new() -> Retained<Self> {
        let this = Self::alloc(mtm()).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

fn action_sel() -> Sel {
    sel!(action:)
}

fn on_action(sender: &AnyObject) {
    let Some((id, kind)) = lookup(sender) else {
        return;
    };
    match kind {
        WidgetKind::PushButton | WidgetKind::CheckBox | WidgetKind::RadioButton => {
            core::notify(id, Notify::Clicked)
        }
        WidgetKind::LineEdit => core::notify(id, Notify::Activated),
        WidgetKind::SpinBox => {
            // Pfeil gedrückt: Wert ins Textfeld übernehmen.
            if let Some(stepper) = sender.downcast_ref::<NSStepper>() {
                let value = stepper.intValue();
                if let Some(field) = spin_field(id) {
                    field.setIntValue(value);
                }
            }
            core::notify(id, Notify::Changed);
        }
        _ => core::notify(id, Notify::Changed),
    }
}

fn on_text_changed(object: &AnyObject) {
    let Some((id, kind)) = lookup(object) else {
        return;
    };
    if kind == WidgetKind::SpinBox
        && let (Some(field), Some(stepper)) =
            (object.downcast_ref::<NSTextField>(), spin_stepper(id))
    {
        stepper.setIntValue(field.intValue());
    }
    core::notify(id, Notify::Changed);
}

// --- Native Verweise --------------------------------------------------------------------------

#[derive(Clone)]
pub enum Extra {
    None,
    Window(Retained<NSWindow>),
    Group(Retained<NSBox>),
    Text(Retained<NSTextView>),
    Table(Retained<NSTableView>),
    Spin {
        field: Retained<NSTextField>,
        stepper: Retained<NSStepper>,
    },
    Scroll(Retained<RufView>),
    Tabs(Retained<NSTabView>),
    Progress(Retained<NSProgressIndicator>),
}

/// Ein Cocoa-Steuerelement. `view` ist austauschbar (Passwortfeld,
/// editierbare Combobox).
#[derive(Clone)]
pub struct CocoaNative {
    id: Id,
    kind: WidgetKind,
    view: Rc<RefCell<Retained<NSView>>>,
    /// Container für Kinder (falls die Art Kinder aufnehmen kann).
    container: Option<Retained<RufView>>,
    extra: Extra,
}

impl CocoaNative {
    fn view(&self) -> Retained<NSView> {
        self.view.borrow().clone()
    }

    fn control(&self) -> Option<Retained<NSControl>> {
        self.view().downcast::<NSControl>().ok()
    }
}

/// Textfeld und Pfeile einer Spinbox.
type SpinParts = (Retained<NSTextField>, Retained<NSStepper>);

thread_local! {
    static SPINS: RefCell<HashMap<Id, SpinParts>> = RefCell::new(HashMap::new());
}

fn spin_field(id: Id) -> Option<Retained<NSTextField>> {
    SPINS.with(|s| s.borrow().get(&id).map(|(f, _)| f.clone()))
}

fn spin_stepper(id: Id) -> Option<Retained<NSStepper>> {
    SPINS.with(|s| s.borrow().get(&id).map(|(_, st)| st.clone()))
}

/// Upcast auf `NSView` (über die Deref-Kette der AppKit-Klassen).
fn up(view: &NSView) -> Retained<NSView> {
    view.retain()
}

fn set_target(control: &NSControl) {
    let target = target();
    unsafe {
        control.setTarget(Some(&target));
        control.setAction(Some(action_sel()));
    }
}

/// Inhalt eines Scrollbereichs an die sichtbare Größe anpassen.
fn layout_scroll(wrapper: usize) {
    let Some((scroll, document, min)) = STATE.with(|s| {
        s.borrow()
            .scrolls
            .get(&wrapper)
            .map(|v| (v.scroll.clone(), v.document.clone(), v.min))
    }) else {
        return;
    };
    let visible = scroll.contentSize();
    let size = NSSize::new(
        visible.width.max(f64::from(min.width)),
        visible.height.max(f64::from(min.height)),
    );
    document.setFrameSize(size);
}

fn line_offsets(text: &str, line: usize) -> Option<(usize, usize)> {
    let mut start = 0usize;
    for (i, l) in text.split('\n').enumerate() {
        let len = l.encode_utf16().count();
        if i == line {
            return Some((start, len + 1));
        }
        start += len + 1;
    }
    None
}

fn key_code_matches(key: &Key, event: &NSEvent) -> bool {
    match key {
        Key::Char(c) => event
            .charactersIgnoringModifiers()
            .is_some_and(|s| s.to_string().to_lowercase().starts_with(*c)),
        Key::F(n) => {
            const CODES: [u16; 12] = [122, 120, 99, 118, 96, 97, 98, 100, 101, 109, 103, 111];
            CODES.get(usize::from(*n).wrapping_sub(1)) == Some(&event.keyCode())
        }
        Key::Enter => event.keyCode() == 36,
        Key::Escape => event.keyCode() == 53,
        Key::Tab => event.keyCode() == 48,
        Key::Delete => event.keyCode() == 117,
        Key::Insert => false,
        Key::Home => event.keyCode() == 115,
        Key::End => event.keyCode() == 119,
        Key::PageUp => event.keyCode() == 116,
        Key::PageDown => event.keyCode() == 121,
        Key::Left => event.keyCode() == 123,
        Key::Right => event.keyCode() == 124,
        Key::Down => event.keyCode() == 125,
        Key::Up => event.keyCode() == 126,
    }
}

fn shortcut_for(event: &NSEvent) -> Option<u64> {
    let window = event.window(mtm())?;
    let flags = event.modifierFlags();
    let command = flags.contains(NSEventModifierFlags::Command);
    let control = flags.contains(NSEventModifierFlags::Control);
    let shift = flags.contains(NSEventModifierFlags::Shift);
    let option = flags.contains(NSEventModifierFlags::Option);
    STATE.with(|s| {
        s.borrow()
            .shortcuts
            .get(&key_of(&*window))
            .and_then(|list| {
                list.iter()
                    .find(|(spec, _)| {
                        // Ctrl+… gilt unter macOS auch als Cmd+….
                        let ctrl_ok = if spec.ctrl {
                            command || control
                        } else {
                            !control && (!command || spec.meta)
                        };
                        ctrl_ok
                            && spec.shift == shift
                            && spec.alt == option
                            && (!spec.meta || command)
                            && key_code_matches(&spec.key, event)
                    })
                    .map(|(_, id)| *id)
            })
    })
}

fn install_shortcut_monitor() {
    let block = RcBlock::new(|event: NonNull<NSEvent>| -> *mut NSEvent {
        let e = unsafe { event.as_ref() };
        match shortcut_for(e) {
            Some(id) => {
                core::shortcut_fired(id);
                std::ptr::null_mut()
            }
            None => event.as_ptr(),
        }
    });
    let monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, &block)
    };
    STATE.with(|s| s.borrow_mut().monitor = monitor);
}

pub struct Cocoa;

impl Backend for Cocoa {
    type Native = CocoaNative;
    const NAME: &'static str = "Cocoa";

    fn version() -> String {
        NSProcessInfo::processInfo()
            .operatingSystemVersionString()
            .to_string()
    }

    fn init() {
        let mtm = mtm();
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        unsafe { app.finishLaunching() };
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
        STATE.with(|s| s.borrow_mut().target = Some(RufTarget::new()));
        install_shortcut_monitor();
    }

    fn run() {
        STATE.with(|s| s.borrow_mut().loops += 1);
        NSApplication::sharedApplication(mtm()).run();
        STATE.with(|s| s.borrow_mut().loops -= 1);
    }

    fn quit() {
        if STATE.with(|s| s.borrow().loops) == 0 {
            return;
        }
        let app = NSApplication::sharedApplication(mtm());
        app.stop(None);
        // `stop` wirkt erst nach dem nächsten Ereignis – eines nachschieben.
        if let Some(event) = unsafe {
            NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
                NSEventType::ApplicationDefined,
                NSPoint::new(0.0, 0.0),
                NSEventModifierFlags::empty(),
                0.0,
                0,
                None,
                0,
                0,
                0,
            )
        } {
            app.postEvent_atStart(&event, true);
        }
    }

    fn process_events() {
        let app = NSApplication::sharedApplication(mtm());
        loop {
            let event = unsafe {
                app.nextEventMatchingMask_untilDate_inMode_dequeue(
                    NSEventMask::Any,
                    Some(&NSDate::distantPast()),
                    NSDefaultRunLoopMode,
                    true,
                )
            };
            match event {
                Some(event) => app.sendEvent(&event),
                None => break,
            }
        }
        // Hauptwarteschlange (GuiSender, verzögerte Meldungen) abarbeiten.
        let run_loop = NSRunLoop::currentRunLoop();
        for _ in 0..3 {
            unsafe {
                run_loop.runMode_beforeDate(
                    NSDefaultRunLoopMode,
                    &NSDate::dateWithTimeIntervalSinceNow(0.002),
                )
            };
        }
        app.updateWindows();
    }

    fn post(f: Box<dyn FnOnce() + Send>) -> bool {
        DispatchQueue::main().exec_async(f);
        true
    }

    fn timer_start(id: u64, msec: u32) {
        let block = RcBlock::new(move |_timer: NonNull<NSTimer>| core::timer_fired(id));
        let timer = unsafe {
            NSTimer::timerWithTimeInterval_repeats_block(
                f64::from(msec.max(1)) / 1000.0,
                true,
                &block,
            )
        };
        // Auch während Menüs/modaler Dialoge feuern.
        unsafe { NSRunLoop::currentRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes) };
        if let Some(old) = STATE.with(|s| s.borrow_mut().timers.insert(id, timer)) {
            old.invalidate();
        }
    }

    fn timer_stop(id: u64) {
        if let Some(timer) = STATE.with(|s| s.borrow_mut().timers.remove(&id)) {
            timer.invalidate();
        }
    }

    fn color_scheme() -> ColorScheme {
        let app = NSApplication::sharedApplication(mtm());
        let name = app.effectiveAppearance().name().to_string();
        if name.contains("Dark") {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        }
    }

    fn set_color_scheme(scheme: ColorScheme) -> bool {
        let app = NSApplication::sharedApplication(mtm());
        let appearance = match scheme {
            ColorScheme::Dark => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameDarkAqua }),
            ColorScheme::Light => NSAppearance::appearanceNamed(unsafe { NSAppearanceNameAqua }),
            ColorScheme::Unknown => None,
        };
        app.setAppearance(appearance.as_deref());
        true
    }

    fn message(_parent: Option<&CocoaNative>, kind: MessageKind, title: &str, text: &str) {
        let alert = NSAlert::new(mtm());
        alert.setMessageText(&ns(title));
        alert.setInformativeText(&ns(text));
        alert.setAlertStyle(match kind {
            MessageKind::Information => NSAlertStyle::Informational,
            MessageKind::Warning => NSAlertStyle::Warning,
            MessageKind::Error => NSAlertStyle::Critical,
        });
        alert.addButtonWithTitle(&ns("OK"));
        alert.runModal();
    }

    fn confirm(_parent: Option<&CocoaNative>, title: &str, text: &str) -> bool {
        let (yes, no) = if crate::dialog::german() {
            ("Ja", "Nein")
        } else {
            ("Yes", "No")
        };
        let alert = NSAlert::new(mtm());
        alert.setMessageText(&ns(title));
        alert.setInformativeText(&ns(text));
        alert.addButtonWithTitle(&ns(yes));
        alert.addButtonWithTitle(&ns(no));
        alert.runModal() == NSAlertFirstButtonReturn
    }

    fn create(id: Id, kind: WidgetKind, parent: Option<&CocoaNative>) -> CocoaNative {
        let mtm = mtm();
        let target = target();
        let delegate = ProtocolObject::<dyn NSObjectProtocol>::from_ref(&*target);
        let _ = delegate;
        let mut container = None;
        let (view, extra): (Retained<NSView>, Extra) = match kind {
            WidgetKind::Window => {
                let window = unsafe {
                    NSWindow::initWithContentRect_styleMask_backing_defer(
                        NSWindow::alloc(mtm),
                        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(640.0, 480.0)),
                        NSWindowStyleMask::Titled
                            | NSWindowStyleMask::Closable
                            | NSWindowStyleMask::Miniaturizable
                            | NSWindowStyleMask::Resizable,
                        NSBackingStoreType::Buffered,
                        false,
                    )
                };
                unsafe { window.setReleasedWhenClosed(false) };
                let content = RufView::new(id);
                window.setContentView(Some(&content));
                let _: () = unsafe { msg_send![&*window, setDelegate: &*target] };
                window.center();
                register(&window, id, kind);
                container = Some(content.clone());
                (up(&content), Extra::Window(window))
            }
            WidgetKind::Container => {
                let v = RufView::new(id);
                container = Some(v.clone());
                (Retained::into_super(v), Extra::None)
            }
            WidgetKind::GroupBox => {
                let v = RufView::new(id);
                let frame = NSBox::new(mtm);
                frame.setTitlePosition(NSTitlePosition::AtTop);
                unsafe { v.addSubview(&frame) };
                container = Some(v.clone());
                (Retained::into_super(v), Extra::Group(frame))
            }
            WidgetKind::PushButton => {
                let b = unsafe {
                    NSButton::buttonWithTitle_target_action(
                        &ns(""),
                        Some(&target),
                        Some(action_sel()),
                        mtm,
                    )
                };
                (up(&b), Extra::None)
            }
            WidgetKind::CheckBox => {
                let b = unsafe {
                    NSButton::checkboxWithTitle_target_action(
                        &ns(""),
                        Some(&target),
                        Some(action_sel()),
                        mtm,
                    )
                };
                (up(&b), Extra::None)
            }
            WidgetKind::RadioButton => {
                let b = unsafe {
                    NSButton::radioButtonWithTitle_target_action(
                        &ns(""),
                        Some(&target),
                        Some(action_sel()),
                        mtm,
                    )
                };
                (up(&b), Extra::None)
            }
            WidgetKind::Label => {
                let l = NSTextField::labelWithString(&ns(""), mtm);
                (up(&l), Extra::None)
            }
            WidgetKind::LineEdit => {
                let f = NSTextField::textFieldWithString(&ns(""), mtm);
                set_target(&f);
                let _: () = unsafe { msg_send![&*f, setDelegate: &*target] };
                (up(&f), Extra::None)
            }
            WidgetKind::TextArea => {
                let scroll = NSTextView::scrollableTextView(mtm);
                let text = scroll
                    .documentView()
                    .and_then(|d| d.downcast::<NSTextView>().ok())
                    .expect("scrollableTextView liefert einen NSTextView");
                text.setRichText(false);
                unsafe { text.setAutomaticQuoteSubstitutionEnabled(false) };
                unsafe { text.setAutomaticDashSubstitutionEnabled(false) };
                let _: () = unsafe { msg_send![&*text, setDelegate: &*target] };
                register(&text, id, kind);
                scroll.setBorderType(NSBorderType::BezelBorder);
                (up(&scroll), Extra::Text(text))
            }
            WidgetKind::ComboBox => {
                let popup = NSPopUpButton::new(mtm);
                set_target(&popup);
                (up(&popup), Extra::None)
            }
            WidgetKind::ListBox => {
                let table = NSTableView::new(mtm);
                let column =
                    NSTableColumn::initWithIdentifier(NSTableColumn::alloc(mtm), &ns("text"));
                column.setResizingMask(NSTableColumnResizingOptions::AutoresizingMask);
                table.addTableColumn(&column);
                table.setHeaderView(None);
                table.setColumnAutoresizingStyle(
                    NSTableViewColumnAutoresizingStyle::UniformColumnAutoresizingStyle,
                );
                let _: () = unsafe { msg_send![&*table, setDataSource: &*target] };
                let _: () = unsafe { msg_send![&*table, setDelegate: &*target] };
                register(&table, id, kind);
                STATE.with(|s| s.borrow_mut().tables.insert(key_of(&*table), Vec::new()));
                let scroll = NSScrollView::new(mtm);
                scroll.setDocumentView(Some(&table));
                scroll.setHasVerticalScroller(true);
                scroll.setBorderType(NSBorderType::BezelBorder);
                (up(&scroll), Extra::Table(table))
            }
            WidgetKind::SpinBox => {
                let v = RufView::new(0);
                let field = NSTextField::textFieldWithString(&ns("0"), mtm);
                let _: () = unsafe { msg_send![&*field, setDelegate: &*target] };
                let stepper = NSStepper::new(mtm);
                set_target(&stepper);
                stepper.setValueWraps(false);
                unsafe {
                    v.addSubview(&field);
                    v.addSubview(&stepper);
                }
                register(&field, id, kind);
                register(&stepper, id, kind);
                SPINS.with(|s| s.borrow_mut().insert(id, (field.clone(), stepper.clone())));
                (Retained::into_super(v), Extra::Spin { field, stepper })
            }
            WidgetKind::Slider => {
                let slider = unsafe {
                    NSSlider::sliderWithTarget_action(Some(&target), Some(action_sel()), mtm)
                };
                slider.setContinuous(true);
                (up(&slider), Extra::None)
            }
            WidgetKind::ProgressBar => {
                let bar = NSProgressIndicator::new(mtm);
                bar.setStyle(NSProgressIndicatorStyle::Bar);
                bar.setIndeterminate(false);
                bar.setMinValue(0.0);
                bar.setMaxValue(100.0);
                (Retained::into_super(bar.clone()), Extra::Progress(bar))
            }
            WidgetKind::TabView => {
                let tabs = NSTabView::new(mtm);
                let _: () = unsafe { msg_send![&*tabs, setDelegate: &*target] };
                (Retained::into_super(tabs.clone()), Extra::Tabs(tabs))
            }
            WidgetKind::ScrollArea => {
                let wrapper = RufView::new(0);
                let scroll = NSScrollView::new(mtm);
                scroll.setHasVerticalScroller(true);
                scroll.setHasHorizontalScroller(true);
                scroll.setAutohidesScrollers(true);
                scroll.setBorderType(NSBorderType::BezelBorder);
                scroll.setAutoresizingMask(
                    NSAutoresizingMaskOptions::ViewWidthSizable
                        | NSAutoresizingMaskOptions::ViewHeightSizable,
                );
                let document = RufView::new(0);
                scroll.setDocumentView(Some(&document));
                unsafe { wrapper.addSubview(&scroll) };
                STATE.with(|s| {
                    s.borrow_mut().scrolls.insert(
                        key_of(&*wrapper),
                        ScrollState {
                            scroll,
                            document: document.clone(),
                            min: Size::default(),
                        },
                    )
                });
                (Retained::into_super(wrapper), Extra::Scroll(document))
            }
        };
        register(&view, id, kind);
        if kind != WidgetKind::Window
            && let Some(host) = parent.and_then(|p| p.container.clone())
        {
            unsafe { host.addSubview(&view) };
        }
        CocoaNative {
            id,
            kind,
            view: Rc::new(RefCell::new(view)),
            container,
            extra,
        }
    }

    fn destroy(native: &CocoaNative, _kind: WidgetKind) {
        let view = native.view();
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.objects.retain(|_, (id, _)| *id != native.id);
            s.scrolls.remove(&key_of(&*view));
            if let Extra::Table(t) = &native.extra {
                s.tables.remove(&key_of(&**t));
            }
        });
        SPINS.with(|s| s.borrow_mut().remove(&native.id));
        match &native.extra {
            Extra::Window(window) => {
                STATE.with(|s| s.borrow_mut().shortcuts.remove(&key_of(&**window)));
                window.orderOut(None);
                window.close();
            }
            _ => unsafe { view.removeFromSuperview() },
        }
    }

    fn set_parent(native: &CocoaNative, kind: WidgetKind, parent: &CocoaNative) {
        if kind == WidgetKind::Window {
            return;
        }
        if let Some(host) = &parent.container {
            let view = native.view();
            unsafe {
                view.removeFromSuperview();
                host.addSubview(&view);
            }
        }
    }

    fn class_name(native: &CocoaNative, _kind: WidgetKind) -> String {
        fn name(object: &AnyObject) -> String {
            object.class().name().to_string_lossy().into_owned()
        }
        match &native.extra {
            Extra::Window(w) => name(w),
            Extra::Text(t) => name(t),
            Extra::Table(t) => name(t),
            _ => name(&native.view()),
        }
    }

    fn set_geometry(native: &CocoaNative, kind: WidgetKind, r: Rect) {
        match &native.extra {
            Extra::Window(window) => {
                window.setContentSize(NSSize::new(
                    f64::from(r.width.max(1)),
                    f64::from(r.height.max(1)),
                ));
                if r.x != 0 || r.y != 0 {
                    // Bildschirmkoordinaten von oben links.
                    let top = window
                        .screen()
                        .or_else(|| NSScreen::mainScreen(mtm()))
                        .map(|s| s.frame().size.height)
                        .unwrap_or(900.0);
                    window.setFrameTopLeftPoint(NSPoint::new(f64::from(r.x), top - f64::from(r.y)));
                }
            }
            _ => {
                let view = native.view();
                let mut frame = rect(r);
                // Einzeilige Labels senkrecht mittig wie bei den anderen Toolkits.
                if kind == WidgetKind::Label
                    && let Some(label) = view.downcast_ref::<NSTextField>()
                    && label.usesSingleLineMode()
                {
                    let height = view.fittingSize().height.min(frame.size.height);
                    frame.origin.y += (frame.size.height - height) / 2.0;
                    frame.size.height = height;
                }
                view.setFrame(frame);
                match &native.extra {
                    Extra::Group(b) => b.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), frame.size)),
                    Extra::Spin { field, stepper } => {
                        let w = frame.size.width;
                        let h = frame.size.height;
                        field.setFrame(NSRect::new(
                            NSPoint::new(0.0, 0.0),
                            NSSize::new((w - 20.0).max(0.0), h),
                        ));
                        stepper.setFrame(NSRect::new(
                            NSPoint::new((w - 18.0).max(0.0), 0.0),
                            NSSize::new(18.0, h),
                        ));
                    }
                    _ => {}
                }
            }
        }
    }

    fn preferred_size(native: &CocoaNative, _kind: WidgetKind) -> Size {
        let size = native.view().fittingSize();
        Size::new(size.width.ceil() as i32, size.height.ceil() as i32)
    }

    fn set_visible(native: &CocoaNative, _kind: WidgetKind, visible: bool) {
        match &native.extra {
            Extra::Window(w) if visible => w.makeKeyAndOrderFront(None),
            Extra::Window(w) => w.orderOut(None),
            _ => native.view().setHidden(!visible),
        }
    }

    fn set_enabled(native: &CocoaNative, _kind: WidgetKind, enabled: bool) {
        match &native.extra {
            Extra::Text(t) => {
                t.setEditable(enabled);
                t.setSelectable(enabled);
            }
            Extra::Spin { field, stepper } => {
                field.setEnabled(enabled);
                stepper.setEnabled(enabled);
            }
            Extra::Table(t) => t.setEnabled(enabled),
            _ => {
                if let Some(c) = native.control() {
                    c.setEnabled(enabled);
                }
            }
        }
    }

    fn set_tool_tip(native: &CocoaNative, _kind: WidgetKind, text: &str) {
        let text = (!text.is_empty()).then(|| ns(text));
        native.view().setToolTip(text.as_deref());
    }

    fn set_style(native: &CocoaNative, _kind: WidgetKind, style: &Style) {
        let font = if style.bold {
            NSFont::boldSystemFontOfSize(NSFont::systemFontSize())
        } else {
            NSFont::systemFontOfSize(NSFont::systemFontSize())
        };
        let view = native.view();
        if let Some(field) = view.downcast_ref::<NSTextField>() {
            field.setTextColor(style.color.map(ns_color).as_deref());
            if let Some(bg) = style.background {
                field.setDrawsBackground(true);
                field.setBackgroundColor(Some(&ns_color(bg)));
            }
            field.setFont(Some(&font));
        } else if let Some(control) = native.control() {
            control.setFont(Some(&font));
        }
    }

    fn screenshot(native: &CocoaNative, _kind: WidgetKind, path: &Path) -> bool {
        let view = native.view();
        let bounds = view.bounds();
        let Some(rep) = view.bitmapImageRepForCachingDisplayInRect(bounds) else {
            return false;
        };
        view.cacheDisplayInRect_toBitmapImageRep(bounds, &rep);
        let properties = NSDictionary::new();
        let Some(data) = (unsafe {
            rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)
        }) else {
            return false;
        };
        std::fs::write(path, data.to_vec()).is_ok()
    }

    fn add_shortcut(window: &CocoaNative, key: &KeySpec, shortcut: u64) -> bool {
        let Extra::Window(w) = &window.extra else {
            return false;
        };
        STATE.with(|s| {
            s.borrow_mut()
                .shortcuts
                .entry(key_of(&**w))
                .or_default()
                .push((key.clone(), shortcut))
        });
        true
    }

    fn set_text(native: &CocoaNative, kind: WidgetKind, text: &str) {
        match (&native.extra, kind) {
            (Extra::Window(w), _) => w.setTitle(&ns(text)),
            (Extra::Group(b), _) => b.setTitle(&ns(text)),
            (Extra::Text(t), _) => t.setString(&ns(text)),
            (_, WidgetKind::Label) => {
                if let Some(c) = native.control() {
                    c.setStringValue(&ns(&markup_to_plain(text)));
                }
            }
            (_, WidgetKind::PushButton | WidgetKind::CheckBox | WidgetKind::RadioButton) => {
                if let Some(b) = native.view().downcast_ref::<NSButton>() {
                    b.setTitle(&ns(text));
                }
            }
            _ => {
                if let Some(c) = native.control() {
                    c.setStringValue(&ns(text));
                }
            }
        }
    }

    fn text(native: &CocoaNative, _kind: WidgetKind) -> String {
        match &native.extra {
            Extra::Text(t) => t.string().to_string(),
            _ => native
                .control()
                .map(|c| c.stringValue().to_string())
                .unwrap_or_default(),
        }
    }

    fn set_placeholder(native: &CocoaNative, _kind: WidgetKind, text: &str) {
        if let Some(field) = native.view().downcast_ref::<NSTextField>() {
            field.setPlaceholderString(Some(&ns(text)));
        }
    }

    fn set_password(native: &CocoaNative, enabled: bool) {
        let old = native.view();
        let is_secure = old.downcast_ref::<NSSecureTextField>().is_some();
        if is_secure == enabled {
            return;
        }
        let mtm = mtm();
        let field: Retained<NSTextField> = if enabled {
            Retained::into_super(NSSecureTextField::new(mtm))
        } else {
            NSTextField::new(mtm)
        };
        if let Some(old_field) = old.downcast_ref::<NSTextField>() {
            field.setStringValue(&old_field.stringValue());
            field.setPlaceholderString(old_field.placeholderString().as_deref());
        }
        set_target(&field);
        let target = target();
        let _: () = unsafe { msg_send![&*field, setDelegate: &*target] };
        field.setFrame(old.frame());
        register(&field, native.id, native.kind);
        if let Some(superview) = unsafe { old.superview() } {
            unsafe { superview.replaceSubview_with(&old, &field) };
        }
        *native.view.borrow_mut() = up(&field);
    }

    fn set_read_only(native: &CocoaNative, _kind: WidgetKind, read_only: bool) {
        match &native.extra {
            Extra::Text(t) => t.setEditable(!read_only),
            Extra::Spin { field, .. } => field.setEditable(!read_only),
            _ => {
                if let Some(field) = native.view().downcast_ref::<NSTextField>() {
                    field.setEditable(!read_only);
                }
            }
        }
    }

    fn set_word_wrap(native: &CocoaNative, wrap: bool) {
        if let Some(label) = native.view().downcast_ref::<NSTextField>() {
            label.setUsesSingleLineMode(!wrap);
            label.setLineBreakMode(if wrap {
                NSLineBreakMode::ByWordWrapping
            } else {
                NSLineBreakMode::ByTruncatingTail
            });
            label.setMaximumNumberOfLines(if wrap { 0 } else { 1 });
        }
    }

    fn set_checked(native: &CocoaNative, _kind: WidgetKind, checked: bool) {
        if let Some(b) = native.view().downcast_ref::<NSButton>() {
            b.setState(if checked {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
    }

    fn is_checked(native: &CocoaNative, kind: WidgetKind) -> bool {
        matches!(kind, WidgetKind::CheckBox | WidgetKind::RadioButton)
            && native
                .view()
                .downcast_ref::<NSButton>()
                .is_some_and(|b| b.state() == NSControlStateValueOn)
    }

    fn set_items(native: &CocoaNative, kind: WidgetKind, items: &[String]) {
        match (&native.extra, kind) {
            (Extra::Table(table), _) => {
                STATE.with(|s| {
                    s.borrow_mut()
                        .tables
                        .insert(key_of(&**table), items.to_vec())
                });
                table.reloadData();
            }
            (_, WidgetKind::ComboBox) => {
                let view = native.view();
                if let Some(combo) = view.downcast_ref::<NSComboBox>() {
                    combo.removeAllItems();
                    for item in items {
                        unsafe { combo.addItemWithObjectValue(&ns(item)) };
                    }
                } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                    popup.removeAllItems();
                    if let Some(menu) = popup.menu() {
                        // Über das Menü statt `addItemWithTitle`: erlaubt doppelte Einträge.
                        for item in items {
                            let entry = unsafe {
                                NSMenuItem::initWithTitle_action_keyEquivalent(
                                    NSMenuItem::alloc(mtm()),
                                    &ns(item),
                                    None,
                                    &ns(""),
                                )
                            };
                            menu.addItem(&entry);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn set_current_index(native: &CocoaNative, kind: WidgetKind, index: i32) {
        match (&native.extra, kind) {
            (Extra::Table(table), _) => {
                if index >= 0 {
                    let set = NSIndexSet::indexSetWithIndex(index as usize);
                    table.selectRowIndexes_byExtendingSelection(&set, false);
                } else {
                    unsafe { table.deselectAll(None) };
                }
            }
            (Extra::Tabs(tabs), _) => {
                if index >= 0 {
                    tabs.selectTabViewItemAtIndex(index as isize);
                }
            }
            (_, WidgetKind::ComboBox) => {
                let view = native.view();
                if let Some(combo) = view.downcast_ref::<NSComboBox>() {
                    if index >= 0 {
                        combo.selectItemAtIndex(index as isize);
                        if let Ok(value) = unsafe { combo.itemObjectValueAtIndex(index as isize) }
                            .downcast::<NSString>()
                        {
                            combo.setStringValue(&value);
                        }
                    } else {
                        let selected = combo.indexOfSelectedItem();
                        if selected >= 0 {
                            combo.deselectItemAtIndex(selected);
                        }
                    }
                } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                    popup.selectItemAtIndex(index as isize);
                }
            }
            _ => {}
        }
    }

    fn current_index(native: &CocoaNative, kind: WidgetKind) -> i32 {
        match (&native.extra, kind) {
            (Extra::Table(table), _) => table.selectedRow() as i32,
            (Extra::Tabs(tabs), _) => tabs
                .selectedTabViewItem()
                .map(|item| tabs.indexOfTabViewItem(&item) as i32)
                .unwrap_or(-1),
            (_, WidgetKind::ComboBox) => {
                let view = native.view();
                if let Some(combo) = view.downcast_ref::<NSComboBox>() {
                    combo.indexOfSelectedItem() as i32
                } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                    popup.indexOfSelectedItem() as i32
                } else {
                    -1
                }
            }
            _ => -1,
        }
    }

    fn set_editable(native: &CocoaNative, editable: bool) -> Option<CocoaNative> {
        let old = native.view();
        if old.downcast_ref::<NSComboBox>().is_some() == editable {
            return None;
        }
        // Freie Eingabe: NSComboBox, sonst NSPopUpButton – austauschen.
        let mtm = mtm();
        let target = target();
        let new: Retained<NSView> = if editable {
            let combo = NSComboBox::new(mtm);
            let _: () = unsafe { msg_send![&*combo, setDelegate: &*target] };
            combo.setCompletes(true);
            up(&combo)
        } else {
            let popup = NSPopUpButton::new(mtm);
            set_target(&popup);
            up(&popup)
        };
        new.setFrame(old.frame());
        register(&new, native.id, native.kind);
        if let Some(superview) = unsafe { old.superview() } {
            unsafe { superview.replaceSubview_with(&old, &new) };
        }
        *native.view.borrow_mut() = new;
        None
    }

    fn tab_add(native: &CocoaNative, page_id: Id, title: &str) -> CocoaNative {
        let page = RufView::new(page_id);
        if let Extra::Tabs(tabs) = &native.extra {
            let item = NSTabViewItem::new();
            item.setLabel(&ns(title));
            item.setView(Some(&page));
            tabs.addTabViewItem(&item);
        }
        CocoaNative {
            id: page_id,
            kind: WidgetKind::Container,
            view: Rc::new(RefCell::new(Retained::into_super(page.clone()))),
            container: Some(page),
            extra: Extra::None,
        }
    }

    fn tab_remove(native: &CocoaNative, index: i32) {
        if let Extra::Tabs(tabs) = &native.extra
            && index >= 0
            && index < tabs.numberOfTabViewItems() as i32
        {
            let item = tabs.tabViewItemAtIndex(index as isize);
            tabs.removeTabViewItem(&item);
        }
    }

    fn tab_set_title(native: &CocoaNative, index: i32, title: &str) {
        if let Extra::Tabs(tabs) = &native.extra
            && index >= 0
            && index < tabs.numberOfTabViewItems() as i32
        {
            tabs.tabViewItemAtIndex(index as isize).setLabel(&ns(title));
        }
    }

    fn set_range(native: &CocoaNative, _kind: WidgetKind, minimum: i32, maximum: i32) {
        match &native.extra {
            Extra::Spin { stepper, .. } => {
                stepper.setMinValue(f64::from(minimum));
                stepper.setMaxValue(f64::from(maximum));
            }
            Extra::Progress(bar) => {
                let busy = minimum == 0 && maximum == 0;
                bar.setIndeterminate(busy);
                if busy {
                    unsafe { bar.startAnimation(None) };
                } else {
                    unsafe { bar.stopAnimation(None) };
                    bar.setMinValue(f64::from(minimum));
                    bar.setMaxValue(f64::from(maximum));
                }
            }
            _ => {
                if let Some(slider) = native.view().downcast_ref::<NSSlider>() {
                    slider.setMinValue(f64::from(minimum));
                    slider.setMaxValue(f64::from(maximum));
                }
            }
        }
    }

    fn set_value(native: &CocoaNative, _kind: WidgetKind, value: i32) {
        match &native.extra {
            Extra::Spin { field, stepper } => {
                stepper.setIntValue(value);
                field.setIntValue(stepper.intValue());
            }
            Extra::Progress(bar) => bar.setDoubleValue(f64::from(value)),
            _ => {
                if let Some(slider) = native.view().downcast_ref::<NSSlider>() {
                    slider.setIntValue(value);
                }
            }
        }
    }

    fn value(native: &CocoaNative, _kind: WidgetKind) -> i32 {
        match &native.extra {
            Extra::Spin { stepper, .. } => stepper.intValue(),
            Extra::Progress(bar) => bar.doubleValue().round() as i32,
            _ => native
                .view()
                .downcast_ref::<NSSlider>()
                .map(|s| s.intValue())
                .unwrap_or(0),
        }
    }

    fn set_vertical(native: &CocoaNative, _kind: WidgetKind, vertical: bool) {
        let view = native.view();
        if let Some(slider) = view.downcast_ref::<NSSlider>() {
            slider.setVertical(vertical);
        } else if matches!(native.extra, Extra::Progress(_)) {
            // Cocoa kennt keinen senkrechten Fortschrittsbalken: drehen.
            view.setFrameCenterRotation(if vertical { 90.0 } else { 0.0 });
        }
    }

    fn scroll_content(native: &CocoaNative, content_id: Id) -> CocoaNative {
        let document = match &native.extra {
            Extra::Scroll(document) => document.clone(),
            _ => RufView::new(content_id),
        };
        document.ivars().id.set(content_id);
        register(&document, content_id, WidgetKind::Container);
        CocoaNative {
            id: content_id,
            kind: WidgetKind::Container,
            view: Rc::new(RefCell::new(Retained::into_super(document.clone()))),
            container: Some(document),
            extra: Extra::None,
        }
    }

    fn set_content_min_size(native: &CocoaNative, size: Size) {
        let key = key_of(&*native.view());
        STATE.with(|s| {
            if let Some(v) = s.borrow_mut().scrolls.get_mut(&key) {
                v.min = size;
            }
        });
        layout_scroll(key);
    }

    fn set_monospace(native: &CocoaNative, _kind: WidgetKind, monospace: bool) {
        let size = NSFont::systemFontSize();
        let font = if monospace {
            NSFont::monospacedSystemFontOfSize_weight(size, unsafe { NSFontWeightRegular })
        } else {
            NSFont::systemFontOfSize(size)
        };
        match &native.extra {
            Extra::Text(t) => t.setFont(Some(&font)),
            _ => {
                if let Some(c) = native.control() {
                    c.setFont(Some(&font));
                }
            }
        }
    }

    fn set_highlights(native: &CocoaNative, spans: &[Span]) {
        let Extra::Text(text) = &native.extra else {
            return;
        };
        let Some(storage) = (unsafe { text.textStorage() }) else {
            return;
        };
        let length = storage.length();
        let all = NSRange::new(0, length);
        let base = text
            .font()
            .unwrap_or_else(|| NSFont::systemFontOfSize(NSFont::systemFontSize()));
        let bold = NSFontManager::sharedFontManager(mtm())
            .convertFont_toHaveTrait(&base, NSFontTraitMask::BoldFontMask);
        storage.beginEditing();
        unsafe {
            storage.removeAttribute_range(NSForegroundColorAttributeName, all);
            storage.addAttribute_value_range(NSFontAttributeName, &base, all);
        }
        for span in spans {
            let range = NSRange::new(span.start_utf16, span.len_utf16);
            if range.location + range.length > length {
                continue;
            }
            unsafe {
                storage.addAttribute_value_range(
                    NSForegroundColorAttributeName,
                    &ns_color(span.color),
                    range,
                );
                if span.bold {
                    storage.addAttribute_value_range(NSFontAttributeName, &bold, range);
                }
            }
        }
        storage.endEditing();
    }

    fn set_marked_line(native: &CocoaNative, line: Option<(usize, Color)>) {
        let Extra::Text(text) = &native.extra else {
            return;
        };
        let Some(storage) = (unsafe { text.textStorage() }) else {
            return;
        };
        let length = storage.length();
        storage.beginEditing();
        unsafe {
            storage.removeAttribute_range(NSBackgroundColorAttributeName, NSRange::new(0, length))
        };
        if let Some((line, color)) = line
            && let Some((start, len)) = line_offsets(&text.string().to_string(), line)
        {
            let len = len.min(length.saturating_sub(start));
            if start < length {
                unsafe {
                    storage.addAttribute_value_range(
                        NSBackgroundColorAttributeName,
                        &ns_color(color),
                        NSRange::new(start, len),
                    )
                };
            }
        }
        storage.endEditing();
    }

    fn goto_line(native: &CocoaNative, line: usize) {
        let Extra::Text(text) = &native.extra else {
            return;
        };
        if let Some((start, _)) = line_offsets(&text.string().to_string(), line) {
            let range = NSRange::new(start, 0);
            text.setSelectedRange(range);
            text.scrollRangeToVisible(range);
        }
    }
}
