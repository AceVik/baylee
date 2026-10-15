//! `cards/enchantments/mv_4/martyrs_tomb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn martyrs_tomb() -> CardIndex {
    card_index("a0baa9eb-0b1e-4d90-a7a1-f7102f575467")
}

fn pyrohemia() -> CardIndex {
    card_index("9ac57a10-3402-4656-9079-f713884cde35")
}

/// "Pay 2 life: Prevent the next 1 damage that would be dealt to target
/// creature this turn.": the life is paid up front, the next point of damage
/// to the Bears is prevented and the one after is not. Pyrohemia is the
/// damage: {R} deals 1 to each creature and each player.
#[test]
fn martyrs_tomb_prevents_exactly_the_next_one_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[martyrs_tomb(), pyrohemia(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");

    activate(&mut engine, p0, martyrs_tomb(), 0);
    aim_at(&mut engine, p0, bears);
    assert_eq!(
        life_of(&engine, p0),
        18,
        "the 2 life is the cost, paid once the target is named"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).expect("bears").damage,
        0,
        "nothing happened yet"
    );

    float(&mut engine, p0, ManaColor::Red, 1);
    activate(&mut engine, p0, pyrohemia(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).expect("bears").damage,
        0,
        "the next 1 damage was prevented"
    );
    assert_eq!(life_of(&engine, p0), 17, "the player was not shielded");

    float(&mut engine, p0, ManaColor::Red, 1);
    activate(&mut engine, p0, pyrohemia(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).expect("bears").damage,
        1,
        "the shield was used up: the second point lands"
    );
}
