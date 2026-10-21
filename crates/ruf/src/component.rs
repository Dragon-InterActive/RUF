//! Der [`Component`]-Vertrag: was jede RNCL-Komponente können muss.
//!
//! Eine Komponente ist eine pure Rust-Struktur, die ein natives [`Widget`]
//! besitzt. Sie muss nur zwei Methoden implementieren – [`Component::widget`]
//! und [`Component::type_name`]. Alles andere (Position, Größe, Sichtbarkeit,
//! Basis-Eigenschaften für den Designer) bringt der Trait bereits mit.
//!
//! Eigene Eigenschaften ergänzt eine Komponente, indem sie
//! [`Component::properties`], [`Component::property`] und
//! [`Component::set_property`] überschreibt und für unbekannte Namen an
//! [`base`] delegiert.

use crate::geometry::{Point, Rect, Size};
use crate::property::{PropertyDescriptor, PropertyError, PropertyValue};
use crate::widget::Widget;

/// Der gemeinsame Vertrag aller RNCL-Komponenten.
pub trait Component {
    /// Das native Widget dieser Komponente.
    fn widget(&self) -> &Widget;

    /// Der eindeutige Typname, unter dem die Komponente in der
    /// [`ComponentRegistry`](crate::ComponentRegistry) steht, z. B. `"RnclButton"`.
    fn type_name(&self) -> &'static str;

    /// Alle Eigenschaften, die der Designer anzeigen und setzen darf.
    fn properties(&self) -> Vec<PropertyDescriptor> {
        base::descriptors()
    }

    /// Liest eine Eigenschaft per Name.
    fn property(&self, name: &str) -> Option<PropertyValue> {
        base::get(self.widget(), name)
    }

    /// Setzt eine Eigenschaft per Name.
    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        base::set(self.widget(), name, value)
    }

    // --- Bequemlichkeitsmethoden (nicht überschreiben) ----------------------

    fn set_position(&self, x: i32, y: i32) {
        self.widget().set_position(x, y);
    }

    fn position(&self) -> Point {
        self.widget().position()
    }

    fn set_size(&self, width: i32, height: i32) {
        self.widget().set_size(width, height);
    }

    fn size(&self) -> Size {
        self.widget().size()
    }

    fn set_geometry(&self, x: i32, y: i32, width: i32, height: i32) {
        self.widget().set_geometry(Rect::new(x, y, width, height));
    }

    fn geometry(&self) -> Rect {
        self.widget().geometry()
    }

    fn show(&self) {
        self.widget().set_visible(true);
    }

    fn hide(&self) {
        self.widget().set_visible(false);
    }

    fn is_visible(&self) -> bool {
        self.widget().is_visible()
    }

    fn set_enabled(&self, enabled: bool) {
        self.widget().set_enabled(enabled);
    }

    fn is_enabled(&self) -> bool {
        self.widget().is_enabled()
    }

    fn set_tool_tip(&self, text: &str) {
        self.widget().set_tool_tip(text);
    }

    fn set_style_sheet(&self, css: &str) {
        self.widget().set_style_sheet(css);
    }

    /// Eindeutiger Bezeichner (Qt `objectName`), z. B. `"btnOk"`.
    fn set_name(&self, name: &str) {
        self.widget().set_object_name(name);
    }

    fn name(&self) -> String {
        self.widget().object_name()
    }
}

/// Die Basis-Eigenschaften, die jede Komponente automatisch besitzt.
pub mod base {
    use super::*;
    use crate::property::PropertyType::{Bool, Int, Text};

    pub const NAME: &str = "name";
    pub const X: &str = "x";
    pub const Y: &str = "y";
    pub const WIDTH: &str = "width";
    pub const HEIGHT: &str = "height";
    pub const VISIBLE: &str = "visible";
    pub const ENABLED: &str = "enabled";
    pub const TOOL_TIP: &str = "tool_tip";
    pub const STYLE_SHEET: &str = "style_sheet";

    pub fn descriptors() -> Vec<PropertyDescriptor> {
        vec![
            PropertyDescriptor::new(NAME, Text, "Eindeutiger Bezeichner (objectName)"),
            PropertyDescriptor::new(X, Int, "Horizontale Position in Pixeln"),
            PropertyDescriptor::new(Y, Int, "Vertikale Position in Pixeln"),
            PropertyDescriptor::new(WIDTH, Int, "Breite in Pixeln"),
            PropertyDescriptor::new(HEIGHT, Int, "Höhe in Pixeln"),
            PropertyDescriptor::new(VISIBLE, Bool, "Nicht ausgeblendet"),
            PropertyDescriptor::new(ENABLED, Bool, "Bedienbarkeit"),
            PropertyDescriptor::new(TOOL_TIP, Text, "Hinweistext bei Mauszeiger"),
            PropertyDescriptor::new(STYLE_SHEET, Text, "Qt-Stylesheet"),
        ]
    }

    pub fn get(widget: &Widget, name: &str) -> Option<PropertyValue> {
        let g = widget.geometry();
        Some(match name {
            NAME => widget.object_name().into(),
            X => g.x.into(),
            Y => g.y.into(),
            WIDTH => g.width.into(),
            HEIGHT => g.height.into(),
            // Designer-Sicht: "wird angezeigt, sobald das Fenster sichtbar ist".
            VISIBLE => (!widget.is_hidden()).into(),
            ENABLED => widget.is_enabled().into(),
            TOOL_TIP => widget.tool_tip().into(),
            STYLE_SHEET => widget.style_sheet().into(),
            _ => return None,
        })
    }

    pub fn set(widget: &Widget, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        let mut g = widget.geometry();
        match (name, &value) {
            (NAME, PropertyValue::Text(v)) => widget.set_object_name(v),
            (X, PropertyValue::Int(v)) => {
                g.x = *v;
                widget.set_geometry(g);
            }
            (Y, PropertyValue::Int(v)) => {
                g.y = *v;
                widget.set_geometry(g);
            }
            (WIDTH, PropertyValue::Int(v)) => {
                g.width = *v;
                widget.set_geometry(g);
            }
            (HEIGHT, PropertyValue::Int(v)) => {
                g.height = *v;
                widget.set_geometry(g);
            }
            (VISIBLE, PropertyValue::Bool(v)) => widget.set_visible(*v),
            (ENABLED, PropertyValue::Bool(v)) => widget.set_enabled(*v),
            (TOOL_TIP, PropertyValue::Text(v)) => widget.set_tool_tip(v),
            (STYLE_SHEET, PropertyValue::Text(v)) => widget.set_style_sheet(v),
            _ => return Err(mismatch_or_unknown(&descriptors(), name, &value)),
        }
        Ok(())
    }

    /// Hilfsfunktion für eigene Komponenten: erwartet einen Text-Wert.
    pub fn expect_text<'a>(name: &str, value: &'a PropertyValue) -> Result<&'a str, PropertyError> {
        value
            .as_text()
            .ok_or_else(|| PropertyError::type_mismatch(name, Text, value))
    }

    /// Hilfsfunktion für eigene Komponenten: erwartet einen Ganzzahl-Wert.
    pub fn expect_int(name: &str, value: &PropertyValue) -> Result<i32, PropertyError> {
        value
            .as_int()
            .ok_or_else(|| PropertyError::type_mismatch(name, Int, value))
    }

    /// Hilfsfunktion für eigene Komponenten: erwartet einen Wahrheitswert.
    pub fn expect_bool(name: &str, value: &PropertyValue) -> Result<bool, PropertyError> {
        value
            .as_bool()
            .ok_or_else(|| PropertyError::type_mismatch(name, Bool, value))
    }

    /// Hilfsfunktion für eigene Komponenten: liefert `TypeMismatch`, wenn der
    /// Name in `descriptors` vorkommt, sonst `Unknown`.
    pub fn mismatch_or_unknown(
        descriptors: &[PropertyDescriptor],
        name: &str,
        value: &PropertyValue,
    ) -> PropertyError {
        match descriptors.iter().find(|d| d.name == name) {
            Some(d) if d.read_only => PropertyError::ReadOnly {
                name: name.to_owned(),
            },
            Some(d) => PropertyError::type_mismatch(name, d.ty, value),
            None => PropertyError::unknown(name),
        }
    }
}
