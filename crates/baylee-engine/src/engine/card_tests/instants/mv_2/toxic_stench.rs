//! `cards/instants/mv_2/toxic_stench.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn toxic_stench_shrinks_below_threshold_and_destroys_without_regeneration_at_threshold() {
    let stench = card_index("9c3be33b-0be9-49ec-8160-898cbcab8814");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[stench, stench])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 6);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("Grizzly Bears");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("Gray Ogre");

    // 1. Cast first Toxic Stench with 6 cards in graveyard: -1/-1 until end of turn.
    let swamps = all_on_battlefield(&engine, p0, swamp());
    for &s in &swamps[..2] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    let first_stench = in_hand(&engine, p0, stench).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: first_stench })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (1, 1));
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());

    // 2. First Toxic Stench is now the 7th card in graveyard (threshold);
    // second Toxic Stench destroys Gray Ogre instead, even through a shield.
    engine
        .dev_state_mut(p1)
        .unwrap()
        .object_mut(ogre)
        .unwrap()
        .regeneration_shields = 1;
    for &s in &swamps[2..4] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    let second_stench = in_hand(&engine, p0, stench).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: second_stench,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
}
