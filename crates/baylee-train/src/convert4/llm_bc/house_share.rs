//! The corpus rule on who answered a seat's questions (the ladder's clean
//! rule): a language-model game is teacher data only where the model, not
//! the house, answered its seats. Where a seat's mind broke (a session that
//! stopped, malformed answers), the collector still admits the game, since
//! admission is technical only, and the house answers that seat's questions
//! in its place; those answers are the house's, and the game is no example of
//! the model's play. So a game in which the house answered more than
//! [`MAX_HOUSE_SHARE_PERCENT`] of either seat's questions stays out of
//! training, both seats with it, and so does a game that does not say who
//! answered.
//!
//! The counts are the collector's audit of each seat's transcript, kept in
//! the technical result the admission receipt binds
//! (`seats[s].transcript_answer_counts`: `llm` the mind's answers, `house`
//! the house's); the share is house / (house + llm). Standing answers (the
//! rail's own) are neither and count for nothing here.

use serde_json::Value;

use super::schema::Manifest;

/// The most of a seat's questions the house may have answered in a game kept
/// as teacher data, in percent: strictly more refuses the game, exactly this
/// much keeps it.
pub const MAX_HOUSE_SHARE_PERCENT: u64 = 5;

/// Why a game's answer counts keep it out of training, or `None` where every
/// seat's mind answered at least 95 % of what was answered for it.
///
/// `technical` is the game's technical result. A seat without counts is a
/// refusal: who answered it is unknown.
#[must_use]
pub fn refusal(technical: &Value) -> Option<String> {
    let Some(seats) = technical["seats"].as_array().filter(|s| !s.is_empty()) else {
        return Some("the technical result names no seats, so who answered is unknown".into());
    };
    let reasons: Vec<String> = seats
        .iter()
        .enumerate()
        .filter_map(|(at, seat)| {
            let index = seat["seat"]
                .as_u64()
                .unwrap_or_else(|| u64::try_from(at).unwrap_or(u64::MAX));
            let counts = &seat["transcript_answer_counts"];
            let (Some(house), Some(mind)) = (counts["house"].as_u64(), counts["llm"].as_u64())
            else {
                return Some(format!(
                    "seat {index} does not say who answered its questions \
                     (transcript_answer_counts house and llm)"
                ));
            };
            let answered = house.saturating_add(mind);
            (house.saturating_mul(100) > MAX_HOUSE_SHARE_PERCENT.saturating_mul(answered)).then(
                || {
                    #[allow(clippy::cast_precision_loss, reason = "a percentage to read")]
                    let percent = house as f64 * 100.0 / answered as f64;
                    format!(
                        "house answered {house} of {answered} ({percent:.1} %) of seat \
                         {index}'s questions, over {MAX_HOUSE_SHARE_PERCENT} %"
                    )
                },
            )
        })
        .collect();
    (!reasons.is_empty()).then(|| reasons.join("; "))
}

/// [`refusal`] for a frozen game, read from the technical result its
/// manifest pins (hash checked).
///
/// # Errors
/// When the pinned technical result cannot be read or does not match its
/// hash.
pub fn manifest_refusal(manifest: &Manifest) -> anyhow::Result<Option<String>> {
    Ok(refusal(&manifest.technical_result.json()?))
}

#[cfg(test)]
mod tests {
    use super::{MAX_HOUSE_SHARE_PERCENT, refusal};
    use serde_json::{Value, json};

    fn technical(seats: &[(u64, u64)]) -> Value {
        json!({"seats": seats.iter().enumerate().map(|(seat, (llm, house))| json!({
            "seat": seat,
            "transcript_answer_counts": {"llm": llm, "house": house, "standing": 40, "other": 0},
        })).collect::<Vec<_>>()})
    }

    #[test]
    fn a_seat_the_house_answered_more_than_five_percent_of_refuses_its_game() {
        assert_eq!(MAX_HOUSE_SHARE_PERCENT, 5);
        // 6 of 100.
        assert_eq!(
            refusal(&technical(&[(120, 0), (94, 6)])).as_deref(),
            Some("house answered 6 of 100 (6.0 %) of seat 1's questions, over 5 %")
        );
        // Segment 2's attempts 29 and 39, as their receipts count them.
        assert_eq!(
            refusal(&technical(&[(117, 0), (11, 49)])).as_deref(),
            Some("house answered 49 of 60 (81.7 %) of seat 1's questions, over 5 %")
        );
        assert!(
            refusal(&technical(&[(1, 64), (90, 0)]))
                .is_some_and(|reason| reason.contains("64 of 65") && reason.contains("seat 0"))
        );
        // Both seats over: both are named.
        let both = refusal(&technical(&[(1, 9), (2, 8)])).unwrap();
        assert!(both.contains("seat 0") && both.contains("seat 1"), "{both}");
    }

    #[test]
    fn exactly_five_percent_is_kept() {
        assert_eq!(refusal(&technical(&[(95, 5), (19, 1)])), None);
        // A seat nobody answered for (only standing answers) is no house share.
        assert_eq!(refusal(&technical(&[(0, 0), (10, 0)])), None);
    }

    #[test]
    fn a_game_that_does_not_say_who_answered_is_refused() {
        let mut silent = technical(&[(100, 0), (100, 0)]);
        silent["seats"][1]
            .as_object_mut()
            .unwrap()
            .remove("transcript_answer_counts");
        assert_eq!(
            refusal(&silent).as_deref(),
            Some(
                "seat 1 does not say who answered its questions (transcript_answer_counts house \
                 and llm)"
            )
        );
        let mut partial = technical(&[(100, 0), (100, 0)]);
        partial["seats"][0]["transcript_answer_counts"]
            .as_object_mut()
            .unwrap()
            .remove("house");
        assert!(refusal(&partial).is_some_and(|reason| reason.starts_with("seat 0")));
        assert!(refusal(&json!({})).is_some());
        assert!(refusal(&json!({"seats": []})).is_some());
    }

    #[test]
    fn a_clean_game_is_kept() {
        assert_eq!(refusal(&technical(&[(368, 0), (250, 0)])), None);
        assert_eq!(refusal(&technical(&[(200, 3), (150, 7)])), None);
    }
}
