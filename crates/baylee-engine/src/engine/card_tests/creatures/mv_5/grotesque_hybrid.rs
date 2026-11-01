//! `cards/creatures/mv_5/grotesque_hybrid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn grotesque_hybrid_discards_for_flying_and_protection_from_green_and_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("c37e69a7-5596-4dce-a3c9-41f899e05f24");
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &[card])
        .battlefield(1, &[grizzly_bears(), index::SERRA_ANGEL])
        .hand(0, &[swamp()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let hybrid = on_battlefield(&game, p0, card).unwrap();
    let bear = on_battlefield(&game, p1, grizzly_bears()).unwrap();
    let angel = on_battlefield(&game, p1, index::SERRA_ANGEL).unwrap();
    assert_eq!(pt(&game, hybrid), (3, 3));
    let discarded = in_hand(&game, p0, swamp()).unwrap();

    activate(&mut game, p0, card, 1);
    let Pending::ChooseCards { options, .. } = game.pending().clone() else {
        panic!(
            "expected ChooseCards for Discard cost, got {:?}",
            game.pending()
        );
    };
    assert!(options.contains(&discarded));
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discarded],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert!(keywords(&game, hybrid).contains(KeywordSet::FLYING));

    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    game.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(hybrid, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        game.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, hybrid)],
            },
        )
        .is_err(),
        "green Grizzly Bears cannot block flying/pro-green Grotesque Hybrid"
    );
    assert!(
        game.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(angel, hybrid)],
            },
        )
        .is_err(),
        "white Serra Angel cannot block Grotesque Hybrid with protection from white"
    );
}

#[test]
fn grotesque_hybrid_destroys_creature_it_deals_combat_damage_to() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("c37e69a7-5596-4dce-a3c9-41f899e05f24");
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &[card])
        .battlefield(1, &[index::WALL_OF_STONE])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let hybrid = on_battlefield(&game, p0, card).unwrap();
    let wall = on_battlefield(&game, p1, index::WALL_OF_STONE).unwrap();
    game.dev_state_mut(p1)
        .unwrap()
        .object_mut(wall)
        .unwrap()
        .regeneration_shields = 1;
    assert_eq!(pt(&game, wall), (0, 8));

    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    game.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(hybrid, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    game.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, hybrid)],
        },
    )
    .unwrap();
    pass_until(&mut game, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });

    assert!(
        on_battlefield(&game, p1, index::WALL_OF_STONE).is_none(),
        "0/8 Wall of Stone survives 3 combat damage and is destroyed by Grotesque Hybrid's trigger"
    );
    assert!(in_graveyard(&game, p1, index::WALL_OF_STONE).is_some());
}
