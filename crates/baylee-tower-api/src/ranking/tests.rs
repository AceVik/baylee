use super::*;

fn name(display: &str, tag: u32) -> Name {
    Name {
        display: display.to_owned(),
        tag,
    }
}

fn listed(rank: u64, wins: u8, defeats: u8) -> Standing {
    Standing {
        rank: Some(rank),
        name: Some(name("Urza", 7)),
        wins,
        defeats,
        yours: false,
    }
}

fn board(best: Option<Standing>, leaders: Vec<Standing>) -> Board {
    Board {
        cohort: Uuid::nil(),
        lane: Lane::Echoes,
        listing: Listing {
            revision: 1,
            listed: best.as_ref().is_some_and(|b| b.rank.is_some()),
        },
        best,
        leaders,
    }
}

#[test]
fn a_public_name_is_three_to_sixteen_ascii_characters_that_neither_start_nor_end_in_a_mark() {
    for good in ["abc", "A-b_9", "0123456789abcdef", "a--b"] {
        assert!(name(good, 1).valid(), "{good}");
    }
    for bad in [
        "",
        "ab",
        "0123456789abcdefg",
        "-abc",
        "abc-",
        "_abc",
        "abc_",
        "a b",
        "a\u{0}b",
        "a\nb",
        "<b>x",
        // Three characters, but more than ASCII: a look-alike or an override.
        "\u{430}bc",
        "a\u{202e}c",
    ] {
        assert!(!name(bad, 1).valid(), "{bad:?}");
    }
}

#[test]
fn a_discriminator_is_positive_and_fits_a_signed_column() {
    assert!(!name("Urza", 0).valid());
    assert!(name("Urza", 1).valid());
    assert!(name("Urza", i32::MAX as u32).valid());
    assert!(!name("Urza", i32::MAX as u32 + 1).valid());
    assert!(!name("Urza", u32::MAX).valid());
}

#[test]
fn a_standing_is_eight_wins_under_five_defeats_or_five_defeats_under_eight_wins() {
    for wins in 0..=10 {
        for defeats in 0..=7 {
            let finished = (wins == 8 && defeats <= 4) || (wins <= 7 && defeats == 5);
            assert_eq!(
                board(None, vec![listed(1, wins, defeats)]).valid(),
                finished,
                "{wins} wins, {defeats} defeats"
            );
        }
    }
}

#[test]
fn equal_scores_share_a_rank_and_the_next_score_skips_the_shared_ones() {
    let shared = vec![
        listed(1, 8, 0),
        listed(1, 8, 0),
        listed(3, 8, 1),
        listed(3, 8, 1),
        listed(5, 7, 5),
    ];
    assert!(board(None, shared.clone()).valid());
    for (index, rank) in [(0, 2), (1, 2), (2, 2), (3, 4), (4, 3), (4, 4), (4, 6)] {
        let mut wrong = shared.clone();
        wrong[index].rank = Some(rank);
        assert!(!board(None, wrong).valid(), "entry {index} ranked {rank}");
    }
}

#[test]
fn leaders_run_from_most_wins_to_fewest_and_then_from_fewest_defeats() {
    assert!(board(None, vec![listed(1, 8, 4), listed(2, 7, 5)]).valid());
    assert!(!board(None, vec![listed(1, 7, 5), listed(2, 8, 4)]).valid());
    assert!(!board(None, vec![listed(1, 8, 1), listed(2, 8, 0)]).valid());
}

#[test]
fn a_podium_holds_at_most_twenty_five() {
    let shared = |n: usize| board(None, vec![listed(1, 8, 0); n]);
    assert!(shared(0).valid());
    assert!(shared(LEADERS).valid());
    assert!(!shared(LEADERS + 1).valid());
}

#[test]
fn a_leader_is_never_anonymous_unranked_or_badly_named() {
    let with = |change: fn(&mut Standing)| {
        let mut entry = listed(1, 8, 0);
        change(&mut entry);
        board(None, vec![entry])
    };
    assert!(with(|_| {}).valid());
    assert!(!with(|s| s.name = None).valid());
    assert!(!with(|s| s.rank = None).valid());
    assert!(!with(|s| s.rank = Some(0)).valid());
    assert!(
        !with(|s| {
            s.name = None;
            s.rank = None;
        })
        .valid()
    );
    assert!(!with(|s| s.name = Some(name("<script>", 7))).valid());
}

#[test]
fn a_private_best_has_no_rank_and_no_name_and_a_listed_one_has_both() {
    let private = Standing {
        rank: None,
        name: None,
        yours: true,
        ..listed(1, 8, 0)
    };
    let public = Standing {
        yours: true,
        ..listed(90, 2, 5)
    };
    assert!(board(Some(private.clone()), vec![]).valid());
    assert!(board(Some(public.clone()), vec![]).valid());

    // Withdrawn, but the board still publishes who it was.
    let mut leaked = board(Some(public.clone()), vec![]);
    leaked.listing.listed = false;
    assert!(!leaked.valid());
    // Listed, but the best is told as a private one.
    let mut unlisted = board(Some(private.clone()), vec![]);
    unlisted.listing.listed = true;
    assert!(!unlisted.valid());
    // Half of each: the name alone identifies a withdrawn player.
    let mut named = private;
    named.name = Some(name("Urza", 7));
    assert!(!board(Some(named), vec![]).valid());
    // Somebody else's run is nobody's personal best.
    let mut foreign = public;
    foreign.yours = false;
    assert!(!board(Some(foreign), vec![]).valid());
}

#[test]
fn the_viewers_own_entry_is_their_best_and_comes_up_once() {
    let mine = Standing {
        yours: true,
        ..listed(1, 8, 0)
    };
    assert!(board(Some(mine.clone()), vec![mine.clone()]).valid());
    assert!(board(Some(mine.clone()), vec![mine.clone(), listed(1, 8, 0)]).valid());
    assert!(!board(Some(mine.clone()), vec![mine.clone(), mine.clone()]).valid());
    assert!(!board(None, vec![mine.clone()]).valid());

    let other = Standing {
        yours: true,
        ..listed(2, 8, 1)
    };
    assert!(!board(Some(other), vec![mine]).valid());
}

#[test]
fn the_wire_takes_no_field_it_does_not_know() {
    let standing = serde_json::to_value(listed(1, 8, 0)).unwrap();
    assert!(serde_json::from_value::<Standing>(standing.clone()).is_ok());
    let mut account = standing;
    account["account"] = "00000000-0000-7000-8000-000000000000".into();
    assert!(serde_json::from_value::<Standing>(account).is_err());
    assert!(
        serde_json::from_str::<Listing>(r#"{"revision":1,"listed":true,"alias":"x"}"#).is_err()
    );
    assert!(serde_json::from_str::<Name>(r#"{"display":"Urza","tag":1,"id":2}"#).is_err());
    assert_eq!(serde_json::to_string(&Lane::House).unwrap(), r#""house""#);
    assert!(serde_json::from_str::<Lane>(r#""mixed""#).is_err());
}
