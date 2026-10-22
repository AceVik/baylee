//! `cards/artifacts/mv_3/phyrexian_grimoire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn phyrexian_grimoire() -> CardIndex {
    card_index("090052a8-9e5b-4024-881f-5f8b65db7aa7")
}

/// Phyrexian Grimoire (`090052a8-9e5b-4024-881f-5f8b65db7aa7`) — {3} Artifact — Book (Partial):
/// "{4}, {T}: Target opponent chooses one of the top two cards of your graveyard. Exile that card and put the other one into your hand."
#[test]
fn phyrexian_grimoire_returns_card_from_graveyard_to_hand() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                phyrexian_grimoire(),
                mountain(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, lightning_bolt());
    legends_kit::aim(&mut engine, &[], &[P1]);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, lightning_bolt()).is_some());

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, phyrexian_grimoire(), 0);
    legends_kit::aim(&mut engine, &[], &[P1]);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, lightning_bolt()).is_some());
}
