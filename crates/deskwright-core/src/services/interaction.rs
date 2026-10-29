//! Clicking, typing into, toggling, scrolling to and dragging elements. Each action is a [`strategy_chain::StrategyChain`] of
//! interchangeable strategies tried in order, so the most precise method is used when the app
//! supports it and a synthetic-input fallback takes over when it doesn't.

pub mod click_service;
pub mod click_strategies;
pub mod pointer_gesture_service;
pub mod pointer_target;
pub mod scroll_service;
pub mod strategy_chain;
pub mod text_entry_service;
pub mod text_entry_strategies;
pub mod toggle_service;
