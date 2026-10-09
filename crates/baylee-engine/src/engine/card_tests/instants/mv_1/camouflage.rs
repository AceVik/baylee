//! `cards/instants/mv_1/camouflage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Camouflage remains partial with no implemented effect. (False Orders
/// left this pin when its effect was written: `remove_from_combat_tests`.)
#[test]
fn partial_instants_with_no_text_written_resolve_doing_nothing() {
    let card = card_index("9cf44db4-627a-4197-9588-6da72e41f03d");
    still_partial(card);
    assert_eq!(
        cast_saying_nothing(card, forest(), 1),
        Zone::Graveyard,
        "Camouflage"
    );
}
