"""End-to-end test for the wasm-viewer's mobile pagination + scroll rail.

Runs Chrome with mobile emulation against `http://localhost:8081`, waits
for slint to load and the first 15 pages to compile, then drives a few
interactions:

  1. Snapshots the initial feed (should show ~one cell + the bottom of
     the catalogue out of view).
  2. Drags the scroll rail to pan the feed down toward the "Show 15 more"
     button — the rail occupies the right ~48 px of the viewport.
  3. Taps where the button should be once it's visible, then snapshots
     again — we expect more cells (count went from 15 → 30).
  4. Drags further down to confirm scrollability continues to work.

Each step writes a PNG to `target/e2e-snaps/` so the verification is
visual (slint renders to canvas, so DOM inspection won't tell us the
cell count). Run with: `py tests/wasm_mobile_e2e.py`.
"""
from __future__ import annotations

import pathlib
import sys
import time

from selenium import webdriver
from selenium.webdriver.chrome.options import Options
from selenium.webdriver.common.actions.action_builder import ActionBuilder
from selenium.webdriver.common.actions.pointer_input import PointerInput
from selenium.webdriver.common.actions import interaction
from selenium.webdriver.common.by import By
from selenium.webdriver.common.action_chains import ActionChains
from selenium.webdriver.support.ui import WebDriverWait

URL = "http://localhost:8081"
VIEWPORT = (412, 800)  # logical px, matches CSS device pixels in Chrome emulation
OUT_DIR = pathlib.Path("target/e2e-snaps")


def setup_driver() -> webdriver.Chrome:
    mobile_emulation = {
        "deviceMetrics": {
            "width": VIEWPORT[0],
            "height": VIEWPORT[1],
            "pixelRatio": 2.0,
            "touch": True,
        },
        "userAgent": (
            "Mozilla/5.0 (Linux; Android 12; Pixel 6) "
            "AppleWebKit/537.36 (KHTML, like Gecko) "
            "Chrome/120.0.0.0 Mobile Safari/537.36"
        ),
    }
    options = Options()
    options.add_experimental_option("mobileEmulation", mobile_emulation)
    options.add_argument("--enable-features=TouchEvents")
    # Run headless so we don't need a graphical session; "new" mode is
    # the only one that supports mobile emulation properly.
    options.add_argument("--headless=new")
    options.add_argument("--no-sandbox")
    options.add_argument("--disable-dev-shm-usage")
    options.add_argument(f"--window-size={VIEWPORT[0]},{VIEWPORT[1]}")

    driver = webdriver.Chrome(options=options)
    return driver


def wait_for_canvas(driver: webdriver.Chrome, timeout: float = 60) -> None:
    """Wait until the slint canvas has a non-trivial size — that's the
    signal slint has booted and JS has pushed the canvas dimensions in
    via `set_canvas_size`.
    """

    def canvas_ready(d: webdriver.Chrome) -> bool:
        size = d.execute_script(
            "var c = document.getElementById('canvas');"
            "return c ? {w: c.width, h: c.height} : null;"
        )
        if not size:
            return False
        return size["w"] > 100 and size["h"] > 100

    end = time.time() + timeout
    while time.time() < end:
        if canvas_ready(driver):
            return
        time.sleep(0.5)
    raise TimeoutError("canvas never reached non-trivial size")


def get_loaded(driver: webdriver.Chrome) -> int:
    """Read `loaded_count()` from the wasm-bindgen exports — same path
    JS uses to know how many pages have compiled. Returns 0 if the
    function isn't wired yet (very early in load).
    """
    return driver.execute_script(
        """
        return (typeof window.__loaded === 'function')
            ? window.__loaded()
            : 0;
        """
    )


def get_mobile(driver: webdriver.Chrome) -> bool:
    return driver.execute_script(
        """
        return (typeof window.__is_mobile === 'function')
            ? window.__is_mobile()
            : false;
        """
    )


def install_probe_bridges(driver: webdriver.Chrome) -> None:
    """Hoist the wasm-bindgen exports `loaded_count` and `is_mobile`
    onto `window.__loaded` / `window.__is_mobile` so the rest of the
    test can call them as plain JS. The wasm module path is the same
    one the page's <script> already imported, so this is a no-op for
    the module loader.
    """
    driver.execute_script(
        """
        return import('./slint-mobile-components-wasm-viewer.js').then(m => {
            window.__loaded    = m.loaded_count;
            window.__is_mobile = m.is_mobile;
        });
        """
    )


def wait_for_loaded(
    driver: webdriver.Chrome,
    at_least: int,
    timeout: float = 60,
) -> int:
    end = time.time() + timeout
    last = 0
    while time.time() < end:
        last = get_loaded(driver)
        if last >= at_least:
            return last
        time.sleep(0.5)
    raise TimeoutError(f"only {last} pages loaded; expected >= {at_least}")


def _touch_pointer(driver: webdriver.Chrome) -> ActionBuilder:
    """W3C touch-pointer ActionBuilder. Slint's web backend listens
    for `pointer*` events, not the legacy `touchstart`/`touchmove`
    family, so CDP `Input.dispatchTouchEvent` calls don't reach it.
    The W3C pointer API (used here) emits `pointerdown` / `pointermove`
    / `pointerup` with `pointerType: 'touch'`, which is exactly what
    slint's winit-web backend hands to its hit-test path.
    """
    pointer = PointerInput(interaction.POINTER_TOUCH, "finger1")
    return ActionBuilder(driver, mouse=pointer)


def touch_drag(
    driver: webdriver.Chrome,
    start_x: int,
    start_y: int,
    end_x: int,
    end_y: int,
    duration_ms: int = 300,
) -> None:
    actions = _touch_pointer(driver)
    p = actions.pointer_action
    p.move_to_location(start_x, start_y)
    p.pointer_down()
    # Break the move into ~12 micro-steps so the drag emits enough
    # `pointermove` events for slint to register as a continuous drag
    # rather than a single jump.
    steps = 12
    step_ms = max(1, duration_ms // steps)
    for i in range(1, steps + 1):
        t = i / steps
        x = int(start_x + (end_x - start_x) * t)
        y = int(start_y + (end_y - start_y) * t)
        p.move_to_location(x, y)
        p.pause(step_ms / 1000.0)
    p.pointer_up()
    actions.perform()


def touch_tap(driver: webdriver.Chrome, x: int, y: int) -> None:
    actions = _touch_pointer(driver)
    p = actions.pointer_action
    p.move_to_location(x, y)
    p.pointer_down()
    p.pause(0.05)
    p.pointer_up()
    actions.perform()


def snap(driver: webdriver.Chrome, label: str) -> None:
    path = OUT_DIR / f"{label}.png"
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    driver.save_screenshot(str(path))
    print(f"  snap -> {path}")


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    print(f"opening {URL} at {VIEWPORT[0]}x{VIEWPORT[1]} (mobile emulation)")
    driver = setup_driver()
    failures: list[str] = []
    try:
        driver.get(URL)

        print("waiting for canvas to mount + slint to boot")
        wait_for_canvas(driver, timeout=120)
        install_probe_bridges(driver)
        # Give the wasm-bindgen re-import a moment to settle.
        time.sleep(0.5)

        # Mobile flag should be true on the (412 < 600) emulated phone.
        if not get_mobile(driver):
            failures.append("mobile flag is false on 412x800 emulated phone")
        print(f"  is_mobile = {get_mobile(driver)}")

        # Wait until the initial 15-page pagination cap is reachable.
        loaded = wait_for_loaded(driver, at_least=16, timeout=90)
        print(f"  loaded = {loaded} pages")
        snap(driver, "01_initial")

        # ---- Drag the rail in two pulls to definitely reach
        #      max-scroll (one drag covers ~97% of the track, the
        #      second clamps to the bottom). ----
        rail_x = VIEWPORT[0] - 24
        print(f"dragging rail at x={rail_x} -- two pulls to max-scroll")
        touch_drag(driver, rail_x, 80, rail_x, 780, duration_ms=400)
        time.sleep(0.5)
        touch_drag(driver, rail_x, 80, rail_x, 780, duration_ms=400)
        time.sleep(0.8)
        snap(driver, "02_scrolled_to_bottom")

        # ---- Find the "Show 15 more" button by tapping a vertical
        # ladder of points in the page-area centre column. The button
        # is a 64 dp tall primary-blue pill below the last cell.
        # After paginating, the feed's content height grows so the
        # rail's max-scroll changes — we re-read shown-count
        # indirectly by checking that a follow-up drag now ends up
        # in different content.
        page_centre_x = (VIEWPORT[0] - 48) // 2
        for guess_y in (760, 740, 720, 700, 680):
            print(f"tapping ({page_centre_x}, {guess_y}) -- button hunt")
            touch_tap(driver, page_centre_x, guess_y)
            time.sleep(0.4)
        time.sleep(1.0)
        snap(driver, "03_after_button_tap")

        # If the button was hit even once, `shown-count` jumped from
        # 15 -> 30 and the feed is now twice as long. A fresh drag
        # from the top of the rail now scrolls to a NEW position
        # roughly halfway through, so the visible content should
        # differ from `02_scrolled_to_bottom`.
        #
        # Scroll back to the very top first so the comparison is
        # unambiguous: top-of-feed will be cell 0 (vaporwave page).
        # If pagination took effect, scrolling all the way down again
        # should show a DIFFERENT page from `02`.
        touch_drag(driver, rail_x, 780, rail_x, 80, duration_ms=400)
        time.sleep(0.5)
        snap(driver, "04_back_to_top")
        touch_drag(driver, rail_x, 80, rail_x, 780, duration_ms=400)
        time.sleep(0.3)
        touch_drag(driver, rail_x, 80, rail_x, 780, duration_ms=400)
        time.sleep(0.8)
        snap(driver, "05_scrolled_to_new_bottom")

        if failures:
            print("FAILURES:")
            for f in failures:
                print(f"  - {f}")
            return 1
        print("done -- snapshots in target/e2e-snaps/")
        return 0
    finally:
        driver.quit()


if __name__ == "__main__":
    sys.exit(main())
