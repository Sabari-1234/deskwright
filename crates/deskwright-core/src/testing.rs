//! In-memory test doubles for every port. Enabled for this crate's tests and, through the
//! `test-support` feature, for other crates' tests.

mod fake_desktop;

pub use fake_desktop::{FakeDesktop, FakeElementId, RecordedAction, element};
