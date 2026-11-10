//! `cards/creatures/mv_4/gustcloak_skirmisher.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Gustcloak Skirmisher (`5c9f4fde-b96f-41bf-b214-f83511df1f8e`) — {3}{W} Creature — Bird Soldier 2/3:
/// "Flying
/// Whenever this creature becomes blocked, you may untap it and remove it from combat."
#[test]
fn gustcloak_skirmisher_untaps_and_leaves_combat_when_blocked() {
    let skirmisher_card = card_index("5c9f4fde-b96f-41bf-b214-f83511df1f8e");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[skirmisher_card])
        .battlefield(1, &[index::AIR_ELEMENTAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let skirmisher = on_battlefield(&engine, p0, skirmisher_card).unwrap();
    let elemental = on_battlefield(&engine, p1, index::AIR_ELEMENTAL).unwrap();
    assert!(keywords(&engine, skirmisher).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(skirmisher, Defender::Player(p1))],
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
                blockers: vec![(elemental, skirmisher)],
            },
        )
        .unwrap();

    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, skirmisher_card).is_some());
    assert!(
        !engine
            .state()
            .object(skirmisher)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(engine.state().object(elemental).unwrap().damage, 0);
}
