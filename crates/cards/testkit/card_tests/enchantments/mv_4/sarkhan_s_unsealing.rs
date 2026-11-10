//! `cards/enchantments/mv_4/sarkhan_s_unsealing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sarkhan_s_unsealing() -> CardIndex {
    card_index("c27c9514-7b5a-461a-a109-f8abafd83523")
}

fn eldrazi_devastator() -> CardIndex {
    card_index("bc08ac66-969c-4905-a2d6-16ff565be475")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// "Whenever you cast a creature spell with power 4, 5, or 6, this
/// enchantment deals 4 damage to any target": Serra Angel (power 4) asks for
/// a target, and the named player takes 4. Hill Giant (power 3) asks nothing.
#[test]
fn sarkhan_s_unsealing_deals_four_to_any_target_for_power_four_to_six() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sarkhan_s_unsealing()])
        .hand(0, &[hill_giant(), serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Red, 4);
    cast_with_floating(&mut engine, p0, hill_giant());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 20, "power 3 does not trigger it");

    float(&mut engine, p0, ManaColor::White, 5);
    cast_with_floating(&mut engine, p0, serra_angel());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player is a legal target for any target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 16, "4 damage to the target");
    assert_eq!(life_of(&engine, p0), 20);
}

/// "Whenever you cast a creature spell with power 7 or greater, this
/// enchantment deals 4 damage to each opponent and each creature and
/// planeswalker they control": an 8-power creature spell hits the opponent
/// and their creature, not my own creature, and asks for no target.
#[test]
fn sarkhan_s_unsealing_sweeps_the_opponents_for_power_seven_or_more() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sarkhan_s_unsealing(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[eldrazi_devastator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Green, 8);
    cast_with_floating(&mut engine, p0, eldrazi_devastator());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 16, "4 damage to the opponent");
    assert_eq!(life_of(&engine, p0), 20, "none to me");
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "their creature took 4 and died"
    );
    assert!(
        in_graveyard(&engine, PlayerId::new(1), grizzly_bears()).is_some(),
        "it went to their graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "my own creature is untouched"
    );
}
