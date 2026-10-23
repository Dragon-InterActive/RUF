//! Menüs: Menüleiste eines Fensters und Kontextmenüs.
//!
//! Jeder Eintrag hat eine Kennung (`id`); ausgewählte Einträge melden sich
//! über `on_menu` des Fensters mit dieser Kennung – egal ob per Maus oder
//! Tastenkürzel.
//!
//! ```
//! use ruf::Menu;
//!
//! let file = Menu::new()
//!     .item("file.open", "Öffnen …", Some("Ctrl+O"))
//!     .separator()
//!     .item("file.quit", "Beenden", Some("Ctrl+Q"));
//! assert_eq!(file.entries.len(), 3);
//! ```

/// Ein Eintrag eines Menüs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    Item {
        id: String,
        label: String,
        /// Tastenkürzel wie `"Ctrl+S"` (wird angezeigt und ausgelöst).
        shortcut: Option<String>,
        enabled: bool,
    },
    Separator,
    Submenu {
        label: String,
        menu: Menu,
    },
}

/// Ein Menü (auch ein Untermenü oder ein Kontextmenü).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Menu {
    pub entries: Vec<MenuEntry>,
}

impl Menu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fügt einen Eintrag hinzu.
    pub fn item(mut self, id: &str, label: &str, shortcut: Option<&str>) -> Self {
        self.entries.push(MenuEntry::Item {
            id: id.to_owned(),
            label: label.to_owned(),
            shortcut: shortcut.map(str::to_owned),
            enabled: true,
        });
        self
    }

    /// Fügt einen ausgegrauten Eintrag hinzu.
    pub fn disabled_item(mut self, id: &str, label: &str) -> Self {
        self.entries.push(MenuEntry::Item {
            id: id.to_owned(),
            label: label.to_owned(),
            shortcut: None,
            enabled: false,
        });
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(MenuEntry::Separator);
        self
    }

    pub fn submenu(mut self, label: &str, menu: Menu) -> Self {
        self.entries.push(MenuEntry::Submenu {
            label: label.to_owned(),
            menu,
        });
        self
    }

    /// Alle Einträge samt Untermenüs, in Anzeigereihenfolge.
    pub fn items(&self) -> Vec<&MenuEntry> {
        let mut out = Vec::new();
        for entry in &self.entries {
            match entry {
                MenuEntry::Submenu { menu, .. } => out.extend(menu.items()),
                MenuEntry::Item { .. } => out.push(entry),
                MenuEntry::Separator => {}
            }
        }
        out
    }
}

/// Die Menüleiste eines Fensters: Titel und Menü je Eintrag.
pub type MenuBar = Vec<(String, Menu)>;
