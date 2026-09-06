//! Commander (CR 903): the command zone, casting out of it, and the cards
//! that ask a question about a commander.
//!
//! The through-line is that commander-ness belongs to the *card* and not to
//! the zone it is sitting in. Every reader in the engine got that backwards
//! in the same way — it looked in the command zone — and every one of them
//! was therefore wrong at exactly the moment it was asked, because the
//! questions ("do you control your commander", "what colour is it") are
//! only ever interesting once the commander has left that zone.

use super::testkit::*;
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::mana::ManaColor;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
/// Katara, the Fearless — {G}{W}{U}, a legendary creature: three lands
/// pay for her, and her colour identity is three colours rather than one,
/// so a reader that answered "colourless" is caught rather than flattered.
fn katara() -> CardIndex {
    card_index("0972d46e-423b-454e-87c7-a2d40fb6fb6d")
}
fn arcane_signet() -> CardIndex {
    card_index("0bc7f093-bef0-4f1a-852c-4b75ebf54838")
}
fn flawless_maneuver() -> CardIndex {
    card_index("4e183439-17d2-47ff-9d99-5e22821d91e3")
}

fn tap_all_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
}

/// True once `seat` has `card` on the battlefield *and* holds priority
/// again.
///
/// Both halves matter: `pass_until` stops the moment its predicate holds,
/// and a test that then reads `LegalActions` off a pending belonging to the
/// other seat is reading the wrong player's options.
fn resolved_and_back_to(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    on_battlefield(engine, seat, card).is_some()
        && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
}

/// The one commander in the command zone, by id.
#[track_caller]
fn commander_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> ObjectId {
    let zone = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Command(seat));
    assert_eq!(zone.len(), 1, "exactly one card in the command zone");
    zone[0]
}

/// Setup: a commander starts in the command zone, and nowhere else. The
/// second half is the one worth asserting — a commander shuffled into the
/// library would be a 101st card that also happens to be drawable.
#[test]
fn a_commander_starts_in_the_command_zone_and_never_in_the_library() {
    let p0 = PlayerId::new(0);
    let engine = Duel::new(3, forest()).commander(0, &[katara()]).start();

    let id = commander_of(&engine, p0);
    let marked = &engine.state().commanders[0];
    assert_eq!(marked.len(), 1, "one marked commander");
    assert_eq!(
        marked[0].object, id,
        "the marker names the card in the zone"
    );
    assert_eq!(marked[0].casts, 0, "it has not been cast yet");

    let state = engine.state();
    for zone in [
        crate::zone::ZoneLocation::Library(p0),
        crate::zone::ZoneLocation::Hand(p0),
    ] {
        assert!(
            state
                .zones
                .list(zone)
                .iter()
                .filter_map(|id| state.object(*id))
                .all(|o| o.card.is_none_or(|c| c.index != katara())),
            "the commander is not in {zone:?}"
        );
    }
}

/// Casting from the command zone (CR 903.8), and the two counters it
/// moves: the seat's (Commander's Insight) and the commander's own (the
/// tax). Nothing offered a command-zone card before this existed, so the
/// commander was a card you could look at and never play.
#[test]
fn a_commander_can_be_cast_out_of_the_command_zone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4, forest())
        .commander(0, &[katara()])
        .battlefield(0, &[forest(), plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = commander_of(&engine, p0);
    // The mana first: `can_cast` filters by what the pool covers, so an
    // untapped board offers nothing and would prove nothing either way.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "the commander is offered from the command zone"
    );

    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));

    assert_eq!(
        engine.state().commander_casts[0],
        1,
        "the seat's count, which Commander's Insight reads"
    );
    assert_eq!(
        engine.state().commanders[0][0].casts,
        1,
        "and the commander's own count, which the tax reads"
    );
}

/// Arcane Signet: "Add one mana of any color in your commander's color
/// identity." The identity is a property of the commander card wherever it
/// is (CR 903.4) — and reading the command *zone* meant the Signet went
/// colourless the instant the commander was cast, which is the only state
/// of the game a Commander deck spends any time in.
#[test]
fn arcane_signet_still_reads_the_commander_once_it_has_been_cast() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(5, forest())
        .commander(0, &[katara()])
        .battlefield(0, &[forest(), plains(), island(), arcane_signet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = commander_of(&engine, p0);
    let signet = on_battlefield(&engine, p0, arcane_signet()).expect("signet deployed");

    // `tap_all_mana` takes only `legal.mana_abilities`, which is the CR
    // 305.6 intrinsic-land shortcut — the Signet's printed ability is an
    // ordinary activated one and is untouched by it.
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));

    // Katara is on the battlefield now, and the command zone is empty.
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Command(p0))
            .is_empty(),
        "the commander left the zone the old lookup was reading"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == signet)
        .expect("the Signet offers its mana ability");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "expected a colour choice, got {:?} — a Signet offering no choice \
             is one that found no commander",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue, ManaColor::Green],
        "Katara's colour identity, in colour order"
    );
}

/// Flawless Maneuver: "If you control a commander, you may cast this spell
/// without paying its mana cost." The condition read the command zone, so
/// it was false in every game ever played — the card has been a {2}{W}
/// instant with a line of text nothing consulted.
#[test]
fn flawless_maneuver_is_free_only_once_the_commander_is_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(6, forest())
        .commander(0, &[katara()])
        .battlefield(0, &[forest(), plains(), island()])
        .hand(0, &[flawless_maneuver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let state = engine.state();
    let maneuver = state
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|id| {
            state
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == flawless_maneuver())
        })
        .expect("the maneuver is in hand");

    // Nothing tapped yet, so {2}{W} is out of reach and the free clause is
    // the only other reading — and its condition is false while the
    // commander is still in the command zone.
    let card = commander_of(&engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&maneuver),
        "no mana and no commander on the battlefield: nothing pays for it"
    );

    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));

    // The pool is empty — the three lands paid for Katara — so the only
    // reading under which this is castable is the free one.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&maneuver),
        "with the commander out it is free"
    );
}
