//! `cards/artifacts/mv_3/fieldmist_borderpost.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fieldmist_borderpost() -> CardIndex {
    card_index("a292ca5b-2472-4b02-a26b-d1b5b717a34b")
}

fn strip_mine() -> CardIndex {
    card_index("d21a89eb-7c5b-459a-acc7-12b20b13bf79")
}

/// Fieldmist Borderpost (a292ca5b-2472-4b02-a26b-d1b5b717a34b) — {1}{W}{U} Artifact:
/// "You may pay {1} and return a basic land you control to its owner's hand
/// rather than pay this spell's mana cost."
/// "This artifact enters tapped."
/// "{T}: Add {W} or {U}."
///
/// Casts via the alternative cost using the very basic land returned to hand,
/// enters tapped, then taps for {W} and on the next turn for {U}.
#[test]
fn fieldmist_borderpost_alternative_cost_enters_tapped_and_taps_for_w_or_u() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .hand(0, &[fieldmist_borderpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, plains()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();

    let post_card = in_hand(&engine, p0, fieldmist_borderpost()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: post_card })
        .expect("cast Fieldmist Borderpost via alternative cost");
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
        .expect("return tapped basic Plains to hand");
    pass_until(&mut engine, stack_is_empty);

    let post = on_battlefield(&engine, p0, fieldmist_borderpost()).unwrap();
    assert!(
        is_tapped(&engine, post),
        "Fieldmist Borderpost enters tapped"
    );
    assert!(
        in_hand(&engine, p0, plains()).is_some(),
        "Plains returned to hand"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // Next p0 turn: tap for {W}.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, post));
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
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );

    // Following p0 turn: tap for {U}.
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
}

/// Normal {1}{W}{U} cast leaves lands on board and still enters tapped;
/// nonbasic lands cannot pay the alternative return cost.
#[test]
fn fieldmist_borderpost_normal_cast_and_nonbasic_rejected_for_alt_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[strip_mine()])
        .hand(0, &[fieldmist_borderpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let post_card = in_hand(&engine, p0, fieldmist_borderpost()).unwrap();
    assert!(
        !priority_offer(&engine).castable.contains(&post_card),
        "nonbasic Strip Mine cannot pay the 'return a basic land' alternative cost"
    );

    let mut engine2 = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), island(), strip_mine()])
        .hand(0, &[fieldmist_borderpost()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);
    tap_all_mana(&mut engine2, p0);
    let pc = in_hand(&engine2, p0, fieldmist_borderpost()).unwrap();
    engine2
        .apply(p0, PlayerAction::CastSpell { card: pc })
        .unwrap();
    let normal = choose_cast_kind(&engine2, CastModeKind::Normal);
    engine2.apply(p0, PlayerAction::ChooseMode(normal)).unwrap();
    pass_until(&mut engine2, stack_is_empty);

    let post = on_battlefield(&engine2, p0, fieldmist_borderpost()).unwrap();
    assert!(
        is_tapped(&engine2, post),
        "still enters tapped on normal cast"
    );
    assert!(on_battlefield(&engine2, p0, plains()).is_some());
    assert!(on_battlefield(&engine2, p0, island()).is_some());
}
