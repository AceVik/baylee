//! `cards/enchantments/mv_2/circle_of_protection_shadow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn circle_of_protection_shadow() -> CardIndex {
    card_index("07c42f46-800e-4d2a-b844-c125ab93190d")
}

/// Circle of Protection: Shadow (07c42f46-800e-4d2a-b844-c125ab93190d) — {1}{W} Enchantment:
/// "{1}: The next time a creature of your choice with shadow would deal damage to you this turn, prevent that damage."
#[test]
fn circle_of_protection_shadow_prevents_next_damage_from_chosen_shadow_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[circle_of_protection_shadow(), plains()])
        .battlefield(1, &[index::SOLTARI_FOOT_SOLDIER])
        .start();
    keep_mulligans(&mut engine);
    let shadow_attacker = on_battlefield(&engine, P1, index::SOLTARI_FOOT_SOLDIER).unwrap();

    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shadow_attacker, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, circle_of_protection_shadow(), 0);
    let selected = baylee_core::ids::DamageSourceRef {
        object: shadow_attacker,
        version: engine.state().object(shadow_attacker).unwrap().version,
    };
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource { choice, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: selected,
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[0].life, 20);
}
