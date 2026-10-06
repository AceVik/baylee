//! `cards/instants/mv_1/camouflage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Camouflage and False Orders remain partial with no implemented effect.
#[test]
fn partial_instants_with_no_text_written_resolve_doing_nothing() {
    for (name, card, land, lands) in [
        (
            "Camouflage",
            card_index("9cf44db4-627a-4197-9588-6da72e41f03d"),
            forest(),
            1,
        ),
        (
            "False Orders",
            card_index("38c5c952-8153-4d98-89b5-a75260383345"),
            mountain(),
            1,
        ),
    ] {
        still_partial(card);
        assert_eq!(
            cast_saying_nothing(card, land, lands),
            Zone::Graveyard,
            "{name}"
        );
    }
}
