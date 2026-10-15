//! `cards/enchantments/mv_6/a_realm_reborn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::granted_ability;

fn a_realm_reborn() -> CardIndex {
    card_index("fd230ab5-874e-4827-9b5c-6d39d079cee6")
}

/// The permanents offered a granted first ability (the slot the Realm's
/// "{T}: Add one mana of any color" comes under) to the seat with priority.
fn granted_sources(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal
        .abilities
        .iter()
        .filter(|(_, index)| *index == granted_ability(0))
        .map(|(source, _)| *source)
        .collect()
}

/// "Other permanents you control have '{T}: Add one mana of any color.'" A
/// Righteous Cause of mine taps for one mana of a colour I choose; A Realm
/// Reborn itself ("other") is offered nothing.
#[test]
fn a_realm_reborn_lets_my_other_permanents_tap_for_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[a_realm_reborn(), righteous_cause()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cause = on_battlefield(&engine, p0, righteous_cause()).expect("Righteous Cause");
    let realm = on_battlefield(&engine, p0, a_realm_reborn()).expect("A Realm Reborn");
    let sources = granted_sources(&engine);
    assert!(
        sources.contains(&cause),
        "the other permanent taps for mana"
    );
    assert!(!sources.contains(&realm), "but not the Realm itself");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cause,
                ability_index: granted_ability(0),
            },
        )
        .expect("the granted ability activates");
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        let (player, action) = answer_one(&engine).expect("a colour to choose");
        engine.apply(player, action).unwrap();
    }
    assert!(is_tapped(&engine, cause), "{{T}} is the cost");
    assert!(!is_tapped(&engine, realm));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana of a colour"
    );
}

/// "…you control": the opponent's permanent is given nothing, though my
/// Realm is out.
#[test]
fn a_realm_reborn_does_not_help_the_opponents_permanents() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[a_realm_reborn(), righteous_cause()])
        .battlefield(1, &[righteous_cause()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let theirs = on_battlefield(&engine, p1, righteous_cause()).expect("their Cause");
    let mine = on_battlefield(&engine, PlayerId::new(0), righteous_cause()).expect("my Cause");
    let sources = granted_sources(&engine);
    assert!(!sources.contains(&theirs), "no mana ability on theirs");
    assert!(
        !sources.contains(&mine),
        "and it is not their priority to use mine"
    );
}
