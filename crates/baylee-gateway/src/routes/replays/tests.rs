//! Who may be shown what in a replay: decided from the table and each
//! seat's consent, never from the request.

use super::*;
use baylee_protocol::v1::project_record::Perspective;

const ME: Uuid = Uuid::from_u128(1);
const FRIEND: Uuid = Uuid::from_u128(2);
const STRANGER: Uuid = Uuid::from_u128(3);

fn person(seat: u8, account: Option<Uuid>, shared: bool) -> ListedSeat {
    ListedSeat {
        seat,
        account,
        delegated_by: None,
        seated: Seated::Person,
        name: account.map(|id| (format!("P{}", id.as_u128()), 0xAF03)),
        colours: Some("U".into()),
        commander: None,
        shared,
    }
}

fn unlinked(seat: u8, seated: Seated) -> ListedSeat {
    ListedSeat {
        seated,
        ..person(seat, None, false)
    }
}

/// I at 0 (not shared with others), a friend at 1 who shares, a stranger at
/// 2 who does not, the house at 3 and a model at 4.
fn table() -> Vec<ListedSeat> {
    vec![
        person(0, Some(ME), false),
        person(1, Some(FRIEND), true),
        person(2, Some(STRANGER), false),
        unlinked(3, Seated::House),
        unlinked(4, Seated::Model),
    ]
}

fn ask(perspective: TapePerspective, seat: Option<u8>) -> TapeAsk {
    TapeAsk {
        perspective,
        seat,
        verify: TapeVerify::default(),
    }
}

fn lens(perspective: TapePerspective, seat: Option<u8>) -> Lens {
    Lens::of(&table(), ME, &ask(perspective, seat), 0)
        .ok()
        .expect("a seat of the table")
}

#[test]
fn the_public_view_holds_no_seat_s_cards_whatever_seat_is_asked() {
    for seat in [None, Some(0), Some(2), Some(255)] {
        let lens = lens(TapePerspective::Public, seat);
        assert_eq!(lens.perspective, Perspective::Public);
        assert!(!lens.own, "{seat:?}");
        assert!(lens.hands.is_empty() && lens.others.is_empty());
        assert_eq!(lens.seat, 0);
    }
}

#[test]
fn my_own_seat_is_shown_whole_with_what_it_was_shown() {
    for seat in [None, Some(0)] {
        let lens = lens(TapePerspective::Seat, seat);
        assert_eq!((lens.seat, lens.own), (0, true));
        assert_eq!(lens.others, [1, 2, 3, 4]);
        assert!(lens.hands.is_empty(), "one seat's view lays no hand open");
    }
}

#[test]
fn another_seat_is_shown_only_as_far_as_its_player_shares() {
    // The stranger keeps its cards, so its view is drawn without them, and
    // what it was shown of me stays only because I am asking.
    let stranger = lens(TapePerspective::Seat, Some(2));
    assert_eq!((stranger.seat, stranger.own), (2, false));
    assert_eq!(stranger.others, [0, 1, 3, 4]);
    assert!(stranger.hands.is_empty());

    let friend = lens(TapePerspective::Seat, Some(1));
    assert!(friend.own);
    assert_eq!(friend.others, [0, 3, 4], "nothing of the stranger");
    for ai in [3, 4] {
        assert!(lens(TapePerspective::Seat, Some(ai)).own, "{ai}");
    }
}

#[test]
fn all_hands_lays_open_only_the_hands_that_are_shared() {
    let mine = lens(TapePerspective::AllHands, None);
    assert_eq!(mine.perspective, Perspective::AllHands);
    assert_eq!(mine.hands, [1, 3, 4], "never the stranger's");
    let stranger = lens(TapePerspective::AllHands, Some(2));
    assert!(!stranger.own);
    assert_eq!(stranger.hands, [0, 1, 3, 4]);
    assert_eq!(stranger.others, [0, 1, 3, 4]);
}

#[test]
fn sharing_is_each_seat_s_own_word_and_mine_is_mine_alone() {
    // As the stranger asks, my unshared seat is as closed as theirs was.
    let theirs = Lens::of(
        &table(),
        STRANGER,
        &ask(TapePerspective::AllHands, Some(0)),
        2,
    )
    .ok()
    .expect("a seat of the table");
    assert!(!theirs.own);
    assert_eq!(theirs.hands, [1, 2, 3, 4]);
    assert_eq!(theirs.others, [1, 2, 3, 4]);
}

#[test]
fn a_seat_the_game_does_not_have_is_refused() {
    for perspective in [TapePerspective::Seat, TapePerspective::AllHands] {
        for seat in [5, 255] {
            let refused = Lens::of(&table(), ME, &ask(perspective, Some(seat)), 0).err();
            let (status, _) = refused.expect("no such seat");
            assert_eq!(status, StatusCode::BAD_REQUEST, "{seat}");
        }
    }
}

#[test]
fn a_seat_is_what_the_record_says_and_a_link_fills_in_for_an_old_one() {
    let seen = |seat: &ListedSeat| who(seat, ME);
    assert_eq!(seen(&person(0, Some(ME), false)), "you");
    assert_eq!(seen(&person(0, Some(FRIEND), false)), "player");
    assert_eq!(seen(&person(0, None, true)), "former");
    assert_eq!(seen(&unlinked(0, Seated::House)), "house");
    assert_eq!(seen(&unlinked(0, Seated::Model)), "model");
    let mut old = unlinked(0, Seated::Unknown);
    assert_eq!(seen(&old), "house");
    old.delegated_by = Some(STRANGER);
    assert_eq!(seen(&old), "model");
    // A linked account is a person even where a bridge was noted too.
    old.account = Some(FRIEND);
    assert_eq!(seen(&old), "player");
}

#[test]
fn a_person_s_cards_are_shared_by_consent_and_an_ai_s_always() {
    for (who, consent, want) in [
        ("you", false, false),
        ("player", false, false),
        ("player", true, true),
        ("former", false, false),
        ("former", true, true),
        ("house", false, true),
        ("model", false, true),
    ] {
        let seat = person(0, Some(FRIEND), consent);
        assert_eq!(shared(&seat, who), want, "{who} {consent}");
    }
}

#[test]
fn only_a_linked_person_is_named() {
    let name = |seat: &ListedSeat| name_of(seat, who(seat, ME));
    assert_eq!(
        name(&person(0, Some(FRIEND), false)),
        crate::handle::handle("P2", 0xAF03)
    );
    // A name left on an unlinked seat is not told.
    let mut left = person(0, None, false);
    left.name = Some(("Alice".into(), 1));
    assert_eq!(name(&left), FORMER);
    for (seated, want) in [(Seated::House, HOUSE), (Seated::Model, MODEL)] {
        let mut ai = unlinked(0, seated);
        ai.name = Some(("Alice".into(), 1));
        assert_eq!(name(&ai), want);
    }
    let mut nameless = person(0, Some(FRIEND), false);
    nameless.name = None;
    assert_eq!(name(&nameless), FORMER);
}

fn game() -> SeatedGame {
    SeatedGame {
        game_id: "g".into(),
        started_at: "s".into(),
        ended_at: None,
        complete: true,
        build: Some(baylee_build::short().to_owned()),
        checked: false,
        allow_spectators: None,
        winners: None,
        turn: Some(3),
        end_reason: None,
        bytes: 1,
        seats: table(),
    }
}

#[test]
fn a_result_is_the_asking_seat_s_and_needs_a_finished_record() {
    let mut game = game();
    assert_eq!(result_of(&game, 0), "unknown");
    for (winners, want) in [(vec![], "draw"), (vec![0, 2], "won"), (vec![1], "lost")] {
        game.winners = Some(winners);
        assert_eq!(result_of(&game, 0), want);
    }
    game.winners = Some(vec![0]);
    game.complete = false;
    assert_eq!(result_of(&game, 0), "unfinished");
}

#[test]
fn a_record_is_exact_only_for_the_build_that_played_it() {
    let mut game = game();
    assert_eq!(check_of(&game), "exact");
    game.build = Some("another".into());
    assert_eq!(check_of(&game), "unverified");
    game.checked = true;
    assert_eq!(check_of(&game), "checked");
    game.build = None;
    assert_eq!(check_of(&game), "checked");
}

#[test]
fn a_listed_game_tells_my_colours_and_nobody_else_s() {
    let mut game = game();
    game.winners = Some(vec![2]);
    let listed = game_out(game, STRANGER, true);
    assert_eq!((listed.seat, listed.result), (2, "won"));
    assert!(listed.archived);
    let told: Vec<_> = listed
        .seats
        .iter()
        .map(|s| (s.seat, s.mine, s.colours.is_some(), s.shared))
        .collect();
    assert_eq!(
        told,
        [
            (0, false, false, false),
            (1, false, false, true),
            (2, true, true, false),
            (3, false, false, true),
            (4, false, false, true),
        ]
    );
}

#[test]
fn a_tape_is_asked_for_with_known_fields_only() {
    let read = |json: &str| serde_json::from_str::<TapeAsk>(json);
    let plain = read(r#"{"perspective":"all_hands"}"#).expect("the short form");
    assert!(plain.perspective == TapePerspective::AllHands && plain.seat.is_none());
    assert!(plain.verify == TapeVerify::default());
    for bad in [
        "{}",
        r#"{"perspective":"everything"}"#,
        r#"{"perspective":"seat","own":true}"#,
        r#"{"perspective":"seat","hands":[0,1]}"#,
        r#"{"perspective":"seat","seat":256}"#,
        r#"{"perspective":"seat","seat":-1}"#,
        r#"{"perspective":"seat","verify":"skip"}"#,
    ] {
        assert!(read(bad).is_err(), "{bad}");
    }
}
