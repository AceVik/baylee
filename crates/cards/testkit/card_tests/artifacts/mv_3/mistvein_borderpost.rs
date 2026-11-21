//! `cards/artifacts/mv_3/mistvein_borderpost.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn mistvein_borderpost() -> CardIndex {
    card_index("1499af11-274f-4f8d-b0b6-20e1693ebab7")
}

fn strip_mine() -> CardIndex {
    card_index("d21a89eb-7c5b-459a-acc7-12b20b13bf79")
}

/// Mistvein Borderpost (1499af11-274f-4f8d-b0b6-20e1693ebab7) — {1}{U}{B} Artifact:
/// "You may pay {1} and return a basic land you control to its owner's hand
/// rather than pay this spell's mana cost."
/// "This artifact enters tapped."
/// "{T}: Add {U} or {B}."
#[test]
fn mistvein_borderpost_alternative_cost_enters_tapped_and_taps_for_u_or_b() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[mistvein_borderpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, island()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();

    let post_card = in_hand(&engine, p0, mistvein_borderpost()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: post_card })
        .expect("cast Mistvein Borderpost via alternative cost");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let alt = choose_cast_kind(&engine, CastModeKind::Alternative(0));
        engine.apply(p0, PlayerAction::ChooseMode(alt)).unwrap();
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("return tapped basic Island to hand");
    pass_until(&mut engine, stack_is_empty);

    let post = on_battlefield(&engine, p0, mistvein_borderpost()).unwrap();
    assert!(
        is_tapped(&engine, post),
        "Mistvein Borderpost enters tapped"
    );
    assert!(in_hand(&engine, p0, island()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // Next p0 turn: tap for {U}.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: post,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );

    // Following p0 turn: tap for {B}.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: post,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
}

/// Normal {1}{U}{B} cast leaves lands on board and still enters tapped;
/// nonbasic lands cannot pay the alternative return cost.
#[test]
fn mistvein_borderpost_normal_cast_and_nonbasic_rejected_for_alt_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[strip_mine()])
        .hand(0, &[mistvein_borderpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let post_card = in_hand(&engine, p0, mistvein_borderpost()).unwrap();
    assert!(!priority_offer(&engine).castable.contains(&post_card));

    let mut engine2 = Duel::new(SEED, island())
        .battlefield(0, &[island(), swamp(), strip_mine()])
        .hand(0, &[mistvein_borderpost()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);
    tap_all_mana(&mut engine2, p0);
    let pc = in_hand(&engine2, p0, mistvein_borderpost()).unwrap();
    engine2
        .apply(p0, PlayerAction::CastSpell { card: pc })
        .unwrap();
    let normal = choose_cast_kind(&engine2, CastModeKind::Normal);
    engine2.apply(p0, PlayerAction::ChooseMode(normal)).unwrap();
    pass_until(&mut engine2, stack_is_empty);

    let post = on_battlefield(&engine2, p0, mistvein_borderpost()).unwrap();
    assert!(is_tapped(&engine2, post));
    assert!(on_battlefield(&engine2, p0, island()).is_some());
    assert!(on_battlefield(&engine2, p0, swamp()).is_some());
}
