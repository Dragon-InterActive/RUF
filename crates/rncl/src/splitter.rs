use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ruf::{
    AsWidget, Color, Component, ComponentInfo, MouseAction, MouseEvent, Point, PropertyDescriptor,
    PropertyError, PropertyValue, Shape, Widget, WidgetKind, base,
};

/// Ein Teiler zwischen zwei Bereichen, den der Anwender mit der Maus
/// verschiebt (wie die Trennlinie zwischen den Ansichten in VS Code).
///
/// Der Teiler legt selbst nichts um: [`RnclSplitter::on_dragged`] meldet,
/// um wie viele Pixel er verschoben wurde; die Anwendung passt ihre Bereiche
/// (und die Lage des Teilers) an.
///
/// ```no_run
/// use rncl::prelude::*;
///
/// let window = RnclWindow::new("Teiler");
/// let splitter = RnclSplitter::new(&window, true);
/// splitter.on_dragged(|delta| println!("um {delta} px verschoben"));
/// ```
#[derive(Clone)]
pub struct RnclSplitter {
    widget: Widget,
    horizontal: bool,
    dragged: Dragged,
}

type Dragged = Rc<RefCell<Option<Box<dyn FnMut(i32)>>>>;

impl std::fmt::Debug for RnclSplitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RnclSplitter")
            .field("widget", &self.widget)
            .field("horizontal", &self.horizontal)
            .finish_non_exhaustive()
    }
}

/// Dicke des Teilers in Pixeln.
pub const SPLITTER_SIZE: i32 = 7;

fn grip(width: i32, height: i32, horizontal: bool) -> Vec<Shape> {
    let color = Color::rgb(0x80, 0x80, 0x80);
    let (cx, cy) = (width / 2, height / 2);
    let line = |from: Point, to: Point| Shape::Line { from, to, color };
    if horizontal {
        vec![line(Point::new(cx - 14, cy), Point::new(cx + 14, cy))]
    } else {
        vec![line(Point::new(cx, cy - 14), Point::new(cx, cy + 14))]
    }
}

impl RnclSplitter {
    pub const TYPE_NAME: &'static str = "RnclSplitter";

    /// Ein Teiler; `horizontal` = waagerechte Linie zwischen einem oberen und
    /// einem unteren Bereich (wird nach oben/unten gezogen).
    pub fn new<P: AsWidget + ?Sized>(parent: &P, horizontal: bool) -> Self {
        Self::with_parent(Some(parent.as_widget()), horizontal)
    }

    pub fn with_parent(parent: Option<&Widget>, horizontal: bool) -> Self {
        let widget = Widget::create(WidgetKind::Canvas, parent);
        if horizontal {
            widget.set_size(200, SPLITTER_SIZE);
        } else {
            widget.set_size(SPLITTER_SIZE, 200);
        }
        widget.set_mouse_cursor(if horizontal {
            "row-resize"
        } else {
            "col-resize"
        });
        let w = widget.clone();
        widget.on_resized(move |width, height| w.set_shapes(grip(width, height, horizontal)));
        let dragged: Dragged = Rc::default();
        // Ziehen: Abstand zum Druckpunkt melden. Wird der Teiler dabei
        // verschoben, bleibt der Druckpunkt unter dem Mauszeiger.
        let pressed: Rc<Cell<Option<i32>>> = Rc::default();
        let d = dragged.clone();
        widget.on_mouse(move |e: MouseEvent| {
            let at = if horizontal { e.y } else { e.x };
            match e.action {
                MouseAction::Pressed if e.button == 1 => pressed.set(Some(at)),
                MouseAction::Released => pressed.set(None),
                MouseAction::Moved => {
                    if let Some(start) = pressed.get()
                        && at != start
                        && let Some(f) = d.borrow_mut().as_mut()
                    {
                        f(at - start);
                    }
                }
                _ => {}
            }
        });
        Self {
            widget,
            horizontal,
            dragged,
        }
    }

    pub fn is_horizontal(&self) -> bool {
        self.horizontal
    }

    /// Wird beim Ziehen mit der Verschiebung (Pixel, nach unten bzw. rechts
    /// positiv) seit der letzten Meldung aufgerufen.
    pub fn on_dragged(&self, f: impl FnMut(i32) + 'static) {
        *self.dragged.borrow_mut() = Some(Box::new(f));
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent, true))
        })
        .category("Container")
        .description("Verschiebbarer Teiler zwischen zwei Bereichen")
    }
}

impl Component for RnclSplitter {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        base::descriptors()
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        base::get(&self.widget, name)
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        base::set(&self.widget, name, value)
    }
}
