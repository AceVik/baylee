//! `cards/artifacts/mv_3/veinfire_borderpost.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn veinfire_borderpost() -> CardIndex {
    card_index("3f65d1f8-3e46-4e3d-b087-5496a4106e4e")
}

fn strip_mine() -> CardIndex {
    card_index("d21a89eb-7c5b-459a-acc7-12b20b13bf79")
}

/// Veinfire Borderpost (3f65d1f8-3e46-4e3d-b087-5496a4106e4e) — {1}{B}{R} Artifact:
/// "You may pay {1} and return a basic land you control to its owner's hand
/// rather than pay this spell's mana cost."
/// "This artifact enters tapped."
/// "{T}: Add {B} or {R}."
#[test]
fn veinfire_borderpost_alternative_cost_enters_tapped_and_taps_for_b_or_r() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(0, &[veinfire_borderpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, swamp()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();

    let post_card = in_hand(&engine, p0, veinfire_borderpost()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: post_card })
        .expect("cast Veinfire Borderpost via alternative cost");
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
        .expect("return tapped basic Swamp to hand");
    pass_until(&mut engine, stack_is_empty);

    let post = on_battlefield(&engine, p0, veinfire_borderpost()).unwrap();
    assert!(
        is_tapped(&engine, post),
        "Veinfire Borderpost enters tapped"
    );
    assert!(in_hand(&engine, p0, swamp()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // Next p0 turn: tap for {B}.
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

    // Following p0 turn: tap for {R}.
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
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
}

/// Normal {1}{B}{R} cast leaves lands on board and still enters tapped;
/// nonbasic lands cannot pay the alternative return cost.
#[test]
fn veinfire_borderpost_normal_cast_and_nonbasic_rejected_for_alt_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[strip_mine()])
        .hand(0, &[veinfire_borderpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let post_card = in_hand(&engine, p0, veinfire_borderpost()).unwrap();
    assert!(!priority_offer(&engine).castable.contains(&post_card));

    let mut engine2 = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), mountain(), strip_mine()])
        .hand(0, &[veinfire_borderpost()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);
    tap_all_mana(&mut engine2, p0);
    let pc = in_hand(&engine2, p0, veinfire_borderpost()).unwrap();
    engine2
        .apply(p0, PlayerAction::CastSpell { card: pc })
        .unwrap();
    let normal = choose_cast_kind(&engine2, CastModeKind::Normal);
    engine2.apply(p0, PlayerAction::ChooseMode(normal)).unwrap();
    pass_until(&mut engine2, stack_is_empty);

    let post = on_battlefield(&engine2, p0, veinfire_borderpost()).unwrap();
    assert!(is_tapped(&engine2, post));
    assert!(on_battlefield(&engine2, p0, swamp()).is_some());
    assert!(on_battlefield(&engine2, p0, mountain()).is_some());
}
