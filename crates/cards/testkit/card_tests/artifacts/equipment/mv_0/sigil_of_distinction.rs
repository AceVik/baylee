//! `cards/artifacts/equipment/mv_0/sigil_of_distinction.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sigil_of_distinction() -> CardIndex {
    card_index("3aa3526b-bd06-417f-af29-5a9ff0844923")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Sigil of Distinction (3aa3526b-bd06-417f-af29-5a9ff0844923) — {X} Artifact — Equipment:
/// "This Equipment enters with X charge counters on it."
/// "Equipped creature gets +1/+1 for each charge counter on this Equipment."
/// "Equip—Remove a charge counter from this Equipment."
///
/// Cast for X = 3: enters with 3 charge counters; equipping removes 1 counter
/// (leaving 2 -> +2/+2); re-equipping removes 1 more (leaving 1 -> +1/+1);
/// re-equipping removes the last (leaving 0 -> +0/+0, equip now illegal).
#[test]
fn sigil_of_distinction_enters_with_x_counters_and_removes_one_per_equip() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), grizzly_bears(), hill_giant()],
        )
        .hand(0, &[sigil_of_distinction()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();

    // Cast Sigil of Distinction with X = 3.
    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, sigil_of_distinction()).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let sigil = on_battlefield(&engine, p0, sigil_of_distinction()).unwrap();
    assert_eq!(counters_on(&engine, sigil, CounterKind::Charge), 3);

    // Equip to Grizzly Bears: removes 1 charge counter (2 left) -> Bears is 4/4.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sigil,
                ability_index: 0,
            },
        )
        .expect("equip by removing 1 charge counter");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, sigil, CounterKind::Charge), 2);
    assert_eq!(pt(&engine, bears), (4, 4), "2/2 + 2/+2 = 4/4");
    assert_eq!(pt(&engine, giant), (3, 3));

    // Re-equip to Hill Giant: removes 1 charge counter (1 left) -> Giant is 4/4, Bears back to 2/2.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sigil,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, sigil, CounterKind::Charge), 1);
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, giant), (4, 4), "3/3 + 1/+1 = 4/4");

    // Re-equip back to Bears: removes last counter (0 left) -> +0/+0, equip no longer offered!
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sigil,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, sigil, CounterKind::Charge), 0);
    assert_eq!(pt(&engine, bears), (2, 2), "0 counters = +0/+0");
    assert!(
        !priority_offer(&engine).abilities.contains(&(sigil, 0)),
        "cannot equip Sigil of Distinction with 0 charge counters"
    );
}

/// Casting Sigil of Distinction for X = 0 enters with 0 charge counters and
/// cannot be equipped.
#[test]
fn sigil_of_distinction_x_zero_cannot_equip() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears()])
        .hand(0, &[sigil_of_distinction()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, sigil_of_distinction()).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let sigil = on_battlefield(&engine, p0, sigil_of_distinction()).unwrap();
    assert_eq!(counters_on(&engine, sigil, CounterKind::Charge), 0);
    assert!(!priority_offer(&engine).abilities.contains(&(sigil, 0)));
}
