//! Read-only checks of the Accessibility adapters against the real Finder. They need the
//! Accessibility permission for the process running the tests (your terminal or IDE) and
//! skip with a message when it isn't granted. Nothing here clicks, types, or changes state.
#![cfg(target_os = "macos")]

use deskwright_core::model::app::ProcessId;
use deskwright_core::ports::{
    AccessibilityPermission, AppDirectory, ElementTreeReader, HitTester, MenuBarLocator,
};
use deskwright_macos::{
    AxAccessibilityPermission, AxHitTester, AxMenuBarLocator, AxTreeReader, WorkspaceAppDirectory,
};

const FINDER_BUNDLE_ID: &str = "com.apple.finder";

/// Finder's process ID, or `None` (with the reason printed) when the test must be skipped.
fn finder_when_permitted() -> Option<ProcessId> {
    if !AxAccessibilityPermission::new().is_granted() {
        eprintln!("skipped: Accessibility permission is not granted to the test process");
        return None;
    }
    let finder = WorkspaceAppDirectory::new()
        .running_apps()
        .expect("running apps can be listed")
        .into_iter()
        .find(|app| app.bundle_id.as_deref() == Some(FINDER_BUNDLE_ID));
    if finder.is_none() {
        eprintln!("skipped: Finder is not running");
    }
    finder.map(|app| app.process_id)
}

#[test]
fn reads_finder_top_level_elements_and_their_attributes() {
    let Some(finder) = finder_when_permitted() else {
        return;
    };
    let tree = AxTreeReader::new();

    let top_level = tree.top_level_elements(finder).unwrap();

    assert!(
        !top_level.is_empty(),
        "Finder always has the desktop or a window"
    );
    let attributes = tree.attributes(&top_level[0]).unwrap();
    assert!(!attributes.role.is_empty());
    assert!(tree.children(&top_level[0]).is_ok());
}

#[test]
fn finds_finder_menu_bar_with_titled_items() {
    let Some(finder) = finder_when_permitted() else {
        return;
    };
    let tree = AxTreeReader::new();

    let menu_bar = AxMenuBarLocator::new().menu_bar(finder).unwrap();
    let menu_titles: Vec<_> = tree
        .children(&menu_bar)
        .unwrap()
        .iter()
        .filter_map(|item| tree.attributes(item).ok()?.title)
        .collect();

    assert!(
        menu_titles.iter().any(|title| title == "File"),
        "{menu_titles:?}"
    );
}

#[test]
fn hit_tests_the_center_of_a_finder_element() {
    let Some(finder) = finder_when_permitted() else {
        return;
    };
    let tree = AxTreeReader::new();
    let top_level = tree.top_level_elements(finder).unwrap();
    let Some((element, bounds)) = top_level
        .iter()
        .find_map(|element| Some((element, tree.on_screen_bounds(element).ok()??)))
    else {
        eprintln!("skipped: no Finder element is on screen");
        return;
    };

    // Whether Finder is on top depends on the desktop's current window order; the call must
    // simply give an answer.
    let answer = AxHitTester::new().is_app_of_element_on_top_at(element, bounds.center());
    assert!(answer.is_ok(), "{answer:?} at {:?}", bounds.center());
}
