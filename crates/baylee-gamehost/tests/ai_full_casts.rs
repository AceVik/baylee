//! The house paying as the client, the seat and the trained AI's menu pay
//! (`AIProfile::FULL_CASTS`, generation 10), on tables a real engine dealt:
//! Phyrexian life (CR 107.4f), restricted mana the engine names (CR 106.6),
//! and cards cast from exile or a graveyard at the price the engine names
//! (an Adventure's creature, CR 716.3d; a foretold card, CR 702.143a; a
//! disturb face, CR 702.146a). Each board is played by the level at
//! generation 9 as well, which leaves the card where it is: the gap the
//! generation closes (`docs/observed-faults.md` 70, 72 and 73).

use baylee_ai::AIProfile;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_gamehost::RegistryLookup;

use crate::house_table::{Table, preset, resolve};

const BEFORE: u16 = AIProfile::BOUNDED_SEARCH;

/// Every fair level, at generation 10 and at generation 9.
fn levels() -> impl Iterator<Item = (&'static str, AIProfile, AIProfile)> {
    AIProfile::NAMED
        .into_iter()
        .filter(|(_, profile)| !profile.peeks())
        .map(|(name, profile)| (name, profile, profile.at_generation(BEFORE)))
}

/// Dismember (`{1}{B/P}{B/P}`) at twenty life beside two Mountains: each
/// Phyrexian symbol is paid with 2 life (CR 107.4f) and a Mountain pays the
/// `{1}`. Read as black only, it was never cast (observed fault 72).
#[test]
fn a_phyrexian_symbol_is_paid_with_life_where_no_mana_pays_it() {
    for (name, now, before) in levels() {
        let mut table = Table::open(
            "Mountain",
            &["Dismember"],
            &["Mountain", "Mountain"],
            &["Grizzly Bears"],
        );
        let dismember = table.card("Dismember");
        assert!(
            !table.house_casts(before, dismember),
            "{name} at generation 9 never casts it"
        );
        let mut table = Table::open(
            "Mountain",
            &["Dismember"],
            &["Mountain", "Mountain"],
            &["Grizzly Bears"],
        );
        assert!(table.house_casts(now, dismember), "{name}");
        assert_eq!(table.life(), 16, "{name}: four life for two symbols");
    }
}

/// The same Dismember beside a Mountain and two Swamps is paid with mana:
/// the planner reaches for life only where no mana pays, and the question
/// whether to pay a symbol with life is answered so too, with the black it
/// tapped floating.
#[test]
fn a_phyrexian_symbol_mana_pays_is_paid_with_mana() {
    for (name, now, _) in levels() {
        let mut table = Table::open(
            "Swamp",
            &["Dismember"],
            &["Mountain", "Swamp", "Swamp"],
            &["Grizzly Bears"],
        );
        let dismember = table.card("Dismember");
        assert!(table.house_casts(now, dismember), "{name}");
        assert_eq!(table.life(), 20, "{name}: no life paid");
    }
}

/// Not at six life: a symbol paid with life never leaves the seat below
/// what the answer to "pay 2 life?" keeps.
#[test]
fn a_phyrexian_symbol_is_not_paid_with_the_last_life() {
    let mut preset = preset(
        "Mountain",
        &["Dismember"],
        &["Mountain", "Mountain"],
        &["Grizzly Bears"],
    );
    preset.seats[0].starting_life = Some(9);
    let engine = Engine::new(&preset, RegistryLookup).expect("the duel starts");
    let mut table = Table { engine };
    table.play_to_my_main(1);
    let dismember = table.card("Dismember");
    assert!(
        !table.house_casts(AIProfile::EXPERT, dismember),
        "nine life less four is below the floor"
    );
}

/// Two Ancient Ziggurats ("Spend this mana only to cast a creature spell")
/// pay a Grizzly Bears' `{1}{G}` between them: the engine names both for the
/// Bears (CR 106.6). One restricted tap a plan, they never did.
#[test]
fn every_restricted_tap_the_engine_names_pays() {
    for (name, now, before) in levels() {
        let mut table = Table::open(
            "Mountain",
            &["Grizzly Bears"],
            &["Ancient Ziggurat", "Ancient Ziggurat"],
            &[],
        );
        let bears = table.card("Grizzly Bears");
        assert!(
            !table.house_casts(before, bears),
            "{name} at generation 9 taps one Ziggurat a plan"
        );
        let mut table = Table::open(
            "Mountain",
            &["Grizzly Bears"],
            &["Ancient Ziggurat", "Ancient Ziggurat"],
            &[],
        );
        assert!(table.house_casts(now, bears), "{name}");
    }
}

/// Doomskar foretold on turn 1, at seat 0's main phase of turn 3 with three
/// Plains.
fn foretold() -> (Table, ObjectId) {
    let mut table = Table::open(
        "Plains",
        &["Doomskar"],
        &["Plains", "Plains", "Plains"],
        &["Grizzly Bears", "Grizzly Bears"],
    );
    let doomskar = table.card("Doomskar");
    for plains in table.mine("Plains").into_iter().take(2) {
        table.apply(PlayerAction::ActivateManaAbility { source: plains });
    }
    table.apply(PlayerAction::Foretell { card: doomskar });
    table.play_to_my_main(3);
    let foretold = table.card("Doomskar");
    (table, foretold)
}

/// A foretold Doomskar is cast from exile for its foretell cost `{1}{W}{W}`
/// with three Plains, its printed `{3}{W}{W}` beyond them (CR 702.143a): the
/// house taps for it at the price the engine names for the way.
#[test]
fn a_foretold_card_is_tapped_for() {
    let (mut before, card) = foretold();
    assert!(
        !before.house_casts(AIProfile::EXPERT.at_generation(BEFORE), card),
        "at generation 9 a foretold card waits for its mana to float"
    );
    let (mut table, card) = foretold();
    assert!(table.house_casts(AIProfile::EXPERT, card));
}

/// Malevolent Hermit discarded to Faithless Looting, with three Islands
/// left: Benevolent Geist, its disturb face, costs `{2}{U}` from the
/// graveyard. The library is Islands, so the Looting's flashback (`{2}{R}`)
/// is out of reach and floats nothing the Geist could be cast with.
fn disturbed() -> (Table, ObjectId) {
    let mut table = Table::open(
        "Island",
        &["Faithless Looting", "Malevolent Hermit"],
        &["Mountain", "Island", "Island", "Island"],
        &[],
    );
    let looting = table.card("Faithless Looting");
    let hermit = table.card("Malevolent Hermit");
    let mountain = table.mine("Mountain")[0];
    table.apply(PlayerAction::ActivateManaAbility { source: mountain });
    table.apply(PlayerAction::CastSpell { card: looting });
    resolve(&mut table, &|table, pending| {
        let (options, count): (Vec<ObjectId>, u8) = match pending {
            Pending::DiscardChoice { count, .. } => {
                (table.view().hand.iter().map(|c| c.id).collect(), *count)
            }
            Pending::ChooseCards { options, min, .. } => (options.clone(), *min),
            _ => return None,
        };
        let mut objects = vec![hermit];
        objects.extend(options.into_iter().filter(|id| *id != hermit));
        objects.truncate(usize::from(count));
        Some(PlayerAction::ChooseObjects { objects })
    });
    let geist = table.card("Malevolent Hermit");
    (table, geist)
}

/// A disturb face in the graveyard is tapped for (CR 702.146a).
#[test]
fn a_disturb_face_is_tapped_for() {
    let (mut before, card) = disturbed();
    assert!(
        !before.house_casts(AIProfile::EXPERT.at_generation(BEFORE), card),
        "at generation 9 a disturb face waits for its mana to float"
    );
    let (mut table, card) = disturbed();
    assert!(table.house_casts(AIProfile::EXPERT, card));
}

/// Brazen Borrower's Petty Theft cast on turn 1, and seat 0's main phase of
/// turn 3 with three Islands: the Borrower waits in exile for `{1}{U}{U}`.
fn adventuring() -> (Table, ObjectId) {
    let mut table = Table::open(
        "Island",
        &["Brazen Borrower"],
        &["Island", "Island", "Island"],
        &["Grizzly Bears"],
    );
    let borrower = table.card("Brazen Borrower");
    for island in table.mine("Island").into_iter().take(2) {
        table.apply(PlayerAction::ActivateManaAbility { source: island });
    }
    table.apply(PlayerAction::CastSpell { card: borrower });
    resolve(&mut table, &|_, pending| match pending {
        Pending::ChooseCastMode { options, .. } => options
            .iter()
            .position(|o| matches!(o.kind, baylee_engine::choice::CastModeKind::Face(_)))
            .map(PlayerAction::ChooseMode),
        Pending::ChooseTargets { options, .. } => Some(PlayerAction::ChooseTargets {
            objects: options.first().copied().into_iter().collect(),
            players: vec![],
        }),
        _ => None,
    });
    table.play_to_my_main(3);
    let exiled = table.card("Brazen Borrower");
    assert!(
        table.view().exile.iter().flatten().any(|o| o.id == exiled),
        "the Borrower is on an adventure"
    );
    (table, exiled)
}

/// An Adventure's creature in exile is tapped for (CR 716.3d).
#[test]
fn an_adventurer_in_exile_is_tapped_for() {
    let (mut before, card) = adventuring();
    assert!(
        !before.house_casts(AIProfile::EXPERT.at_generation(BEFORE), card),
        "at generation 9 an adventurer waits for its mana to float"
    );
    let (mut table, card) = adventuring();
    assert!(table.house_casts(AIProfile::EXPERT, card));
}
