//! [`GuiSender`]: Ergebnisse aus Hintergrund-Threads sicher in den GUI-Thread bringen.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::Application;
use crate::backend::{Backend, Platform};
use crate::core::guarded;

type Handler = Box<dyn FnMut(Box<dyn Any + Send>)>;

thread_local! {
    // Nur im GUI-Thread befüllt: Empfänger je GuiSender-Familie.
    static HANDLERS: RefCell<HashMap<u64, Handler>> = RefCell::new(HashMap::new());
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Führt `f` im GUI-Thread aus. Darf aus jedem Thread aufgerufen werden.
///
/// `f` muss `Send` sein und kann deshalb keine Widgets einfangen – für
/// Rückmeldungen an die Oberfläche [`GuiSender`] verwenden. Liefert `false`,
/// wenn keine [`Application`] (mehr) existiert; `f` wird dann verworfen.
pub fn post_to_gui(f: impl FnOnce() + Send + 'static) -> bool {
    Platform::post(Box::new(move || guarded(f)))
}

/// Ein Sender, der Werte aus beliebigen Threads an eine Closure im GUI-Thread
/// liefert.
///
/// Native Widgets dürfen nur im GUI-Thread benutzt werden. Lange Arbeiten
/// (Netzwerk, Dateien, externe Prozesse) laufen deshalb in einem eigenen
/// Thread und schicken ihr Ergebnis per `GuiSender` zurück. Der Handler darf
/// Widgets einfangen, denn er läuft immer im GUI-Thread.
///
/// ```no_run
/// # use ruf::{Application, Widget, WidgetKind, GuiSender};
/// # let app = Application::new();
/// let label = Widget::new(WidgetKind::Label);
/// let l = label.clone();
/// let sender = GuiSender::new(move |text: String| l.set_text(&text));
///
/// std::thread::spawn(move || {
///     let result = "fertig".to_owned(); // z. B. eine HTTP-Antwort
///     sender.send(result);
/// });
/// ```
///
/// `GuiSender` ist `Clone + Send + Sync`. Der Handler wird freigegeben, sobald
/// alle Klone gedroppt sind.
pub struct GuiSender<T: Send + 'static> {
    inner: Arc<SenderId>,
    _marker: PhantomData<fn(T)>,
}

struct SenderId(u64);

impl Drop for SenderId {
    fn drop(&mut self) {
        let id = self.0;
        // Asynchron entfernen: bereits eingereihte Werte werden noch zugestellt.
        post_to_gui(move || {
            HANDLERS.with(|h| h.borrow_mut().remove(&id));
        });
    }
}

impl<T: Send + 'static> GuiSender<T> {
    /// Meldet `handler` an. Muss im GUI-Thread aufgerufen werden.
    ///
    /// # Panics
    ///
    /// Wenn noch keine [`Application`] existiert.
    pub fn new(mut handler: impl FnMut(T) + 'static) -> Self {
        assert!(
            Application::exists(),
            "ruf: GuiSender::new muss im GUI-Thread nach der Application aufgerufen werden"
        );
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let handler: Handler = Box::new(move |value: Box<dyn Any + Send>| {
            if let Ok(value) = value.downcast::<T>() {
                handler(*value);
            }
        });
        HANDLERS.with(|h| h.borrow_mut().insert(id, handler));
        Self {
            inner: Arc::new(SenderId(id)),
            _marker: PhantomData,
        }
    }

    /// Stellt `value` dem Handler im GUI-Thread zu (asynchron, in Sendereihenfolge).
    /// Liefert `false`, wenn die Anwendung nicht mehr existiert.
    pub fn send(&self, value: T) -> bool {
        let id = self.inner.0;
        post_to_gui(move || deliver(id, Box::new(value)))
    }
}

impl<T: Send + 'static> Clone for GuiSender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            _marker: PhantomData,
        }
    }
}

impl<T: Send + 'static> std::fmt::Debug for GuiSender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuiSender")
            .field("id", &self.inner.0)
            .finish()
    }
}

fn deliver(id: u64, value: Box<dyn Any + Send>) {
    // Den Handler während des Aufrufs herausnehmen: er darf selbst neue
    // GuiSender anlegen (das bräuchte die Tabelle veränderbar).
    let Some(mut handler) = HANDLERS.with(|h| h.borrow_mut().remove(&id)) else {
        return;
    };
    guarded(|| handler(value));
    HANDLERS.with(|h| {
        h.borrow_mut().entry(id).or_insert(handler);
    });
}
