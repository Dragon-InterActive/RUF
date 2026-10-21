//! Die [`ComponentRegistry`]: hier melden RNCL-Bibliotheken (auch von
//! Drittentwicklern) ihre Komponenten an. Der Form-Designer erzeugt
//! Komponenten ausschließlich über diese Registry – per Typname.

use std::collections::BTreeMap;
use std::fmt;

use crate::component::Component;
use crate::widget::Widget;

/// Erzeugt eine Komponente, optional als Kind von `parent`.
pub type ComponentFactory = fn(parent: Option<&Widget>) -> Box<dyn Component>;

/// Metadaten und Fabrik einer registrierten Komponente.
#[derive(Clone)]
pub struct ComponentInfo {
    pub type_name: &'static str,
    /// Gruppierung in der Designer-Palette, z. B. `"Standard"` oder `"Eingabe"`.
    pub category: &'static str,
    pub description: &'static str,
    /// Kann die Komponente andere Komponenten aufnehmen?
    pub container: bool,
    pub factory: ComponentFactory,
}

impl ComponentInfo {
    pub const fn new(type_name: &'static str, factory: ComponentFactory) -> Self {
        Self {
            type_name,
            category: "Standard",
            description: "",
            container: false,
            factory,
        }
    }

    pub const fn category(mut self, category: &'static str) -> Self {
        self.category = category;
        self
    }

    pub const fn description(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }

    pub const fn container(mut self, container: bool) -> Self {
        self.container = container;
        self
    }
}

impl fmt::Debug for ComponentInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentInfo")
            .field("type_name", &self.type_name)
            .field("category", &self.category)
            .field("description", &self.description)
            .field("container", &self.container)
            .finish_non_exhaustive()
    }
}

/// Fehler der Registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Ein Typname wurde doppelt registriert.
    Duplicate(&'static str),
    /// Kein Eintrag für diesen Typnamen.
    NotFound(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(name) => write!(f, "Komponente `{name}` ist bereits registriert"),
            Self::NotFound(name) => write!(f, "Komponente `{name}` ist nicht registriert"),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Verzeichnis aller verfügbaren Komponenten, sortiert nach Typname.
#[derive(Debug, Default, Clone)]
pub struct ComponentRegistry {
    entries: BTreeMap<&'static str, ComponentInfo>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Meldet eine Komponente an. Doppelte Typnamen werden abgewiesen.
    pub fn register(&mut self, info: ComponentInfo) -> Result<(), RegistryError> {
        if self.entries.contains_key(info.type_name) {
            return Err(RegistryError::Duplicate(info.type_name));
        }
        self.entries.insert(info.type_name, info);
        Ok(())
    }

    pub fn get(&self, type_name: &str) -> Option<&ComponentInfo> {
        self.entries.get(type_name)
    }

    pub fn contains(&self, type_name: &str) -> bool {
        self.entries.contains_key(type_name)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Alle Einträge, sortiert nach Typname.
    pub fn iter(&self) -> impl Iterator<Item = &ComponentInfo> {
        self.entries.values()
    }

    /// Alle Kategorien (sortiert, ohne Duplikate) – für die Designer-Palette.
    pub fn categories(&self) -> Vec<&'static str> {
        let mut cats: Vec<_> = self.entries.values().map(|i| i.category).collect();
        cats.sort_unstable();
        cats.dedup();
        cats
    }

    /// Erzeugt eine Komponente per Typname.
    pub fn create(
        &self,
        type_name: &str,
        parent: Option<&Widget>,
    ) -> Result<Box<dyn Component>, RegistryError> {
        let info = self
            .get(type_name)
            .ok_or_else(|| RegistryError::NotFound(type_name.to_owned()))?;
        Ok((info.factory)(parent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn never(_: Option<&Widget>) -> Box<dyn Component> {
        unreachable!("in diesen Tests wird nichts erzeugt")
    }

    #[test]
    fn register_and_lookup() {
        let mut r = ComponentRegistry::new();
        r.register(ComponentInfo::new("B", never).category("Eingabe"))
            .unwrap();
        r.register(ComponentInfo::new("A", never)).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(
            r.iter().map(|i| i.type_name).collect::<Vec<_>>(),
            ["A", "B"]
        );
        assert_eq!(r.categories(), ["Eingabe", "Standard"]);
        assert_eq!(
            r.register(ComponentInfo::new("A", never)),
            Err(RegistryError::Duplicate("A"))
        );
        assert!(matches!(
            r.create("C", None),
            Err(RegistryError::NotFound(_))
        ));
    }
}
