//! `cards/creatures/mv_3/thorn_thallid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_cards_dsl::counters::SPORE;

fn thorn_thallid() -> CardIndex {
    card_index("1239d3bf-06e8-4732-b1e2-f6eb145e5894")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn offered(engine: &Engine<RegistryLookup>, card: CardIndex, index: u32) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.iter().any(|(id, ai)| {
        *ai == index
            && engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
    })
}

fn target(engine: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let seat = engine
        .pending()
        .asked()
        .expect("a target choice is pending");
    engine
        .apply(seat, PlayerAction::ChooseTargets { objects, players })
        .expect("a legal target");
}

/// A Thallid at p0's main phase on its third upkeep, with three spore
/// counters, having checked on the way that each own upkeep (and no other)
/// adds one and that two are not enough to pay.
fn thallid_with_three_spores() -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thorn_thallid()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let thallid = on_battlefield(&engine, p0, thorn_thallid()).expect("the Thallid is out");
    assert_eq!(
        counters_on(&engine, thallid, SPORE),
        1,
        "one upkeep of its controller so far"
    );
    assert!(
        !offered(&engine, thorn_thallid(), 1),
        "one counter cannot pay for three"
    );
    for expected in 2u16..=3 {
        reach_their_main_phase(&mut engine, p1);
        assert_eq!(
            counters_on(&engine, thallid, SPORE),
            expected - 1,
            "the opponent's upkeep adds nothing"
        );
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        assert_eq!(counters_on(&engine, thallid, SPORE), expected);
    }
    assert!(
        offered(&engine, thorn_thallid(), 1),
        "three counters pay for it"
    );
    engine
}

/// Thorn Thallid (1239d3bf-06e8-4732-b1e2-f6eb145e5894) — "At the beginning
/// of your upkeep, put a spore counter on this creature. / Remove three spore
/// counters from this creature: It deals 1 damage to any target."
#[test]
fn thorn_thallid_gains_a_spore_each_own_upkeep_and_pings_a_player_for_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = thallid_with_three_spores();
    let thallid = on_battlefield(&engine, p0, thorn_thallid()).expect("the Thallid is out");
    activate(&mut engine, p0, thorn_thallid(), 1);
    target(&mut engine, vec![], vec![p1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19, "1 damage to the player");
    assert_eq!(
        counters_on(&engine, thallid, SPORE),
        0,
        "three counters removed"
    );
    assert!(
        !offered(&engine, thorn_thallid(), 1),
        "no counters, no ability"
    );
}

/// The same ability aimed at a creature: "any target".
#[test]
fn thorn_thallid_pings_a_creature_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = thallid_with_three_spores();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears are out");
    activate(&mut engine, p0, thorn_thallid(), 1);
    target(&mut engine, vec![bears], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).expect("the Bears live").damage,
        1,
        "1 damage marked on the Bears"
    );
    assert_eq!(engine.state().players[1].life, 20, "the player was not hit");
    let thallid = on_battlefield(&engine, p0, thorn_thallid()).expect("the Thallid is out");
    assert_eq!(counters_on(&engine, thallid, SPORE), 0);
}
