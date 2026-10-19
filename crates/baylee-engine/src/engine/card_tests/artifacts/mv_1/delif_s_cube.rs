//! `cards/artifacts/mv_1/delif_s_cube.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn delif_s_cube() -> CardIndex {
    card_index("3106ba15-08d7-474c-a384-6848b713ba8d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

fn cube_counters(engine: &Engine<RegistryLookup>, cube: ObjectId) -> u16 {
    engine
        .state()
        .object(cube)
        .expect("the Cube is still out")
        .counters
        .get(baylee_cards_dsl::counters::CUBE)
}

/// Taps exactly the Forests in `which` (indices into `forests`).
#[track_caller]
fn tap_forests(engine: &mut Engine<RegistryLookup>, forests: &[ObjectId], which: &[usize]) {
    let chosen: Vec<ObjectId> = which.iter().map(|&i| forests[i]).collect();
    let tapped = tap_mana_where(engine, PlayerId::new(0), |id| chosen.contains(&id));
    assert_eq!(tapped, which.len());
}

/// p0 holds the Cube, Bears (and `extra`), six Forests; the Cube's first
/// ability is paid with two of them and resolved on the Bears. Returns the
/// engine, the Cube, the Bears and the Forests.
fn cube_aimed_at_the_bears(
    extra: &[CardIndex],
    theirs: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, Vec<ObjectId>) {
    let p0 = PlayerId::new(0);
    let mut mine = vec![delif_s_cube(), grizzly_bears()];
    mine.extend_from_slice(extra);
    mine.extend([forest(); 6]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cube = on_battlefield(&engine, p0, delif_s_cube()).expect("the Cube");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears");
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 6);

    tap_forests(&mut engine, &forests, &[0, 1]);
    activate(&mut engine, p0, delif_s_cube(), 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, cube), "{{T}} was part of the price");
    assert_eq!(
        cube_counters(&engine, cube),
        0,
        "nothing yet: the Bears have not attacked"
    );
    (engine, cube, bears, forests)
}

#[track_caller]
fn attack_with(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) {
    pass_until(engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareAttackers
    });
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&a| (a, Defender::Player(PlayerId::new(1))))
                    .collect(),
            },
        )
        .expect("the attackers were on the offer");
}

#[track_caller]
fn blocks(engine: &mut Engine<RegistryLookup>, blockers: Vec<(ObjectId, ObjectId)>) {
    pass_until(engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
    });
    engine
        .apply(PlayerId::new(1), PlayerAction::DeclareBlockers { blockers })
        .expect("the blocks were on the offer");
}

fn to_second_main(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });
}

/// Delif's Cube (3106ba15-08d7-474c-a384-6848b713ba8d) — "{2}, {T}: This turn,
/// when target creature you control attacks and isn't blocked, it assigns no
/// combat damage this turn and you put a cube counter on this artifact."
///
/// Bears and Lions attack unblocked: the Bears deal nothing, the Lions deal
/// their 2, and exactly one counter lands on the Cube (the Lions were not the
/// target).
#[test]
fn delif_s_cube_stops_its_unblocked_creature_and_stores_one_counter() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, cube, bears, _forests) = cube_aimed_at_the_bears(&[savannah_lions()], &[]);
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("Lions");
    attack_with(&mut engine, &[bears, lions]);
    blocks(&mut engine, vec![]);
    to_second_main(&mut engine);
    assert_eq!(
        life(&engine, p1),
        18,
        "the Lions hit, the Bears assigned none"
    );
    assert_eq!(cube_counters(&engine, cube), 1, "one cube counter");
}

/// "Isn't blocked": blocked, the Bears are an ordinary attacker, no counter.
#[test]
fn delif_s_cube_does_nothing_when_the_creature_is_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, cube, bears, _forests) = cube_aimed_at_the_bears(&[], &[hill_giant()]);
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("their Giant");
    attack_with(&mut engine, &[bears]);
    blocks(&mut engine, vec![(giant, bears)]);
    to_second_main(&mut engine);
    assert_eq!(
        cube_counters(&engine, cube),
        0,
        "no counter for a blocked creature"
    );
    assert!(
        in_graveyard(&engine, p0, grizzly_bears()).is_some(),
        "the Giant's damage killed the Bears: combat was normal"
    );
}

/// "{2}, Remove a cube counter from this artifact: Regenerate target
/// creature." Not offered with no counter, even with the {2} floating; once a
/// counter is stored, paying removes it and gives the target a shield.
#[test]
fn delif_s_cube_spends_a_counter_to_regenerate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, cube, bears, forests) = cube_aimed_at_the_bears(&[], &[]);

    tap_forests(&mut engine, &forests, &[2, 3]);
    assert!(
        !priority_offer(&engine).abilities.contains(&(cube, 1)),
        "no cube counter, no regeneration"
    );

    attack_with(&mut engine, &[bears]);
    blocks(&mut engine, vec![]);
    to_second_main(&mut engine);
    assert_eq!(life(&engine, p1), 20);
    assert_eq!(cube_counters(&engine, cube), 1);

    tap_forests(&mut engine, &forests, &[4, 5]);
    assert!(
        priority_offer(&engine).abilities.contains(&(cube, 1)),
        "a counter and {{2}}: the second ability is on offer"
    );
    activate(&mut engine, p0, delif_s_cube(), 1);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(cube_counters(&engine, cube), 0, "the counter was the price");
    assert_eq!(
        engine.state().object(bears).unwrap().regeneration_shields,
        1,
        "the Bears hold a regeneration shield"
    );
}
