//! `cards/creatures/mv_4/gustcloak_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Gustcloak Sentinel (`715b4a12-a392-411b-935b-7bd64a270421`) — {2}{W}{W} Creature — Human Soldier 3/3:
/// "Whenever this creature becomes blocked, you may untap it and remove it from combat."
#[test]
fn gustcloak_sentinel_untaps_and_leaves_combat_when_blocked() {
    let sentinel_card = card_index("715b4a12-a392-411b-935b-7bd64a270421");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[sentinel_card])
        .battlefield(1, &[index::AIR_ELEMENTAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sentinel = on_battlefield(&engine, p0, sentinel_card).unwrap();
    let elemental = on_battlefield(&engine, p1, index::AIR_ELEMENTAL).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sentinel, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elemental, sentinel)],
            },
        )
        .unwrap();

    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, sentinel_card).is_some());
    assert!(
        !engine
            .state()
            .object(sentinel)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(engine.state().object(elemental).unwrap().damage, 0);
}
