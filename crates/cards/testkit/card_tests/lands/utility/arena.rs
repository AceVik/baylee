//! `cards/lands/utility/arena.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn arena() -> CardIndex {
    card_index("12326d23-1fb5-4084-a23a-d6e2c90ea36c")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

/// A board with Arena and three Mountains for p0, who also has a Golem, and a
/// Golem and Bears for p1. Returns the engine with p0 in its main phase and
/// the ids `(arena, my golem, their golem, their bears)`.
fn board() -> (
    Engine<RegistryLookup>,
    ObjectId,
    ObjectId,
    ObjectId,
    ObjectId,
) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                arena(),
                mountain(),
                mountain(),
                mountain(),
                obsianus_golem(),
            ],
        )
        .battlefield(1, &[obsianus_golem(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let land = on_battlefield(&engine, p0, arena()).expect("Arena is out");
    let mine = on_battlefield(&engine, p0, obsianus_golem()).expect("my Golem is out");
    let theirs = on_battlefield(&engine, p1, obsianus_golem()).expect("their Golem is out");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears are out");
    (engine, land, mine, theirs, bears)
}

/// Pays for and presses Arena, aims it at `mine`, and has p1 choose `chosen`.
/// Returns the menu p1 chose from.
#[track_caller]
fn fight(
    engine: &mut Engine<RegistryLookup>,
    land: ObjectId,
    mine: ObjectId,
    chosen: ObjectId,
) -> Vec<ObjectId> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    tap_mana_except(engine, p0, land);
    activate(engine, p0, arena(), 0);
    let first = aim_at(engine, p0, mine);
    assert!(
        first.contains(&mine),
        "my own creature is a legal first target: {first:?}"
    );
    let menu = aim_at(engine, p1, chosen);
    pass_until(engine, stack_is_empty);
    menu
}

/// Arena (12326d23-1fb5-4084-a23a-d6e2c90ea36c) — "{3}, {T}: Tap target
/// creature you control and target creature of an opponent's choice they
/// control. Those creatures fight each other."
///
/// Two Golems fight: each takes 4, both are tapped, and the opponent made
/// the second choice from their own creatures only.
#[test]
fn arena_taps_both_creatures_and_makes_them_fight() {
    let (mut engine, land, mine, theirs, bears) = board();

    let menu = fight(&mut engine, land, mine, theirs);
    assert!(
        menu.contains(&theirs) && menu.contains(&bears),
        "the opponent chooses among their own creatures: {menu:?}"
    );
    assert!(!menu.contains(&mine), "and not among mine: {menu:?}");

    for (name, golem) in [("mine", mine), ("theirs", theirs)] {
        let object = engine
            .state()
            .object(golem)
            .expect("a 4/6 survives 4 damage");
        assert_eq!(object.zone, Zone::Battlefield);
        assert_eq!(object.damage, 4, "{name} took the other Golem's power");
        assert!(object.status.contains(Status::TAPPED), "{name} is tapped");
    }
    let bears_object = engine
        .state()
        .object(bears)
        .expect("the Bears were not chosen");
    assert_eq!(bears_object.damage, 0);
    assert!(!bears_object.status.contains(Status::TAPPED));
    assert!(
        engine
            .state()
            .object(land)
            .expect("Arena is there")
            .status
            .contains(Status::TAPPED),
        "Arena's {{T}} was paid"
    );
}

/// The opponent's choice matters: when they offer the Bears, the Bears die
/// to the Golem's 4 and the Golem takes 2.
#[test]
fn arena_fights_the_creature_the_opponent_chose() {
    let (mut engine, land, mine, theirs, bears) = board();

    fight(&mut engine, land, mine, bears);

    assert_eq!(
        engine
            .state()
            .object(mine)
            .expect("my Golem survives")
            .damage,
        2,
        "the Bears deal their 2"
    );
    assert!(
        in_graveyard(&engine, PlayerId::new(1), grizzly_bears()).is_some(),
        "the Golem's 4 kills the Bears"
    );
    assert_eq!(
        engine
            .state()
            .object(theirs)
            .expect("their Golem was not chosen")
            .damage,
        0
    );
}
