//! `cards/lands/utility/diamond_valley.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn diamond_valley() -> CardIndex {
    card_index("84cef34a-c3e1-4059-b4cd-c481938a53a5")
}

/// A 2/1 Cat: power and toughness differ, so the life gained says which
/// of the two numbers was read.
fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn glorious_anthem() -> CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

/// Diamond Valley prints "{T}, Sacrifice a creature: You gain life equal to
/// the sacrificed creature's toughness." Lions are a 2/1; Giant Growth makes
/// them 5/4 before the sacrifice, so the life gained (4) is neither the
/// printed toughness (1) nor the power (5) but the toughness the creature
/// had when it left (CR 608.2h).
#[test]
fn diamond_valley_gains_the_pumped_toughness_the_creature_had_when_it_left() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[diamond_valley(), forest(), savannah_lions()])
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let valley = on_battlefield(&engine, p0, diamond_valley()).expect("the Valley is out");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions are out");
    assert_eq!(pt(&engine, lions), (2, 1), "the printed body");

    let forest_id = on_battlefield(&engine, p0, forest()).expect("a Forest");
    tap_mana_where(&mut engine, p0, |id| id == forest_id);
    cast_with_floating(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, lions);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, lions), (5, 4), "Giant Growth: +3/+3");

    let life = engine.state().players[0].life;
    sacrifice_lions(&mut engine, p0, lions);
    assert!(is_tapped(&engine, valley), "{{T}} is part of the cost");
    assert!(
        in_graveyard(&engine, p0, savannah_lions()).is_some(),
        "the creature was sacrificed"
    );
    assert_eq!(
        engine.state().players[0].life,
        life + 4,
        "life equal to the toughness it last had (4), not printed 1 or power 5"
    );
}

/// A +1/+1 counter counts: the Lions are a 3/2 when sacrificed.
#[test]
fn diamond_valley_counts_a_plus_one_counter_in_the_toughness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[diamond_valley(), savannah_lions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions are out");
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::replacement::put_counters(state, lions, baylee_cards_dsl::CounterKind::P1P1, 1);
    // The counter was written behind the engine's back; the next turn's
    // refresh projects it.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    assert_eq!(pt(&engine, lions), (3, 2), "a +1/+1 counter");

    let life = engine.state().players[0].life;
    sacrifice_lions(&mut engine, p0, lions);
    assert_eq!(engine.state().players[0].life, life + 2);
}

/// A static anthem raises the toughness the sacrificed creature last had.
#[test]
fn diamond_valley_counts_an_anthem_in_the_toughness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[diamond_valley(), glorious_anthem(), savannah_lions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions are out");
    assert_eq!(pt(&engine, lions), (3, 2), "Glorious Anthem: +1/+1");

    let life = engine.state().players[0].life;
    sacrifice_lions(&mut engine, p0, lions);
    assert_eq!(engine.state().players[0].life, life + 2);
}

/// Only creatures its controller controls are on offer, and the land itself
/// is no creature.
#[test]
fn diamond_valley_offers_only_your_own_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[diamond_valley(), savannah_lions(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions");

    activate(&mut engine, p0, diamond_valley(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("which creature to sacrifice, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 2, "my two creatures: {options:?}");
    assert!(options.contains(&lions));
    assert!(!options.contains(&theirs), "never the opponent's creature");
}

/// Without a creature to sacrifice the ability is not offered.
#[test]
fn diamond_valley_is_not_offered_without_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[diamond_valley()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let valley = on_battlefield(&engine, p0, diamond_valley()).expect("the Valley");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority expected")
    };
    assert!(!legal.abilities.contains(&(valley, 0)));
}

#[track_caller]
fn sacrifice_lions(engine: &mut Engine<RegistryLookup>, p0: PlayerId, lions: ObjectId) {
    activate(engine, p0, diamond_valley(), 0);
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        assert!(options.contains(&lions), "the Lions are on the offer");
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![lions],
                },
            )
            .expect("sacrifice the Lions");
    }
    pass_until(engine, stack_is_empty);
}
