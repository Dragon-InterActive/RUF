//! Zeigt, welches Hell/Dunkel RUF vom System übernimmt.
//!
//! ```bash
//! cargo run -p rncl --example color_scheme
//! ```

use rncl::prelude::*;

fn main() {
    let _app = Application::new();
    println!("Farbschema: {:?}", Application::color_scheme());
}
