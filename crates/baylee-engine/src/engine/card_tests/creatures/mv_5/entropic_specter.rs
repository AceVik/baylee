//! `cards/creatures/mv_5/entropic_specter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn entropic_specter() -> CardIndex {
    card_index("a9c44443-85c7-4d0a-82ab-36137151e4e5")
}

/// Entropic Specter (`a9c44443-85c7-4d0a-82ab-36137151e4e5`) — {3}{B}{B} Creature — Specter Spirit (Partial):
/// "Flying. As this creature enters, choose an opponent. Whenever this creature deals damage to a player, that player discards a card."
#[test]
fn entropic_specter_causes_damaged_player_to_discard_a_card() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[entropic_specter()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    reach_main_phase(&mut engine, P0);

    let specter = on_battlefield(&engine, P0, entropic_specter()).unwrap();
    let card_in_p1_hand = in_hand(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    kit::declare_attack(&mut engine, &[specter], P1);
    kit::drive(
        &mut engine,
        |e| matches!(e.state().turn.phase, Phase::SecondMain),
        |_, p| kit::choose_cards(p, &[card_in_p1_hand]),
    );

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
