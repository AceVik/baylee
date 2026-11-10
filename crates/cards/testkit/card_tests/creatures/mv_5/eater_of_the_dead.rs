//! `cards/creatures/mv_5/eater_of_the_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn eater_of_the_dead() -> CardIndex {
    card_index("3c414ace-fd04-4fac-9379-24f5910f20e4")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn exiled(engine: &Engine<RegistryLookup>) -> usize {
    [PlayerId::new(0), PlayerId::new(1)]
        .into_iter()
        .map(|seat| engine.state().zones.list(ZoneLocation::Exile(seat)).len())
        .sum()
}

/// p0: the Eater (tapped or not), a Bears and a Forest; p1: a Hill Giant and a
/// Forest. The Bears, the Giant and p1's Forest are put into their graveyards.
fn graveyards(
    tapped: bool,
) -> (
    Engine<RegistryLookup>,
    ObjectId,
    ObjectId,
    ObjectId,
    ObjectId,
) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[eater_of_the_dead(), grizzly_bears()])
        .battlefield(1, &[hill_giant(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let eater = on_battlefield(&engine, p0, eater_of_the_dead()).expect("the Eater is out");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("my Bears are out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant is out");
    let land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    bury(&mut engine, &[bears, giant, land]);
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state.set_tapped(eater, tapped);
    }
    engine.refresh_offer();
    (engine, eater, bears, giant, land)
}

/// Eater of the Dead (3c414ace-fd04-4fac-9379-24f5910f20e4) — "{0}: If this
/// creature is tapped, exile target creature card from a graveyard and untap
/// this creature."
#[test]
fn eater_of_the_dead_tapped_exiles_a_creature_card_and_untaps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, eater, bears, giant, land) = graveyards(true);
    assert!(is_tapped(&engine, eater));

    activate(&mut engine, p0, eater_of_the_dead(), 0);
    let options = aim_at(&mut engine, p0, giant);
    assert!(
        options.contains(&giant) && options.contains(&bears),
        "creature cards from either graveyard: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a land card is not a creature card: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, hill_giant()).is_none(),
        "the Giant left the graveyard"
    );
    assert_eq!(exiled(&engine), 1, "and is in exile");
    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "the other card stays"
    );
    assert!(!is_tapped(&engine, eater), "the Eater untapped");
}

/// "If this creature is tapped": untapped, the ability does nothing at all.
#[test]
fn eater_of_the_dead_untapped_exiles_nothing() {
    let p0 = PlayerId::new(0);
    let (mut engine, eater, _bears, giant, _land) = graveyards(false);
    activate(&mut engine, p0, eater_of_the_dead(), 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, PlayerId::new(1), hill_giant()).is_some(),
        "the Giant stays where it is"
    );
    assert_eq!(exiled(&engine), 0);
    assert!(!is_tapped(&engine, eater));
}

/// Free and repeatable only as often as it is tapped: after untapping, a
/// second use does nothing until the Eater is tapped again.
#[test]
fn eater_of_the_dead_needs_to_be_tapped_again_for_a_second_exile() {
    let p0 = PlayerId::new(0);
    let (mut engine, eater, bears, giant, _land) = graveyards(true);
    activate(&mut engine, p0, eater_of_the_dead(), 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(exiled(&engine), 1);

    activate(&mut engine, p0, eater_of_the_dead(), 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        exiled(&engine),
        1,
        "untapped: the second use exiles nothing"
    );

    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state.set_tapped(eater, true);
    }
    engine.refresh_offer();
    activate(&mut engine, p0, eater_of_the_dead(), 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(exiled(&engine), 2, "tapped again: it works again");
    assert!(!is_tapped(&engine, eater));
}
