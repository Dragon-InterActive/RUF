//! Daten der Baumansicht ([`crate::WidgetKind::TreeView`]).

/// Ein Knoten der Baumansicht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeItem {
    /// Eindeutige Kennung; Auswahl und Ereignisse nennen sie.
    pub id: String,
    pub text: String,
    pub children: Vec<TreeItem>,
    /// Kinder sichtbar?
    pub expanded: bool,
}

impl TreeItem {
    /// Ein aufgeklappter Knoten ohne Kinder.
    pub fn new(id: &str, text: &str) -> Self {
        Self {
            id: id.to_owned(),
            text: text.to_owned(),
            children: Vec::new(),
            expanded: true,
        }
    }

    pub fn with_child(mut self, child: TreeItem) -> Self {
        self.children.push(child);
        self
    }

    pub fn collapsed(mut self) -> Self {
        self.expanded = false;
        self
    }
}

/// Eine sichtbare Zeile, wie das Backend sie anzeigt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TreeRow {
    pub id: String,
    pub text: String,
    pub depth: usize,
    pub has_children: bool,
    pub expanded: bool,
}

/// Die sichtbaren Zeilen (aufgeklappte Knoten mit ihren Kindern).
pub(crate) fn visible_rows(items: &[TreeItem]) -> Vec<TreeRow> {
    fn walk(items: &[TreeItem], depth: usize, out: &mut Vec<TreeRow>) {
        for item in items {
            out.push(TreeRow {
                id: item.id.clone(),
                text: item.text.clone(),
                depth,
                has_children: !item.children.is_empty(),
                expanded: item.expanded,
            });
            if item.expanded {
                walk(&item.children, depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(items, 0, &mut out);
    out
}

/// Klappt den Knoten `id` um; `true`, wenn er gefunden wurde.
pub(crate) fn toggle(items: &mut [TreeItem], id: &str) -> bool {
    for item in items {
        if item.id == id {
            item.expanded = !item.expanded;
            return true;
        }
        if toggle(&mut item.children, id) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_follow_expansion() {
        let mut tree = vec![
            TreeItem::new("a", "A").with_child(TreeItem::new("a1", "A1")),
            TreeItem::new("b", "B")
                .with_child(TreeItem::new("b1", "B1"))
                .collapsed(),
        ];
        let ids =
            |t: &[TreeItem]| -> Vec<String> { visible_rows(t).into_iter().map(|r| r.id).collect() };
        assert_eq!(ids(&tree), ["a", "a1", "b"]);
        assert!(toggle(&mut tree, "b"));
        assert_eq!(ids(&tree), ["a", "a1", "b", "b1"]);
        assert_eq!(visible_rows(&tree)[3].depth, 1);
        assert!(!toggle(&mut tree, "x"));
    }
}
