//! `cards/creatures/mv_4/doubtless_one.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn doubtless_one_scales_with_clerics_and_gains_life_on_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let doubtless = card_index("23425b11-e40d-4f68-8903-3084ad45319f");
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[doubtless, ondu_cleric()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let avatar = on_battlefield(&engine, p0, doubtless).unwrap();
    // Three Clerics on the battlefield (Doubtless One + two Ondu Clerics) -> 3/3.
    assert_eq!(pt(&engine, avatar), (3, 3));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(avatar, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(engine.state().players[0].life, 23);
}
