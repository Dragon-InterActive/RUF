//! Backend für Windows: Win32 und Common Controls über `windows-rs`.
//!
//! * Container (Fenster, Panel, Gruppe, Registerkarten-Seite, Scroll-Inhalt)
//!   sind Fenster der eigenen Klasse `RufContainer`. Ihre Fensterprozedur
//!   empfängt die Benachrichtigungen aller Kinder (`WM_COMMAND`, `WM_NOTIFY`,
//!   `WM_HSCROLL` …) und meldet sie an den Kern.
//! * Moderne Steuerelemente (Common Controls 6) werden zur Laufzeit über
//!   einen Activation-Context eingeschaltet – Programme brauchen kein
//!   eigenes Manifest.
//! * Koordinaten sind logische Pixel (96 dpi); das Backend skaliert auf die
//!   Bildschirmauflösung.
//! * Tab-Navigation zwischen Feldern über `IsDialogMessageW`.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicIsize, Ordering};

use windows::Win32::Foundation::{
    COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BACKGROUND_MODE, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC,
    CreateFontIndirectW, CreateSolidBrush, DIB_USAGE, DRAW_TEXT_FORMAT, DeleteDC, DeleteObject,
    DrawTextW, GetDC, GetDIBits, GetSysColorBrush, GetTextExtentPoint32W, HBRUSH, HDC, HFONT,
    HGDIOBJ, InvalidateRect, LOGFONTW, ReleaseDC, SYS_COLOR_INDEX, SelectObject, SetBkMode,
    SetTextColor, UpdateWindow,
};
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
use windows::Win32::System::ApplicationInstallationAndServicing::{
    ACTCTXW, ActivateActCtx, CreateActCtxW,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, LoadLibraryW};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, REG_ROUTINE_FLAGS, RegGetValueW,
};
use windows::Win32::UI::Controls::{
    ICC_BAR_CLASSES, ICC_PROGRESS_CLASS, ICC_STANDARD_CLASSES, ICC_TAB_CLASSES, ICC_UPDOWN_CLASS,
    ICC_WIN95_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, NMHDR, SetScrollInfo,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_SYSTEM_AWARE, GetDpiForSystem, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, GetKeyState};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;

use super::{Backend, MessageKind};
use crate::app::ColorScheme;
use crate::core::{self, Id, Notify, WidgetKind};
use crate::geometry::{Rect, Size};
use crate::style::{Color, Style};
use crate::text::{Key, KeySpec, Span, markup_to_plain};

// --- Konstanten (Werte aus den Windows-Headern) ------------------------------------

const WM_APP_POST: u32 = 0x8000 + 1;
const WM_SETFONT_: u32 = 0x0030;
const WM_SETREDRAW_: u32 = 0x000B;

const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const BCM_GETIDEALSIZE: u32 = 0x1601;
const BN_CLICKED: u32 = 0;
const BS_PUSHBUTTON: u32 = 0;
const BS_AUTOCHECKBOX: u32 = 3;
const BS_RADIOBUTTON: u32 = 4;
const BS_GROUPBOX: u32 = 7;

const SS_LEFTNOWORDWRAP: u32 = 0x000C;
const SS_NOPREFIX: u32 = 0x0080;
/// Einzeiliger Text senkrecht mittig (wie bei anderen Toolkits).
const SS_CENTERIMAGE: u32 = 0x0200;

const ES_MULTILINE: u32 = 0x0004;
const ES_AUTOVSCROLL: u32 = 0x0040;
const ES_AUTOHSCROLL: u32 = 0x0080;
const ES_WANTRETURN: u32 = 0x1000;
const EM_GETSEL: u32 = 0x00B0;
const EM_SETSEL: u32 = 0x00B1;
const EM_SCROLLCARET: u32 = 0x00B7;
const EM_LINEINDEX: u32 = 0x00BB;
const EM_LINELENGTH: u32 = 0x00C1;
const EM_SETPASSWORDCHAR: u32 = 0x00CC;
const EM_SETREADONLY: u32 = 0x00CF;
const EM_SETCUEBANNER: u32 = 0x1501;
const EN_CHANGE: u32 = 0x0300;
// RichEdit
const EM_SETCHARFORMAT: u32 = 0x0444;
const EM_SETEVENTMASK: u32 = 0x0445;
const EM_GETSCROLLPOS: u32 = 0x04DD;
const EM_SETSCROLLPOS: u32 = 0x04DE;
const ENM_CHANGE: isize = 0x0001;
const SCF_SELECTION: usize = 0x0001;
const SCF_ALL: usize = 0x0004;
const CFM_BOLD: u32 = 0x0000_0001;
const CFM_FACE: u32 = 0x2000_0000;
const CFM_COLOR: u32 = 0x4000_0000;
const CFM_BACKCOLOR: u32 = 0x0400_0000;
const CFE_BOLD: u32 = 0x0000_0001;
const CFE_AUTOCOLOR: u32 = 0x4000_0000;
const CFE_AUTOBACKCOLOR: u32 = 0x0400_0000;

const CBS_DROPDOWN: u32 = 0x0002;
const CBS_DROPDOWNLIST: u32 = 0x0003;
const CBS_AUTOHSCROLL: u32 = 0x0040;
const CB_ADDSTRING: u32 = 0x0143;
const CB_GETCURSEL: u32 = 0x0147;
const CB_GETLBTEXT: u32 = 0x0148;
const CB_GETLBTEXTLEN: u32 = 0x0149;
const CB_RESETCONTENT: u32 = 0x014B;
const CB_SETCURSEL: u32 = 0x014E;
const CB_SETMINVISIBLE: u32 = 0x1701;
const CBN_SELCHANGE: u32 = 1;
const CBN_EDITCHANGE: u32 = 5;

const LBS_NOTIFY: u32 = 0x0001;
const LBS_NOINTEGRALHEIGHT: u32 = 0x0100;
const LB_ADDSTRING: u32 = 0x0180;
const LB_RESETCONTENT: u32 = 0x0184;
const LB_SETCURSEL: u32 = 0x0186;
const LB_GETCURSEL: u32 = 0x0188;
const LBN_SELCHANGE: u32 = 1;

const UDS_SETBUDDYINT: u32 = 0x0002;
const UDS_ARROWKEYS: u32 = 0x0020;
const UDS_NOTHOUSANDS: u32 = 0x0080;
const UDM_SETBUDDY: u32 = 0x0469;
const UDM_SETRANGE32: u32 = 0x046F;
const UDM_SETPOS32: u32 = 0x0471;
const UDM_GETPOS32: u32 = 0x0472;

const TBS_VERT: u32 = 0x0002;
const TBS_NOTICKS: u32 = 0x0010;
const TBM_GETPOS: u32 = 0x0400;
const TBM_SETPOS: u32 = 0x0405;
const TBM_SETRANGEMIN: u32 = 0x0407;
const TBM_SETRANGEMAX: u32 = 0x0408;

const PBS_VERTICAL: u32 = 0x0004;
const PBS_MARQUEE: u32 = 0x0008;
const PBM_SETRANGE32: u32 = 0x0406;
const PBM_SETPOS: u32 = 0x0402;
const PBM_GETPOS: u32 = 0x0408;
const PBM_SETMARQUEE: u32 = 0x040A;

const TCM_DELETEITEM: u32 = 0x1308;
const TCM_GETCURSEL: u32 = 0x130B;
const TCM_SETCURSEL: u32 = 0x130C;
const TCM_ADJUSTRECT: u32 = 0x1328;
const TCM_SETITEMW: u32 = 0x133D;
const TCM_INSERTITEMW: u32 = 0x133E;
const TCN_SELCHANGE: u32 = (-551i32) as u32;
const TCIF_TEXT: u32 = 0x0001;

const TTM_ADDTOOLW: u32 = 0x0432;
const TTM_DELTOOLW: u32 = 0x0433;
const TTF_IDISHWND: u32 = 0x0001;
const TTF_SUBCLASS: u32 = 0x0010;

const WS_CHILD_: u32 = 0x4000_0000;
const WS_VISIBLE_: u32 = 0x1000_0000;
const WS_TABSTOP_: u32 = 0x0001_0000;
const WS_VSCROLL_: u32 = 0x0020_0000;
const WS_HSCROLL_: u32 = 0x0010_0000;
const WS_CLIPCHILDREN_: u32 = 0x0200_0000;
const WS_CLIPSIBLINGS_: u32 = 0x0400_0000;
const WS_OVERLAPPEDWINDOW_: u32 = 0x00CF_0000;
const WS_EX_CLIENTEDGE_: u32 = 0x0000_0200;
const WS_EX_TRANSPARENT_: u32 = 0x0000_0020;
const WS_EX_CONTROLPARENT_: u32 = 0x0001_0000;

const DLGC_WANTALLKEYS: isize = 0x0004;

// --- Hilfen -------------------------------------------------------------------------------

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn hwnd_key(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

fn send(hwnd: HWND, msg: u32, w: usize, l: isize) -> isize {
    unsafe { SendMessageW(hwnd, msg, Some(WPARAM(w)), Some(LPARAM(l))).0 }
}

fn instance() -> HINSTANCE {
    unsafe {
        GetModuleHandleW(PCWSTR::null())
            .map(|m| HINSTANCE(m.0))
            .unwrap_or_default()
    }
}

fn colorref(color: Color) -> COLORREF {
    COLORREF(u32::from(color.r) | (u32::from(color.g) << 8) | (u32::from(color.b) << 16))
}

/// Logische Pixel → Bildschirmpixel.
fn px(v: i32) -> i32 {
    let scale = STATE.with(|s| s.borrow().scale);
    (f64::from(v) * scale).round() as i32
}

/// Bildschirmpixel → logische Pixel.
fn lx(v: i32) -> i32 {
    let scale = STATE.with(|s| s.borrow().scale);
    (f64::from(v) / scale).round() as i32
}

fn window_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }
}

fn set_window_text(hwnd: HWND, text: &str) {
    let text = wide(text);
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR(text.as_ptr()));
    }
}

fn client_size(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut rect);
    }
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn style_of(hwnd: HWND) -> u32 {
    unsafe { GetWindowLongW(hwnd, GWL_STYLE) as u32 }
}

fn set_style_bits(hwnd: HWND, bits: u32, on: bool) {
    let style = style_of(hwnd);
    let new = if on { style | bits } else { style & !bits };
    unsafe {
        SetWindowLongW(hwnd, GWL_STYLE, new as i32);
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SET_WINDOW_POS_FLAGS(0x0001 | 0x0002 | 0x0004 | 0x0020),
        );
        let _ = InvalidateRect(Some(hwnd), None, true);
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct CharFormat2W {
    cbSize: u32,
    dwMask: u32,
    dwEffects: u32,
    yHeight: i32,
    yOffset: i32,
    crTextColor: u32,
    bCharSet: u8,
    bPitchAndFamily: u8,
    szFaceName: [u16; 32],
    wWeight: u16,
    sSpacing: i16,
    crBackColor: u32,
    lcid: u32,
    dwReserved: u32,
    sStyle: i16,
    wKerning: u16,
    bUnderlineType: u8,
    bAnimation: u8,
    bRevAuthor: u8,
    bUnderlineColor: u8,
}

impl CharFormat2W {
    fn new(mask: u32) -> Self {
        Self {
            cbSize: std::mem::size_of::<Self>() as u32,
            dwMask: mask,
            dwEffects: 0,
            yHeight: 0,
            yOffset: 0,
            crTextColor: 0,
            bCharSet: 0,
            bPitchAndFamily: 0,
            szFaceName: [0; 32],
            wWeight: 0,
            sSpacing: 0,
            crBackColor: 0,
            lcid: 0,
            dwReserved: 0,
            sStyle: 0,
            wKerning: 0,
            bUnderlineType: 0,
            bAnimation: 0,
            bRevAuthor: 0,
            bUnderlineColor: 0,
        }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct TcItemW {
    mask: u32,
    dwState: u32,
    dwStateMask: u32,
    pszText: *mut u16,
    cchTextMax: i32,
    iImage: i32,
    lParam: isize,
}

#[repr(C)]
#[allow(non_snake_case)]
struct ToolInfoW {
    cbSize: u32,
    uFlags: u32,
    hwnd: HWND,
    uId: usize,
    rect: RECT,
    hinst: HINSTANCE,
    lpszText: *mut u16,
    lParam: isize,
    lpReserved: *mut std::ffi::c_void,
}

// --- Zustand ------------------------------------------------------------------------------

/// Ein Steuerelement mit seiner Kern-Nummer.
#[derive(Clone, Copy)]
struct Control {
    id: Id,
    kind: WidgetKind,
}

#[derive(Default)]
struct ScrollState {
    content: isize,
    min: (i32, i32),
    offset: (i32, i32),
}

struct State {
    scale: f64,
    msg_window: HWND,
    font: HFONT,
    bold_font: HFONT,
    mono_font: HFONT,
    tooltip: HWND,
    controls: HashMap<isize, Control>,
    styles: HashMap<isize, Style>,
    brushes: HashMap<u32, HBRUSH>,
    scrolls: HashMap<isize, ScrollState>,
    tabs: HashMap<isize, Vec<HWND>>,
    spins: HashMap<isize, (HWND, HWND)>,
    shortcuts: HashMap<isize, Vec<(KeySpec, u64)>>,
    loops: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scale: 1.0,
            msg_window: HWND::default(),
            font: HFONT::default(),
            bold_font: HFONT::default(),
            mono_font: HFONT::default(),
            tooltip: HWND::default(),
            controls: HashMap::new(),
            styles: HashMap::new(),
            brushes: HashMap::new(),
            scrolls: HashMap::new(),
            tabs: HashMap::new(),
            spins: HashMap::new(),
            shortcuts: HashMap::new(),
            loops: 0,
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Nachrichtenfenster für `post` (aus anderen Threads erreichbar).
static MSG_WINDOW: AtomicIsize = AtomicIsize::new(0);
static POSTED: Mutex<VecDeque<Box<dyn FnOnce() + Send>>> = Mutex::new(VecDeque::new());

fn control(hwnd: HWND) -> Option<Control> {
    STATE.with(|s| s.borrow().controls.get(&hwnd_key(hwnd)).copied())
}

fn register(hwnd: HWND, id: Id, kind: WidgetKind) {
    STATE.with(|s| {
        s.borrow_mut()
            .controls
            .insert(hwnd_key(hwnd), Control { id, kind })
    });
}

fn default_font() -> HFONT {
    STATE.with(|s| s.borrow().font)
}

fn set_font(hwnd: HWND, font: HFONT) {
    send(hwnd, WM_SETFONT_, font.0 as usize, 1);
}

// --- Native Verweise ---------------------------------------------------------------------

#[derive(Clone)]
pub enum Extra {
    None,
    Group {
        frame: HWND,
    },
    Spin {
        edit: HWND,
        updown: HWND,
    },
    Slider {
        vertical: Rc<Cell<bool>>,
        range: Rc<Cell<(i32, i32)>>,
    },
    Scroll {
        content: HWND,
    },
}

/// Ein Win32-Steuerelement. `hwnd` kann ausgetauscht werden, wenn ein
/// Stil nur beim Erzeugen wirkt (editierbare Combobox, senkrechter Regler).
#[derive(Clone)]
pub struct WinNative {
    id: Id,
    kind: WidgetKind,
    hwnd: Rc<Cell<HWND>>,
    extra: Extra,
}

impl WinNative {
    fn hwnd(&self) -> HWND {
        self.hwnd.get()
    }

    /// Wohin Kinder kommen.
    fn container(&self) -> HWND {
        self.hwnd()
    }
}

// --- Fensterklassen -------------------------------------------------------------------------

const CONTAINER_CLASS: &str = "RufContainer";
const WINDOW_CLASS: &str = "RufWindow";
const MESSAGE_CLASS: &str = "RufMessage";

fn register_class(name: &str, proc_: WNDPROC, background: Option<HBRUSH>) {
    let name = wide(name);
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: proc_,
        hInstance: instance(),
        lpszClassName: PCWSTR(name.as_ptr()),
        hbrBackground: background.unwrap_or_default(),
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default() },
        ..Default::default()
    };
    unsafe {
        RegisterClassExW(&class);
    }
}

fn create_window(ex: u32, class: &str, text: &str, style: u32, parent: Option<HWND>) -> HWND {
    let class = wide(class);
    let text = wide(text);
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(ex),
            PCWSTR(class.as_ptr()),
            PCWSTR(text.as_ptr()),
            WINDOW_STYLE(style),
            if parent.is_some() { 0 } else { CW_USEDEFAULT },
            if parent.is_some() { 0 } else { CW_USEDEFAULT },
            px(100),
            px(30),
            parent,
            None,
            Some(instance()),
            None,
        )
        .unwrap_or_default()
    }
}

/// Erzeugt ein Kind-Steuerelement mit Standardschrift.
fn create_child(ex: u32, class: &str, style: u32, parent: HWND) -> HWND {
    let hwnd = create_window(
        ex,
        class,
        "",
        WS_CHILD_ | WS_VISIBLE_ | WS_CLIPSIBLINGS_ | style,
        Some(parent),
    );
    set_font(hwnd, default_font());
    hwnd
}

fn create_container(parent: Option<HWND>, extra_style: u32) -> HWND {
    let style = WS_CLIPCHILDREN_ | WS_CLIPSIBLINGS_ | extra_style;
    match parent {
        Some(p) => create_window(
            WS_EX_CONTROLPARENT_,
            CONTAINER_CLASS,
            "",
            WS_CHILD_ | WS_VISIBLE_ | style,
            Some(p),
        ),
        None => create_window(WS_EX_CONTROLPARENT_, CONTAINER_CLASS, "", style, None),
    }
}

// --- Fensterprozeduren ----------------------------------------------------------------------

unsafe extern "system" fn message_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER => {
            core::timer_fired(w.0 as u64);
            LRESULT(0)
        }
        WM_APP_POST => {
            loop {
                let next = POSTED.lock().map(|mut q| q.pop_front()).unwrap_or(None);
                match next {
                    Some(f) => f(),
                    None => break,
                }
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, w, l) },
    }
}

unsafe extern "system" fn container_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let child = HWND(l.0 as *mut _);
            if !child.is_invalid() {
                on_command(child, (w.0 >> 16) as u32 & 0xFFFF);
            }
            LRESULT(0)
        }
        WM_NOTIFY => {
            let header = unsafe { &*(l.0 as *const NMHDR) };
            if header.code == TCN_SELCHANGE {
                on_tab_changed(header.hwndFrom);
            }
            LRESULT(0)
        }
        WM_HSCROLL | WM_VSCROLL => {
            let child = HWND(l.0 as *mut _);
            if child.is_invalid() {
                scroll_viewport(hwnd, msg == WM_VSCROLL, (w.0 & 0xFFFF) as u32);
            } else if let Some(c) = control(child)
                && c.kind == WidgetKind::Slider
            {
                core::notify(c.id, Notify::Changed);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((w.0 >> 16) & 0xFFFF) as u16 as i16;
            if STATE.with(|s| s.borrow().scrolls.contains_key(&hwnd_key(hwnd))) {
                scroll_by(hwnd, true, -i32::from(delta) * px(40) / 120);
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, w, l) }
        }
        WM_SIZE => {
            let (width, height) = ((l.0 & 0xFFFF) as i32, ((l.0 >> 16) & 0xFFFF) as i32);
            if STATE.with(|s| s.borrow().scrolls.contains_key(&hwnd_key(hwnd))) {
                layout_viewport(hwnd);
            }
            if let Some(c) = control(hwnd) {
                if c.kind == WidgetKind::SpinBox {
                    layout_spin(hwnd, width, height);
                }
                core::notify(c.id, Notify::Resized(Size::new(lx(width), lx(height))));
            }
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORBTN | WM_CTLCOLORLISTBOX => {
            let hdc = HDC(w.0 as *mut _);
            let child = HWND(l.0 as *mut _);
            LRESULT(control_colors(hdc, child, msg).0 as isize)
        }
        WM_CLOSE => {
            if let Some(c) = control(hwnd) {
                core::close_requested(c.id);
            }
            // Der Kern blendet das Fenster aus; zerstört wird es nur über `destroy`.
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, w, l) },
    }
}

/// Eingabefeld: Enter melden und für `IsDialogMessage` alle Tasten wollen.
unsafe extern "system" fn edit_subclass(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _subclass: usize,
    id: usize,
) -> LRESULT {
    match msg {
        WM_GETDLGCODE => LRESULT(unsafe { DefSubclassProc(hwnd, msg, w, l).0 } | DLGC_WANTALLKEYS),
        WM_KEYDOWN if w.0 == 0x0D => {
            core::notify(id as Id, Notify::Activated);
            LRESULT(0)
        }
        WM_CHAR if w.0 == 0x0D => LRESULT(0),
        _ => unsafe { DefSubclassProc(hwnd, msg, w, l) },
    }
}

fn on_command(child: HWND, code: u32) {
    let Some(c) = control(child) else { return };
    match (c.kind, code) {
        (WidgetKind::PushButton | WidgetKind::CheckBox, BN_CLICKED) => {
            core::notify(c.id, Notify::Clicked)
        }
        (WidgetKind::RadioButton, BN_CLICKED) => {
            send(child, BM_SETCHECK, 1, 0);
            core::notify(c.id, Notify::Clicked);
        }
        (WidgetKind::LineEdit | WidgetKind::TextArea | WidgetKind::SpinBox, EN_CHANGE) => {
            core::notify(c.id, Notify::Changed)
        }
        (WidgetKind::ComboBox, CBN_SELCHANGE) => {
            // Bei editierbaren Comboboxen steht der neue Text erst danach im Feld.
            let index = send(child, CB_GETCURSEL, 0, 0);
            if index >= 0 && style_of(child) & 0x3 == CBS_DROPDOWN {
                let len = send(child, CB_GETLBTEXTLEN, index as usize, 0).max(0) as usize;
                let mut buf = vec![0u16; len + 1];
                send(
                    child,
                    CB_GETLBTEXT,
                    index as usize,
                    buf.as_mut_ptr() as isize,
                );
                set_window_text(child, &String::from_utf16_lossy(&buf[..len]));
            }
            core::notify(c.id, Notify::Changed);
        }
        (WidgetKind::ComboBox, CBN_EDITCHANGE) => core::notify(c.id, Notify::Changed),
        (WidgetKind::ListBox, LBN_SELCHANGE) => core::notify(c.id, Notify::Changed),
        _ => {}
    }
}

fn on_tab_changed(tab: HWND) {
    show_current_page(tab);
    if let Some(c) = control(tab) {
        core::notify(c.id, Notify::Changed);
    }
}

fn control_colors(hdc: HDC, child: HWND, msg: u32) -> HBRUSH {
    let style = STATE.with(|s| s.borrow().styles.get(&hwnd_key(child)).cloned());
    let style = style.unwrap_or_default();
    unsafe {
        if let Some(color) = style.color {
            SetTextColor(hdc, colorref(color));
        }
        match style.background {
            Some(bg) => {
                windows::Win32::Graphics::Gdi::SetBkColor(hdc, colorref(bg));
                brush(bg)
            }
            None if msg == WM_CTLCOLOREDIT || msg == WM_CTLCOLORLISTBOX => {
                GetSysColorBrush(SYS_COLOR_INDEX(5))
            }
            None => {
                SetBkMode(hdc, BACKGROUND_MODE(1));
                GetSysColorBrush(SYS_COLOR_INDEX(15))
            }
        }
    }
}

fn brush(color: Color) -> HBRUSH {
    let key = colorref(color).0;
    STATE.with(|s| {
        *s.borrow_mut()
            .brushes
            .entry(key)
            .or_insert_with(|| unsafe { CreateSolidBrush(COLORREF(key)) })
    })
}

// --- Spinbox, Registerkarten, Scrollbereich ---------------------------------------------------

fn layout_spin(container: HWND, width: i32, height: i32) {
    let Some((edit, updown)) = STATE.with(|s| s.borrow().spins.get(&hwnd_key(container)).copied())
    else {
        return;
    };
    let button = px(18).min(width / 2);
    unsafe {
        let _ = SetWindowPos(
            edit,
            None,
            0,
            0,
            width - button,
            height,
            SET_WINDOW_POS_FLAGS(0x0004 | 0x0010),
        );
        let _ = SetWindowPos(
            updown,
            None,
            width - button,
            0,
            button,
            height,
            SET_WINDOW_POS_FLAGS(0x0004 | 0x0010),
        );
    }
}

fn tab_page_rect(tab: HWND) -> RECT {
    let (w, h) = client_size(tab);
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    send(tab, TCM_ADJUSTRECT, 0, &mut rect as *mut RECT as isize);
    rect
}

fn layout_tab_pages(tab: HWND) {
    let rect = tab_page_rect(tab);
    let pages = STATE.with(|s| {
        s.borrow()
            .tabs
            .get(&hwnd_key(tab))
            .cloned()
            .unwrap_or_default()
    });
    for page in pages {
        unsafe {
            let _ = SetWindowPos(
                page,
                None,
                rect.left,
                rect.top,
                (rect.right - rect.left).max(0),
                (rect.bottom - rect.top).max(0),
                SET_WINDOW_POS_FLAGS(0x0004 | 0x0010),
            );
        }
    }
}

fn show_current_page(tab: HWND) {
    let current = send(tab, TCM_GETCURSEL, 0, 0);
    let pages = STATE.with(|s| {
        s.borrow()
            .tabs
            .get(&hwnd_key(tab))
            .cloned()
            .unwrap_or_default()
    });
    for (i, page) in pages.iter().enumerate() {
        unsafe {
            let _ = ShowWindow(
                *page,
                SHOW_WINDOW_CMD(if i as isize == current { 5 } else { 0 }),
            );
        }
    }
}

/// Inhalt an Größe und Position im Scrollbereich anpassen.
fn layout_viewport(viewport: HWND) {
    let Some((content, min, mut offset)) = STATE.with(|s| {
        s.borrow()
            .scrolls
            .get(&hwnd_key(viewport))
            .map(|v| (HWND(v.content as *mut _), v.min, v.offset))
    }) else {
        return;
    };
    let (vw, vh) = client_size(viewport);
    let (cw, ch) = (vw.max(min.0), vh.max(min.1));
    offset.0 = offset.0.clamp(0, (cw - vw).max(0));
    offset.1 = offset.1.clamp(0, (ch - vh).max(0));
    STATE.with(|s| {
        if let Some(v) = s.borrow_mut().scrolls.get_mut(&hwnd_key(viewport)) {
            v.offset = offset;
        }
    });
    for (bar, total, page, pos) in [(SB_HORZ, cw, vw, offset.0), (SB_VERT, ch, vh, offset.1)] {
        let info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SCROLLINFO_MASK(0x1 | 0x2 | 0x4),
            nMin: 0,
            nMax: total - 1,
            nPage: page.max(0) as u32,
            nPos: pos,
            nTrackPos: 0,
        };
        unsafe {
            SetScrollInfo(viewport, bar, &info, true);
        }
    }
    unsafe {
        let _ = SetWindowPos(
            content,
            None,
            -offset.0,
            -offset.1,
            cw,
            ch,
            SET_WINDOW_POS_FLAGS(0x0004 | 0x0010),
        );
    }
}

fn scroll_by(viewport: HWND, vertical: bool, delta: i32) {
    STATE.with(|s| {
        if let Some(v) = s.borrow_mut().scrolls.get_mut(&hwnd_key(viewport)) {
            if vertical {
                v.offset.1 += delta;
            } else {
                v.offset.0 += delta;
            }
        }
    });
    layout_viewport(viewport);
}

fn scroll_viewport(viewport: HWND, vertical: bool, request: u32) {
    let (vw, vh) = client_size(viewport);
    let page = if vertical { vh } else { vw };
    let delta = match request {
        0 => -px(20),
        1 => px(20),
        2 => -page,
        3 => page,
        4 | 5 => {
            let mut info = SCROLLINFO {
                cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                fMask: SCROLLINFO_MASK(0x10),
                ..Default::default()
            };
            unsafe {
                let _ = GetScrollInfo(
                    viewport,
                    if vertical { SB_VERT } else { SB_HORZ },
                    &mut info,
                );
            }
            let current = STATE.with(|s| {
                s.borrow()
                    .scrolls
                    .get(&hwnd_key(viewport))
                    .map(|v| if vertical { v.offset.1 } else { v.offset.0 })
                    .unwrap_or(0)
            });
            info.nTrackPos - current
        }
        _ => 0,
    };
    scroll_by(viewport, vertical, delta);
}

// --- Tastenkürzel und Nachrichtenschleife ------------------------------------------------------

fn key_matches(spec: &KeySpec, vk: usize) -> bool {
    let down = |key: i32| unsafe { GetKeyState(key) } < 0;
    if spec.ctrl != down(0x11) || spec.shift != down(0x10) || spec.alt != down(0x12) {
        return false;
    }
    let wanted = match &spec.key {
        Key::Char(c) => c.to_ascii_uppercase() as usize,
        Key::F(n) => 0x6F + usize::from(*n),
        Key::Enter => 0x0D,
        Key::Escape => 0x1B,
        Key::Tab => 0x09,
        Key::Delete => 0x2E,
        Key::Insert => 0x2D,
        Key::Home => 0x24,
        Key::End => 0x23,
        Key::PageUp => 0x21,
        Key::PageDown => 0x22,
        Key::Left => 0x25,
        Key::Up => 0x26,
        Key::Right => 0x27,
        Key::Down => 0x28,
    };
    wanted == vk
}

/// Verarbeitet eine Nachricht; Tastenkürzel und Tab-Navigation zuerst.
fn dispatch(msg: &MSG) {
    unsafe {
        if msg.message == WM_KEYDOWN || msg.message == WM_SYSKEYDOWN {
            let root = GetAncestor(msg.hwnd, GA_ROOT);
            let hit = STATE.with(|s| {
                s.borrow().shortcuts.get(&hwnd_key(root)).and_then(|list| {
                    list.iter()
                        .find(|(spec, _)| key_matches(spec, msg.wParam.0))
                        .map(|(_, id)| *id)
                })
            });
            if let Some(id) = hit {
                core::shortcut_fired(id);
                return;
            }
            if !root.is_invalid() && IsDialogMessageW(root, msg).as_bool() {
                return;
            }
        }
        let _ = TranslateMessage(msg);
        DispatchMessageW(msg);
    }
}

// --- Backend ---------------------------------------------------------------------------------

pub struct Win32;

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency><dependentAssembly>
    <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
      processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
  </dependentAssembly></dependency>
</assembly>"#;

/// Common Controls 6 (moderne Optik) ohne eingebettetes Manifest einschalten.
fn activate_visual_styles() {
    let path = std::env::temp_dir().join("ruf-common-controls-6.manifest");
    if std::fs::write(&path, MANIFEST).is_err() {
        return;
    }
    let source = wide(&path.to_string_lossy());
    let context = ACTCTXW {
        cbSize: std::mem::size_of::<ACTCTXW>() as u32,
        lpSource: PCWSTR(source.as_ptr()),
        ..Default::default()
    };
    unsafe {
        if let Ok(handle) = CreateActCtxW(&context) {
            let mut cookie = 0usize;
            let _ = ActivateActCtx(Some(handle), &mut cookie);
        }
    }
}

fn message_font(weight: i32, face: Option<&str>) -> HFONT {
    unsafe {
        let mut metrics = NONCLIENTMETRICSW {
            cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
            ..Default::default()
        };
        let _ = SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            metrics.cbSize,
            Some(&mut metrics as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        let mut font: LOGFONTW = metrics.lfMessageFont;
        font.lfWeight = weight;
        if let Some(face) = face {
            let face: Vec<u16> = face.encode_utf16().collect();
            font.lfFaceName = [0; 32];
            font.lfFaceName[..face.len()].copy_from_slice(&face);
        }
        CreateFontIndirectW(&font)
    }
}

fn registry_string(
    root: windows::Win32::System::Registry::HKEY,
    key: &str,
    value: &str,
) -> Option<String> {
    let (key, value) = (wide(key), wide(value));
    let mut buf = [0u16; 128];
    let mut size = (buf.len() * 2) as u32;
    let result = unsafe {
        RegGetValueW(
            root,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            REG_ROUTINE_FLAGS(0x0000_0002),
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    (result.0 == 0).then(|| {
        let len = (size as usize / 2).saturating_sub(1);
        String::from_utf16_lossy(&buf[..len.min(buf.len())])
    })
}

fn registry_dword(
    root: windows::Win32::System::Registry::HKEY,
    key: &str,
    value: &str,
) -> Option<u32> {
    let (key, value) = (wide(key), wide(value));
    let mut data = 0u32;
    let mut size = 4u32;
    let result = unsafe {
        RegGetValueW(
            root,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            REG_ROUTINE_FLAGS(0x0000_0010),
            None,
            Some((&mut data as *mut u32).cast()),
            Some(&mut size),
        )
    };
    (result.0 == 0).then_some(data)
}

fn root_window(native: Option<&WinNative>) -> Option<HWND> {
    native.map(|n| unsafe { GetAncestor(n.hwnd(), GA_ROOT) })
}

/// Erzeugt das eigentliche Steuerelement einer Art (ohne Container-Arten).
fn create_control(id: Id, kind: WidgetKind, parent: HWND, editable: bool, vertical: bool) -> HWND {
    let hwnd = match kind {
        WidgetKind::PushButton => create_child(0, "BUTTON", WS_TABSTOP_ | BS_PUSHBUTTON, parent),
        WidgetKind::CheckBox => create_child(0, "BUTTON", WS_TABSTOP_ | BS_AUTOCHECKBOX, parent),
        WidgetKind::RadioButton => create_child(0, "BUTTON", WS_TABSTOP_ | BS_RADIOBUTTON, parent),
        WidgetKind::Label => create_child(
            0,
            "STATIC",
            SS_NOPREFIX | SS_LEFTNOWORDWRAP | SS_CENTERIMAGE,
            parent,
        ),
        WidgetKind::LineEdit => {
            let edit = create_child(
                WS_EX_CLIENTEDGE_,
                "EDIT",
                WS_TABSTOP_ | ES_AUTOHSCROLL,
                parent,
            );
            unsafe {
                let _ = SetWindowSubclass(edit, Some(edit_subclass), 1, id as usize);
            }
            edit
        }
        WidgetKind::TextArea => {
            let edit = create_child(
                WS_EX_CLIENTEDGE_,
                "RICHEDIT50W",
                WS_TABSTOP_
                    | WS_VSCROLL_
                    | WS_HSCROLL_
                    | ES_MULTILINE
                    | ES_AUTOVSCROLL
                    | ES_AUTOHSCROLL
                    | ES_WANTRETURN,
                parent,
            );
            send(edit, EM_SETEVENTMASK, 0, ENM_CHANGE);
            edit
        }
        WidgetKind::ComboBox => {
            let style = if editable {
                CBS_DROPDOWN | CBS_AUTOHSCROLL
            } else {
                CBS_DROPDOWNLIST
            };
            let combo = create_child(0, "COMBOBOX", WS_TABSTOP_ | WS_VSCROLL_ | style, parent);
            send(combo, CB_SETMINVISIBLE, 12, 0);
            combo
        }
        WidgetKind::ListBox => create_child(
            WS_EX_CLIENTEDGE_,
            "LISTBOX",
            WS_TABSTOP_ | WS_VSCROLL_ | LBS_NOTIFY | LBS_NOINTEGRALHEIGHT,
            parent,
        ),
        WidgetKind::Slider => create_child(
            0,
            "msctls_trackbar32",
            WS_TABSTOP_ | TBS_NOTICKS | if vertical { TBS_VERT } else { 0 },
            parent,
        ),
        WidgetKind::ProgressBar => create_child(
            0,
            "msctls_progress32",
            if vertical { PBS_VERTICAL } else { 0 },
            parent,
        ),
        WidgetKind::TabView => {
            create_child(0, "SysTabControl32", WS_TABSTOP_ | WS_CLIPCHILDREN_, parent)
        }
        _ => create_container(Some(parent), 0),
    };
    register(hwnd, id, kind);
    hwnd
}

/// Ersetzt ein Steuerelement, dessen Stil nur beim Erzeugen wirkt.
fn recreate(native: &WinNative, editable: bool, vertical: bool) {
    let old = native.hwnd();
    let parent = unsafe { GetParent(old).unwrap_or_default() };
    let mut rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(old, &mut rect);
        let mut points = [POINT {
            x: rect.left,
            y: rect.top,
        }];
        windows::Win32::Graphics::Gdi::MapWindowPoints(None, Some(parent), &mut points);
        // Eigenes Sichtbarkeits-Bit – IsWindowVisible berücksichtigt auch die Eltern.
        let visible = style_of(old) & WS_VISIBLE_ != 0;
        let enabled = windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(old).as_bool();
        let new = create_control(native.id, native.kind, parent, editable, vertical);
        let _ = SetWindowPos(
            new,
            Some(old),
            points[0].x,
            points[0].y,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SET_WINDOW_POS_FLAGS(0x0010),
        );
        let _ = ShowWindow(new, SHOW_WINDOW_CMD(if visible { 5 } else { 0 }));
        let _ = EnableWindow(new, enabled);
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.controls.remove(&hwnd_key(old));
            if let Some(style) = s.styles.remove(&hwnd_key(old)) {
                s.styles.insert(hwnd_key(new), style);
            }
        });
        let _ = DestroyWindow(old);
        native.hwnd.set(new);
    }
}

fn measure_text(hwnd: HWND, text: &str, wrap_width: Option<i32>) -> (i32, i32) {
    unsafe {
        let hdc = GetDC(Some(hwnd));
        let font = HFONT(send(hwnd, 0x0031, 0, 0) as *mut _); // WM_GETFONT
        let old = SelectObject(
            hdc,
            HGDIOBJ(if font.is_invalid() {
                default_font().0
            } else {
                font.0
            }),
        );
        let mut buf: Vec<u16> = if text.is_empty() {
            vec![b'X' as u16]
        } else {
            text.encode_utf16().collect()
        };
        let result = match wrap_width {
            Some(w) => {
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: w,
                    bottom: 0,
                };
                DrawTextW(hdc, &mut buf, &mut rect, DRAW_TEXT_FORMAT(0x400 | 0x10)); // DT_CALCRECT | DT_WORDBREAK
                (rect.right, rect.bottom)
            }
            None => {
                let mut size = SIZE::default();
                let _ = GetTextExtentPoint32W(hdc, &buf, &mut size);
                (if text.is_empty() { 0 } else { size.cx }, size.cy)
            }
        };
        SelectObject(hdc, old);
        ReleaseDC(Some(hwnd), hdc);
        result
    }
}

fn line_range(edit: HWND, line: usize) -> (isize, isize) {
    let start = send(edit, EM_LINEINDEX, line, 0);
    let len = send(edit, EM_LINELENGTH, start.max(0) as usize, 0);
    (start, start + len + 1)
}

fn set_char_format(edit: HWND, from: isize, to: isize, format: &CharFormat2W) {
    let range = [from as i32, to as i32];
    send(edit, 0x0437, 0, range.as_ptr() as isize); // EM_EXSETSEL
    send(
        edit,
        EM_SETCHARFORMAT,
        SCF_SELECTION,
        format as *const _ as isize,
    );
}

/// Formatiert einen Textbereich ohne Flackern, Ereignisse und Auswahlverlust.
fn with_frozen_edit(edit: HWND, f: impl FnOnce()) {
    let mut start = 0u32;
    let mut end = 0u32;
    let mut scroll = POINT::default();
    send(
        edit,
        EM_GETSEL,
        &mut start as *mut u32 as usize,
        &mut end as *mut u32 as isize,
    );
    send(edit, EM_GETSCROLLPOS, 0, &mut scroll as *mut POINT as isize);
    send(edit, EM_SETEVENTMASK, 0, 0);
    send(edit, WM_SETREDRAW_, 0, 0);
    f();
    send(edit, EM_SETSEL, start as usize, end as isize);
    send(edit, EM_SETSCROLLPOS, 0, &scroll as *const POINT as isize);
    send(edit, WM_SETREDRAW_, 1, 0);
    send(edit, EM_SETEVENTMASK, 0, ENM_CHANGE);
    unsafe {
        let _ = InvalidateRect(Some(edit), None, true);
    }
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

impl Backend for Win32 {
    type Native = WinNative;
    const NAME: &'static str = "Win32";

    fn version() -> String {
        let key = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
        let product = registry_string(HKEY_LOCAL_MACHINE, key, "ProductName")
            .unwrap_or_else(|| "Windows".into());
        let build = registry_string(HKEY_LOCAL_MACHINE, key, "CurrentBuild").unwrap_or_default();
        format!("{product} (Build {build})").trim().to_owned()
    }

    fn init() {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
        }
        activate_visual_styles();
        unsafe {
            let controls = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_WIN95_CLASSES
                    | ICC_STANDARD_CLASSES
                    | ICC_TAB_CLASSES
                    | ICC_UPDOWN_CLASS
                    | ICC_PROGRESS_CLASS
                    | ICC_BAR_CLASSES,
            };
            let _ = InitCommonControlsEx(&controls);
            let msftedit = wide("Msftedit.dll");
            let _ = LoadLibraryW(PCWSTR(msftedit.as_ptr()));
        }
        let background = unsafe { GetSysColorBrush(SYS_COLOR_INDEX(15)) };
        register_class(CONTAINER_CLASS, Some(container_proc), Some(background));
        register_class(WINDOW_CLASS, Some(container_proc), Some(background));
        register_class(MESSAGE_CLASS, Some(message_proc), None);

        let dpi = unsafe { GetDpiForSystem() };
        STATE.with(|s| s.borrow_mut().scale = if dpi == 0 { 1.0 } else { f64::from(dpi) / 96.0 });

        let message = {
            let class = wide(MESSAGE_CLASS);
            unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    PCWSTR(class.as_ptr()),
                    PCWSTR::null(),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    Some(instance()),
                    None,
                )
                .unwrap_or_default()
            }
        };
        MSG_WINDOW.store(hwnd_key(message), Ordering::SeqCst);

        let tooltip = create_window(0, "tooltips_class32", "", 0x80 | 0x01 | 0x02, None); // WS_POPUP-artig: TTS_*
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.msg_window = message;
            s.font = message_font(400, None);
            s.bold_font = message_font(700, None);
            s.mono_font = message_font(400, Some("Consolas"));
            s.tooltip = tooltip;
        });
    }

    fn run() {
        STATE.with(|s| s.borrow_mut().loops += 1);
        let mut msg = MSG::default();
        loop {
            let result = unsafe { GetMessageW(&mut msg, None, 0, 0) };
            if result.0 <= 0 {
                break;
            }
            dispatch(&msg);
        }
        STATE.with(|s| s.borrow_mut().loops -= 1);
    }

    fn quit() {
        if STATE.with(|s| s.borrow().loops) > 0 {
            unsafe { PostQuitMessage(0) };
        }
    }

    fn process_events() {
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            if msg.message == WM_QUIT {
                // Gehört einer äußeren Schleife – weiterreichen.
                unsafe { PostQuitMessage(msg.wParam.0 as i32) };
                break;
            }
            dispatch(&msg);
        }
    }

    fn post(f: Box<dyn FnOnce() + Send>) -> bool {
        let window = MSG_WINDOW.load(Ordering::SeqCst);
        if window == 0 {
            return false;
        }
        if let Ok(mut queue) = POSTED.lock() {
            queue.push_back(f);
        }
        unsafe {
            PostMessageW(
                Some(HWND(window as *mut _)),
                WM_APP_POST,
                WPARAM(0),
                LPARAM(0),
            )
            .is_ok()
        }
    }

    fn timer_start(id: u64, msec: u32) {
        let window = STATE.with(|s| s.borrow().msg_window);
        unsafe {
            SetTimer(Some(window), id as usize, msec.max(1), None);
        }
    }

    fn timer_stop(id: u64) {
        let window = STATE.with(|s| s.borrow().msg_window);
        unsafe {
            let _ = KillTimer(Some(window), id as usize);
        }
    }

    fn color_scheme() -> ColorScheme {
        match registry_dword(
            HKEY_CURRENT_USER,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
            "AppsUseLightTheme",
        ) {
            Some(0) => ColorScheme::Dark,
            Some(_) => ColorScheme::Light,
            None => ColorScheme::Unknown,
        }
    }

    fn set_color_scheme(_scheme: ColorScheme) -> bool {
        // Klassische Win32-Steuerelemente kennen keinen offiziellen Dunkelmodus.
        false
    }

    fn message(parent: Option<&WinNative>, kind: MessageKind, title: &str, text: &str) {
        let icon = match kind {
            MessageKind::Information => 0x40,
            MessageKind::Warning => 0x30,
            MessageKind::Error => 0x10,
        };
        let (title, text) = (wide(title), wide(text));
        unsafe {
            MessageBoxW(
                root_window(parent),
                PCWSTR(text.as_ptr()),
                PCWSTR(title.as_ptr()),
                MESSAGEBOX_STYLE(icon),
            );
        }
    }

    fn confirm(parent: Option<&WinNative>, title: &str, text: &str) -> bool {
        let (title, text) = (wide(title), wide(text));
        let result = unsafe {
            MessageBoxW(
                root_window(parent),
                PCWSTR(text.as_ptr()),
                PCWSTR(title.as_ptr()),
                MESSAGEBOX_STYLE(0x04 | 0x20),
            )
        };
        result.0 == 6
    }

    fn create(id: Id, kind: WidgetKind, parent: Option<&WinNative>) -> WinNative {
        let parent_hwnd = parent.map(WinNative::container);
        let (hwnd, extra) = match (kind, parent_hwnd) {
            (WidgetKind::Window, _) | (_, None) => {
                let hwnd = create_window(
                    WS_EX_CONTROLPARENT_,
                    WINDOW_CLASS,
                    "",
                    WS_OVERLAPPEDWINDOW_ | WS_CLIPCHILDREN_,
                    None,
                );
                (hwnd, Extra::None)
            }
            (WidgetKind::Container, Some(p)) => (create_container(Some(p), 0), Extra::None),
            (WidgetKind::GroupBox, Some(p)) => {
                let container = create_container(Some(p), 0);
                let frame = create_child(WS_EX_TRANSPARENT_, "BUTTON", BS_GROUPBOX, container);
                (container, Extra::Group { frame })
            }
            (WidgetKind::SpinBox, Some(p)) => {
                let container = create_container(Some(p), 0);
                let edit = create_child(WS_EX_CLIENTEDGE_, "EDIT", WS_TABSTOP_ | 0x2000, container); // ES_NUMBER
                let updown = create_child(
                    0,
                    "msctls_updown32",
                    UDS_SETBUDDYINT | UDS_ARROWKEYS | UDS_NOTHOUSANDS,
                    container,
                );
                send(updown, UDM_SETBUDDY, edit.0 as usize, 0);
                register(edit, id, kind);
                STATE.with(|s| {
                    s.borrow_mut()
                        .spins
                        .insert(hwnd_key(container), (edit, updown))
                });
                (container, Extra::Spin { edit, updown })
            }
            (WidgetKind::ScrollArea, Some(p)) => {
                let viewport = create_window(
                    WS_EX_CLIENTEDGE_ | WS_EX_CONTROLPARENT_,
                    CONTAINER_CLASS,
                    "",
                    WS_CHILD_
                        | WS_VISIBLE_
                        | WS_CLIPCHILDREN_
                        | WS_CLIPSIBLINGS_
                        | WS_VSCROLL_
                        | WS_HSCROLL_,
                    Some(p),
                );
                let content = create_container(Some(viewport), 0);
                STATE.with(|s| {
                    s.borrow_mut().scrolls.insert(
                        hwnd_key(viewport),
                        ScrollState {
                            content: hwnd_key(content),
                            ..Default::default()
                        },
                    )
                });
                (viewport, Extra::Scroll { content })
            }
            (WidgetKind::Slider, Some(p)) => (
                create_control(id, kind, p, false, false),
                Extra::Slider {
                    vertical: Rc::new(Cell::new(false)),
                    range: Rc::new(Cell::new((0, 99))),
                },
            ),
            (_, Some(p)) => (create_control(id, kind, p, false, false), Extra::None),
        };
        register(hwnd, id, kind);
        WinNative {
            id,
            kind,
            hwnd: Rc::new(Cell::new(hwnd)),
            extra,
        }
    }

    fn destroy(native: &WinNative, _kind: WidgetKind) {
        let hwnd = native.hwnd();
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            let key = hwnd_key(hwnd);
            s.shortcuts.remove(&key);
            s.scrolls.remove(&key);
            s.tabs.remove(&key);
            s.spins.remove(&key);
            s.styles.remove(&key);
            // Einträge der Kinder: anhand der Kern-Nummern nicht auffindbar –
            // tote HWNDs stören nicht, sie werden nie mehr gemeldet.
            s.controls.retain(|_, c| c.id != native.id);
        });
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }

    fn set_parent(native: &WinNative, kind: WidgetKind, parent: &WinNative) {
        if kind == WidgetKind::Window {
            return;
        }
        unsafe {
            let _ = SetParent(native.hwnd(), Some(parent.container()));
        }
    }

    fn class_name(native: &WinNative, _kind: WidgetKind) -> String {
        let mut buf = [0u16; 64];
        let n = unsafe { GetClassNameW(native.hwnd(), &mut buf) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    fn set_geometry(native: &WinNative, kind: WidgetKind, rect: Rect) {
        let hwnd = native.hwnd();
        let (w, h) = (px(rect.width.max(0)), px(rect.height.max(0)));
        unsafe {
            if kind == WidgetKind::Window && GetParent(hwnd).is_err() {
                let style = WINDOW_STYLE(style_of(hwnd));
                let mut outer = RECT {
                    left: 0,
                    top: 0,
                    right: w,
                    bottom: h,
                };
                let _ = AdjustWindowRectEx(
                    &mut outer,
                    style,
                    false,
                    WINDOW_EX_STYLE(WS_EX_CONTROLPARENT_),
                );
                // (0, 0) heißt "noch nicht positioniert": Windows wählt die Stelle.
                let mut flags = 0x0004 | 0x0010;
                if rect.x == 0 && rect.y == 0 {
                    flags |= 0x0002;
                }
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    px(rect.x) + outer.left,
                    px(rect.y) + outer.top,
                    outer.right - outer.left,
                    outer.bottom - outer.top,
                    SET_WINDOW_POS_FLAGS(flags),
                );
                return;
            }
            // Comboboxen: die Höhe umfasst auch die aufgeklappte Liste.
            let height = if kind == WidgetKind::ComboBox {
                h + px(220)
            } else {
                h
            };
            let _ = SetWindowPos(
                hwnd,
                None,
                px(rect.x),
                px(rect.y),
                w,
                height,
                SET_WINDOW_POS_FLAGS(0x0004 | 0x0010),
            );
        }
        match &native.extra {
            Extra::Group { frame } => unsafe {
                let _ = SetWindowPos(
                    *frame,
                    Some(HWND_BOTTOM),
                    0,
                    0,
                    w,
                    h,
                    SET_WINDOW_POS_FLAGS(0x0010),
                );
            },
            Extra::Spin { .. } => layout_spin(hwnd, w, h),
            _ => {}
        }
        if kind == WidgetKind::TabView {
            layout_tab_pages(hwnd);
        }
    }

    fn preferred_size(native: &WinNative, kind: WidgetKind) -> Size {
        let hwnd = native.hwnd();
        let text = match kind {
            WidgetKind::Label => markup_to_plain(&window_text(hwnd)),
            _ => window_text(hwnd),
        };
        let (tw, th) = measure_text(hwnd, &text, None);
        let (w, h) = match kind {
            WidgetKind::PushButton | WidgetKind::CheckBox | WidgetKind::RadioButton => {
                let mut ideal = SIZE::default();
                if send(hwnd, BCM_GETIDEALSIZE, 0, &mut ideal as *mut SIZE as isize) != 0
                    && ideal.cx > 0
                {
                    let extra = if kind == WidgetKind::PushButton {
                        px(16)
                    } else {
                        0
                    };
                    (ideal.cx + extra, ideal.cy.max(th + px(8)))
                } else {
                    (
                        tw + px(if kind == WidgetKind::PushButton {
                            24
                        } else {
                            20
                        }),
                        th + px(10),
                    )
                }
            }
            WidgetKind::Label => (tw, th),
            WidgetKind::LineEdit | WidgetKind::ComboBox | WidgetKind::SpinBox => {
                (px(120).max(tw + px(30)), th + px(10))
            }
            _ => (px(100), px(30)),
        };
        Size::new(lx(w), lx(h))
    }

    fn set_visible(native: &WinNative, kind: WidgetKind, visible: bool) {
        unsafe {
            let _ = ShowWindow(native.hwnd(), SHOW_WINDOW_CMD(if visible { 5 } else { 0 }));
            if visible && kind == WidgetKind::Window {
                let _ = UpdateWindow(native.hwnd());
            }
        }
    }

    fn set_enabled(native: &WinNative, _kind: WidgetKind, enabled: bool) {
        unsafe {
            let _ = EnableWindow(native.hwnd(), enabled);
            if let Extra::Spin { edit, updown } = &native.extra {
                let _ = EnableWindow(*edit, enabled);
                let _ = EnableWindow(*updown, enabled);
            }
        }
    }

    fn set_tool_tip(native: &WinNative, _kind: WidgetKind, text: &str) {
        let tooltip = STATE.with(|s| s.borrow().tooltip);
        let target = match &native.extra {
            Extra::Spin { edit, .. } => *edit,
            _ => native.hwnd(),
        };
        let mut text = wide(text);
        let mut info = ToolInfoW {
            cbSize: std::mem::size_of::<ToolInfoW>() as u32,
            uFlags: TTF_IDISHWND | TTF_SUBCLASS,
            hwnd: unsafe { GetParent(target).unwrap_or_default() },
            uId: target.0 as usize,
            rect: RECT::default(),
            hinst: HINSTANCE::default(),
            lpszText: text.as_mut_ptr(),
            lParam: 0,
            lpReserved: std::ptr::null_mut(),
        };
        send(
            tooltip,
            TTM_DELTOOLW,
            0,
            &mut info as *mut ToolInfoW as isize,
        );
        if text.len() > 1 {
            send(
                tooltip,
                TTM_ADDTOOLW,
                0,
                &mut info as *mut ToolInfoW as isize,
            );
        }
    }

    fn set_style(native: &WinNative, kind: WidgetKind, style: &Style) {
        let hwnd = native.hwnd();
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            if style.is_empty() {
                s.styles.remove(&hwnd_key(hwnd));
            } else {
                s.styles.insert(hwnd_key(hwnd), style.clone());
            }
        });
        let font = STATE.with(|s| {
            let s = s.borrow();
            if style.bold { s.bold_font } else { s.font }
        });
        if kind != WidgetKind::TextArea {
            set_font(hwnd, font);
        }
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, true);
        }
    }

    fn screenshot(native: &WinNative, kind: WidgetKind, path: &Path) -> bool {
        let hwnd = native.hwnd();
        let root = unsafe { GetAncestor(hwnd, GA_ROOT) };
        let hidden = !unsafe { IsWindowVisible(root) }.as_bool();
        unsafe {
            if hidden {
                let _ = ShowWindow(root, SHOW_WINDOW_CMD(4)); // SW_SHOWNOACTIVATE
            }
            let _ = UpdateWindow(root);
        }
        Win32::process_events();
        let (w, h) = client_size(hwnd);
        let ok = w > 0
            && h > 0
            && unsafe {
                let screen = GetDC(None);
                let memory = CreateCompatibleDC(Some(screen));
                let bitmap = CreateCompatibleBitmap(screen, w, h);
                let old = SelectObject(memory, HGDIOBJ(bitmap.0));
                let flags = if kind == WidgetKind::Window {
                    0x1 | 0x2
                } else {
                    0x2
                }; // PW_CLIENTONLY | PW_RENDERFULLCONTENT
                let printed = PrintWindow(hwnd, memory, PRINT_WINDOW_FLAGS(flags)).as_bool();
                let mut info = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: w,
                        biHeight: -h,
                        biPlanes: 1,
                        biBitCount: 32,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let mut pixels = vec![0u8; (w * h * 4) as usize];
                SelectObject(memory, old);
                let lines = GetDIBits(
                    memory,
                    bitmap,
                    0,
                    h as u32,
                    Some(pixels.as_mut_ptr().cast()),
                    &mut info,
                    DIB_USAGE(0),
                );
                let _ = DeleteObject(HGDIOBJ(bitmap.0));
                let _ = DeleteDC(memory);
                ReleaseDC(None, screen);
                // BGRA → RGBA, voll deckend.
                for px in pixels.as_chunks_mut::<4>().0 {
                    px.swap(0, 2);
                    px[3] = 255;
                }
                printed && lines > 0 && write_png(path, w as u32, h as u32, &pixels)
            };
        if hidden {
            unsafe {
                let _ = ShowWindow(root, SHOW_WINDOW_CMD(0));
            }
        }
        ok
    }

    fn add_shortcut(window: &WinNative, key: &KeySpec, shortcut: u64) -> bool {
        let root = unsafe { GetAncestor(window.hwnd(), GA_ROOT) };
        STATE.with(|s| {
            s.borrow_mut()
                .shortcuts
                .entry(hwnd_key(root))
                .or_default()
                .push((key.clone(), shortcut))
        });
        true
    }

    fn set_text(native: &WinNative, kind: WidgetKind, text: &str) {
        let hwnd = native.hwnd();
        match (kind, &native.extra) {
            (WidgetKind::GroupBox, Extra::Group { frame }) => set_window_text(*frame, text),
            (WidgetKind::Label, _) => {
                set_window_text(hwnd, &markup_to_plain(text).replace('\n', "\r\n"))
            }
            (WidgetKind::TextArea, _) => set_window_text(hwnd, &text.replace('\n', "\r\n")),
            _ => set_window_text(hwnd, text),
        }
    }

    fn text(native: &WinNative, kind: WidgetKind) -> String {
        match kind {
            WidgetKind::TextArea => normalize_newlines(&window_text(native.hwnd())),
            _ => window_text(native.hwnd()),
        }
    }

    fn set_placeholder(native: &WinNative, kind: WidgetKind, text: &str) {
        // Mehrzeilige Felder kennen unter Win32 keinen Platzhalter.
        if matches!(kind, WidgetKind::LineEdit | WidgetKind::ComboBox) {
            let text = wide(text);
            let message = if kind == WidgetKind::ComboBox {
                0x1703
            } else {
                EM_SETCUEBANNER
            };
            send(native.hwnd(), message, 1, text.as_ptr() as isize);
        }
    }

    fn set_password(native: &WinNative, enabled: bool) {
        send(
            native.hwnd(),
            EM_SETPASSWORDCHAR,
            if enabled { 0x25CF } else { 0 },
            0,
        );
        unsafe {
            let _ = InvalidateRect(Some(native.hwnd()), None, true);
        }
    }

    fn set_read_only(native: &WinNative, _kind: WidgetKind, read_only: bool) {
        let target = match &native.extra {
            Extra::Spin { edit, .. } => *edit,
            _ => native.hwnd(),
        };
        send(target, EM_SETREADONLY, usize::from(read_only), 0);
    }

    fn set_word_wrap(native: &WinNative, wrap: bool) {
        // Umbrechender Text steht oben, einzeiliger senkrecht mittig.
        set_style_bits(native.hwnd(), SS_LEFTNOWORDWRAP | SS_CENTERIMAGE, !wrap);
    }

    fn set_checked(native: &WinNative, _kind: WidgetKind, checked: bool) {
        send(native.hwnd(), BM_SETCHECK, usize::from(checked), 0);
    }

    fn is_checked(native: &WinNative, kind: WidgetKind) -> bool {
        matches!(kind, WidgetKind::CheckBox | WidgetKind::RadioButton)
            && send(native.hwnd(), BM_GETCHECK, 0, 0) == 1
    }

    fn set_items(native: &WinNative, kind: WidgetKind, items: &[String]) {
        let hwnd = native.hwnd();
        let (reset, add) = match kind {
            WidgetKind::ComboBox => (CB_RESETCONTENT, CB_ADDSTRING),
            WidgetKind::ListBox => (LB_RESETCONTENT, LB_ADDSTRING),
            _ => return,
        };
        send(hwnd, reset, 0, 0);
        for item in items {
            let text = wide(item);
            send(hwnd, add, 0, text.as_ptr() as isize);
        }
    }

    fn set_current_index(native: &WinNative, kind: WidgetKind, index: i32) {
        let hwnd = native.hwnd();
        match kind {
            WidgetKind::ComboBox => {
                send(hwnd, CB_SETCURSEL, index as isize as usize, 0);
            }
            WidgetKind::ListBox => {
                send(hwnd, LB_SETCURSEL, index as isize as usize, 0);
            }
            WidgetKind::TabView => {
                send(hwnd, TCM_SETCURSEL, index as isize as usize, 0);
                show_current_page(hwnd);
            }
            _ => {}
        }
    }

    fn current_index(native: &WinNative, kind: WidgetKind) -> i32 {
        let message = match kind {
            WidgetKind::ComboBox => CB_GETCURSEL,
            WidgetKind::ListBox => LB_GETCURSEL,
            WidgetKind::TabView => TCM_GETCURSEL,
            _ => return -1,
        };
        send(native.hwnd(), message, 0, 0) as i32
    }

    fn set_editable(native: &WinNative, editable: bool) -> Option<WinNative> {
        recreate(native, editable, false);
        None
    }

    fn tab_add(native: &WinNative, page_id: Id, title: &str) -> WinNative {
        let tab = native.hwnd();
        let mut text = wide(title);
        let count = STATE.with(|s| s.borrow().tabs.get(&hwnd_key(tab)).map_or(0, Vec::len));
        let item = TcItemW {
            mask: TCIF_TEXT,
            dwState: 0,
            dwStateMask: 0,
            pszText: text.as_mut_ptr(),
            cchTextMax: 0,
            iImage: -1,
            lParam: 0,
        };
        send(
            tab,
            TCM_INSERTITEMW,
            count,
            &item as *const TcItemW as isize,
        );
        let page = create_container(Some(tab), 0);
        register(page, page_id, WidgetKind::Container);
        STATE.with(|s| {
            s.borrow_mut()
                .tabs
                .entry(hwnd_key(tab))
                .or_default()
                .push(page)
        });
        layout_tab_pages(tab);
        show_current_page(tab);
        WinNative {
            id: page_id,
            kind: WidgetKind::Container,
            hwnd: Rc::new(Cell::new(page)),
            extra: Extra::None,
        }
    }

    fn tab_remove(native: &WinNative, index: i32) {
        let tab = native.hwnd();
        let page = STATE.with(|s| {
            s.borrow_mut()
                .tabs
                .get_mut(&hwnd_key(tab))
                .and_then(|pages| {
                    (index >= 0 && (index as usize) < pages.len())
                        .then(|| pages.remove(index as usize))
                })
        });
        send(tab, TCM_DELETEITEM, index as usize, 0);
        if let Some(page) = page {
            unsafe {
                let _ = DestroyWindow(page);
            }
        }
        if send(tab, TCM_GETCURSEL, 0, 0) < 0
            && STATE.with(|s| {
                s.borrow()
                    .tabs
                    .get(&hwnd_key(tab))
                    .is_some_and(|p| !p.is_empty())
            })
        {
            send(tab, TCM_SETCURSEL, 0, 0);
        }
        show_current_page(tab);
    }

    fn tab_set_title(native: &WinNative, index: i32, title: &str) {
        let mut text = wide(title);
        let item = TcItemW {
            mask: TCIF_TEXT,
            dwState: 0,
            dwStateMask: 0,
            pszText: text.as_mut_ptr(),
            cchTextMax: 0,
            iImage: -1,
            lParam: 0,
        };
        send(
            native.hwnd(),
            TCM_SETITEMW,
            index as usize,
            &item as *const TcItemW as isize,
        );
    }

    fn set_range(native: &WinNative, kind: WidgetKind, minimum: i32, maximum: i32) {
        let hwnd = native.hwnd();
        match (kind, &native.extra) {
            (WidgetKind::SpinBox, Extra::Spin { updown, .. }) => {
                send(
                    *updown,
                    UDM_SETRANGE32,
                    minimum as isize as usize,
                    maximum as isize,
                );
            }
            (WidgetKind::Slider, Extra::Slider { range, .. }) => {
                range.set((minimum, maximum));
                send(hwnd, TBM_SETRANGEMIN, 0, minimum as isize);
                send(hwnd, TBM_SETRANGEMAX, 1, maximum as isize);
            }
            (WidgetKind::ProgressBar, _) => {
                let busy = minimum == 0 && maximum == 0;
                set_style_bits(hwnd, PBS_MARQUEE, busy);
                send(hwnd, PBM_SETMARQUEE, usize::from(busy), 30);
                if !busy {
                    send(
                        hwnd,
                        PBM_SETRANGE32,
                        minimum as isize as usize,
                        maximum as isize,
                    );
                }
            }
            _ => {}
        }
    }

    fn set_value(native: &WinNative, kind: WidgetKind, value: i32) {
        let hwnd = native.hwnd();
        match (kind, &native.extra) {
            (WidgetKind::SpinBox, Extra::Spin { updown, .. }) => {
                send(*updown, UDM_SETPOS32, 0, value as isize);
            }
            (WidgetKind::Slider, Extra::Slider { vertical, range }) => {
                // Senkrecht: Minimum unten (Win32 zählt von oben).
                let (min, max) = range.get();
                let pos = if vertical.get() {
                    max + min - value
                } else {
                    value
                };
                send(hwnd, TBM_SETPOS, 1, pos as isize);
            }
            (WidgetKind::ProgressBar, _) => {
                send(hwnd, PBM_SETPOS, value as isize as usize, 0);
            }
            _ => {}
        }
    }

    fn value(native: &WinNative, kind: WidgetKind) -> i32 {
        let hwnd = native.hwnd();
        match (kind, &native.extra) {
            (WidgetKind::SpinBox, Extra::Spin { updown, .. }) => {
                send(*updown, UDM_GETPOS32, 0, 0) as i32
            }
            (WidgetKind::Slider, Extra::Slider { vertical, range }) => {
                let pos = send(hwnd, TBM_GETPOS, 0, 0) as i32;
                let (min, max) = range.get();
                if vertical.get() { max + min - pos } else { pos }
            }
            (WidgetKind::ProgressBar, _) => send(hwnd, PBM_GETPOS, 0, 0) as i32,
            _ => 0,
        }
    }

    fn set_vertical(native: &WinNative, kind: WidgetKind, vertical: bool) {
        match (kind, &native.extra) {
            (
                WidgetKind::Slider,
                Extra::Slider {
                    vertical: flag,
                    range,
                },
            ) => {
                let value = Win32::value(native, kind);
                flag.set(vertical);
                recreate(native, false, vertical);
                let (min, max) = range.get();
                Win32::set_range(native, kind, min, max);
                Win32::set_value(native, kind, value);
            }
            (WidgetKind::ProgressBar, _) => {
                let (value, min, max) = (
                    send(native.hwnd(), PBM_GETPOS, 0, 0),
                    send(native.hwnd(), 0x0407, 1, 0), // PBM_GETRANGE(TRUE): Minimum
                    send(native.hwnd(), 0x0407, 0, 0), // Maximum
                );
                recreate(native, false, vertical);
                send(native.hwnd(), PBM_SETRANGE32, min as usize, max);
                send(native.hwnd(), PBM_SETPOS, value as usize, 0);
            }
            _ => {}
        }
    }

    fn scroll_content(native: &WinNative, content_id: Id) -> WinNative {
        let content = match &native.extra {
            Extra::Scroll { content } => *content,
            _ => create_container(Some(native.hwnd()), 0),
        };
        register(content, content_id, WidgetKind::Container);
        WinNative {
            id: content_id,
            kind: WidgetKind::Container,
            hwnd: Rc::new(Cell::new(content)),
            extra: Extra::None,
        }
    }

    fn set_content_min_size(native: &WinNative, size: Size) {
        let viewport = native.hwnd();
        let min = (px(size.width.max(0)), px(size.height.max(0)));
        STATE.with(|s| {
            if let Some(v) = s.borrow_mut().scrolls.get_mut(&hwnd_key(viewport)) {
                v.min = min;
            }
        });
        layout_viewport(viewport);
    }

    fn set_monospace(native: &WinNative, _kind: WidgetKind, monospace: bool) {
        let font = STATE.with(|s| {
            let s = s.borrow();
            if monospace { s.mono_font } else { s.font }
        });
        set_font(native.hwnd(), font);
    }

    fn set_highlights(native: &WinNative, spans: &[Span]) {
        let edit = native.hwnd();
        with_frozen_edit(edit, || {
            // Alles zurücksetzen, dann die Bereiche einfärben.
            let mut plain = CharFormat2W::new(CFM_COLOR | CFM_BOLD);
            plain.dwEffects = CFE_AUTOCOLOR;
            send(edit, EM_SETCHARFORMAT, SCF_ALL, &plain as *const _ as isize);
            for span in spans {
                let mut format = CharFormat2W::new(CFM_COLOR | CFM_BOLD);
                format.crTextColor = colorref(span.color).0;
                format.dwEffects = if span.bold { CFE_BOLD } else { 0 };
                let from = span.start_utf16 as isize;
                set_char_format(edit, from, from + span.len_utf16 as isize, &format);
            }
        });
    }

    fn set_marked_line(native: &WinNative, line: Option<(usize, Color)>) {
        let edit = native.hwnd();
        with_frozen_edit(edit, || {
            let mut clear = CharFormat2W::new(CFM_BACKCOLOR);
            clear.dwEffects = CFE_AUTOBACKCOLOR;
            send(edit, EM_SETCHARFORMAT, SCF_ALL, &clear as *const _ as isize);
            if let Some((line, color)) = line {
                let (from, to) = line_range(edit, line);
                if from >= 0 {
                    let mut mark = CharFormat2W::new(CFM_BACKCOLOR);
                    mark.crBackColor = colorref(color).0;
                    set_char_format(edit, from, to, &mark);
                }
            }
        });
        let _ = CFM_FACE;
    }

    fn goto_line(native: &WinNative, line: usize) {
        let edit = native.hwnd();
        let start = send(edit, EM_LINEINDEX, line, 0);
        if start >= 0 {
            send(edit, EM_SETSEL, start as usize, start);
            send(edit, EM_SCROLLCARET, 0, 0);
        }
    }
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> bool {
    let Ok(file) = std::fs::File::create(path) else {
        return false;
    };
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let Ok(mut writer) = encoder.write_header() else {
        return false;
    };
    writer.write_image_data(rgba).is_ok()
}
