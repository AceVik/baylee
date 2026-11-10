//! `cards/enchantments/auras/mv_2/agility.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn agility() -> CardIndex {
    card_index("0f5f8b47-d104-4e29-b052-02d730d76ec0")
}

#[test]
fn agility_grants_plus_one_plus_one_and_flanking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[agility()])
        .battlefield(0, &[GRIZZLY_BEARS, mountain(), mountain()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let blocker = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, agility());
    aim_at(&mut engine, p0, attacker);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, attacker), (3, 3));
    assert!(
        engine
            .state()
            .object(attacker)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLANKING)
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
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
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, blocker), (1, 1));
}

const KNIGHT_OF_VALOR: &str = "f2edb543-de33-4eed-892c-98038eec96e2";

#[test]
fn agility_does_not_shrink_a_blocker_that_has_flanking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let knight_card = card_index(KNIGHT_OF_VALOR);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[agility()])
        .battlefield(0, &[GRIZZLY_BEARS, mountain(), mountain()])
        .battlefield(1, &[knight_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let knight = on_battlefield(&engine, p1, knight_card).unwrap();
    assert!(keywords(&engine, knight).contains(KeywordSet::FLANKING));
    cast_from_hand(&mut engine, p0, agility());
    aim_at(&mut engine, p0, attacker);
    pass_until(&mut engine, stack_is_empty);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
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
                blockers: vec![(knight, attacker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, knight),
        (2, 2),
        "a creature with flanking is not given -1/-1"
    );
}
