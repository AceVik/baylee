//! Summoning sickness where a player actually meets it: the tap symbol.
//!
//! CR 302.6 has two sentences, and only the second one — "a creature can't
//! attack" — had ever been implemented. The first is about `{T}` and `{Q}`
//! in an activation cost, and without it a mana creature cast on turn three
//! made mana on turn three. The offer said so too, so this was not a client
//! drawing an affordance the engine would refuse; the engine agreed.
//!
//! The second half of the fix is *whose* turn the clock belongs to. One
//! game-wide "the turn began at" woke every creature as soon as anybody
//! untapped, so a creature cast on your own turn was awake through the
//! opponent's. Combat could not see that — attackers are declared on your
//! own turn, where a shared clock and a private one agree — which is why it
//! survived until an activated ability was gated on the same predicate.

use super::testkit::{
    Duel, RegistryLookup, card_index, keep_mulligans, on_battlefield, pass_until, reach_main_phase,
};
use super::*;
use baylee_core::ids::CardIndex;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// `{1}{G}` Creature — Human Druid Ally, "{T}: Add X mana of any one color,
/// where X is the number of Allies you control."
fn harabaz_druid() -> CardIndex {
    card_index("ead985ec-f29f-4a3b-b8b1-061142cc5bd1")
}

/// Whether `card`'s printed abilities are offered to `seat` right now.
#[track_caller]
fn offered(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    let Pending::Priority { player, legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(*player, seat, "somebody else holds priority");
    let id = on_battlefield(engine, seat, card).expect("the permanent is on the battlefield");
    legal.abilities.iter().any(|(source, _)| *source == id) || legal.mana_abilities.contains(&id)
}

/// Casts the druid off two Forests and hands the turn back to seat 0 with
/// the creature on the battlefield.
fn druid_cast_this_turn(seed: u64) -> (Engine<RegistryLookup>, PlayerId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[harabaz_druid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .expect("a Forest taps");
    }
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the druid is cast");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, harabaz_druid()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    (engine, p0)
}

/// CR 302.6, first sentence. The ability is not offered, and it is not
/// accepted either — the enumeration and the validation read one predicate,
/// so a client cannot reach past the list it was given.
#[test]
fn a_mana_creature_cast_this_turn_makes_no_mana() {
    let (mut engine, p0) = druid_cast_this_turn(31);

    assert!(
        !offered(&engine, p0, harabaz_druid()),
        "the druid was offered its tap ability the turn it entered"
    );
    let druid = on_battlefield(&engine, p0, harabaz_druid()).expect("on the battlefield");
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: druid,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the engine accepted an activation it never offered"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the refusal still made mana"
    );
}

/// "Continuously since *their* most recent turn began" — an opponent's
/// untap step is not the controller's turn beginning, so the druid is still
/// asleep all through it, and wakes on seat 0's own next turn.
#[test]
fn the_opponents_turn_does_not_wake_it_and_the_next_own_turn_does() {
    let (mut engine, p0) = druid_cast_this_turn(32);

    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(engine.state().turn.active, PlayerId::new(1));
    assert!(
        !offered(&engine, p0, harabaz_druid()),
        "an opponent untapping woke my creature a turn early"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(engine.state().turn.active, p0);
    assert!(
        offered(&engine, p0, harabaz_druid()),
        "the druid never woke up on its controller's own turn"
    );
}

/// The counter-test the whole fix hangs on: a *land* played this turn taps
/// straight away. CR 302.6 says nothing about lands, and a check written as
/// "did this permanent enter this turn" would have stopped this one.
#[test]
fn a_land_played_this_turn_taps_at_once() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(33, forest()).hand(0, &[forest()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land is played");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is in play");
    assert!(
        legal.mana_abilities.contains(&land),
        "a Forest played this turn was refused its own mana"
    );
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .expect("the Forest taps");
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
}
