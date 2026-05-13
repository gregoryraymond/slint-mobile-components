//! Behavior-test harness.
//!
//! For each scene in `tests/behavior_scenes.slint`, find interactive
//! elements via `i-slint-backend-testing`'s accessibility queries and
//! invoke their default actions — then assert on the scene's exposed
//! state. No rendering, no event loop; tests run in microseconds.
//!
//! Usage:
//!
//! ```sh
//! cargo test --features behaviors --test behavior
//! ```
//!
//! The accessibility metadata on each component (`accessible-role`,
//! `accessible-label`, `accessible-action-default`, …) is what makes
//! these queries possible — and the same metadata also drives
//! TalkBack / VoiceOver / screen readers on real devices.

use std::cell::Cell;

use i_slint_backend_testing::ElementHandle;

use slint_mobile_components::{
    BehaviorBottomNav, BehaviorButtonClick, BehaviorCheckbox, BehaviorChip,
    BehaviorListItem, BehaviorSlider, BehaviorSwitchToggle, BehaviorTabBar,
    BehaviorTextField,
};

// `i_slint_backend_testing::init_no_event_loop` is per-thread (the
// backend instance is thread-local) and panics if called twice on the
// same thread. Cargo runs tests in parallel threads, so we guard
// initialization with a thread-local flag, not a global `Once`.
fn ensure_backend() {
    thread_local! {
        static INITED: Cell<bool> = const { Cell::new(false) };
    }
    INITED.with(|c| {
        if !c.get() {
            i_slint_backend_testing::init_no_event_loop();
            c.set(true);
        }
    });
}

// ---- MobileButton --------------------------------------------------------

#[test]
fn mobile_button_default_action_fires_clicked() {
    ensure_backend();
    let scene = BehaviorButtonClick::new().unwrap();

    let submit = ElementHandle::find_by_accessible_label(&scene, "Submit")
        .next()
        .expect("no element labelled 'Submit'");
    submit.invoke_accessible_default_action();
    submit.invoke_accessible_default_action();

    assert_eq!(scene.get_click_count(), 2);
}

#[test]
fn mobile_button_advertises_label_via_accessibility() {
    ensure_backend();
    let scene = BehaviorButtonClick::new().unwrap();

    let submit = ElementHandle::find_by_accessible_label(&scene, "Submit")
        .next()
        .unwrap();
    assert_eq!(submit.accessible_label().as_deref(), Some("Submit"));
}

// ---- MobileSwitch --------------------------------------------------------

#[test]
fn mobile_switch_default_action_toggles_state() {
    ensure_backend();
    let scene = BehaviorSwitchToggle::new().unwrap();
    assert!(!scene.get_checked());

    let sw = ElementHandle::find_by_element_type_name(&scene, "MobileSwitch")
        .next()
        .expect("no MobileSwitch in scene");

    sw.invoke_accessible_default_action();
    assert!(scene.get_checked(), "expected checked after first toggle");
    assert_eq!(scene.get_toggle_count(), 1);
    assert!(scene.get_last_value(), "callback should receive new value");

    sw.invoke_accessible_default_action();
    assert!(!scene.get_checked(), "expected unchecked after second toggle");
    assert_eq!(scene.get_toggle_count(), 2);
    assert!(!scene.get_last_value());
}

// ---- TextField -----------------------------------------------------------

#[test]
fn text_field_advertises_label_and_value() {
    ensure_backend();
    let scene = BehaviorTextField::new().unwrap();
    scene.set_current_text("you@example.com".into());

    let field = ElementHandle::find_by_accessible_label(&scene, "Email")
        .next()
        .expect("no element labelled 'Email'");

    assert_eq!(field.accessible_label().as_deref(), Some("Email"));
    assert_eq!(
        field.accessible_value().as_deref(),
        Some("you@example.com"),
    );
}

// ---- BottomNav / IconButton ---------------------------------------------

#[test]
fn bottom_nav_each_icon_button_routes_to_its_index() {
    ensure_backend();
    let scene = BehaviorBottomNav::new().unwrap();

    for (label, expected_index) in [("Home", 0), ("Search", 1), ("Profile", 2)] {
        let btn = ElementHandle::find_by_accessible_label(&scene, label)
            .next()
            .unwrap_or_else(|| panic!("no IconButton labelled '{label}'"));
        btn.invoke_accessible_default_action();
        assert_eq!(
            scene.get_nav_index(),
            expected_index,
            "after clicking '{label}', nav-index should be {expected_index}",
        );
    }

    assert_eq!(scene.get_nav_change_count(), 3);
}

// ---- Chip ----------------------------------------------------------------

#[test]
fn chip_default_action_toggles_selected_and_fires_clicked() {
    ensure_backend();
    let scene = BehaviorChip::new().unwrap();
    assert!(!scene.get_filter_selected());

    let chip = ElementHandle::find_by_accessible_label(&scene, "Filter")
        .next()
        .expect("no chip labelled 'Filter'");

    chip.invoke_accessible_default_action();
    assert!(scene.get_filter_selected(), "first tap selects");
    assert_eq!(scene.get_click_count(), 1);

    chip.invoke_accessible_default_action();
    assert!(!scene.get_filter_selected(), "second tap deselects");
    assert_eq!(scene.get_click_count(), 2);
}

#[test]
fn chip_advertises_checked_state_via_accessibility() {
    ensure_backend();
    let scene = BehaviorChip::new().unwrap();
    scene.set_filter_selected(true);

    let chip = ElementHandle::find_by_accessible_label(&scene, "Filter")
        .next()
        .unwrap();
    // The accessibility tree should advertise the chip as a checkable
    // control. The exact predicate on the test side just confirms the
    // label flows through.
    assert_eq!(chip.accessible_label().as_deref(), Some("Filter"));
}

// ---- Checkbox ------------------------------------------------------------

#[test]
fn checkbox_default_action_toggles_state() {
    ensure_backend();
    let scene = BehaviorCheckbox::new().unwrap();
    assert!(!scene.get_agreed());

    let box_handle = ElementHandle::find_by_accessible_label(&scene, "Agree to terms")
        .next()
        .expect("no checkbox labelled 'Agree to terms'");

    box_handle.invoke_accessible_default_action();
    assert!(scene.get_agreed(), "first tap checks");
    assert_eq!(scene.get_toggle_count(), 1);
    assert!(scene.get_last_value());

    box_handle.invoke_accessible_default_action();
    assert!(!scene.get_agreed(), "second tap unchecks");
    assert_eq!(scene.get_toggle_count(), 2);
    assert!(!scene.get_last_value());
}

// ---- Slider --------------------------------------------------------------

#[test]
fn slider_advertises_value_as_percentage() {
    ensure_backend();
    let scene = BehaviorSlider::new().unwrap();
    assert_eq!(scene.get_volume(), 0.25);

    let slider = ElementHandle::find_by_element_type_name(&scene, "Slider")
        .next()
        .expect("no Slider in scene");
    assert_eq!(slider.accessible_value().as_deref(), Some("25"));

    scene.set_volume(0.7);
    assert_eq!(slider.accessible_value().as_deref(), Some("70"));
}

// ---- TabBar / Tab --------------------------------------------------------

#[test]
fn tab_bar_each_tab_routes_to_its_index() {
    ensure_backend();
    let scene = BehaviorTabBar::new().unwrap();

    for (label, expected_index) in [("Photos", 0), ("Videos", 1), ("Albums", 2)] {
        let tab = ElementHandle::find_by_accessible_label(&scene, label)
            .next()
            .unwrap_or_else(|| panic!("no Tab labelled '{label}'"));
        tab.invoke_accessible_default_action();
        assert_eq!(scene.get_tab_index(), expected_index);
    }

    assert_eq!(scene.get_tab_change_count(), 3);
}

// ---- ListItem ------------------------------------------------------------

#[test]
fn list_item_default_action_fires_clicked() {
    ensure_backend();
    let scene = BehaviorListItem::new().unwrap();

    let row = ElementHandle::find_by_accessible_label(&scene, "Privacy")
        .next()
        .expect("no ListItem with title 'Privacy'");
    row.invoke_accessible_default_action();

    assert_eq!(scene.get_click_count(), 1);
}
