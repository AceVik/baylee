//! `cards/creatures/mv_2/thalakos_seer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::TERROR;

fn thalakos_seer() -> CardIndex {
    card_index("1baec770-0785-4cbc-be18-83572fd8e9c1")
}

/// `Thalakos Seer` ({U}{U}, 1/1 Creature — Thalakos Wizard,
/// `Coverage::Implemented`):
/// `Shadow`
/// `When this creature leaves the battlefield, draw a card.`
#[test]
fn thalakos_seer_has_shadow_and_draws_a_card_when_it_leaves_the_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[thalakos_seer()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[TERROR])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let seer = on_battlefield(&engine, p0, thalakos_seer()).unwrap();
    assert!(
        engine
            .state()
            .object(seer)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHADOW)
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p1, TERROR);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![seer],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, thalakos_seer()).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}

#[test]
fn leaving_for_the_hand_also_draws_a_card() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = lk::game(SEED, &[thalakos_seer()], &[], &[index::BOOMERANG], &[]);
    let seer = on_battlefield(&e, p0, thalakos_seer()).unwrap();
    let before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    lk::float(&mut e, p0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut e, p0, index::BOOMERANG);
    lk::aim(&mut e, &[seer], &[]);
    lk::settle(&mut e);
    assert!(in_hand(&e, p0, thalakos_seer()).is_some());
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
}
