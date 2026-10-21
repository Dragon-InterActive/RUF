//! Das Eigenschaftsmodell – die Sprache zwischen Komponenten und dem
//! Form-Designer von Freya TD.
//!
//! Jede Komponente beschreibt ihre Eigenschaften mit [`PropertyDescriptor`]s.
//! Der Designer kann sie so auflisten, anzeigen und per Name setzen, ohne den
//! konkreten Rust-Typ zu kennen.

use std::fmt;

/// Der Datentyp einer Eigenschaft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyType {
    Bool,
    Int,
    Text,
}

impl fmt::Display for PropertyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Text => "text",
        })
    }
}

/// Ein konkreter Eigenschaftswert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PropertyValue {
    Bool(bool),
    Int(i32),
    Text(String),
}

impl PropertyValue {
    pub fn ty(&self) -> PropertyType {
        match self {
            Self::Bool(_) => PropertyType::Bool,
            Self::Int(_) => PropertyType::Int,
            Self::Text(_) => PropertyType::Text,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i32> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(v) => Some(v),
            _ => None,
        }
    }
}

impl fmt::Display for PropertyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(v) => write!(f, "{v}"),
            Self::Int(v) => write!(f, "{v}"),
            Self::Text(v) => f.write_str(v),
        }
    }
}

impl From<bool> for PropertyValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i32> for PropertyValue {
    fn from(v: i32) -> Self {
        Self::Int(v)
    }
}

impl From<&str> for PropertyValue {
    fn from(v: &str) -> Self {
        Self::Text(v.to_owned())
    }
}

impl From<String> for PropertyValue {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}

/// Beschreibt eine Eigenschaft einer Komponente.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PropertyDescriptor {
    /// Eindeutiger Name in `snake_case`, z. B. `"text"` oder `"width"`.
    pub name: &'static str,
    pub ty: PropertyType,
    /// Kurzbeschreibung für Designer-Tooltips und KI-Assistenten.
    pub description: &'static str,
    /// Nur lesbar (z. B. berechnete Werte)?
    pub read_only: bool,
}

impl PropertyDescriptor {
    pub const fn new(name: &'static str, ty: PropertyType, description: &'static str) -> Self {
        Self {
            name,
            ty,
            description,
            read_only: false,
        }
    }

    pub const fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

/// Fehler beim Setzen einer Eigenschaft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyError {
    /// Die Komponente kennt keine Eigenschaft dieses Namens.
    Unknown { name: String },
    /// Der Wert hat den falschen Typ.
    TypeMismatch {
        name: String,
        expected: PropertyType,
        found: PropertyType,
    },
    /// Die Eigenschaft kann nicht geschrieben werden.
    ReadOnly { name: String },
}

impl PropertyError {
    pub fn unknown(name: &str) -> Self {
        Self::Unknown {
            name: name.to_owned(),
        }
    }

    pub fn type_mismatch(name: &str, expected: PropertyType, value: &PropertyValue) -> Self {
        Self::TypeMismatch {
            name: name.to_owned(),
            expected,
            found: value.ty(),
        }
    }
}

impl fmt::Display for PropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown { name } => write!(f, "unbekannte Eigenschaft `{name}`"),
            Self::TypeMismatch {
                name,
                expected,
                found,
            } => write!(
                f,
                "Eigenschaft `{name}` erwartet `{expected}`, erhalten `{found}`"
            ),
            Self::ReadOnly { name } => write!(f, "Eigenschaft `{name}` ist schreibgeschützt"),
        }
    }
}

impl std::error::Error for PropertyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_accessors() {
        assert_eq!(PropertyValue::from(true).as_bool(), Some(true));
        assert_eq!(PropertyValue::from(3).as_int(), Some(3));
        assert_eq!(PropertyValue::from("a").as_text(), Some("a"));
        assert_eq!(PropertyValue::from("a").as_int(), None);
    }

    #[test]
    fn error_messages() {
        let e = PropertyError::type_mismatch("x", PropertyType::Int, &"a".into());
        assert_eq!(
            e.to_string(),
            "Eigenschaft `x` erwartet `int`, erhalten `text`"
        );
    }
}
