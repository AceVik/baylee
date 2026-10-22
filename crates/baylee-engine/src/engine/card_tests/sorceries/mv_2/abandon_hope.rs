//! `cards/sorceries/mv_2/abandon_hope.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn abandon_hope() -> CardIndex {
    card_index("8adbba6e-03ef-4278-aec5-8a4496b377a8")
}

/// Abandon Hope (8adbba6e-03ef-4278-aec5-8a4496b377a8) — {X}{1}{B} Sorcery (Partial):
/// "As an additional cost to cast this spell, discard X cards. Look at target opponent's hand and choose X cards from it. That player discards those cards."
#[test]
fn abandon_hope_discards_additional_cost_and_makes_opponent_discard_chosen_card() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[abandon_hope(), quiet_creature()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let fodder = in_hand(&engine, P0, quiet_creature()).unwrap();
    let opp_card = in_hand(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    let spell = in_hand(&engine, P0, abandon_hope()).unwrap();
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    // Announce X = 1.
    assert!(matches!(engine.pending(), Pending::ChooseNumber { .. }));
    engine.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();

    // Choose target opponent P1, then pay mandatory additional discard cost.
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();

    // Resolve and choose the card from P1's hand to discard.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![opp_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, quiet_creature()).is_some());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
