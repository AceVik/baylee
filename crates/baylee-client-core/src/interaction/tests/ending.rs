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
