//! `cards/creatures/mv_2/badgermole_cub.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Badgermole Cub: "When this creature enters, earthbend 1." and "Whenever
/// you tap a creature for mana, add an additional {G}."
///
/// The Forest it earthbends is a 1/1 land creature with haste (CR 701.66a),
/// so it taps for mana the turn it was animated, and tapping it is tapping
/// a creature for mana: {G}{G}. The second {G} comes from a triggered mana
/// ability (CR 605.1b), which resolves the moment it triggers (CR 605.4a):
/// both are in the pool when the player next has priority, the stack is
/// empty, and no ability was put on it. A Forest that is only a land adds
/// its one.
#[test]
fn badgermole_cub_earthbends_a_forest_that_then_taps_for_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[badgermole_cub()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let forests = all_on_battlefield(&engine, p0, forest());
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest");
    let (land, plain) = (forests[2], forests[3]);

    earthbend_with_the_cub(&mut engine, [forests[0], forests[1]], land);
    assert!(
        !engine
            .state()
            .object(their_land)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE)),
        "their land was never a choice"
    );
    let t = types(&engine, land);
    assert!(t.contains(TypeSet::LAND) && t.contains(TypeSet::CREATURE));
    assert!(keywords(&engine, land).contains(KeywordSet::HASTE));
    assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, land), (1, 1), "0/0 and one counter");
    assert_eq!(earthbend_watches(&engine), 1, "the delayed trigger waits");

    let green = |engine: &Engine<RegistryLookup>| {
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green)
    };
    assert_eq!(green(&engine), 0, "the Cub took both");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: plain })
        .expect("a Forest taps for {G}");
    assert_eq!(green(&engine), 1, "a land that is no creature adds its one");

    let before = engine.journal().entries().len();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .expect("the earthbent Forest has haste and taps for {G}");
    assert_eq!(
        green(&engine),
        3,
        "its {{G}} and the Cub's additional {{G}}"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "straight back to the player who tapped, got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "a mana ability never uses the stack"
    );
    assert!(
        !engine.journal().entries()[before..]
            .iter()
            .any(|e| matches!(e.event, crate::event::GameEvent::AbilityTriggered { .. })),
        "nothing was put on the stack to resolve later"
    );
}

/// Badgermole Cub's mana ability answers a creature tapped **for mana**
/// (CR 106.12) and nothing else: Llanowar Elves tapped for {G} makes {G}{G},
/// and the Cub and the Elves tapped to attack make nothing.
#[test]
fn badgermole_cub_adds_for_a_creature_tapped_for_mana_and_not_for_an_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[badgermole_cub(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cub = on_battlefield(&engine, p0, badgermole_cub()).expect("the Cub");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves");

    // Its printed "{T}: Add {G}.", activated as the player presses it.
    activate(&mut engine, p0, llanowar_elves(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2,
        "the Elves' {{G}} and the Cub's"
    );

    // Untapped again for the attack, which is the harness setting a board
    // up, not a rule.
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .set_tapped(elves, false);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let before = engine.journal().entries().len();
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (cub, baylee_core::ids::Defender::Player(p1)),
                    (elves, baylee_core::ids::Defender::Player(p1)),
                ],
            },
        )
        .expect("both attack");
    assert!(is_tapped(&engine, cub) && is_tapped(&engine, elves));
    assert!(
        !engine.journal().entries()[before..]
            .iter()
            .any(|e| matches!(e.event, crate::event::GameEvent::ManaProduced { .. })),
        "tapped to attack is not tapped for mana"
    );
}

/// Earthbend's last sentence (CR 701.66a): "When that land dies or is put
/// into exile, return it to the battlefield tapped under your control."
///
/// A Lightning Bolt kills the 1/1 Forest. The delayed trigger goes on the
/// stack and returns it once, tapped, and as the new object it now is
/// (CR 400.7): a land and no creature, without the counter or haste, so it
/// does not die again.
#[test]
fn badgermole_cub_s_land_comes_back_tapped_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[badgermole_cub()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let forests = all_on_battlefield(&engine, p0, forest());
    let land = forests[2];
    earthbend_with_the_cub(&mut engine, [forests[0], forests[1]], land);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    let bolt = in_hand(&engine, p1, lightning_bolt()).expect("the Bolt");
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .expect("a Mountain pays for the Bolt");
    aim_at(&mut engine, p1, land);
    let before = engine.journal().entries().len();
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && moves_of(e, land, Zone::Graveyard, Zone::Battlefield) > 0
    });

    assert!(
        engine.journal().entries()[before..]
            .iter()
            .any(|e| matches!(
                e.event,
                crate::event::GameEvent::AbilityTriggered { ability_index, .. }
                    if ability_index == baylee_core::ids::AbilityRef::SYNTHETIC
            )),
        "a delayed triggered ability, on the stack"
    );
    assert_eq!(
        moves_of(&engine, land, Zone::Battlefield, Zone::Graveyard),
        1
    );
    assert_eq!(
        moves_of(&engine, land, Zone::Graveyard, Zone::Battlefield),
        1
    );
    let back = engine.state().object(land).expect("the Forest");
    assert_eq!(back.zone, Zone::Battlefield);
    assert_eq!(back.controller, p0, "under your control");
    assert!(is_tapped(&engine, land), "tapped");
    let t = types(&engine, land);
    assert!(
        t.contains(TypeSet::LAND) && !t.contains(TypeSet::CREATURE),
        "a land again"
    );
    assert!(!keywords(&engine, land).contains(KeywordSet::HASTE));
    assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 0);
    assert_eq!(earthbend_watches(&engine), 0, "the watch triggered once");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the game goes on, got {:?}",
        engine.pending()
    );
}

/// Earthbend's "or is put into exile": Swords to Plowshares exiles the 1/1
/// Forest, and it comes back tapped from exile.
#[test]
fn badgermole_cub_s_land_comes_back_tapped_when_it_is_exiled() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[badgermole_cub()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let forests = all_on_battlefield(&engine, p0, forest());
    let land = forests[2];
    earthbend_with_the_cub(&mut engine, [forests[0], forests[1]], land);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    let swords = in_hand(&engine, p1, swords_to_plowshares()).expect("the Swords");
    engine
        .apply(p1, PlayerAction::CastSpell { card: swords })
        .expect("a Plains pays for the Swords");
    aim_at(&mut engine, p1, land);
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && moves_of(e, land, Zone::Exile, Zone::Battlefield) > 0
    });

    assert_eq!(moves_of(&engine, land, Zone::Battlefield, Zone::Exile), 1);
    assert_eq!(moves_of(&engine, land, Zone::Exile, Zone::Battlefield), 1);
    assert!(is_tapped(&engine, land), "tapped");
    assert!(!types(&engine, land).contains(TypeSet::CREATURE));
    assert_eq!(earthbend_watches(&engine), 0);
}

/// A land that leaves the battlefield any other way has left for good as
/// far as earthbend knows: Unsummon returns the 1/1 Forest to its owner's
/// hand, it stays there, and the watch is spent (CR 400.7, 603.7b).
#[test]
fn badgermole_cub_s_land_bounced_to_hand_stays_there() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[badgermole_cub()])
        .battlefield(1, &[island()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let forests = all_on_battlefield(&engine, p0, forest());
    let land = forests[2];
    earthbend_with_the_cub(&mut engine, [forests[0], forests[1]], land);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    let bounce = in_hand(&engine, p1, unsummon()).expect("the Unsummon");
    engine
        .apply(p1, PlayerAction::CastSpell { card: bounce })
        .expect("an Island pays for the Unsummon");
    aim_at(&mut engine, p1, land);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(land).map(|o| o.zone),
        Some(Zone::Hand),
        "returned to its owner's hand, and nothing brought it back"
    );
    assert_eq!(earthbend_watches(&engine), 0, "the watch is spent");
}
