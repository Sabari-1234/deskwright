//! Use-case services: each one fulfils a single user goal by orchestrating ports.

pub mod app_launch_service;
pub mod app_quit_service;
pub mod app_resolver;
pub mod element_locator_service;
pub mod element_read_service;
pub mod element_search_service;
pub mod element_wait_service;
pub mod expectation_service;
pub mod interaction;
pub mod keyboard_service;
pub mod locator_suggestion_service;
pub mod menu_navigator;
pub mod menu_service;
pub mod option_service;
pub mod screenshot_service;
pub mod snapshot_service;
mod tree_walker;
pub mod window_service;
