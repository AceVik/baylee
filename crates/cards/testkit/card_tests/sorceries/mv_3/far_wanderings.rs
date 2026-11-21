//! `cards/sorceries/mv_3/far_wanderings.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn far_wanderings() -> CardIndex {
    card_index("2ef797ef-c05f-4309-85c7-1bd151dac3fa")
}

/// Far Wanderings (2ef797ef-c05f-4309-85c7-1bd151dac3fa) — {2}{G} Sorcery:
/// Without threshold (<7 graveyard cards), fetches 1 basic land tapped; with
/// threshold (>=7 graveyard cards), fetches up to 3 basic lands tapped.
#[test]
fn far_wanderings_fetches_one_tapped_basic_or_up_to_three_with_threshold() {
    let mut base = lk::game(
        SEED,
        &[forest(), forest(), forest()],
        &[],
        &[far_wanderings(), badlands()],
        &[],
    );
    let nonbasic = hand_to_library_top(&mut base, P0, badlands());
    seed_graveyard(&mut base, P0, 6);

    cast_from_hand(&mut base, P0, far_wanderings());
    pass_until(&mut base, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = base.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!((min, max), (0, 1));
    assert!(!options.contains(&nonbasic), "nonbasic lands excluded");
    let found = options[0];
    base.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![found],
        },
    )
    .unwrap();
    lk::settle(&mut base);
    assert_eq!(lk::zone_of(&base, found), Zone::Battlefield);
    assert!(lk::tapped(&base, found));

    // With 7 cards in graveyard, Threshold offers up to 3 basic lands tapped!
    let mut thresh = lk::game(
        SEED,
        &[forest(), forest(), forest()],
        &[],
        &[far_wanderings()],
        &[],
    );
    seed_graveyard(&mut thresh, P0, 7);

    cast_from_hand(&mut thresh, P0, far_wanderings());
    let mut fetched = Vec::new();
    for _ in 0..4 {
        if stack_is_empty(&thresh) {
            break;
        }
        match thresh.pending().clone() {
            Pending::Priority { player, .. } => {
                thresh.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards { options, max, .. } => {
                let take = usize::from(max).min(3 - fetched.len());
                let picks: Vec<ObjectId> = options.into_iter().take(take).collect();
                fetched.extend_from_slice(&picks);
                thresh
                    .apply(P0, PlayerAction::ChooseObjects { objects: picks })
                    .unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    lk::settle(&mut thresh);
    assert_eq!(fetched.len(), 3);
    for land in fetched {
        assert_eq!(lk::zone_of(&thresh, land), Zone::Battlefield);
        assert!(lk::tapped(&thresh, land));
    }
}
