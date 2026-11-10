//! `cards/creatures/artifacts/mv_5/iron_man_tony_stark.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn iron_man_tony_stark() -> CardIndex {
    card_index("188e1516-5766-406b-aacb-468f2d9a530a")
}

fn mogg_fanatic() -> CardIndex {
    card_index("7efd5d62-6da2-428a-98e3-5b842f668656")
}

/// Iron Man, Tony Stark — flying; "Attacking creatures you control get
/// +1/+0" (his own attack included, a creature that stays home does not).
#[test]
fn iron_man_tony_stark_pumps_my_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[iron_man_tony_stark(), wild_elephant(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tony = on_battlefield(&engine, p0, iron_man_tony_stark()).expect("seated");
    let elephant = on_battlefield(&engine, p0, wild_elephant()).expect("seated");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("seated");
    assert!(keywords(&engine, tony).contains(KeywordSet::FLYING));
    let (tony_before, elephant_before, ogre_before) =
        (pt(&engine, tony), pt(&engine, elephant), pt(&engine, ogre));
    assert_eq!(tony_before, (3, 3));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (tony, Defender::Player(p1)),
                    (elephant, Defender::Player(p1)),
                ],
            },
        )
        .expect("both attack");
    assert_eq!(pt(&engine, tony), (tony_before.0 + 1, tony_before.1));
    assert_eq!(
        pt(&engine, elephant),
        (elephant_before.0 + 1, elephant_before.1)
    );
    assert_eq!(
        pt(&engine, ogre),
        ogre_before,
        "a creature that stays home is not boosted"
    );
}

/// "Whenever you cast a red spell, create a 2/1 colorless Robot Hero artifact
/// creature token with flying." A red spell does; a colorless one does not.
#[test]
fn iron_man_tony_stark_makes_a_robot_when_i_cast_a_red_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sol_ring(), mogg_fanatic()])
        .battlefield(0, &[iron_man_tony_stark(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mountains = all_on_battlefield(&engine, p0, mountain());

    tap_mana_where(&mut engine, p0, |id| id == mountains[0]);
    cast_with_floating(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert!(tokens_of(&engine, p0).is_empty(), "Sol Ring is colorless");

    tap_mana_where(&mut engine, p0, |id| id == mountains[1]);
    cast_with_floating(&mut engine, p0, mogg_fanatic());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Robot Hero for the red spell");
    let robot = tokens[0];
    assert_eq!(pt(&engine, robot), (2, 1));
    assert!(keywords(&engine, robot).contains(KeywordSet::FLYING));
    assert!(is_artifact_creature(&engine, robot));
}
