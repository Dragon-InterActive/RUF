//! Gemeinsame Eigenschafts-Helfer für Komponenten mit gleichartigen Eigenschaften.
//!
//! Jeder Helfer liefert Deskriptoren, liest per `get` und schreibt per `set`.
//! `set` gibt `None` zurück, wenn der Name nicht zu ihm gehört – dann ist der
//! nächste Helfer bzw. [`ruf::base::set`] an der Reihe.

/// Eigenschaft `text`.
pub(crate) mod text {
    use ruf::{PropertyDescriptor, PropertyType, PropertyValue, Widget};

    pub const TEXT: &str = "text";

    pub fn descriptor(description: &'static str) -> PropertyDescriptor {
        PropertyDescriptor::new(TEXT, PropertyType::Text, description)
    }

    pub fn get(widget: &Widget, name: &str) -> Option<PropertyValue> {
        (name == TEXT).then(|| widget.text().into())
    }
}

/// Eigenschaften `items` und `current_index` (Combobox, Liste).
pub(crate) mod items {
    use ruf::{PropertyDescriptor, PropertyError, PropertyType, PropertyValue, Widget, base};

    pub const ITEMS: &str = "items";
    pub const CURRENT_INDEX: &str = "current_index";

    pub fn descriptors() -> [PropertyDescriptor; 2] {
        [
            PropertyDescriptor::new(ITEMS, PropertyType::TextList, "Einträge"),
            PropertyDescriptor::new(
                CURRENT_INDEX,
                PropertyType::Int,
                "Ausgewählter Eintrag (-1 = keiner)",
            ),
        ]
    }

    pub fn get(widget: &Widget, name: &str) -> Option<PropertyValue> {
        match name {
            ITEMS => Some(widget.items().into()),
            CURRENT_INDEX => Some(widget.current_index().into()),
            _ => None,
        }
    }

    pub fn set(
        widget: &Widget,
        name: &str,
        value: &PropertyValue,
    ) -> Option<Result<(), PropertyError>> {
        let result = match name {
            ITEMS => base::expect_text_list(name, value).map(|v| widget.set_items(v)),
            CURRENT_INDEX => base::expect_int(name, value).map(|v| widget.set_current_index(v)),
            _ => return None,
        };
        Some(result)
    }
}

/// Eigenschaften `value`, `minimum`, `maximum` (Spinbox, Schieberegler, Fortschritt).
pub(crate) mod range {
    use ruf::{PropertyDescriptor, PropertyError, PropertyType, PropertyValue, Widget, base};

    pub const VALUE: &str = "value";
    pub const MINIMUM: &str = "minimum";
    pub const MAXIMUM: &str = "maximum";

    pub fn descriptors() -> [PropertyDescriptor; 3] {
        [
            PropertyDescriptor::new(VALUE, PropertyType::Int, "Aktueller Wert"),
            PropertyDescriptor::new(MINIMUM, PropertyType::Int, "Kleinster Wert"),
            PropertyDescriptor::new(MAXIMUM, PropertyType::Int, "Größter Wert"),
        ]
    }

    pub fn get(widget: &Widget, name: &str) -> Option<PropertyValue> {
        match name {
            VALUE => Some(widget.value().into()),
            MINIMUM => Some(widget.minimum().into()),
            MAXIMUM => Some(widget.maximum().into()),
            _ => None,
        }
    }

    pub fn set(
        widget: &Widget,
        name: &str,
        value: &PropertyValue,
    ) -> Option<Result<(), PropertyError>> {
        let result = match name {
            VALUE => base::expect_int(name, value).map(|v| widget.set_value(v)),
            MINIMUM => base::expect_int(name, value).map(|v| widget.set_range(v, widget.maximum())),
            MAXIMUM => base::expect_int(name, value).map(|v| widget.set_range(widget.minimum(), v)),
            _ => return None,
        };
        Some(result)
    }
}

/// Eigenschaft `vertical` (Schieberegler, Fortschritt).
pub(crate) mod orientation {
    use ruf::{PropertyDescriptor, PropertyError, PropertyType, PropertyValue, Widget, base};

    pub const VERTICAL: &str = "vertical";

    pub fn descriptor() -> PropertyDescriptor {
        PropertyDescriptor::new(VERTICAL, PropertyType::Bool, "Senkrechte Ausrichtung")
    }

    pub fn get(widget: &Widget, name: &str) -> Option<PropertyValue> {
        (name == VERTICAL).then(|| widget.is_vertical().into())
    }

    pub fn set(
        widget: &Widget,
        name: &str,
        value: &PropertyValue,
    ) -> Option<Result<(), PropertyError>> {
        (name == VERTICAL).then(|| base::expect_bool(name, value).map(|v| widget.set_vertical(v)))
    }
}
