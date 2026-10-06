use super::*;

/// The ledge's source as one text, as `ledge.rs` was before it was split:
/// every file of it, `ledge.rs` itself last, so a scan that stops at its
/// `#[cfg(test)]` still reads all of them.
const SOURCE: &str = concat!(
    include_str!("shelf.rs"),
    include_str!("sync.rs"),
    include_str!("answers.rs"),
    include_str!("buttons.rs"),
    include_str!("clock.rs"),
    include_str!("../ledge.rs"),
);

mod answer_tests;
mod button_tests;
mod shelf_tests;
