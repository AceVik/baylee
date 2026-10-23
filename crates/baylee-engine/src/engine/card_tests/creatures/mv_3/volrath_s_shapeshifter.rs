//! `cards/creatures/mv_3/volrath_s_shapeshifter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn volrath_s_shapeshifter() -> CardIndex {
    card_index("16e9a4ee-0398-4e76-bc7e-dd2d52056aaf")
}

/// Volrath's Shapeshifter (`16e9a4ee-0398-4e76-bc7e-dd2d52056aaf`) — {1}{U}{U} Creature — Phyrexian Shapeshifter (Partial):
/// "{2}: Discard a card."
#[test]
fn volrath_s_shapeshifter_activates_to_discard_a_card() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[volrath_s_shapeshifter(), island(), island()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let in_hand = engine.state().zones.list(ZoneLocation::Hand(P0))[0];
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, volrath_s_shapeshifter(), 0);
    kit::settle(&mut engine, P0, |_, p| match p {
        Pending::DiscardChoice { .. } => Some(PlayerAction::ChooseObjects {
            objects: vec![in_hand],
        }),
        _ => kit::choose_cards(p, &[in_hand]),
    });

    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P0))
            .is_empty()
    );
}
