//! `cards/creatures/artifacts/mv_2/diversion_unit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn diversion_unit() -> CardIndex {
    card_index("4d739edc-4e02-4b35-a2a0-e9685c6e2889")
}

/// The red instant that stands in for "an instant spell".
fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

/// Seat 0 casts Shock at the opponent and answers it with the Unit: `{U}`,
/// sacrifice, counter unless its controller (also seat 0) pays `{3}`.
fn shock_met_by_the_unit(pay: bool) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[shock()], &[diversion_unit(), mountain(), island()], &[]);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, shock());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent is a legal target");
    let spell = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .last()
        .copied()
        .expect("Shock is on the stack");
    activate(&mut engine, p0, diversion_unit(), 0);
    aim_at(&mut engine, p0, spell);
    assert!(
        in_graveyard(&engine, p0, diversion_unit()).is_some(),
        "the sacrifice is a cost"
    );
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((player, mana), (p0, 3));
    engine.apply(p0, PlayerAction::YesNo(pay)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    engine
}

/// Diversion Unit: a 2/1 flier. Declining the tax counters Shock.
#[test]
fn diversion_unit_counters_an_instant_when_the_tax_is_declined() {
    let engine = shock_met_by_the_unit(false);
    assert_eq!(life_of(&engine, 1), 20, "Shock was countered");
    assert!(in_graveyard(&engine, PlayerId::new(0), shock()).is_some());
}

/// ... and when the controller can pay it, the spell resolves.
#[test]
fn diversion_unit_lets_the_spell_resolve_when_the_tax_is_paid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[shock()],
        &[
            diversion_unit(),
            mountain(),
            island(),
            forest(),
            forest(),
            forest(),
        ],
        &[],
    );
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, shock());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    let spell = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .last()
        .copied()
        .expect("on the stack");
    activate(&mut engine, p0, diversion_unit(), 0);
    aim_at(&mut engine, p0, spell);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 1), 18, "Shock resolved: the tax was paid");
}

/// Only instant or sorcery spells: a creature spell is not on the menu.
#[test]
fn diversion_unit_cannot_target_a_creature_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[grizzly_bears()],
        &[diversion_unit(), forest(), forest(), island()],
        &[],
    );
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    let offered_unit = engine.pending().clone();
    let unit = on_battlefield(&engine, p0, diversion_unit()).expect("seated");
    let _ = offered_unit;
    // With no legal target the ability is not on offer at all, or it asks
    // for a target and the Bears are not among them.
    let offer = priority_offer(&engine);
    if offer.abilities.contains(&(unit, 0)) {
        activate(&mut engine, p0, diversion_unit(), 0);
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!("expected a target question")
        };
        assert!(options.is_empty(), "no instant or sorcery on the stack");
    }
}

/// Flying, 2/1.
#[test]
fn diversion_unit_is_a_two_one_flier() {
    let p0 = PlayerId::new(0);
    let engine = table(&[], &[diversion_unit()], &[]);
    let unit = on_battlefield(&engine, p0, diversion_unit()).expect("seated");
    assert_eq!(pt(&engine, unit), (2, 1));
    assert!(keywords(&engine, unit).contains(KeywordSet::FLYING));
}
