//! `cards/creatures/mv_3/gustcloak_harrier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Gustcloak Harrier (`7f34f6b4-18ef-40d7-a599-5c29ba468b89`) — {1}{W}{W} Creature — Bird Soldier 2/2:
/// "Flying
/// Whenever this creature becomes blocked, you may untap it and remove it from combat."
#[test]
fn gustcloak_harrier_untaps_and_leaves_combat_when_blocked() {
    let harrier_card = card_index("7f34f6b4-18ef-40d7-a599-5c29ba468b89");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[harrier_card])
        .battlefield(1, &[index::AIR_ELEMENTAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let harrier = on_battlefield(&engine, p0, harrier_card).unwrap();
    let elemental = on_battlefield(&engine, p1, index::AIR_ELEMENTAL).unwrap();
    assert!(keywords(&engine, harrier).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(harrier, Defender::Player(p1))],
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
                blockers: vec![(elemental, harrier)],
            },
        )
        .unwrap();

    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, harrier_card).is_some());
    assert!(
        !engine
            .state()
            .object(harrier)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(engine.state().object(elemental).unwrap().damage, 0);
}
