//! `cards/enchantments/mv_4/ill_gotten_inheritance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ill_gotten_inheritance() -> CardIndex {
    card_index("84e8ca5e-730e-468b-943c-7c6358ade95c")
}

/// "At the beginning of your upkeep, this enchantment deals 1 damage to each
/// opponent and you gain 1 life": each of two opponents takes 1.
#[test]
fn ill_gotten_inheritance_pings_each_opponent_at_my_upkeep() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[ill_gotten_inheritance()])
        .start();
    keep_mulligans(&mut engine);

    walk_to_step_trigger(&mut engine, p0, Step::Upkeep);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 21, "I gain 1");
    assert_eq!(life_of(&engine, p1), 19, "1 damage to the first opponent");
    assert_eq!(life_of(&engine, p2), 19, "and to the second");
}

/// "{5}{B}, Sacrifice this enchantment: It deals 4 damage to target opponent
/// and you gain 4 life": the chosen opponent alone takes the 4, the cost
/// includes the sacrifice.
#[test]
fn ill_gotten_inheritance_sacrifices_for_four_damage_to_the_chosen_opponent() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[ill_gotten_inheritance()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    pass_until(&mut engine, stack_is_empty);
    let before = [
        life_of(&engine, p0),
        life_of(&engine, p1),
        life_of(&engine, p2),
    ];
    float(&mut engine, p0, ManaColor::Black, 6);

    activate(&mut engine, p0, ill_gotten_inheritance(), 1);
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!("a target opponent is asked, got {:?}", engine.pending())
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p2],
            },
        )
        .expect("the third seat is an opponent");
    assert!(
        in_graveyard(&engine, p0, ill_gotten_inheritance()).is_some(),
        "the sacrifice was part of the cost"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), before[0] + 4, "I gain 4");
    assert_eq!(
        life_of(&engine, p1),
        before[1],
        "the other opponent is spared"
    );
    assert_eq!(
        life_of(&engine, p2),
        before[2] - 4,
        "4 damage to the target"
    );
}
