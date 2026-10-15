//! `cards/creatures/artifacts/mv_6/darksteel_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn darksteel_sentinel() -> CardIndex {
    card_index("b91c8946-591b-4c0d-a37e-36803df40db7")
}

/// Darksteel Sentinel — flash: in the opponent's upkeep, with the mana
/// floating, it is castable and a creature without flash is not.
#[test]
fn darksteel_sentinel_can_be_cast_at_instant_speed() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[darksteel_sentinel(), gray_ogre()])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    let offer = priority_offer(&engine);
    let sentinel = in_hand(&engine, p0, darksteel_sentinel()).expect("in hand");
    let ogre = in_hand(&engine, p0, gray_ogre()).expect("in hand");
    assert!(offer.castable.contains(&sentinel), "flash: any time");
    assert!(
        !offer.castable.contains(&ogre),
        "a creature without flash is not castable in their upkeep"
    );
    cast_with_floating(&mut engine, p0, darksteel_sentinel());
    pass_until(&mut engine, stack_is_empty);
    let body = on_battlefield(&engine, p0, darksteel_sentinel()).expect("resolved");
    assert_eq!(pt(&engine, body), (3, 3));
    assert!(is_artifact_creature(&engine, body));
}

/// Vigilance and indestructible: attacking leaves it untapped, and a
/// destruction does not kill it.
#[test]
fn darksteel_sentinel_has_vigilance_and_is_indestructible() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[darksteel_sentinel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let sentinel = on_battlefield(&engine, p0, darksteel_sentinel()).expect("seated");
    let kw = keywords(&engine, sentinel);
    assert!(kw.contains(KeywordSet::VIGILANCE) && kw.contains(KeywordSet::INDESTRUCTIBLE));
    attack_alone(&mut engine, p0, sentinel, p1);
    through_combat(&mut engine);
    assert!(!is_tapped(&engine, sentinel), "vigilance");
    assert_eq!(life_of(&engine, 1), 17);
    kill(&mut engine, sentinel);
    assert_eq!(
        engine.state().object(sentinel).map(|o| o.zone),
        Some(Zone::Battlefield),
        "indestructible"
    );
}
