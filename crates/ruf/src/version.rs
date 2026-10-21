//! Qt-Versionsinformationen – zum Bauen und zur Laufzeit.
//!
//! RUF bindet das jeweils installierte Qt (6.x, später 7). Neuere Qt-Funktionen
//! werden in der C++-Schicht per `QT_VERSION_CHECK` freigeschaltet; mit
//! [`QtVersion::at_least`] kann Rust-Code dasselbe zur Laufzeit prüfen.

use std::fmt;

use crate::ffi::ffi;

/// Eine Qt-Version `major.minor.patch`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QtVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl QtVersion {
    pub const fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Die Qt-Version, gegen die RUF kompiliert wurde (`QT_VERSION`).
    pub fn compile_time() -> Self {
        let v = ffi::qt_compile_version();
        Self::new((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    /// Die Qt-Version, die zur Laufzeit geladen ist (`qVersion()`).
    pub fn runtime() -> Self {
        Self::parse(&ffi::qt_runtime_version().to_string()).unwrap_or_else(Self::compile_time)
    }

    /// Ist diese Version mindestens `major.minor`?
    pub const fn at_least(&self, major: u8, minor: u8) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }

    /// Liest eine Version im Format `"6.12.0"` (Patch optional).
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.trim().split('.').map(|p| p.parse::<u8>().ok());
        let major = parts.next()??;
        let minor = parts.next()??;
        let patch = parts.next().flatten().unwrap_or(0);
        Some(Self::new(major, minor, patch))
    }
}

impl fmt::Display for QtVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_compare() {
        let v = QtVersion::parse("6.12.0").unwrap();
        assert_eq!(v, QtVersion::new(6, 12, 0));
        assert!(v.at_least(6, 8));
        assert!(v.at_least(6, 12));
        assert!(!v.at_least(6, 13));
        assert!(!v.at_least(7, 0));
        assert_eq!(QtVersion::parse("7.1"), Some(QtVersion::new(7, 1, 0)));
        assert_eq!(QtVersion::parse("x"), None);
    }

    #[test]
    fn compile_time_is_qt6_or_later() {
        assert!(QtVersion::compile_time().major >= 6);
    }
}
