//! `cards/enchantments/mv_4/last_laugh.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn last_laugh() -> CardIndex {
    card_index("5facb256-b993-43f8-b971-b3a59a7434bf")
}

/// "Whenever a permanent other than this enchantment is put into a graveyard
/// from the battlefield, this enchantment deals 1 damage to each creature
/// and each player": a creature dying does it (the survivor keeps a damage
/// mark, both players lose 1).
#[test]
fn last_laugh_pings_everything_when_a_creature_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[last_laugh(), grizzly_bears(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = all_on_battlefield(&engine, p0, grizzly_bears());
    kill(&mut engine, bears[0]);
    assert_eq!(life_of(&engine, p0), 19);
    assert_eq!(life_of(&engine, p1), 19);
    assert_eq!(
        engine.state().object(bears[1]).expect("survivor").damage,
        1,
        "the surviving 2/2 took 1 from the enchantment"
    );
}

/// The same for a permanent that is not a creature: the printed sentence says
/// "a permanent", not "a creature".
#[test]
fn last_laugh_pings_everything_when_a_noncreature_permanent_goes_to_the_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[last_laugh(), grizzly_bears(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let artifact = on_battlefield(&engine, p0, quiet_artifact()).expect("seated");
    kill(&mut engine, artifact);
    assert_eq!(life_of(&engine, p0), 19, "1 damage to each player");
    assert_eq!(life_of(&engine, p1), 19);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("2/2 survives 1");
    assert_eq!(engine.state().object(bears).expect("bears").damage, 1);
}

/// "When no creatures are on the battlefield, sacrifice this enchantment."
#[test]
fn last_laugh_is_sacrificed_when_no_creatures_are_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[last_laugh()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, last_laugh()).is_some(),
        "no creature anywhere: sacrificed"
    );
}
