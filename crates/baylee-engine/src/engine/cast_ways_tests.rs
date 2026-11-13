//! `LegalActions::cast_ways` for a card cast from anywhere but the hand.
//!
//! A planner taps lands before the engine has priced a cast, so the price it
//! taps for is the one the offer names. For a card in hand that is its mana
//! cost unless the card prints a way of its own; a card in a graveyard or in
//! exile is cast at another price as often as not, and which price is a fact
//! of the game (a printed or granted flashback, CR 702.34a; a disturb face,
//! CR 702.146a; a foretell cost after the turn it was foretold, CR
//! 702.143a). These hold that the offer names each such way at the price the
//! cast charges, and that the price is the one paid.

use super::testkit::*;
use super::*;
use crate::choice::{CastModeKind, LegalActions};
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;

const P0: PlayerId = PlayerId::new(0);

fn named(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"))
}

fn legal(engine: &Engine<RegistryLookup>) -> LegalActions {
    match engine.pending() {
        Pending::Priority { legal, .. } => (**legal).clone(),
        other => panic!("expected priority, got {other:?}"),
    }
}

/// The ways the offer names for `card`, as `(kind, price)`.
fn ways(legal: &LegalActions, card: ObjectId) -> Vec<(CastModeKind, ManaCost)> {
    legal
        .cast_ways
        .iter()
        .filter(|(id, _)| *id == card)
        .map(|(_, way)| (way.kind, way.cost))
        .collect()
}

/// Seat 0 in its first main phase over `board`, holding `hand`, with
/// `buried` put into its graveyard.
fn table(board: &[&str], hand: &[&str], buried: &[&str]) -> Engine<RegistryLookup> {
    let board: Vec<CardIndex> = board.iter().copied().map(named).collect();
    let hand: Vec<CardIndex> = hand
        .iter()
        .copied()
        .chain(buried.iter().copied())
        .map(named)
        .collect();
    let mut engine = Duel::new(7, named("Island"))
        .battlefield(0, &board)
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    for name in buried {
        let card = in_hand(&engine, P0, named(name)).expect("dealt into the hand");
        engine
            .dev_state_mut(P0)
            .expect("the harness sets boards up")
            .move_object(
                card,
                ZoneLocation::Graveyard(P0),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("into the graveyard");
    }
    engine.refresh_offer();
    engine
}

/// Faithless Looting in the graveyard, three Mountains up: payable, and
/// named at its flashback {2}{R}, the price the cast then charges.
#[test]
fn a_flashback_card_is_offered_at_its_flashback_cost() {
    let mut engine = table(
        &["Mountain", "Mountain", "Mountain"],
        &[],
        &["Faithless Looting"],
    );
    let looting = in_graveyard(&engine, P0, named("Faithless Looting")).expect("buried");
    let offer = legal(&engine);
    assert!(offer.payable.contains(&looting), "{offer:?}");
    assert_eq!(
        ways(&offer, looting),
        vec![(CastModeKind::Flashback, ManaCost::parse("{2}{R}"))]
    );
    tap_all_mana(&mut engine, P0);
    engine
        .apply(P0, PlayerAction::CastSpell { card: looting })
        .expect("three Mountains pay the flashback");
    assert!(on_stack(&engine, named("Faithless Looting")).is_some());
}

/// Malevolent Hermit in the graveyard is disturbed as Benevolent Geist for
/// {2}{U}, never for its front's {1}{U}: with two Islands the way is named
/// and the cast it names is not affordable.
#[test]
fn a_disturb_card_is_offered_at_its_back_face_cost() {
    let engine = table(&["Island", "Island"], &[], &["Malevolent Hermit"]);
    let hermit = in_graveyard(&engine, P0, named("Malevolent Hermit")).expect("buried");
    let offer = legal(&engine);
    assert!(offer.payable.contains(&hermit), "{offer:?}");
    assert_eq!(
        ways(&offer, hermit),
        vec![(CastModeKind::Face(1), ManaCost::parse("{2}{U}"))]
    );
}

/// Doomskar foretold on turn 1 is named at its foretell cost {1}{W}{W} on
/// turn 3, and three Plains cast it.
#[test]
fn a_foretold_card_is_offered_at_its_foretell_cost() {
    let mut engine = Duel::new(7, named("Plains"))
        .battlefield(0, &[named("Plains"), named("Plains"), named("Plains")])
        .hand(0, &[named("Doomskar")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let doomskar = in_hand(&engine, P0, named("Doomskar")).expect("in hand");
    let kept = on_battlefield(&engine, P0, named("Plains")).expect("a Plains");
    tap_mana_except(&mut engine, P0, kept);
    engine
        .apply(P0, PlayerAction::Foretell { card: doomskar })
        .expect("{2} pays the foretell");
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == P0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let foretold = engine
        .state()
        .zones
        .list(ZoneLocation::Exile(P0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == named("Doomskar")))
        })
        .expect("foretold");
    let offer = legal(&engine);
    assert!(offer.payable.contains(&foretold), "{offer:?}");
    assert_eq!(
        ways(&offer, foretold),
        vec![(CastModeKind::Foretell, ManaCost::parse("{1}{W}{W}"))]
    );
    tap_all_mana(&mut engine, P0);
    engine
        .apply(P0, PlayerAction::CastSpell { card: foretold })
        .expect("three Plains pay the foretell cost");
    assert!(on_stack(&engine, named("Doomskar")).is_some());
}

/// A card in hand with no way of its own names none: the offer stays as
/// small as it was for the ordinary cast.
#[test]
fn a_plain_card_in_hand_names_no_way() {
    let engine = table(&["Mountain"], &["Lightning Bolt"], &[]);
    let offer = legal(&engine);
    let bolt = in_hand(&engine, P0, named("Lightning Bolt")).expect("in hand");
    assert!(offer.payable.contains(&bolt));
    assert!(offer.cast_ways.is_empty(), "{:?}", offer.cast_ways);
}
