//! `cards/creatures/mv_4/harbor_guardian.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HARBOR_GUARDIAN;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Harbor Guardian (b423bf05-db28-4ed9-a086-9fb0e3f8102f) — {2}{W}{U} 3/4:
/// "Reach. Whenever this creature attacks, defending player may draw a card."
#[test]
fn harbor_guardian_has_reach_and_lets_defending_player_draw_when_attacking() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[HARBOR_GUARDIAN])
        .start();
    keep_mulligans(&mut engine);

    let guardian = on_battlefield(&engine, P0, HARBOR_GUARDIAN).unwrap();
    assert!(
        engine
            .state()
            .object(guardian)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::REACH)
    );
    let before_hand = engine.state().zones.list(ZoneLocation::Hand(P1)).len();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(guardian, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        before_hand + 1
    );
}
