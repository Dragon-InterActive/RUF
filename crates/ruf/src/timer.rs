//! [`Timer`]: zeitgesteuerte Callbacks im GUI-Thread.

use crate::Application;
use crate::core;

/// Ein Timer, der eine Closure im GUI-Thread aufruft.
///
/// Typische Anwendung ist das Entprellen von Eingaben: bei jeder Änderung
/// [`Timer::start`] aufrufen – der Callback feuert erst, wenn eine Weile
/// nichts mehr passiert ist.
///
/// ```no_run
/// # let app = ruf::Application::new();
/// let timer = ruf::Timer::new();
/// timer.set_single_shot(true);
/// timer.on_timeout(|| println!("Ruhe eingekehrt"));
/// timer.start(300); // erneutes start() setzt die Wartezeit zurück
/// ```
///
/// Der Timer stoppt, wenn der `Timer`-Wert gedroppt wird. Für Closures in
/// `Rc` legen.
pub struct Timer {
    id: u64,
    _not_send: std::marker::PhantomData<*const ()>,
}

impl Timer {
    /// Erzeugt einen gestoppten, wiederholenden Timer.
    ///
    /// # Panics
    ///
    /// Wenn noch keine [`Application`] existiert.
    pub fn new() -> Self {
        assert!(
            Application::exists(),
            "ruf: Vor dem ersten Timer muss eine ruf::Application erzeugt werden"
        );
        Self {
            id: core::timer_new(false),
            _not_send: std::marker::PhantomData,
        }
    }

    /// Ruft `f` bei jedem Ablauf auf (mehrere Callbacks sind möglich).
    pub fn on_timeout(&self, f: impl FnMut() + 'static) {
        core::timer_on_timeout(self.id, f);
    }

    /// Startet den Timer bzw. startet ihn neu mit `msec` Millisekunden.
    pub fn start(&self, msec: i32) {
        core::timer_start(self.id, msec);
    }

    pub fn stop(&self) {
        core::timer_stop(self.id);
    }

    pub fn is_active(&self) -> bool {
        core::timer_state(self.id).0
    }

    /// Nur einmal feuern statt wiederholt.
    pub fn set_single_shot(&self, single_shot: bool) {
        core::timer_set_single_shot(self.id, single_shot);
    }

    pub fn is_single_shot(&self) -> bool {
        core::timer_state(self.id).1
    }

    /// Das zuletzt mit [`Timer::start`] gesetzte Intervall in Millisekunden.
    pub fn interval(&self) -> i32 {
        core::timer_state(self.id).2
    }

    /// Ruft `f` einmalig nach `msec` Millisekunden auf – ohne `Timer`-Wert.
    ///
    /// # Panics
    ///
    /// Wenn noch keine [`Application`] existiert.
    pub fn single_shot(msec: i32, f: impl FnOnce() + 'static) {
        assert!(
            Application::exists(),
            "ruf: Timer::single_shot benötigt eine ruf::Application"
        );
        let id = core::timer_new(true);
        let mut f = Some(f);
        core::timer_on_timeout(id, move || {
            if let Some(f) = f.take() {
                f();
            }
        });
        core::timer_start(id, msec);
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        core::timer_drop(self.id);
    }
}

impl std::fmt::Debug for Timer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timer")
            .field("active", &self.is_active())
            .field("interval", &self.interval())
            .finish()
    }
}
