//! The house answering a real Snuff Out offer, with a Swamp and without.
//!
//! Snuff Out: "If you control a Swamp, you may pay 4 life rather than pay
//! this spell's mana cost. Destroy target nonblack creature. It can't be
//! regenerated." Its alternative cost is an `AltCondition::If` over a Swamp
//! controlled (CR 118.9). Seat 0 casts it at a table and the house answers
//! every question the cast asks; seat 1's Grizzly Bears is the only target.

use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MOUNTAIN, SNUFF_OUT, SWAMP, VAULT_OF_WHISPERS};
use baylee_engine::choice::CastModeKind;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card,
        print: PrintRef::new(0),
    }
}

/// Seat 0 holds Snuff Out over `mine`; seat 1 has a Grizzly Bears. Both
/// chairs are the test's, so nothing answers but what the test asks.
fn table(mine: &[CardIndex]) -> GamePreset {
    let mut preset = test_preset();
    preset.house_rules.starting_player = Some(0);
    preset.seats[1].controller = SeatController::Open;
    preset.seats[0].starting_hand = Some(vec![entry(SNUFF_OUT)]);
    preset.seats[0].starting_battlefield = mine.iter().copied().map(entry).collect();
    preset.seats[1].starting_battlefield = vec![entry(GRIZZLY_BEARS)];
    preset
}

fn on(session: &Session, zone: ZoneLocation, card: CardIndex) -> bool {
    session.state().zones.list(zone).iter().any(|id| {
        session
            .state()
            .object(*id)
            .and_then(|o| o.card)
            .is_some_and(|c| c.index == card)
    })
}

/// Keeps both hands, passes to seat 0's first main phase, and casts Snuff
/// Out there.
fn cast(preset: &GamePreset) -> Session {
    let mut session = Session::new(preset).expect("the table builds");
    session.pump();
    for seat in [ME, THEM] {
        session
            .act(seat, PlayerAction::MulliganKeep)
            .expect("a keep");
    }
    session.pump();
    for _ in 0..20 {
        if session.state().turn.phase == baylee_engine::turn::Phase::FirstMain
            && matches!(session.pending(), Pending::Priority { player: ME, .. })
        {
            break;
        }
        let seat = session.awaiting_seat().expect("somebody is asked");
        session
            .act(seat, PlayerAction::PassPriority)
            .expect("a pass");
        session.pump();
    }
    let Pending::Priority {
        player: ME, legal, ..
    } = session.pending()
    else {
        panic!("expected priority in the first main phase");
    };
    // The cast-mode offer reads the player's mana pool. Every permanent
    // controlled by this seat is a land with a single mana ability.
    for source in legal.mana_abilities.clone() {
        session
            .act(ME, PlayerAction::ActivateManaAbility { source })
            .expect("tap the land for mana");
    }
    let card = session
        .state()
        .zones
        .list(ZoneLocation::Hand(ME))
        .iter()
        .copied()
        .find(|id| {
            session
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == SNUFF_OUT)
        })
        .expect("Snuff Out in hand");
    session
        .act(ME, PlayerAction::CastSpell { card })
        .expect("Snuff Out is castable");
    session.pump();
    session
}

/// What the cast offered and what the house chose, after the house has
/// answered every question seat 0 is asked until the stack is empty (seat 1
/// passes).
fn house_finishes(session: &mut Session) -> (Vec<CastModeKind>, Option<CastModeKind>) {
    let mut offered = Vec::new();
    let mut chosen = None;
    for _ in 0..30 {
        if session.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(session.pending(), Pending::Priority { .. })
        {
            return (offered, chosen);
        }
        let seat = session.awaiting_seat().expect("somebody is asked");
        let action = if seat == ME {
            let action = session.house_action(ME).expect("the house answers");
            assert!(
                session.pending().answer_fault(&action).is_none(),
                "the house's answer {action:?} to {:?} is legal",
                session.pending()
            );
            if let (Pending::ChooseCastMode { options, .. }, PlayerAction::ChooseMode(at)) =
                (session.pending(), &action)
            {
                offered = options.iter().map(|option| option.kind).collect();
                chosen = options.get(*at).map(|option| option.kind);
            }
            action
        } else {
            PlayerAction::PassPriority
        };
        session.act(seat, action).expect("an answer");
        session.pump();
    }
    panic!("the stack did not empty: {:?}", session.pending());
}

/// A Swamp and three Mountains: both costs are offered, and either answer
/// the house gives destroys the Bears; the 4 life is paid exactly when it
/// chose the alternative.
#[test]
fn with_a_swamp_the_house_answers_the_offer_of_both_costs() {
    let mut session = cast(&table(&[SWAMP, MOUNTAIN, MOUNTAIN, MOUNTAIN]));
    assert!(
        matches!(session.pending(), Pending::ChooseCastMode { .. }),
        "both costs are a question: {:?}",
        session.pending()
    );
    let (offered, chosen) = house_finishes(&mut session);
    assert!(
        offered.contains(&CastModeKind::Normal) && offered.contains(&CastModeKind::Alternative(0)),
        "{offered:?}"
    );
    assert!(on(&session, ZoneLocation::Graveyard(THEM), GRIZZLY_BEARS));
    assert!(on(&session, ZoneLocation::Graveyard(ME), SNUFF_OUT));
    let life = session.state().players[0].life;
    match chosen {
        Some(CastModeKind::Alternative(0)) => assert_eq!(life, 16, "paid 4 life"),
        Some(CastModeKind::Normal) => assert_eq!(life, 20, "paid {{3}}{{B}}"),
        other => panic!("the house chose {other:?}"),
    }
}

/// A Swamp alone pays no {3}{B}: only the 4 life can cast it, and the house
/// casts it that way.
#[test]
fn with_only_a_swamp_the_house_pays_the_life() {
    let mut session = cast(&table(&[SWAMP]));
    let (offered, _) = house_finishes(&mut session);
    assert!(
        !offered.contains(&CastModeKind::Normal),
        "the mana cost is not payable: {offered:?}"
    );
    assert!(on(&session, ZoneLocation::Graveyard(THEM), GRIZZLY_BEARS));
    assert_eq!(session.state().players[0].life, 16);
}

/// No Swamp, and {3}{B} from three Mountains and Vault of Whispers: the
/// alternative is never offered, and the house pays the mana.
#[test]
fn without_a_swamp_the_alternative_is_never_offered() {
    let mut session = cast(&table(&[MOUNTAIN, MOUNTAIN, MOUNTAIN, VAULT_OF_WHISPERS]));
    let (offered, chosen) = house_finishes(&mut session);
    assert!(
        !offered.contains(&CastModeKind::Alternative(0))
            && chosen != Some(CastModeKind::Alternative(0)),
        "{offered:?}"
    );
    assert!(on(&session, ZoneLocation::Graveyard(THEM), GRIZZLY_BEARS));
    assert!(on(&session, ZoneLocation::Graveyard(ME), SNUFF_OUT));
    assert_eq!(session.state().players[0].life, 20);
}
