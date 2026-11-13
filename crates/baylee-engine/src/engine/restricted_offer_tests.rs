//! What an offer says restricted mana may pay for (CR 106.6):
//! `LegalActions::restricted_taps` and `LegalActions::restricted_floating`.
//!
//! The view says what a land makes and never what its mana may be spent on,
//! so these hold that the engine names it as its own payment reads it: for a
//! mana ability still in the land and for mana already floating, per way of
//! casting (a face being a spell of its own, CR 601.3e), and for the payment
//! a window is open for.

use super::testkit::*;
use super::*;
use crate::choice::{CastModeKind, LegalActions, Spend};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaColor;

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

fn table(board: &[&str], hand: &[&str]) -> Engine<RegistryLookup> {
    let board: Vec<CardIndex> = board.iter().copied().map(named).collect();
    let hand: Vec<CardIndex> = hand.iter().copied().map(named).collect();
    let mut engine = Duel::new(7, named("Mountain"))
        .battlefield(0, &board)
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn on_board_all(engine: &Engine<RegistryLookup>, name: &str) -> Vec<ObjectId> {
    let card = named(name);
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == P0 && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Cavern of Souls played from the hand, naming Elf.
fn cavern_naming_elf(board: &[&str], hand: &[&str]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut hand = hand.to_vec();
    hand.push("Cavern of Souls");
    let mut engine = table(board, &hand);
    let cavern = in_hand(&engine, P0, named("Cavern of Souls")).expect("in hand");
    engine
        .apply(P0, PlayerAction::PlayLand { card: cavern })
        .expect("a land drop");
    engine
        .apply(
            P0,
            PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::ELF),
        )
        .expect("Elf is offered");
    let cavern = on_board_all(&engine, "Cavern of Souls")[0];
    (engine, cavern)
}

/// Ancient Ziggurat: "Spend this mana only to cast a creature spell." Each
/// Ziggurat is named for Grizzly Bears and neither for Lightning Bolt.
#[test]
fn a_creature_only_land_is_named_for_creature_spells_alone() {
    let engine = table(
        &["Ancient Ziggurat", "Ancient Ziggurat"],
        &["Grizzly Bears", "Lightning Bolt"],
    );
    let offer = legal(&engine);
    let bears = in_hand(&engine, P0, named("Grizzly Bears")).expect("in hand");
    let bolt = in_hand(&engine, P0, named("Lightning Bolt")).expect("in hand");
    let ziggurats = on_board_all(&engine, "Ancient Ziggurat");
    assert!(offer.payable.contains(&bears) && offer.payable.contains(&bolt));
    let mut named_for_bears = offer.restricted_taps_for(Spend::Cast(bears, CastModeKind::Normal));
    named_for_bears.sort();
    let mut expected: Vec<(ObjectId, u32)> = ziggurats.iter().map(|id| (*id, 0)).collect();
    expected.sort();
    assert_eq!(named_for_bears, expected);
    assert!(
        offer
            .restricted_taps_for(Spend::Cast(bolt, CastModeKind::Normal))
            .is_empty()
    );
    assert!(offer.restricted_floating.is_empty());
}

/// Cavern of Souls naming Elf: its coloured mana is named for an Elf and
/// not for a Bear, still in the land and floating alike.
#[test]
fn a_chosen_type_land_is_named_for_that_type_alone() {
    let (mut engine, cavern) = cavern_naming_elf(
        &["Forest", "Mountain"],
        &["Elvish Archdruid", "Grizzly Bears"],
    );
    let archdruid = in_hand(&engine, P0, named("Elvish Archdruid")).expect("in hand");
    let bears = in_hand(&engine, P0, named("Grizzly Bears")).expect("in hand");
    let offer = legal(&engine);
    assert_eq!(
        offer.restricted_taps_for(Spend::Cast(archdruid, CastModeKind::Normal)),
        vec![(cavern, 1)]
    );
    assert!(
        offer
            .restricted_taps_for(Spend::Cast(bears, CastModeKind::Normal))
            .is_empty()
    );

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: cavern,
                ability_index: 1,
            },
        )
        .expect("the Cavern's coloured mana");
    if matches!(engine.pending(), Pending::ChooseColor { .. }) {
        engine
            .apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
            .expect("green");
    }
    let offer = legal(&engine);
    let mut green = [0; 6];
    green[ManaColor::Green.index()] = 1;
    assert_eq!(
        offer.restricted_floating_for(Spend::Cast(archdruid, CastModeKind::Normal)),
        green
    );
    assert_eq!(
        offer.restricted_floating_for(Spend::Cast(bears, CastModeKind::Normal)),
        [0; 6]
    );
}

/// The window a payable Elf's cast opens is named too: the Cavern's mana may
/// pay what it owes.
#[test]
fn the_payment_a_window_is_open_for_is_named() {
    let (mut engine, cavern) = cavern_naming_elf(&["Forest", "Mountain"], &["Elvish Archdruid"]);
    let archdruid = in_hand(&engine, P0, named("Elvish Archdruid")).expect("in hand");
    assert!(legal(&engine).payable.contains(&archdruid));
    engine
        .apply(P0, PlayerAction::CastSpell { card: archdruid })
        .expect("cast before its mana");
    assert!(engine.payment_window().is_some(), "a window is open");
    let offer = legal(&engine);
    assert_eq!(offer.restricted_taps_for(Spend::Owed), vec![(cavern, 1)]);
}

/// Brazen Borrower beside a Ziggurat: its creature may be paid with the
/// Ziggurat's mana, Petty Theft, an instant, may not (CR 601.3e).
#[test]
fn a_face_cast_is_asked_as_the_spell_it_is() {
    let mut engine = Duel::new(7, named("Island"))
        .battlefield(
            0,
            &[named("Ancient Ziggurat"), named("Island"), named("Island")],
        )
        .battlefield(1, &[named("Grizzly Bears")])
        .hand(0, &[named("Brazen Borrower")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let borrower = in_hand(&engine, P0, named("Brazen Borrower")).expect("in hand");
    let ziggurat = on_board_all(&engine, "Ancient Ziggurat")[0];
    let offer = legal(&engine);
    assert!(
        offer
            .cast_ways
            .iter()
            .any(|(id, way)| *id == borrower && way.kind == CastModeKind::Face(1)),
        "Petty Theft has a Bear to return: {:?}",
        offer.cast_ways
    );
    assert_eq!(
        offer.restricted_taps_for(Spend::Cast(borrower, CastModeKind::Normal)),
        vec![(ziggurat, 0)]
    );
    assert!(
        offer
            .restricted_taps_for(Spend::cast(borrower, CastModeKind::Face(1)))
            .is_empty(),
        "{:?}",
        offer.restricted_taps
    );
}

/// A board with no restricted mana states nothing.
#[test]
fn an_offer_without_restricted_mana_names_nothing() {
    let engine = table(&["Forest", "Forest"], &["Grizzly Bears"]);
    let offer = legal(&engine);
    assert!(offer.restricted_taps.is_empty() && offer.restricted_floating.is_empty());
}

/// `count` Powerstones and `count` Clues made for P0 behind the engine's
/// back, and the offer asked again; with the Powerstones and the Clues.
fn powerstones_and_clues(count: u32) -> (Engine<RegistryLookup>, Vec<ObjectId>, Vec<ObjectId>) {
    let mut engine = table(&[], &[]);
    let powerstones = crate::resolve::tokens::create_tokens(
        &mut engine.state,
        P0,
        &baylee_cards::tokens::POWERSTONE,
        None,
        count,
    );
    let clues = crate::resolve::tokens::create_tokens(
        &mut engine.state,
        P0,
        &baylee_cards::tokens::CLUE,
        None,
        count,
    );
    engine.state.invalidate_projections();
    engine.refresh_offer();
    (engine, powerstones, clues)
}

/// Every tap and spend `offer` names.
fn entries(offer: &LegalActions) -> usize {
    offer
        .restricted_taps
        .iter()
        .map(|class| class.taps.len() + class.spends.len())
        .sum()
}

/// A board of Powerstones beside Clues: the Powerstones' mana pays every
/// Clue, and the offer says so as one class, as long as the board and not
/// as the board squared, in entries and on the wire alike.
#[test]
fn powerstones_beside_clues_are_named_as_one_class() {
    let small = legal(&powerstones_and_clues(48).0);
    let large = legal(&powerstones_and_clues(96).0);
    for (offer, count) in [(&small, 48), (&large, 96)] {
        assert_eq!(offer.restricted_taps.len(), 1, "one class");
        let class = &offer.restricted_taps[0];
        assert_eq!(class.taps.len(), count, "every Powerstone");
        let clues = class
            .spends
            .iter()
            .filter(|spend| matches!(spend, Spend::Activate(_)))
            .count();
        assert_eq!(clues, count, "every Clue: {:?}", class.spends);
        assert!(entries(offer) <= 2 * count + 8, "{}", entries(offer));
    }
    let bytes = |offer: &LegalActions| baylee_core::wire::encode(&offer.restricted_taps).len();
    assert!(
        bytes(&large) <= 2 * bytes(&small) + 64,
        "twice the board is twice the bytes: {} then {}",
        bytes(&small),
        bytes(&large)
    );
}

/// Past [`RESTRICTED_ENTRIES`] the offer names less and never something
/// else: every tap it keeps is a Powerstone, every spend a Clue it may pay.
#[test]
fn a_board_past_the_backstop_is_cut_and_not_misnamed() {
    let (engine, powerstones, clues) = powerstones_and_clues(2100);
    let offer = legal(&engine);
    assert_eq!(entries(&offer), crate::choice::RESTRICTED_ENTRIES);
    assert!(offer.restricted_floating.len() <= crate::choice::RESTRICTED_ENTRIES);
    let powerstones: std::collections::BTreeSet<ObjectId> = powerstones.into_iter().collect();
    let clues: std::collections::BTreeSet<ObjectId> = clues.into_iter().collect();
    for class in &offer.restricted_taps {
        assert!(class.taps.iter().all(|(id, _)| powerstones.contains(id)));
        assert!(
            class
                .spends
                .iter()
                .all(|spend| matches!(spend, Spend::Activate(id) if clues.contains(id)))
        );
    }
}

/// The opponent's Ziggurat, and the restricted mana floating in the
/// opponent's pool, are none of this seat's business: its offer names its
/// own Ziggurat alone and no floating mana.
#[test]
fn an_opponents_restricted_mana_is_never_named() {
    let mut engine = Duel::new(7, named("Mountain"))
        .battlefield(0, &[named("Ancient Ziggurat")])
        .battlefield(1, &[named("Ancient Ziggurat")])
        .hand(0, &[named("Grizzly Bears")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let p1 = PlayerId::new(1);
    let zig = named("Ancient Ziggurat");
    let ziggurat_of = |engine: &Engine<RegistryLookup>, owner: PlayerId| {
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .iter()
            .copied()
            .find(|id| {
                engine.state().object(*id).is_some_and(|o| {
                    o.controller == owner && o.card.is_some_and(|c| c.index == zig)
                })
            })
            .expect("a Ziggurat")
    };
    let mine = ziggurat_of(&engine, P0);
    let theirs = ziggurat_of(&engine, p1);
    let restriction = engine
        .restricted_mana_ability(P0, mine, 0)
        .expect("the Ziggurat's mana is restricted");
    let provenance = casting::restricted_provenance(&engine.state, p1, theirs, restriction);
    assert!(crate::mana_provenance::deposit(
        &mut engine.state,
        p1,
        ManaColor::Green,
        3,
        false,
        provenance,
    ));
    engine.state.invalidate_projections();
    engine.refresh_offer();
    let bears = in_hand(&engine, P0, named("Grizzly Bears")).expect("in hand");
    let offer = legal(&engine);
    assert_eq!(
        offer.restricted_taps_for(Spend::Cast(bears, CastModeKind::Normal)),
        vec![(mine, 0)]
    );
    assert!(
        offer
            .restricted_taps
            .iter()
            .flat_map(|class| &class.taps)
            .all(|(id, _)| *id == mine),
        "{:?}",
        offer.restricted_taps
    );
    assert!(
        offer.restricted_floating.is_empty(),
        "{:?}",
        offer.restricted_floating
    );
}

/// Three hundred classes of restricted green floating beside three hundred
/// creature spells: asking each class for each spell is past
/// `RESTRICTED_ASKS`, so the offer stops asking there. What it named before
/// is whole and true, and nothing after it is named.
#[test]
fn asking_stops_at_the_backstop_and_names_only_whole_answers() {
    const SPELLS: usize = 300;
    let hand = vec![named("Grizzly Bears"); SPELLS];
    let mut engine = Duel::new(7, named("Mountain"))
        .battlefield(0, &[named("Ancient Ziggurat")])
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let ziggurat = on_board_all(&engine, "Ancient Ziggurat")[0];
    let restriction = engine
        .restricted_mana_ability(P0, ziggurat, 0)
        .expect("the Ziggurat's mana is restricted");
    for subtype in 0..SPELLS {
        // A captured choice the restriction never reads still keeps each
        // unit a class of its own.
        let mut provenance =
            casting::restricted_provenance(&engine.state, P0, ziggurat, restriction);
        provenance.chosen_subtype = Some(baylee_core::ids::SubtypeId::new(
            u16::try_from(subtype).expect("small"),
        ));
        assert!(crate::mana_provenance::deposit(
            &mut engine.state,
            P0,
            ManaColor::Green,
            1,
            false,
            provenance,
        ));
    }
    engine.state.invalidate_projections();
    engine.refresh_offer();
    let offer = legal(&engine);
    let bears: Vec<ObjectId> = offer
        .castable
        .iter()
        .chain(&offer.payable)
        .copied()
        .collect();
    assert_eq!(bears.len(), SPELLS, "every Bear may be cast");
    // Each spell asks the Ziggurat's own class and then every floating one.
    let whole = super::restricted_offer::RESTRICTED_ASKS / (1 + SPELLS);
    assert_eq!(offer.restricted_floating.len(), whole);
    let mut green = [0; 6];
    green[ManaColor::Green.index()] = u32::try_from(SPELLS).expect("small");
    for (spend, by_colour) in &offer.restricted_floating {
        assert!(matches!(spend, Spend::Cast(id, CastModeKind::Normal) if bears.contains(id)));
        assert_eq!(*by_colour, green, "every class, for {spend:?}");
    }
    assert!(entries(&offer) <= 1 + whole + 1, "{}", entries(&offer));
}
