//! `cards/enchantments/mv_4/pestilence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pestilence: "{B}: This enchantment deals 1 damage to each creature and
/// each player." Everyone and everything takes the 1, including the
/// controller's own side: a 1-toughness creature dies, and a 3-toughness
/// one merely takes the damage.
#[test]
fn pestilence_deals_one_damage_to_each_creature_and_each_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let pestilence = pestilence();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[pestilence, swamp(), llanowar_elves()])
        .battlefield(1, &[wretched_anurid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0's creature");
    let anurid = on_battlefield(&engine, p1, wretched_anurid()).expect("p1's creature");
    assert_eq!(pt(&engine, elf), (1, 1));
    assert_eq!(pt(&engine, anurid), (3, 3));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, pestilence, 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 19, "the controller too");
    assert_eq!(engine.state().players[1].life, 19);
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "1 damage killed the 1-toughness Elves"
    );
    assert_eq!(
        engine
            .state()
            .object(anurid)
            .expect("the Anurid survives")
            .damage,
        1,
        "and marked 1 damage on the 3-toughness Anurid, which stands"
    );
}

/// Pestilence's other printed line: "At the beginning of the end step, if
/// no creatures are on the battlefield, sacrifice this enchantment." With
/// none around, it is gone by the following turn; with a creature standing
/// on either side of the table, the same end step leaves it alone.
#[test]
fn pestilence_sacrifices_itself_at_the_end_step_with_no_creatures_on_the_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let pestilence = pestilence();
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[pestilence])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        in_graveyard(&engine, p0, pestilence).is_some(),
        "no creatures anywhere, so it sacrificed itself at the end step"
    );

    // Counter-board: a creature on either side of the table means "no
    // creatures are on the battlefield" is false, so the same end step
    // leaves Pestilence standing.
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[pestilence, festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, pestilence).is_some(),
        "a creature on its own controller's side, so Pestilence was not sacrificed"
    );

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[pestilence])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, pestilence).is_some(),
        "a creature on the other side of the table, so Pestilence was not sacrificed"
    );
}
