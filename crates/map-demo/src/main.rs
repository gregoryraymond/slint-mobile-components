//! Live map-page demo.
//!
//! Wires a `slint_mapping::FileTileSource` (pointed at the bundled OSM
//! sample tiles) into one of the six map-using pages from
//! `slint-mobile-components`, picked by CLI arg. Pan / scroll-zoom
//! drive real tile rendering — the rest of each page's chrome (search
//! card, bottom sheet, navigation panel, pin overlays, …) sits over
//! the live map exactly as it would in a shipping app.
//!
//! ```sh
//! cargo run -p slint-mobile-components-map-demo
//! cargo run -p slint-mobile-components-map-demo -- ride-share-booking
//! cargo run -p slint-mobile-components-map-demo -- turn-by-turn-nav /path/to/tiles
//! ```
//!
//! Available page names: map (default), turn-by-turn-nav, driver-on-the-way,
//! ride-share-booking, parking-session, store-locator.

use slint::{ComponentHandle, ModelRc, VecModel};
use slint_mapping::sources::FileTileSource;
use slint_mapping::TileSource;
use std::path::PathBuf;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let mut args = std::env::args().skip(1);
    let page_name = args.next().unwrap_or_else(|| "map".to_string());
    let tiles_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(slint_mapping::SAMPLE_TILES_DIR));

    eprintln!("page  = {page_name}");
    eprintln!("tiles = {}", tiles_dir.display());
    let source = FileTileSource::new(tiles_dir);

    match page_name.as_str() {
        "map" => run_map_page(source),
        "turn-by-turn-nav" => run_turn_by_turn(source),
        "driver-on-the-way" => run_driver_on_the_way(source),
        "ride-share-booking" => run_ride_share_booking(source),
        "parking-session" => run_parking_session(source),
        "store-locator" => run_store_locator(source),
        other => {
            eprintln!("unknown page: {other}");
            eprintln!("known: map, turn-by-turn-nav, driver-on-the-way, ride-share-booking, parking-session, store-locator");
            std::process::exit(1);
        }
    }
}

// Each page has identical `map-*` properties and `map-pan` /
// `map-zoom-by` callbacks, but its Rust struct is a different type. A
// macro keeps the wiring DRY without forcing a generic trait the
// pages can't share (slint doesn't generate cross-component traits).
macro_rules! wire_and_run {
    ($PageType:ident, $source:expr) => {{
        let page = $PageType::new()?;
        // Default centre — Greater London at zoom 10. The OSM sample
        // bundle covers London tiles at zoom 4-12 (plus the whole
        // world at 0-3), so the demo opens with real street-level
        // detail and can zoom in two more levels before running out.
        page.set_map_latitude(51.5074);
        page.set_map_longitude(-0.1276);
        page.set_map_zoom(10.0);

        let state = Rc::new(MapState {
            source: Box::new($source),
            tiles_model: Rc::new(VecModel::<Tile>::from(Vec::new())),
        });
        page.set_map_tiles(ModelRc::from(state.tiles_model.clone()));

        // Once the window has been laid out we know its size; refresh.
        {
            let page_weak = page.as_weak();
            let state = Rc::clone(&state);
            slint::Timer::single_shot(std::time::Duration::from_millis(0), move || {
                if let Some(page) = page_weak.upgrade() {
                    refresh(&page, &state);
                }
            });
        }

        // pan: pixel delta → camera shift via projection.
        {
            let page_weak = page.as_weak();
            let state = Rc::clone(&state);
            page.on_map_pan(move |dx, dy| {
                let Some(page) = page_weak.upgrade() else {
                    return;
                };
                let tile_size = state.source.tile_size() as f64;
                let z = page.get_map_zoom() as f64;
                let (tx, ty) = slint_mapping::projection::lonlat_to_tile(
                    page.get_map_longitude() as f64,
                    page.get_map_latitude() as f64,
                    z,
                );
                let (lon, lat) = slint_mapping::projection::tile_to_lonlat(
                    tx - dx as f64 / tile_size,
                    ty - dy as f64 / tile_size,
                    z,
                );
                page.set_map_longitude(lon as f32);
                page.set_map_latitude(lat as f32);
                refresh(&page, &state);
            });
        }

        // zoom-by: scroll delta + anchor → re-centred zoom step.
        {
            let page_weak = page.as_weak();
            let state = Rc::clone(&state);
            page.on_map_zoom_by(move |delta, anchor_x, anchor_y| {
                let Some(page) = page_weak.upgrade() else {
                    return;
                };
                let tile_size = state.source.tile_size() as f64;
                let min_z = state.source.min_zoom() as f64;
                let max_z = state.source.max_zoom() as f64;
                let (vw, vh) = logical_size(&page);
                let z_before = page.get_map_zoom() as f64;
                let lon = page.get_map_longitude() as f64;
                let lat = page.get_map_latitude() as f64;

                let (tx_c, ty_c) = slint_mapping::projection::lonlat_to_tile(lon, lat, z_before);
                let adx = anchor_x as f64 - vw / 2.0;
                let ady = anchor_y as f64 - vh / 2.0;
                let (alon, alat) = slint_mapping::projection::tile_to_lonlat(
                    tx_c + adx / tile_size,
                    ty_c + ady / tile_size,
                    z_before,
                );
                let z_after = (z_before + delta as f64).clamp(min_z, max_z);
                let (tx_an, ty_an) = slint_mapping::projection::lonlat_to_tile(alon, alat, z_after);
                let (nlon, nlat) = slint_mapping::projection::tile_to_lonlat(
                    tx_an - adx / tile_size,
                    ty_an - ady / tile_size,
                    z_after,
                );
                page.set_map_longitude(nlon as f32);
                page.set_map_latitude(nlat as f32);
                page.set_map_zoom(z_after as f32);
                refresh(&page, &state);
            });
        }

        page.run()
    }};
}

fn run_map_page(s: FileTileSource) -> Result<(), slint::PlatformError> {
    wire_and_run!(MapPage, s)
}
fn run_turn_by_turn(s: FileTileSource) -> Result<(), slint::PlatformError> {
    wire_and_run!(TurnByTurnNavPage, s)
}
fn run_driver_on_the_way(s: FileTileSource) -> Result<(), slint::PlatformError> {
    wire_and_run!(DriverOnTheWayPage, s)
}
fn run_ride_share_booking(s: FileTileSource) -> Result<(), slint::PlatformError> {
    wire_and_run!(RideShareBookingPage, s)
}
fn run_parking_session(s: FileTileSource) -> Result<(), slint::PlatformError> {
    wire_and_run!(ParkingSessionPage, s)
}
fn run_store_locator(s: FileTileSource) -> Result<(), slint::PlatformError> {
    wire_and_run!(StoreLocatorPage, s)
}

struct MapState {
    source: Box<dyn TileSource>,
    tiles_model: Rc<VecModel<Tile>>,
}

// Trait so the macro can call `logical_size` / `refresh` against any of
// the page types without per-type duplication. Implemented via blanket
// impl for anything with the right methods.
trait MapPageHandle: ComponentHandle {
    fn get_map_longitude(&self) -> f32;
    fn get_map_latitude(&self) -> f32;
    fn get_map_zoom(&self) -> f32;
}

macro_rules! impl_map_page_handle {
    ($t:ident) => {
        impl MapPageHandle for $t {
            fn get_map_longitude(&self) -> f32 {
                $t::get_map_longitude(self)
            }
            fn get_map_latitude(&self) -> f32 {
                $t::get_map_latitude(self)
            }
            fn get_map_zoom(&self) -> f32 {
                $t::get_map_zoom(self)
            }
        }
    };
}
impl_map_page_handle!(MapPage);
impl_map_page_handle!(TurnByTurnNavPage);
impl_map_page_handle!(DriverOnTheWayPage);
impl_map_page_handle!(RideShareBookingPage);
impl_map_page_handle!(ParkingSessionPage);
impl_map_page_handle!(StoreLocatorPage);

fn refresh<P: MapPageHandle>(page: &P, state: &Rc<MapState>) {
    let (vw, vh) = logical_size(page);
    if vw <= 0.0 || vh <= 0.0 {
        return;
    }
    let placed = slint_mapping::viewport::visible_tiles(
        page.get_map_longitude() as f64,
        page.get_map_latitude() as f64,
        page.get_map_zoom() as f64,
        vw,
        vh,
        state.source.tile_size(),
    );
    let mut rows: Vec<Tile> = Vec::with_capacity(placed.len());
    for p in placed {
        if let Some(image) = state.source.tile(p.key) {
            rows.push(Tile {
                x: p.x,
                y: p.y,
                size: p.size,
                image,
            });
        }
    }
    state.tiles_model.set_vec(rows);
}

fn logical_size<P: ComponentHandle>(page: &P) -> (f64, f64) {
    let w = page.window();
    let phys = w.size();
    let scale = w.scale_factor() as f64;
    let scale = if scale == 0.0 { 1.0 } else { scale };
    (phys.width as f64 / scale, phys.height as f64 / scale)
}
