//! `cards/creatures/mv_8/ashen_firebeast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ashen_firebeast() -> CardIndex {
    card_index("7246e3a0-f8b7-4c1b-ae75-a1eb8990a728")
}

fn damage_on(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .expect("on the battlefield")
        .damage
}

/// Ashen Firebeast is a 6/6 for {6}{R}{R}.
#[test]
fn ashen_firebeast_is_cast_for_eight_as_a_six_six() {
    let mut engine = at_main(&[mountain(); 8], &[ashen_firebeast()]);
    let beast = cast_and_settle(&mut engine, ashen_firebeast());
    assert_eq!(pt(&engine, beast), (6, 6));
}

/// "{1}{R}: This creature deals 1 damage to each creature without flying":
/// the Bears and the Firebeast itself are marked for 1, the flying Serra
/// Angel is not, and no player is hurt.
#[test]
fn ashen_firebeast_pings_every_creature_without_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ashen_firebeast(), mountain(), mountain()])
        .battlefield(1, &[grizzly_bears(), serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let beast = on_battlefield(&engine, p0, ashen_firebeast()).expect("the Firebeast");
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("the bear");
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("the Angel");
    let life = (life_of(&engine, 0), life_of(&engine, 1));
    assert!(keywords(&engine, angel).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ashen_firebeast(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(damage_on(&engine, bear), 1, "no flying: hit");
    assert_eq!(damage_on(&engine, beast), 1, "the Firebeast hits itself");
    assert_eq!(damage_on(&engine, angel), 0, "flying: spared");
    assert_eq!(
        (life_of(&engine, 0), life_of(&engine, 1)),
        life,
        "creatures only"
    );
}
