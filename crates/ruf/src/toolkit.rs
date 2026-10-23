//! Welches native Toolkit läuft?

use crate::backend::{Backend, Platform};

/// Name und Version des nativen Toolkits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolkit {
    /// `"GTK"`.
    pub name: &'static str,
    /// Version des Toolkits bzw. Betriebssystems zur Laufzeit.
    pub version: String,
}

/// Das aktive Toolkit, z. B. für "Über"-Dialoge oder Fehlerberichte.
///
/// ```no_run
/// let app = ruf::Application::new();
/// println!("{} {}", ruf::toolkit().name, ruf::toolkit().version);
/// ```
pub fn toolkit() -> Toolkit {
    Toolkit {
        name: Platform::NAME,
        version: Platform::version(),
    }
}

impl std::fmt::Display for Toolkit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.version)
    }
}
