//! `cards/creatures/mv_3/vodalian_war_machine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn vodalian_war_machine() -> CardIndex {
    card_index("f72511b0-9993-4998-b646-65a855cb8170")
}

fn vodalian_soldiers() -> CardIndex {
    card_index("0e3ac78c-8a98-4786-b86c-ecfe61e13c83")
}

fn vodalian_mage() -> CardIndex {
    card_index("8dee4577-0315-4c9f-ab1c-0c68e2ab6a76")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// The War Machine, two Vodalian Soldiers, a Vodalian Mage (all Merfolk) and
/// a Grizzly Bears (not), on p0's side, at p0's main phase.
struct Table {
    engine: Engine<RegistryLookup>,
    machine: ObjectId,
    merfolk: [ObjectId; 3],
    bears: ObjectId,
}

fn table() -> Table {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                vodalian_war_machine(),
                vodalian_soldiers(),
                vodalian_soldiers(),
                vodalian_mage(),
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let machine = on_battlefield(&engine, p0, vodalian_war_machine()).expect("the Machine is out");
    let soldiers = all_on_battlefield(&engine, p0, vodalian_soldiers());
    assert_eq!(soldiers.len(), 2);
    let mage = on_battlefield(&engine, p0, vodalian_mage()).expect("the Mage is out");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    Table {
        engine,
        machine,
        merfolk: [soldiers[0], soldiers[1], mage],
        bears,
    }
}

/// Activates ability `ability`, taps `merfolk` for it, lets it resolve, and
/// returns the menu the tap was chosen from.
fn tap_merfolk_for(
    engine: &mut Engine<RegistryLookup>,
    ability: u32,
    merfolk: ObjectId,
) -> Vec<ObjectId> {
    let p0 = PlayerId::new(0);
    activate(engine, p0, vodalian_war_machine(), ability);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the tap choice, got {:?}", engine.pending())
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![merfolk],
            },
        )
        .expect("an offered Merfolk");
    pass_until(engine, stack_is_empty);
    options
}

fn survives(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .contains(&id)
}

/// Vodalian War Machine (f72511b0-9993-4998-b646-65a855cb8170) — "Defender
/// (This creature can't attack.) / Tap an untapped Merfolk you control: This
/// creature can attack this turn as though it didn't have defender. / Tap an
/// untapped Merfolk you control: This creature gets +2/+1 until end of turn.
/// / When this creature dies, destroy all Merfolk tapped this turn to pay for
/// its abilities."
#[test]
fn vodalian_war_machine_is_a_defender_that_does_not_attack_on_its_own() {
    let p0 = PlayerId::new(0);
    let mut t = table();
    assert!(keywords(&t.engine, t.machine).contains(KeywordSet::DEFENDER));
    assert_eq!(pt(&t.engine, t.machine), (0, 4));
    pass_until(
        &mut t.engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = t.engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(!attackers.contains(&t.machine), "defender: {attackers:?}");
    assert!(attackers.contains(&t.bears), "the Bears may attack");
}

#[test]
fn vodalian_war_machine_taps_a_merfolk_to_attack_and_pumps_for_another() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut t = table();
    let [first, second, third] = t.merfolk;

    let menu = tap_merfolk_for(&mut t.engine, 0, first);
    assert!(
        menu.contains(&first) && menu.contains(&second) && menu.contains(&third),
        "every untapped Merfolk can pay: {menu:?}"
    );
    assert!(
        !menu.contains(&t.bears) && !menu.contains(&t.machine),
        "the Bears are no Merfolk and the Machine is not one either: {menu:?}"
    );
    assert!(is_tapped(&t.engine, first), "the Merfolk is tapped");

    let menu = tap_merfolk_for(&mut t.engine, 1, second);
    assert!(
        !menu.contains(&first),
        "a tapped Merfolk cannot pay again: {menu:?}"
    );
    assert!(is_tapped(&t.engine, second));
    assert_eq!(pt(&t.engine, t.machine), (2, 5), "+2/+1");

    pass_until(
        &mut t.engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = t.engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&t.machine),
        "it attacks as though it had no defender: {attackers:?}"
    );
    t.engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(t.machine, Defender::Player(p1))],
            },
        )
        .expect("the Machine may attack");
    pass_until(&mut t.engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(t.engine.state().players[1].life, 18, "2 damage unblocked");

    reach_their_main_phase(&mut t.engine, p1);
    assert_eq!(pt(&t.engine, t.machine), (0, 4), "until end of turn");
}

/// The attack permission alone: the Machine attacks as a 0/4.
#[test]
fn vodalian_war_machine_attack_permission_does_not_pump() {
    let p0 = PlayerId::new(0);
    let mut t = table();
    tap_merfolk_for(&mut t.engine, 0, t.merfolk[0]);
    assert_eq!(pt(&t.engine, t.machine), (0, 4));
    pass_until(
        &mut t.engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = t.engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(attackers.contains(&t.machine));
}

/// Same turn: the Merfolk tapped for either ability die with the Machine; a
/// Merfolk that was not tapped for it, and one that was tapped by something
/// else, do not.
#[test]
fn vodalian_war_machine_dying_destroys_the_merfolk_tapped_this_turn_to_pay() {
    let mut t = table();
    let [first, second, third] = t.merfolk;
    tap_merfolk_for(&mut t.engine, 0, first);
    tap_merfolk_for(&mut t.engine, 1, second);
    kill(&mut t.engine, t.machine);

    assert!(!survives(&t.engine, t.machine), "the Machine died");
    assert!(
        !survives(&t.engine, first),
        "tapped for the attack permission"
    );
    assert!(!survives(&t.engine, second), "tapped for the pump");
    assert!(survives(&t.engine, third), "never tapped");
    assert!(survives(&t.engine, t.bears), "no Merfolk, never tapped");
}

#[test]
fn vodalian_war_machine_spares_a_merfolk_tapped_by_other_means() {
    let mut t = table();
    let [first, _second, third] = t.merfolk;
    tap_merfolk_for(&mut t.engine, 1, first);
    {
        let state = t
            .engine
            .dev_state_mut(PlayerId::new(0))
            .expect("the harness may set boards up");
        state.set_tapped(third, true);
    }
    t.engine.refresh_offer();
    kill(&mut t.engine, t.machine);
    assert!(!survives(&t.engine, first), "tapped to pay");
    assert!(
        survives(&t.engine, third),
        "tapped, but not to pay for the Machine's abilities"
    );
}

/// The regression: a Merfolk tapped for an ability in an earlier turn is not
/// "tapped this turn". Tapped on p0's turn, the Machine dies on p1's.
#[test]
fn vodalian_war_machine_spares_a_merfolk_tapped_on_an_earlier_turn_when_it_dies_on_the_opponents() {
    let p1 = PlayerId::new(1);
    let mut t = table();
    let [first, _second, _third] = t.merfolk;
    let turn_of_tapping = t.engine.state().turn.number;
    tap_merfolk_for(&mut t.engine, 1, first);

    reach_their_main_phase(&mut t.engine, p1);
    assert!(
        t.engine.state().turn.number > turn_of_tapping,
        "a later turn"
    );
    kill(&mut t.engine, t.machine);
    assert!(!survives(&t.engine, t.machine), "the Machine died");
    assert!(
        survives(&t.engine, first),
        "it was tapped last turn, not this one"
    );
}

/// The same, with the Machine dying on its controller's next turn, where a
/// second Merfolk is tapped for it: only that one dies.
#[test]
fn vodalian_war_machine_destroys_only_the_merfolk_tapped_in_the_turn_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut t = table();
    let [first, second, third] = t.merfolk;
    let turn_of_tapping = t.engine.state().turn.number;
    tap_merfolk_for(&mut t.engine, 0, first);

    reach_their_main_phase(&mut t.engine, p1);
    assert!(walk_to_own_main(&mut t.engine, p0), "p0's next own main");
    assert!(
        t.engine.state().turn.number > turn_of_tapping,
        "a later turn"
    );
    assert!(!is_tapped(&t.engine, first), "untapped by the untap step");

    let menu = tap_merfolk_for(&mut t.engine, 1, second);
    assert!(menu.contains(&first), "untapped again, so it could pay");
    kill(&mut t.engine, t.machine);

    assert!(!survives(&t.engine, t.machine), "the Machine died");
    assert!(!survives(&t.engine, second), "tapped to pay this turn");
    assert!(
        survives(&t.engine, first),
        "tapped to pay in an earlier turn"
    );
    assert!(survives(&t.engine, third), "never tapped");
}
