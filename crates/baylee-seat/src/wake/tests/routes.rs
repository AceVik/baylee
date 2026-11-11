//! A card cast from a graveyard or from exile, reached at the price the
//! engine names for the way it is cast there (`LegalActions::cast_ways`):
//! a disturb face's cost (CR 702.146a), a foretell cost (CR 702.143a).
//!
//! The orders priced a graveyard card the engine offered at its front's mana
//! cost and an exiled one at its printed cost, so a Benevolent Geist was
//! offered beside two Islands that do not pay its `{2}{U}`, and a foretold
//! Doomskar was never offered beside the three Plains that pay `{1}{W}{W}`.

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::{DeckEntry, GamePreset, SeatController};
use baylee_engine::choice::{CastModeKind, LegalActions, Pending, PlayerAction, timeout_answer};
use baylee_gamehost::Session;
use baylee_view::{Phase, PlayerView};

use super::super::raised::Way;
use super::super::reachable;
use super::played::reach_of;

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

/// A table where seat 0 starts over `mine`, holding `hand`, both libraries
/// `library`.
fn table(library: &str, mine: &[&str], hand: &[&str]) -> Session {
    let mut preset: GamePreset =
        baylee_cards::decks::probe_preset(41, entry(library).card).unwrap();
    preset.house_rules.starting_player = Some(0);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Human { user_id: 0 };
        seat.deck = (0..40).map(|_| entry(library)).collect();
        seat.starting_hand = Some(Vec::new());
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    preset.seats[0].starting_battlefield = mine.iter().map(|name| entry(name)).collect();
    Session::new(&preset).unwrap()
}

/// Plays on, every question answered by `answer` or else its timeout answer,
/// until seat 0 holds priority with nothing on the stack in its first main
/// phase of turn `turn` or later.
fn play_to(
    session: &mut Session,
    turn: u32,
    answer: &mut dyn FnMut(&Pending) -> Option<PlayerAction>,
) -> (PlayerView, LegalActions) {
    for _ in 0..400 {
        let pending = session.pending().clone();
        let player = pending.asked().expect("the table stays playable");
        if let Pending::Priority { player, .. } = pending
            && player == ME
        {
            let view = session.observe_view(ME);
            if view.phase == Phase::FirstMain
                && view.active == ME
                && view.turn >= turn
                && view.stack.is_empty()
            {
                let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
                let Pending::Priority { legal, .. } = pending else {
                    unreachable!("seat 0 holds priority");
                };
                return (view, *legal);
            }
        }
        let action = answer(&pending)
            .or_else(|| timeout_answer(&pending))
            .unwrap_or_else(|| panic!("no answer to {pending:?}"));
        session.act_slice(player, action, 0).unwrap();
    }
    panic!("seat 0's main phase of turn {turn} was not reached");
}

fn id_of(view: &PlayerView, name: &str) -> ObjectId {
    view.hand
        .iter()
        .find(|card| card.name == name)
        .map(|card| card.id)
        .or_else(|| {
            view.battlefield
                .iter()
                .find(|o| o.controller == ME && o.name.as_ref() == name)
                .map(|o| o.id)
        })
        .unwrap_or_else(|| panic!("{name} is not in seat 0's hand or on its battlefield"))
}

/// Malevolent Hermit discarded to Faithless Looting: two Islands do not pay
/// Benevolent Geist's disturb `{2}{U}`, three do, and the reach names the
/// face the engine casts.
#[test]
fn a_disturb_card_is_reached_at_its_disturb_cost() {
    for (islands, reached) in [(2, false), (3, true)] {
        let mut mine = vec!["Mountain"];
        mine.extend(std::iter::repeat_n("Island", islands));
        let mut session = table(
            "Mountain",
            &mine,
            &["Faithless Looting", "Malevolent Hermit"],
        );
        let (view, _) = play_to(&mut session, 1, &mut |_| None);
        let hermit = id_of(&view, "Malevolent Hermit");
        let looting = id_of(&view, "Faithless Looting");
        let mountain = id_of(&view, "Mountain");
        session
            .act_slice(
                ME,
                PlayerAction::ActivateManaAbility { source: mountain },
                0,
            )
            .unwrap();
        session
            .act_slice(ME, PlayerAction::CastSpell { card: looting }, 0)
            .unwrap();
        let (view, legal) = play_to(&mut session, 1, &mut |pending| match pending {
            Pending::ChooseCards { options, min, .. } if options.contains(&hermit) => {
                Some(PlayerAction::ChooseObjects {
                    objects: std::iter::once(hermit)
                        .chain(options.iter().copied().filter(|id| *id != hermit))
                        .take(usize::from(*min))
                        .collect(),
                })
            }
            Pending::ChooseCards { options, min, .. } => Some(PlayerAction::ChooseObjects {
                objects: options.iter().copied().take(usize::from(*min)).collect(),
            }),
            _ => None,
        });
        assert!(
            view.graveyards[0].iter().any(|o| o.id == hermit),
            "the Hermit was discarded"
        );
        let reach = reachable(&view, &legal);
        let geist = reach_of(&reach, "Malevolent Hermit");
        assert_eq!(
            geist.is_some(),
            reached,
            "{islands} Islands and Benevolent Geist's {{2}}{{U}}: {reach:?}"
        );
        if let Some(geist) = geist {
            assert_eq!(geist.plan.steps.len(), 3);
            assert_eq!(geist.way, Some(Way::Mode(CastModeKind::Face(1))));
        }
    }
}

/// Doomskar foretold on turn 1 is reached on turn 3 for its foretell cost:
/// three Plains, which do not pay its printed `{3}{W}{W}`.
#[test]
fn a_foretold_card_is_reached_at_its_foretell_cost() {
    let mut session = table("Plains", &["Plains", "Plains", "Plains"], &["Doomskar"]);
    let (view, _) = play_to(&mut session, 1, &mut |_| None);
    let doomskar = id_of(&view, "Doomskar");
    for plains in view
        .battlefield
        .iter()
        .filter(|o| o.controller == ME)
        .take(2)
        .map(|o| o.id)
    {
        session
            .act_slice(ME, PlayerAction::ActivateManaAbility { source: plains }, 0)
            .unwrap();
    }
    session
        .act_slice(ME, PlayerAction::Foretell { card: doomskar }, 0)
        .unwrap();
    let (view, legal) = play_to(&mut session, 3, &mut |_| None);
    let reach = reachable(&view, &legal);
    let foretold = reach_of(&reach, "Doomskar").expect("three Plains pay {1}{W}{W}");
    assert_eq!(foretold.plan.steps.len(), 3);
    assert_eq!(foretold.way, Some(Way::Mode(CastModeKind::Foretell)));
}
