//! Compile the wasm-viewer's chrome AND pre-bake every other `.slint`
//! source in the workspace into a self-contained tree the runtime
//! interpreter can serve through `Compiler::set_file_loader` without
//! ever touching a filesystem.
//!
//! Two passes:
//!
//! 1. **Pre-bake.** Walk theme/, components/, every pages-*/ ui dir.
//!    For each `.slint` source:
//!      - Strip `import "..ttf";` lines — fonts are statically embedded
//!        by the chrome's slint-build pass (see the `force_font_load`
//!        slint at the top of `ui/wasm-viewer.slint`); the interpreter
//!        re-importing them at runtime would fail because the bytes
//!        aren't on disk in the browser.
//!      - Rewrite every `@image-url("...")` literal to an inlined
//!        `data:image/<mime>;base64,<…>` URL. slint's compiler natively
//!        handles `data:` URIs (`embed_images.rs:98`), so the
//!        interpreter accepts these without any patching.
//!      - Resolve `@mobile-theme/...` / `@mobile-components/...`
//!        aliases against the on-disk source layout; resolve relative
//!        paths against the source file's own directory.
//!      - Write the rewritten file under `$OUT_DIR/embedded/<role>/...`
//!        where `<role>` is `mobile-theme` / `mobile-components` /
//!        `mobile-pages-<cat>` — matching the runtime virtual library
//!        prefix.
//!
//! 2. **Chrome compile.** Standard slint-build pass on
//!    `ui/wasm-viewer.slint`, with `mobile-theme` + `mobile-components`
//!    library paths pointed at the real on-disk source so the chrome's
//!    imports resolve normally at build time (and the bundled fonts
//!    get statically embedded).
//!
//! Map-using pages are skipped entirely from the pre-bake (they import
//! from `@mapping/...` which would need a separate wasm-friendly tile
//! pipeline). A short list of their stems is filtered out at runtime
//! page-discovery time instead of being silently broken.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine as _;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// `mobile-*` library aliases → on-disk source directories. Mirrors
/// what `slint_mobile_components::library_paths()` returns at runtime
/// for consumers — but here at build time we only need the two that
/// image URLs ever reference.
fn build_time_library_paths(root: &Path) -> HashMap<&'static str, PathBuf> {
    let mut m = HashMap::new();
    m.insert("mobile-theme", root.join("crates/theme/ui"));
    m.insert("mobile-components", root.join("crates/components/ui"));
    // @mapping resolves to slint-mapping's ui/ directory. UI_LIBRARY_DIR
    // is a `pub const &str` exported by the crate — at build time it
    // points into the cargo registry where the published 0.1.0 was
    // unpacked. Map pages import `MapEmbed` from
    // `@mapping/map.slint`; pre-baking that here lets the rewriter
    // resolve any @image-url references inside it (currently none,
    // but future-proof).
    m.insert("mapping", PathBuf::from(slint_mapping::UI_LIBRARY_DIR));
    m
}

/// Source directories the pre-bake should walk. The output directory
/// segment determines the virtual library prefix the runtime loader
/// will see (`/embedded/<segment>/...`).
fn input_roots(root: &Path) -> Vec<(String, PathBuf)> {
    let mut roots = vec![
        ("mobile-theme".to_string(), root.join("crates/theme/ui")),
        (
            "mobile-components".to_string(),
            root.join("crates/components/ui"),
        ),
        // slint-mapping's ui/ — same UI_LIBRARY_DIR const, now used
        // as an input root so map.slint (and its associated structs)
        // are embedded into the runtime virtual fs as
        // /embedded/mapping/map.slint. Without this, an interpreted
        // map page's `import { MapEmbed } from "@mapping/map.slint"`
        // would hit `set_file_loader` looking for that path and
        // get None.
        (
            "mapping".to_string(),
            PathBuf::from(slint_mapping::UI_LIBRARY_DIR),
        ),
    ];
    for entry in fs::read_dir(root.join("crates")).expect("read crates/") {
        let entry = entry.expect("read crates entry");
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else {
            continue;
        };
        if let Some(cat) = name_str.strip_prefix("pages-") {
            let ui = entry.path().join("ui");
            if ui.is_dir() {
                roots.push((format!("mobile-pages-{cat}"), ui));
            }
        }
    }
    roots
}

fn mime_for(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// Resolve an `@image-url` path argument against the source file's
/// location and the build-time library_paths. Returns `None` if the
/// path can't be resolved to an on-disk file (e.g. dynamic value, or
/// a typo) — the rewriter leaves those untouched so the failure
/// surfaces at interpreter parse time rather than being silently
/// dropped.
fn resolve_image_path(
    raw: &str,
    source_file: &Path,
    libs: &HashMap<&'static str, PathBuf>,
) -> Option<PathBuf> {
    if raw.starts_with("data:") {
        return None;
    }
    if let Some(rest) = raw.strip_prefix('@') {
        let (alias, sub) = rest.split_once('/')?;
        let base = libs.get(alias)?;
        let resolved = base.join(sub);
        return resolved.is_file().then_some(resolved);
    }
    // Relative path — resolve against the source file's directory.
    let parent = source_file.parent()?;
    let resolved = parent.join(raw);
    resolved.is_file().then_some(resolved)
}

/// Rewrite a single `.slint` source: drop ttf imports, inline image
/// URLs as data URIs. Returns the new source text.
fn rewrite_source(text: &str, source_file: &Path, libs: &HashMap<&'static str, PathBuf>) -> String {
    let mut out = String::with_capacity(text.len() * 2);

    for raw_line in text.lines() {
        let line = raw_line.trim_start();
        // Drop TTF imports — the chrome's slint-build pass already
        // statically embedded them; the interpreter would fail trying
        // to re-load them from disk in the browser.
        if line.starts_with("import \"") && line.contains(".ttf") {
            continue;
        }
        out.push_str(&rewrite_image_urls(raw_line, source_file, libs));
        out.push('\n');
    }

    out
}

/// Walk one line of slint source and rewrite each `@image-url("…")`
/// occurrence. Returns the line with substitutions applied.
fn rewrite_image_urls(
    line: &str,
    source_file: &Path,
    libs: &HashMap<&'static str, PathBuf>,
) -> String {
    let needle = "@image-url(\"";
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0;

    while let Some(start) = line[cursor..].find(needle) {
        let url_start = cursor + start + needle.len();
        let Some(rel_end) = line[url_start..].find('"') else {
            // Unterminated — let slint's own parser complain.
            out.push_str(&line[cursor..]);
            return out;
        };
        let url_end = url_start + rel_end;
        let raw_url = &line[url_start..url_end];

        // Copy verbatim up to and including the opening "@image-url(\"".
        out.push_str(&line[cursor..url_start]);

        if let Some(image_path) = resolve_image_path(raw_url, source_file, libs) {
            let bytes = fs::read(&image_path).unwrap_or_else(|e| {
                panic!("read image {}: {e}", image_path.display());
            });
            let mime = mime_for(
                image_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
            );
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            out.push_str("data:");
            out.push_str(mime);
            out.push_str(";base64,");
            out.push_str(&b64);
        } else {
            // Couldn't resolve — leave original URL so the interpreter's
            // error message is informative rather than silent.
            out.push_str(raw_url);
        }

        // Cursor advances past the closing quote.
        cursor = url_end;
    }

    out.push_str(&line[cursor..]);
    out
}

/// Walk `src_root`, rewrite every `.slint`, and write the result under
/// `out_root/<role>/<relative path>`. Non-.slint files are skipped
/// entirely (font TTFs, SVG icons — those are inlined into the
/// .slint sources or registered statically by the chrome).
fn prebake_role(
    role: &str,
    src_root: &Path,
    out_root: &Path,
    libs: &HashMap<&'static str, PathBuf>,
) {
    let target_role_dir = out_root.join(role);
    fs::create_dir_all(&target_role_dir).expect("create role dir");

    for entry in walkdir::WalkDir::new(src_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("slint") {
            continue;
        }
        let rel = path.strip_prefix(src_root).expect("strip src root");
        let target = target_role_dir.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("create dest parent");
        }
        let src = fs::read_to_string(path).expect("read .slint");
        let rewritten = rewrite_source(&src, path, libs);
        fs::write(&target, rewritten).expect("write rewritten .slint");

        // Tell cargo to re-run the build if any source changes.
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

/// Walk slint-mapping's bundled `sample-tiles/` (from the published
/// crate in the cargo registry), re-encode every PNG as JPEG-Q70, and
/// write the result to `$OUT_DIR/jpeg-tiles/{z}/{x}/{y}.jpg`. lib.rs
/// `include_dir!`s that directory so the runtime EmbeddedTileSource
/// has zero-dependency access to the bytes.
fn transcode_sample_tiles(out_dir: &Path) {
    let src_root = PathBuf::from(slint_mapping::SAMPLE_TILES_DIR);
    let dest_root = out_dir.join("jpeg-tiles");
    if dest_root.exists() {
        fs::remove_dir_all(&dest_root).expect("clean jpeg-tiles/");
    }
    fs::create_dir_all(&dest_root).expect("create jpeg-tiles/");

    let mut count = 0u32;
    for entry in walkdir::WalkDir::new(&src_root)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("png") {
            continue;
        }
        let rel = path.strip_prefix(&src_root).expect("strip src root");
        let dest = dest_root.join(rel).with_extension("jpg");
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).expect("create dest parent");
        }
        // Decode PNG, drop alpha (JPEG can't carry it; OSM tiles are
        // opaque anyway), encode as JPEG-Q70 with default 4:2:0
        // subsampling. The image crate's JPEG encoder is pure-Rust
        // so no system libjpeg is needed at build time.
        let img = image::open(path).unwrap_or_else(|e| panic!("decode {}: {e}", path.display()));
        let rgb = img.to_rgb8();
        let mut out = fs::File::create(&dest).expect("create dest");
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 70);
        encoder
            .encode(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)
            .expect("encode jpeg");
        count += 1;
    }
    println!("cargo:warning=wasm-viewer: transcoded {count} tiles to JPEG-Q70");
    // Re-run if the source bundle changes (e.g. slint-mapping bump).
    println!("cargo:rerun-if-changed={}", src_root.display());
}

fn main() {
    // `ComponentContainer` + `component-factory` are gated behind
    // Slint's experimental flag. Match the desktop viewer's build.rs
    // so the same chrome compiles cleanly. The interpreter at
    // runtime reads the same env var.
    std::env::set_var("SLINT_ENABLE_EXPERIMENTAL_FEATURES", "1");

    let root = workspace_root();
    let libs = build_time_library_paths(&root);

    // ---- Pass 1: pre-bake every .slint source ----
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let embedded_root = out_dir.join("embedded");
    if embedded_root.exists() {
        fs::remove_dir_all(&embedded_root).expect("clean embedded/");
    }
    for (role, src) in input_roots(&root) {
        prebake_role(&role, &src, &embedded_root, &libs);
    }

    // Surface the OUT_DIR/embedded path to lib.rs for `include_dir!`.
    // It's already at a deterministic location (`$OUT_DIR/embedded`)
    // but emitting it explicitly lets the macro use `env!`.

    // ---- Pass 1b: transcode the slint-mapping sample tiles to JPEG-Q70 ----
    // slint-mapping 0.1.0 ships its `sample-tiles/` directory (worldwide
    // z0–3 + Greater London z4–12) inside the published crate. The PNGs
    // are ~5.6 MB total; re-encoded as JPEG-Q70 they shrink to ~2.4 MB
    // with no visible quality loss for OSM photo-like tiles. The
    // converted tree lives at $OUT_DIR/jpeg-tiles/{z}/{x}/{y}.jpg —
    // lib.rs `include_dir!`s that path, so the conversion is fully
    // deterministic and the wasm-viewer's own repo doesn't need to
    // commit binary tile blobs.
    transcode_sample_tiles(&out_dir);

    // ---- Pass 2: compile the chrome ----
    let mut chrome_paths = HashMap::new();
    chrome_paths.insert(
        "mobile-theme".to_string(),
        PathBuf::from(slint_mobile_theme::UI_LIBRARY_DIR),
    );
    chrome_paths.insert(
        "mobile-components".to_string(),
        PathBuf::from(slint_mobile_components_widgets::UI_LIBRARY_DIR),
    );

    // EmbedFiles forces slint-build to bake the 5 bundled font TTFs
    // (referenced via `import "..ttf"` at the top of
    // ui/wasm-viewer.slint) into the generated Rust as static byte
    // arrays. Without this the default behaviour on wasm is to leave
    // them as `image-url`-style filesystem references, which would
    // fail to resolve in the browser — pages would render with a
    // fallback font instead of Inter / Plex / Grotesk / Fraunces /
    // JetBrains Mono.
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(chrome_paths)
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles);
    slint_build::compile_with_config("ui/wasm-viewer.slint", config).expect("slint-build compile");
}
