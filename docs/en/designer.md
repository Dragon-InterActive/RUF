# Designer integration (Freya TD)

The form designer talks to **only** three abstractions – never to concrete Rust types.
That way, third-party components automatically show up in the palette.

| Abstraction | Purpose |
|---|---|
| `ComponentRegistry` | palette: which components exist, in which category? Create by type name. |
| `Component` (as `Box<dyn Component>`) | the placed instance |
| `PropertyDescriptor` / `PropertyValue` | property inspector: list, read, write |

## Building the palette

```rust
let registry = rncl::registry();
for category in registry.categories() {
    println!("[{category}]");
    for info in registry.iter().filter(|i| i.category == category) {
        println!("  {} – {}", info.type_name, info.description);
    }
}
```

## Placing a component

```rust
let form = registry.create("RnclWindow", None)?;
let mut button = registry.create("RnclButton", Some(form.widget()))?;
button.set_name("btnOk");
button.set_property("text", "OK".into())?;
button.set_property("x", 20.into())?;
```

## Property inspector

```rust
for d in button.properties() {
    let value = button.property(d.name);
    println!("{:<12} {:<5} {:?}  // {}", d.name, d.ty, value, d.description);
}
```

`set_property` returns a `PropertyError` on failure:

| Error | Meaning |
|---|---|
| `Unknown { name }` | the component has no such property |
| `TypeMismatch { name, expected, found }` | wrong value type |
| `ReadOnly { name }` | the property is read-only |

Value types (`PropertyType`): `Bool`, `Int`, `Text` and `TextList` (e.g. `items` of a
combo box or `tabs` of a tab view):

```rust
combo.set_property("items", ["Berlin", "Hamburg"].into())?;
```

## Loading a form from a description

`crates/rncl/examples/designer.rs` builds a form from a text description – a preview of
the future Freya TD file format:

```text
RnclWindow    frmMain   title="Designer-Demo" width=360 height=200
RnclLabel     lblName   text="Name:" x=20 y=24
RnclTextField txtName   x=100 y=20 width=230 placeholder="Vor- und Nachname"
RnclButton    btnOk     text="OK" x=100 y=110 width=100
```

```bash
cargo run -p rncl --example designer -- --dump        # properties of all components
cargo run -p rncl --example designer -- --screenshot form.png
```

## Preview images

`Widget::save_screenshot(path)` renders any component including its children into an
image file – offscreen too. The designer can use it to generate preview images for the
palette or documentation.
