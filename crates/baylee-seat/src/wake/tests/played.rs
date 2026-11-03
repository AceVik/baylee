//! Reach on positions a real engine dealt.
//!
//! The rest of this file's siblings build the view and the offer by hand,
//! which proves the orders read them right and nothing about what the engine
//! puts in them. What a cast costs under a reducer and whether a spell has
//! anything to point at are the engine's to say (`LegalActions`), so these
//! tables are dealt by a `Session` and read exactly as the bridge reads one.

use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, GamePreset, SeatController};
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_view::{Phase, PlayerView};

use super::super::{Reach, reachable};

const ME: PlayerId = PlayerId::new(0);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name)
            .unwrap_or_else(|| panic!("{name} is not in the pool")),
        print: baylee_core::ids::PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

/// Seat 0's first main phase with `mine` on its battlefield and `hand` in its
/// hand, `theirs` across the table, and the view and offer it is asked with.
pub(super) fn position(
    mine: &[&str],
    hand: &[&str],
    theirs: &[&str],
) -> (Session, PlayerView, LegalActions) {
    let mut preset: GamePreset =
        baylee_cards::decks::probe_preset(41, entry("Mountain").card).unwrap();
    preset.house_rules.starting_player = Some(0);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Human { user_id: 0 };
        seat.starting_hand = Some(Vec::new());
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    preset.seats[0].starting_battlefield = mine.iter().map(|name| entry(name)).collect();
    preset.seats[1].starting_battlefield = theirs.iter().map(|name| entry(name)).collect();
    let mut session = Session::new(&preset).unwrap();
    for _ in 0..100 {
        let pending = session.pending().clone();
        let player = pending.asked().expect("the table stays playable");
        if let Pending::Priority { player, .. } = pending
            && player == ME
            && session.observe_view(ME).phase == Phase::FirstMain
        {
            let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
            let Pending::Priority { legal, .. } = pending else {
                unreachable!("seat 0 holds priority");
            };
            return (session, view, *legal);
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected setup question: {other:?}"),
        };
        session.act_slice(player, action, 0).unwrap();
    }
    panic!("seat 0's first main phase was not reached");
}

/// The reach for the card named `name`, if the orders found one.
pub(super) fn reach_of<'a>(reach: &'a [Reach], name: &str) -> Option<&'a Reach> {
    let card = entry(name).card;
    reach.iter().find(|r| r.card.index == card)
}

/// Goblin Warchief: "Goblin spells you cast cost {1} less to cast." The
/// engine charges the Sky Raider `{1}{R}` under it, and the seat priced the
/// printed `{2}{R}`: with two Mountains the Goblin was never offered, and with
/// three it was offered tapping all three, leaving a mana floating that
/// nothing could spend (NEW-03, 3.11.2026; CR 601.2f).
#[test]
fn a_goblin_under_the_warchief_is_reached_for_its_reduced_price() {
    let (_, view, legal) = position(
        &["Goblin Warchief", "Mountain", "Mountain"],
        &["Goblin Sky Raider"],
        &[],
    );
    let reach = reachable(&view, &legal);
    let raider = reach_of(&reach, "Goblin Sky Raider")
        .expect("two Mountains pay the Sky Raider's {1}{R} under the Warchief");
    assert_eq!(raider.plan.steps.len(), 2);

    let (_, view, legal) = position(
        &["Goblin Warchief", "Mountain", "Mountain", "Mountain"],
        &["Goblin Sky Raider"],
        &[],
    );
    let reach = reachable(&view, &legal);
    let raider = reach_of(&reach, "Goblin Sky Raider").expect("three Mountains pay it");
    assert_eq!(
        raider.plan.steps.len(),
        2,
        "the third Mountain is not tapped for a price the engine does not charge"
    );
}

/// A spell that must target and has no legal target cannot be cast
/// (CR 601.2c), and the orders tapped the lands for it anyway: Terror over
/// empty battlefields, Terror at a black creature, Fear (an Aura targets
/// what it will enchant, CR 303.4a) with no creature, Disenchant over a board
/// of creatures, Righteousness with nothing blocking. The engine refused the
/// cast with the mana already floating (NEW-04, 3.11.2026). What a spell may
/// target is the engine's to say: it lists the card in neither `castable`
/// nor `payable`.
#[test]
fn a_spell_with_no_legal_target_is_not_reached_for() {
    for (spell, lands, theirs) in [
        ("Terror", &["Swamp", "Swamp"][..], &[][..]),
        ("Terror", &["Swamp", "Swamp"][..], &["Vampire Bats"][..]),
        ("Fear", &["Swamp", "Swamp"][..], &[][..]),
        (
            "Disenchant",
            &["Plains", "Plains"][..],
            &["Grizzly Bears"][..],
        ),
        // "Target blocking creature": in a main phase nothing blocks.
        ("Righteousness", &["Plains"][..], &["Grizzly Bears"][..]),
    ] {
        let (_, view, legal) = position(lands, &[spell], theirs);
        assert!(
            reach_of(&reachable(&view, &legal), spell).is_none(),
            "{spell} reached for with nothing to target among {theirs:?}"
        );
    }
    // And with a target it is reached for: the engine's answer, not a blanket no.
    let (_, view, legal) = position(&["Swamp", "Swamp"], &["Terror"], &["Grizzly Bears"]);
    let reach = reachable(&view, &legal);
    let terror = reach_of(&reach, "Terror").expect("the Bears are a target");
    assert_eq!(terror.plan.steps.len(), 2);
}

/// Mana that may be spent only to cast a spell pays for no ability, whatever
/// the ability's source is (CR 106.6). The engine refuses it
/// (`mana_provenance::admits`), and the seat planned it: Delighted
/// Halfling's coloured mana ("only to cast a legendary spell") for the
/// legendary Yawgmoth's `{B}{B}` proliferate, and Mishra's Workshop's ("only
/// to cast artifact spells") for Rod of Ruin's `{3}`. The filter named the
/// source, so the plan tapped it, and the payment refused the mana
/// (NEW-05, 3.11.2026).
#[test]
fn mana_only_for_spells_is_never_planned_for_an_ability() {
    let ability_of = |reach: &[super::super::AbilityReach], view: &PlayerView, name: &str| {
        let source = view
            .battlefield
            .iter()
            .find(|o| o.name == name)
            .unwrap_or_else(|| panic!("{name} is on the battlefield"))
            .id;
        reach.iter().any(|r| r.source == source)
    };
    for (mine, source, reached) in [
        (
            &["Yawgmoth, Thran Physician", "Delighted Halfling", "Swamp"][..],
            "Yawgmoth, Thran Physician",
            false,
        ),
        (
            &["Yawgmoth, Thran Physician", "Swamp", "Swamp"][..],
            "Yawgmoth, Thran Physician",
            true,
        ),
        (
            &["Rod of Ruin", "Mishra's Workshop"][..],
            "Rod of Ruin",
            false,
        ),
        (
            &[
                "Rod of Ruin",
                "Mishra's Workshop",
                "Mountain",
                "Mountain",
                "Mountain",
            ][..],
            "Rod of Ruin",
            true,
        ),
    ] {
        // A card in hand, for the proliferate's discard.
        let (_, view, legal) = position(mine, &["Mountain"], &[]);
        let reach = super::super::reachable_abilities(&view, &legal);
        assert_eq!(
            ability_of(&reach, &view, source),
            reached,
            "{source} over {mine:?}"
        );
        if reached {
            let plan = &reach
                .iter()
                .find(|r| view.object(r.source).is_some_and(|o| o.name == source))
                .unwrap()
                .plan;
            for step in &plan.steps {
                let tapped = view.object(step.source).map(|o| o.name.as_str());
                assert!(
                    !matches!(tapped, Some("Mishra's Workshop" | "Delighted Halfling")),
                    "{source}'s plan tapped {tapped:?}"
                );
            }
        }
    }
}

/// Urborg, Tomb of Yawgmoth makes each land a Swamp and Yavimaya, Cradle of
/// Growth each a Forest, so together each is a Swamp Forest that prints no
/// mana ability and taps for `{B}` or `{G}` (CR 305.6). The engine offers the
/// shortcut with a colour to choose; the planner read the shortcut only for a
/// land of one basic type and dropped both, so a Grizzly Bears beside them was
/// never offered (NEW-09, 3.11.2026). The report's Blooming Marsh, made a Swamp
/// by Evil Presence and a Forest by Yavimaya, is the same shape.
#[test]
fn a_land_with_two_basic_types_is_a_source_of_either_colour() {
    let (_, view, legal) = position(
        &["Urborg, Tomb of Yawgmoth", "Yavimaya, Cradle of Growth"],
        &["Grizzly Bears"],
        &[],
    );
    assert_eq!(
        legal.mana_abilities.len(),
        2,
        "the premise: the engine offers both lands' CR 305.6 shortcut"
    );
    let reach = reachable(&view, &legal);
    let bears = reach_of(&reach, "Grizzly Bears").expect("two Swamp Forests pay {1}{G}");
    assert_eq!(bears.plan.steps.len(), 2);
}
