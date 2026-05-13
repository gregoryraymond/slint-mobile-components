//! Desktop preview app for slint-mobile-components.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example gallery --features gallery
//! ```
//!
//! The window opens at the standard mobile aspect (412 × 892) but is
//! resizable. A tab strip at the top switches between the three example
//! pages (Home / Settings / Login) and a "Toolbox" view that catalogues
//! every individual component.

use slint::ComponentHandle;

fn main() -> Result<(), slint::PlatformError> {
    slint_mobile_components::Gallery::new()?.run()
}
