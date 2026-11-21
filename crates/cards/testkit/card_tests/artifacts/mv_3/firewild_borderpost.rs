//! `cards/artifacts/mv_3/firewild_borderpost.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn firewild_borderpost() -> CardIndex {
    card_index("312a5a75-9b68-4e4b-97fe-3ab644e3e493")
}

fn strip_mine() -> CardIndex {
    card_index("d21a89eb-7c5b-459a-acc7-12b20b13bf79")
}

/// Firewild Borderpost (312a5a75-9b68-4e4b-97fe-3ab644e3e493) — {1}{R}{G} Artifact:
/// "You may pay {1} and return a basic land you control to its owner's hand
/// rather than pay this spell's mana cost."
/// "This artifact enters tapped."
/// "{T}: Add {R} or {G}."
#[test]
fn firewild_borderpost_alternative_cost_enters_tapped_and_taps_for_r_or_g() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[firewild_borderpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, mountain()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();

    let post_card = in_hand(&engine, p0, firewild_borderpost()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: post_card })
        .expect("cast Firewild Borderpost via alternative cost");
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
        .expect("return tapped basic Mountain to hand");
    pass_until(&mut engine, stack_is_empty);

    let post = on_battlefield(&engine, p0, firewild_borderpost()).unwrap();
    assert!(
        is_tapped(&engine, post),
        "Firewild Borderpost enters tapped"
    );
    assert!(in_hand(&engine, p0, mountain()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // Next p0 turn: tap for {R}.
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

    // Following p0 turn: tap for {G}.
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
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}

/// Normal {1}{R}{G} cast leaves lands on board and still enters tapped;
/// nonbasic lands cannot pay the alternative return cost.
#[test]
fn firewild_borderpost_normal_cast_and_nonbasic_rejected_for_alt_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[strip_mine()])
        .hand(0, &[firewild_borderpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let post_card = in_hand(&engine, p0, firewild_borderpost()).unwrap();
    assert!(!priority_offer(&engine).castable.contains(&post_card));

    let mut engine2 = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), forest(), strip_mine()])
        .hand(0, &[firewild_borderpost()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);
    tap_all_mana(&mut engine2, p0);
    let pc = in_hand(&engine2, p0, firewild_borderpost()).unwrap();
    engine2
        .apply(p0, PlayerAction::CastSpell { card: pc })
        .unwrap();
    let normal = choose_cast_kind(&engine2, CastModeKind::Normal);
    engine2.apply(p0, PlayerAction::ChooseMode(normal)).unwrap();
    pass_until(&mut engine2, stack_is_empty);

    let post = on_battlefield(&engine2, p0, firewild_borderpost()).unwrap();
    assert!(is_tapped(&engine2, post));
    assert!(on_battlefield(&engine2, p0, mountain()).is_some());
    assert!(on_battlefield(&engine2, p0, forest()).is_some());
}
