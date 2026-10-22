//! `cards/enchantments/auras/mv_2/volrath_s_curse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LLANOWAR_ELVES;

fn volrath_s_curse() -> CardIndex {
    card_index("f4ff3efb-c9ed-4451-a7f2-5ff52ffb6cab")
}

/// `Volrath's Curse` ({1}{U}, Enchantment — Aura, `Coverage::Partial`):
/// `Enchant creature`
/// `Enchanted creature can't attack or block, and its activated abilities can't
/// be activated.`
/// `{1}{U}: Return this Aura to its owner's hand.`
#[test]
fn volraths_curse_prevents_attacking_and_activating_and_returns_to_hand_for_1u() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), LLANOWAR_ELVES])
        .hand(0, &[volrath_s_curse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, LLANOWAR_ELVES).unwrap();
    cast_from_hand(&mut engine, p0, volrath_s_curse());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Llanowar Elves cannot activate its mana ability while enchanted by Volrath's Curse.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: elves,
                    ability_index: 0,
                },
            )
            .is_err()
    );

    // Pay {1}{U} to return Volrath's Curse to owner's hand.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, volrath_s_curse(), 4);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, volrath_s_curse()).is_none());
    assert!(in_hand(&engine, p0, volrath_s_curse()).is_some());
}
