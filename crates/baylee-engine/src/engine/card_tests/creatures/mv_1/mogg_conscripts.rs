//! `cards/creatures/mv_1/mogg_conscripts.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mogg_conscripts() -> CardIndex {
    card_index("b015686d-07e1-46c7-8e51-2bec646860d6")
}

/// Mogg Conscripts (`b015686d-07e1-46c7-8e51-2bec646860d6`) — {R} Creature — Goblin (Partial):
/// "This creature can't attack unless you've cast a creature spell this turn."
#[test]
fn mogg_conscripts_attacks_after_casting_a_spell_this_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mogg_conscripts(), mountain(), forest(), forest()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mogg = on_battlefield(&engine, P0, mogg_conscripts()).unwrap();
    cast_from_hand(&mut engine, P0, index::GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mogg, Defender::Player(P1))],
            },
        )
        .unwrap();
}
