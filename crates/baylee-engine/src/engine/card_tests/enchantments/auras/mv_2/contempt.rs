//! `cards/enchantments/auras/mv_2/contempt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn contempt() -> CardIndex {
    card_index("811c8364-fafb-49c8-a5bf-7b6cdfceb475")
}

/// Contempt (`811c8364-fafb-49c8-a5bf-7b6cdfceb475`) — {1}{U} Enchantment — Aura (Implemented):
/// "Enchant creature. When enchanted creature attacks, return it and this Aura to their owners' hands at end of combat."
#[test]
fn contempt_bounces_enchanted_creature_and_itself_at_end_of_combat() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), index::GRIZZLY_BEARS])
        .hand(0, &[contempt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, contempt());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatDamage && stack_is_empty(e)
    });
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P0, contempt()).is_some());
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, P0, contempt()).is_some());
}
