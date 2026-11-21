//! `cards/artifacts/mv_5/dragon_arch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dragon_arch() -> CardIndex {
    card_index("fdf21a39-7392-49b1-b9c4-ee8122631cab")
}

fn watchwolf() -> CardIndex {
    card_index("a35c2e20-eb90-4132-b65f-be1fcb569819")
}

fn flowstone_charger() -> CardIndex {
    card_index("a19e0a7f-d356-4d2c-8414-70a40ce18674")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn steel_wall() -> CardIndex {
    card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9")
}

fn lightning_helix() -> CardIndex {
    card_index("800c258a-cfc4-4a54-a667-065ea8dea69e")
}

fn fires_of_yavimaya() -> CardIndex {
    card_index("e23d6f3b-0e18-423b-943b-15db7837255b")
}

/// Dragon Arch (fdf21a39-7392-49b1-b9c4-ee8122631cab) — {5} Artifact:
/// "{2}, {T}: You may put a multicolored creature card from your hand onto the
///  battlefield."
///
/// Offers only multicolored creature cards from hand (excluding monocolored
/// creatures, colorless artifact creatures, and multicolored noncreatures),
/// puts the chosen card onto the battlefield untapped, and allows declining.
#[test]
fn dragon_arch_puts_multicolored_creature_only_and_may_decline() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dragon_arch(), forest(), forest()])
        .hand(
            0,
            &[
                watchwolf(),
                flowstone_charger(),
                grizzly_bears(),
                steel_wall(),
                lightning_helix(),
                fires_of_yavimaya(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wolf = in_hand(&engine, p0, watchwolf()).unwrap();
    let charger = in_hand(&engine, p0, flowstone_charger()).unwrap();
    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let wall = in_hand(&engine, p0, steel_wall()).unwrap();
    let helix = in_hand(&engine, p0, lightning_helix()).unwrap();
    let fires = in_hand(&engine, p0, fires_of_yavimaya()).unwrap();

    // 1. Activate Dragon Arch and put Watchwolf onto the battlefield.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, dragon_arch(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 1), "optional single choice");
    assert!(
        options.contains(&wolf) && options.contains(&charger),
        "multicolored creatures in hand are offered: {options:?}"
    );
    assert!(
        !options.contains(&bears)
            && !options.contains(&wall)
            && !options.contains(&helix)
            && !options.contains(&fires),
        "monocolored, colorless, and multicolored noncreature cards excluded: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wolf],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(wolf).unwrap().zone, Zone::Battlefield);
    assert!(!is_tapped(&engine, wolf), "enters untapped");

    // 2. Next turn: activate Dragon Arch again and decline ("you may").
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, dragon_arch(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(charger).unwrap().zone,
        Zone::Hand,
        "declining keeps Flowstone Charger in hand"
    );
}
