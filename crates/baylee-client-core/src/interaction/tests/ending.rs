//! The two lines a finished game is shown as: `verdict`, which is read from one chair, and `ending_reason`, which is read from none. Pinned here: the seat that won and the seat that did not never read the same line, a team's win belongs to everyone sitting on it rather than to the seat the engine named, a draw is the one ending that gets no second line, and every verdict starts with a capital because the end screen sets it as a headline. `Interaction` is absent on purpose - a finished game asks nobody anything, so selection, confirmation and the prompt bar are elsewhere.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn the_seat_that_won_and_the_seat_that_did_not_read_different_lines() {
    let result = ended(
        Some(Victor::Player(PlayerId::new(0))),
        EndReason::LastPlayerStanding,
    );
    let mine = verdict(Lang::En, &result, PlayerId::new(0), None);
    let theirs = verdict(Lang::En, &result, PlayerId::new(1), None);
    assert_ne!(mine, theirs);
    assert_eq!(mine, Phrase::YouWon.text(Lang::En));
    assert_eq!(theirs, Phrase::YouLost.text(Lang::En));
}

#[test]
fn a_team_wins_for_everyone_sitting_on_it() {
    let result = ended(Some(Victor::Team(2)), EndReason::LastTeamStanding);
    let ours = verdict(Lang::En, &result, PlayerId::new(3), Some(2));
    let theirs = verdict(Lang::En, &result, PlayerId::new(1), Some(1));
    // The seat that won is on the team, not the one the engine named.
    assert_eq!(ours, Phrase::YourTeamWon.fill(Lang::En, &["2"]));
    assert_eq!(theirs, Phrase::TheirTeamWon.fill(Lang::En, &["2"]));
}

#[test]
fn a_verdict_is_a_headline_and_is_written_like_one() {
    // Every other line the prompt bar shows starts with a capital; these
    // five were the outliers, and the end screen sets them at 44 px.
    for lang in Lang::ALL {
        for line in [
            verdict(lang, &ended(None, EndReason::Draw), me(), None),
            verdict(
                lang,
                &ended(Some(Victor::Player(me())), EndReason::EffectWin),
                me(),
                None,
            ),
            verdict(
                lang,
                &ended(Some(Victor::Player(PlayerId::new(9))), EndReason::EffectWin),
                me(),
                None,
            ),
            verdict(
                lang,
                &ended(Some(Victor::Team(1)), EndReason::LastTeamStanding),
                me(),
                Some(1),
            ),
        ] {
            let first = line.chars().next().expect("a verdict is never empty");
            assert!(first.is_uppercase(), "{lang:?}: {line:?}");
        }
    }
}

#[test]
fn a_draw_is_the_one_ending_that_gets_no_second_line() {
    for lang in Lang::ALL {
        assert_eq!(ending_reason(lang, &ended(None, EndReason::Draw)), None);
        for reason in [
            EndReason::LastPlayerStanding,
            EndReason::LastTeamStanding,
            EndReason::EffectWin,
        ] {
            let line =
                ending_reason(lang, &ended(None, reason)).expect("every other ending says how");
            assert!(!line.is_empty());
        }
    }
}

#[test]
fn the_reason_never_takes_a_side() {
    // The winner and the loser read the same second line, so it may not
    // be written from either chair: one text per reason, per language.
    for lang in Lang::ALL {
        let mut seen: Vec<String> = Vec::new();
        for reason in [
            EndReason::LastPlayerStanding,
            EndReason::LastTeamStanding,
            EndReason::EffectWin,
        ] {
            let line = ending_reason(lang, &ended(Some(Victor::Player(me())), reason))
                .expect("every other ending says how");
            let other = ending_reason(lang, &ended(Some(Victor::Player(PlayerId::new(9))), reason))
                .expect("every other ending says how");
            assert_eq!(line, other, "{lang:?} {reason:?}");
            assert!(!seen.contains(&line), "two reasons share a line: {line:?}");
            seen.push(line);
        }
    }
}

/// Every way out of a game, so a new `LossCause` cannot go unsaid: the
/// `match` below stops compiling when one is added, and the list is what
/// the tests walk.
const CAUSES: [LossCause; 6] = [
    LossCause::Life,
    LossCause::EmptyDraw,
    LossCause::Poison,
    LossCause::CommanderDamage,
    LossCause::Conceded,
    LossCause::Effect,
];

const fn listed(cause: LossCause) -> usize {
    match cause {
        LossCause::Life => 0,
        LossCause::EmptyDraw => 1,
        LossCause::Poison => 2,
        LossCause::CommanderDamage => 3,
        LossCause::Conceded => 4,
        LossCause::Effect => 5,
    }
}

fn seat_out(cause: Option<LossCause>, house: Option<HouseAnswer>) -> SeatView {
    let mut seat = crate::test_support::ViewBuilder::new(1).build().seats[0].clone();
    seat.loss = cause;
    seat.house_answered = house;
    seat
}

/// Each cause says why in its own words, to the seat that lost and about
/// any other, in every language: six different lines, the other seat
/// named in its own.
#[test]
fn every_way_to_lose_is_said_to_the_loser_and_about_them() {
    for lang in Lang::ALL {
        let mut seen: Vec<String> = Vec::new();
        for (at, cause) in CAUSES.into_iter().enumerate() {
            assert_eq!(listed(cause), at);
            let seat = seat_out(Some(cause), None);
            let you = loss_lines(lang, &seat, None);
            let other = loss_lines(lang, &seat, Some("Bob#1a2b"));
            assert_eq!(you.len(), 1, "{lang:?} {cause:?}");
            assert!(!you[0].is_empty());
            assert!(!you[0].contains("Bob#1a2b"));
            assert!(
                other[0].contains("Bob#1a2b"),
                "{lang:?} {cause:?}: {other:?}"
            );
            assert!(!seen.contains(&you[0]), "two causes share a line: {you:?}");
            seen.push(you[0].clone());
        }
    }
}

/// A seat still in the game gets no line, even where the house answered
/// its last decision: that is a seat playing on, not a loss to explain.
#[test]
fn a_seat_still_playing_is_not_explained() {
    for house in [None, Some(HouseAnswer::Clock), Some(HouseAnswer::StandIn)] {
        assert!(loss_lines(Lang::En, &seat_out(None, house), None).is_empty());
    }
}

/// A loss to the clock is the loss and then the clock, exactly where the
/// view records both; a stand-in says the seat was not connected; a loss
/// the player answered for alone is one line.
#[test]
fn a_loss_to_the_clock_says_so() {
    let clocked = loss_lines(
        Lang::En,
        &seat_out(Some(LossCause::Life), Some(HouseAnswer::Clock)),
        None,
    );
    assert_eq!(
        clocked,
        vec![
            Phrase::LostLifeYou.text(Lang::En).to_string(),
            Phrase::HouseClockYou.text(Lang::En).to_string(),
        ]
    );
    let away = loss_lines(
        Lang::De,
        &seat_out(Some(LossCause::Conceded), Some(HouseAnswer::StandIn)),
        Some("Bob#1a2b"),
    );
    assert_eq!(
        away,
        vec![
            "Bob#1a2b hat aufgegeben".to_string(),
            "Bob#1a2b war nicht verbunden, und das Haus traf die letzte Entscheidung".to_string(),
        ]
    );
    assert_eq!(
        loss_lines(Lang::En, &seat_out(Some(LossCause::Poison), None), None).len(),
        1
    );
}

/// The whole table: the reader's own loss first, then every other seat
/// that is out in seat order, named, and nothing for a seat still in.
#[test]
fn the_table_s_losses_start_with_the_reader() {
    let mut view = crate::test_support::ViewBuilder::new(4).build();
    view.seats[0].loss = Some(LossCause::Life);
    view.seats[2].loss = Some(LossCause::Conceded);
    view.seats[2].house_answered = Some(HouseAnswer::StandIn);
    view.seats[3].house_answered = Some(HouseAnswer::Clock);
    let lines = table_losses(Lang::En, &view, None, PlayerId::new(2));
    let seat_0 = seat_name(Lang::En, None, PlayerId::new(0));
    assert_eq!(
        lines,
        vec![
            Phrase::LostConcededYou.text(Lang::En).to_string(),
            Phrase::HouseStandInYou.text(Lang::En).to_string(),
            Phrase::LostLifeOther.fill(Lang::En, &[&seat_0]),
        ]
    );
}
