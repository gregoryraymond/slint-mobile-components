// Compile the validation stub so any syntax break in theme.slint is
// caught at build time. Generated Rust is included by src/lib.rs.
fn main() {
    std::env::set_var("SLINT_EMIT_DEBUG_INFO", "1");
    slint_build::compile("ui/_validate.slint").expect("Slint build failed");
}
