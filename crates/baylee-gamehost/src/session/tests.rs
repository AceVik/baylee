use super::*;
use baylee_engine::zone::ZoneLocation;
use baylee_view::{GameStatic, LogEvent, PolicyAnswer, SeatSetting};

mod chairs;
mod clocks;
mod frames;
mod holds;
mod log;
mod mulligans;
mod printings;
mod refusals;
mod scouting;
mod shared_hand;

/// The awaited seat's clock answer, as a one-seat table asks it.
fn timeout(session: &Session) -> Option<(PlayerId, PlayerAction)> {
    let seat = session.awaiting_seat()?;
    Some((seat, session.timeout_action(seat)?))
}

/// The house's answer for the awaited seat, as a one-seat table asks it.
fn house(session: &Session) -> Option<(PlayerId, PlayerAction)> {
    let seat = session.awaiting_seat()?;
    Some((seat, session.house_action(seat)?))
}

/// `seat`'s clock running out on the question it owes now.
fn clock_answers(
    session: &mut Session,
    seat: PlayerId,
) -> Option<Result<Vec<(PlayerId, Envelope)>, String>> {
    let asked_at = session.asked_at(seat)?;
    session.answer_by_clock(seat, asked_at)
}
use baylee_core::ids::PrintRef;
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, HouseRules, PrintInfo, SeatSpec,
};
use baylee_view::LogEntry;

fn island() -> CardIndex {
    baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
        .unwrap()
        .index
}

fn test_preset() -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60)
        .map(|_| DeckEntry {
            card: island(),
            print: PrintRef::new(0),
        })
        .collect();
    let mk = |ai: bool| SeatSpec {
        controller: if ai {
            SeatController::Ai(AIProfile::default())
        } else {
            SeatController::Open
        },
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed: 7,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(false), mk(true)],
    }
}

fn agent() -> HeuristicAgent {
    HeuristicAgent::new(AIProfile::default())
}

/// A distinct number per seat kind, and the guard the table below needs:
/// all three predicates are `matches!`, so a fifth kind answers `false`
/// to every one of them and is silently a seat nobody sends anything to.
/// This match is exhaustive.
fn kind_index(kind: &SeatKind) -> usize {
    match kind {
        SeatKind::Human => 0,
        SeatKind::Ai(_) => 1,
        SeatKind::Driven(_) => 2,
        SeatKind::StandIn(_) => 3,
    }
}

pub(crate) fn teamed_preset(teams: [Option<u8>; 4]) -> GamePreset {
    let mut preset = test_preset();
    let seat = preset.seats[0].clone();
    preset.seats = teams
        .iter()
        .map(|team| SeatSpec {
            team: *team,
            ..seat.clone()
        })
        .collect();
    preset
}

fn forest() -> CardIndex {
    baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
        .expect("Forest is in the registry")
        .index
}

/// Two seats with nothing in common: seat 0 plays Islands, seat 1 plays
/// Forests and starts one on the battlefield, so it is visible at once.
fn split_preset() -> GamePreset {
    let deck = |card: CardIndex, print: u16| -> Vec<DeckEntry> {
        (0..60)
            .map(|_| DeckEntry {
                card,
                print: PrintRef::new(print),
            })
            .collect()
    };
    let mk = |card, print, battlefield: Vec<DeckEntry>| SeatSpec {
        controller: SeatController::Open,
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck(card, print),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: battlefield,
        emblems: vec![],
        team: None,
    };
    let print = |n: u128| PrintInfo {
        scryfall_id: uuid::Uuid::from_u128(n),
        lang: "EN".into(),
        finish: Finish::Normal,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed: 7,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![print(1), print(2)],
        seats: vec![
            mk(island(), 0, vec![]),
            mk(
                forest(),
                1,
                vec![DeckEntry {
                    card: forest(),
                    print: PrintRef::new(1),
                }],
            ),
        ],
    }
}

/// The view a seat is sent, decoded back out of its envelope.
pub(crate) fn seat_view(session: &Session, seat: PlayerId) -> baylee_view::PlayerView {
    session
        .snapshot(seat)
        .into_iter()
        .find_map(|e| match e.msg {
            Some(v1::envelope::Msg::StateDelta(delta)) => {
                Some(serde_json::from_slice(&delta.view_json).expect("the view decodes"))
            }
            _ => None,
        })
        .expect("every snapshot carries a view")
}

/// The same, out of what an action actually *sent* the seat.
///
/// The difference matters for anything a client has to be told: `snapshot`
/// is read-only and builds a fresh view every time it is asked, so it
/// would report a field as delivered even if the live path — `act`, then
/// `pump` — never produced a frame at all.
pub(crate) fn routed_view(
    routed: &[(PlayerId, Envelope)],
    seat: PlayerId,
) -> baylee_view::PlayerView {
    routed
        .iter()
        .filter(|(s, _)| *s == seat)
        .find_map(|(_, e)| match &e.msg {
            Some(v1::envelope::Msg::StateDelta(delta)) => {
                Some(serde_json::from_slice(&delta.view_json).expect("the view decodes"))
            }
            _ => None,
        })
        .expect("a pump sends every human seat its own view")
}

/// Plays a couple of steps so the session has a sequence number to be
/// behind or current on.
fn started_session() -> (Session, PlayerId) {
    let mut session = Session::new(&test_preset()).expect("session builds");
    let human = session.human_seats()[0];
    let _ = session.pump();
    let _ = session.act(human, PlayerAction::MulliganKeep);
    assert!(session.seq() > 0, "the game moved");
    (session, human)
}

/// Answers for every seat but `seat` until `seat` is asked, as the
/// players at those seats would: with the house's legal answer, through
/// the socket door, so none of them is marked as answered by the house.
fn until_asked(session: &mut Session, seat: PlayerId) {
    for _ in 0..200 {
        let asked = session.awaiting_seat().expect("the game goes on");
        if asked == seat {
            return;
        }
        let (player, action) = timeout(session).expect("a question is out");
        session.act(player, action).expect("a legal answer");
    }
    panic!("seat {seat:?} was never asked");
}

/// Two human seats, both on a 30-second decision clock.
fn two_humans() -> GamePreset {
    let mut preset = test_preset();
    preset.seats[1].controller = SeatController::Open;
    preset.house_rules.decision_timeout_secs = 30;
    preset
}

/// Every question `routed` asks, by seat.
fn asked(routed: &[(PlayerId, Envelope)]) -> Vec<(u8, Pending)> {
    routed
        .iter()
        .filter_map(|(seat, env)| match &env.msg {
            Some(v1::envelope::Msg::ChoiceRequest(req)) => Some((
                seat.get(),
                serde_json::from_slice(&req.pending_json).expect("the question decodes"),
            )),
            _ => None,
        })
        .collect()
}

// ---- teammates' hands (#265) -------------------------------------------

/// A four-chair table past its opening hands: `teams` per seat, and `ai`
/// the chairs the house plays. Every player keeps.
pub(crate) fn a_kept_table(teams: [Option<u8>; 4], ai: [bool; 4]) -> Session {
    a_kept_table_from(teamed_preset(teams), ai)
}

/// [`a_kept_table`] from a preset a test has already changed.
pub(crate) fn a_kept_table_from(mut preset: GamePreset, ai: [bool; 4]) -> Session {
    for (spec, &house) in preset.seats.iter_mut().zip(&ai) {
        if house {
            spec.controller = SeatController::Ai(AIProfile::default());
        }
    }
    let mut session = Session::new(&preset).expect("a four-seat game");
    session.pump();
    for (seat, &house) in ai.iter().enumerate() {
        if !house {
            session
                .act(PlayerId::new(seat_byte(seat)), PlayerAction::MulliganKeep)
                .expect("a keep");
        }
    }
    assert!(session.deciding().is_empty(), "every seat has kept");
    session
}

/// `seat` shows its hand to exactly `to`.
pub(crate) fn show_to(
    session: &mut Session,
    seat: u8,
    to: &[u8],
) -> Result<Vec<(PlayerId, Envelope)>, String> {
    let to = to.iter().map(|&s| PlayerId::new(s)).collect();
    session.seat_setting(PlayerId::new(seat), SeatSetting::ShareHand(to))
}

/// `seat` asks `owner` to be shown their hand.
pub(crate) fn ask(
    session: &mut Session,
    seat: u8,
    owner: u8,
) -> Result<Vec<(PlayerId, Envelope)>, String> {
    session.seat_setting(
        PlayerId::new(seat),
        SeatSetting::RequestHand(PlayerId::new(owner)),
    )
}

/// Whose hands a view shows.
pub(crate) fn hands_shown(view: &PlayerView) -> Vec<PlayerId> {
    view.shared_hands.iter().map(|h| h.player).collect()
}

fn seats(of: &[u8]) -> SeatSet {
    of.iter().map(|&s| PlayerId::new(s)).collect()
}
