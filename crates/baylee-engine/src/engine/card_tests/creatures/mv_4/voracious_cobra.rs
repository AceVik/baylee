//! Voracious Cobra — {2}{R}{G} — Creature — Snake
//! Oracle: First strike
//! Oracle: Whenever this creature deals combat damage to a creature, destroy that creature.

use super::*;
use baylee_core::generated::index;

#[test]
fn voracious_cobra_deals_first_strike_combat_damage_and_destroys_damaged_blocker_before_normal_damage()
 {
    let cobra = card_index("5b2e1105-5af0-4f34-bfe9-15a2f6881edd");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cobra])
        .battlefield(1, &[index::SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let cobra_id = on_battlefield(&engine, p0, cobra).expect("Voracious Cobra on board");
    let angel_id = on_battlefield(&engine, p1, index::SERRA_ANGEL).unwrap();
    assert_eq!(pt(&engine, cobra_id), (2, 2));
    assert!(
        engine
            .state()
            .object(cobra_id)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(cobra_id, Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(angel_id, cobra_id)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(
        on_battlefield(&engine, p0, cobra).is_some(),
        "Voracious Cobra survives because Serra Angel is destroyed after first-strike combat damage"
    );
    assert!(on_battlefield(&engine, p1, index::SERRA_ANGEL).is_none());
    assert!(in_graveyard(&engine, p1, index::SERRA_ANGEL).is_some());
}
