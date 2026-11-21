//! `cards/sorceries/mv_3/primal_growth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn primal_growth() -> CardIndex {
    card_index("3cbdf37b-7fe3-4791-b33c-8591158b0ce5")
}

/// Primal Growth (3cbdf37b-7fe3-4791-b33c-8591158b0ce5) — {2}{G} Sorcery:
/// searches your library for a basic land card and puts it onto the battlefield
/// untapped when unkicked.
#[test]
fn primal_growth_fetches_one_untapped_basic_land_unkicked() {
    let mut unkicked = lk::game(
        SEED,
        &[forest(), forest(), forest(), aurochs()],
        &[],
        &[primal_growth(), badlands()],
        &[],
    );
    let nonbasic = hand_to_library_top(&mut unkicked, P0, badlands());
    let beast = lk::obj(&unkicked, P0, aurochs());

    tap_all_mana(&mut unkicked, P0);
    cast_with_floating(&mut unkicked, P0, primal_growth());
    if matches!(unkicked.pending(), Pending::YesNo { .. }) {
        lk::yes_no(&mut unkicked, false);
    } else if matches!(unkicked.pending(), Pending::ChooseCastMode { .. }) {
        let slot = choose_cast_kind(&unkicked, CastModeKind::Normal);
        unkicked.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
    }
    pass_until(&mut unkicked, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = unkicked.pending().clone() else {
        unreachable!();
    };
    assert!(!options.contains(&nonbasic));
    let one = options[0];
    unkicked
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![one] })
        .unwrap();
    lk::settle(&mut unkicked);
    assert_eq!(lk::zone_of(&unkicked, beast), Zone::Battlefield);
    assert_eq!(lk::zone_of(&unkicked, one), Zone::Battlefield);
    assert!(!lk::tapped(&unkicked, one), "enters untapped");
}

/// Primal Growth — Kicker—Sacrifice a creature: searches your library for up to
/// two basic land cards instead and puts them onto the battlefield untapped.
#[test]
fn primal_growth_sacrifices_creature_for_kicker_to_fetch_two_untapped_basics() {
    let mut kicked = lk::game(
        SEED,
        &[forest(), forest(), forest(), aurochs()],
        &[],
        &[primal_growth()],
        &[],
    );
    let sac_beast = lk::obj(&kicked, P0, aurochs());
    tap_all_mana(&mut kicked, P0);
    cast_with_floating(&mut kicked, P0, primal_growth());
    for _ in 0..6 {
        match kicked.pending().clone() {
            Pending::YesNo { .. } => lk::yes_no(&mut kicked, true),
            Pending::ChooseCastMode { .. } => {
                let slot = choose_cast_kind(&kicked, CastModeKind::Kicked);
                kicked.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
            }
            Pending::ChooseCards { options, .. } if options.contains(&sac_beast) => {
                kicked
                    .apply(
                        P0,
                        PlayerAction::ChooseObjects {
                            objects: vec![sac_beast],
                        },
                    )
                    .unwrap();
            }
            _ => break,
        }
    }
    let mut fetched = Vec::new();
    for _ in 0..4 {
        if stack_is_empty(&kicked) {
            break;
        }
        match kicked.pending().clone() {
            Pending::Priority { player, .. } => {
                kicked.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards { options, max, .. } => {
                let take = usize::from(max).min(2 - fetched.len());
                let picks: Vec<ObjectId> = options.into_iter().take(take).collect();
                fetched.extend_from_slice(&picks);
                kicked
                    .apply(P0, PlayerAction::ChooseObjects { objects: picks })
                    .unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    lk::settle(&mut kicked);
    assert_eq!(lk::zone_of(&kicked, sac_beast), Zone::Graveyard);
    assert_eq!(fetched.len(), 2);
    for land in fetched {
        assert_eq!(lk::zone_of(&kicked, land), Zone::Battlefield);
        assert!(!lk::tapped(&kicked, land));
    }
}
