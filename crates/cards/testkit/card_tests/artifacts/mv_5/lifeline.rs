//! `cards/artifacts/mv_5/lifeline.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn lifeline() -> CardIndex {
    card_index("f475b636-eb7f-4dd7-bc87-a1af07064ffe")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

/// Lifeline (f475b636-eb7f-4dd7-bc87-a1af07064ffe) — {5} Artifact:
/// "Whenever a creature dies, if another creature is on the battlefield, return
///  the first card to the battlefield under its owner's control at the
///  beginning of the next end step."
///
/// Both your creature and the opponent's creature return under their owners'
/// control at the next end step when another creature remains on the
/// battlefield.
#[test]
fn lifeline_returns_both_players_creatures_at_end_step_when_another_remains() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                lifeline(),
                grizzly_bears(),
                llanowar_elves(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    // Kill opponent's Elves while two creatures remain; kill own Bears while one remains.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, their_elves);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(their_elves).unwrap().zone,
        Zone::Graveyard
    );

    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, my_bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(my_bears).unwrap().zone,
        Zone::Graveyard
    );

    // Neither returns before the end step.
    assert_eq!(on_battlefield(&engine, p0, grizzly_bears()), None);
    assert_eq!(on_battlefield(&engine, p1, llanowar_elves()), None);

    // Cross p0's end step into p1's turn: both returned under their owners' control.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "own Bears returned under p0's control at end step"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "opponent's Elves returned under p1's control at end step"
    );
}

/// When the ONLY creature on the battlefield dies, "if another creature is on
/// the battlefield" is false and Lifeline does not return it.
#[test]
fn lifeline_does_not_return_the_last_creature_on_the_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[lifeline(), llanowar_elves(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let solo_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, solo_elf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(solo_elf).unwrap().zone,
        Zone::Graveyard
    );

    // Cross p0's end step into p1's turn: solo Elf stays in the graveyard.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(
        on_battlefield(&engine, p0, llanowar_elves()),
        None,
        "no other creature was on the battlefield when it died"
    );
    assert_eq!(
        engine.state().object(solo_elf).unwrap().zone,
        Zone::Graveyard
    );
}

/// "Under its owner's control": a creature owned by p0 but stolen by p1 via
/// Control Magic returns under p0's control, not p1's.
#[test]
fn lifeline_returns_stolen_creature_under_its_owners_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[lifeline(), grizzly_bears(), llanowar_elves(), mountain()],
        )
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[control_magic()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p1, control_magic());
    aim_at(&mut engine, p1, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().controller, p1);

    // On p0's turn, p0 Bolts the stolen Bears (while p0's Elves remains on the battlefield).
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Graveyard);

    // Cross end step: Grizzly Bears returns under p0's (owner's) control.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "returned under owner p0's control"
    );
    assert_eq!(on_battlefield(&engine, p1, grizzly_bears()), None);
}
