//! `cards/creatures/artifacts/mv_5/snow_fortress.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn snow_fortress() -> CardIndex {
    card_index("7709ed84-ed71-4dd2-b002-b257418dd79e")
}

/// Snow Fortress (7709ed84-ed71-4dd2-b002-b257418dd79e) — {5} 0/4 Artifact Creature — Wall:
/// - "Defender"
/// - "{1}: This creature gets +1/+0 until end of turn."
/// - "{1}: This creature gets +0/+1 until end of turn."
/// - "{3}: This creature deals 1 damage to target creature without flying that's attacking you."
#[test]
fn snow_fortress_pumps_and_pings_only_nonflying_creatures_attacking_you() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                snow_fortress(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), wind_drake(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let fortress = on_battlefield(&engine, P0, snow_fortress()).unwrap();
    assert_eq!(pt(&engine, fortress), (0, 4));
    assert!(keywords(&engine, fortress).contains(KeywordSet::DEFENDER));

    // Activate ability 0 (+1/+0) and ability 1 (+0/+1).
    let lands = lands_of(&engine, P0);
    tap_mana_where(&mut engine, P0, |id| id == lands[0]);
    activate(&mut engine, P0, snow_fortress(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, fortress), (1, 4));

    tap_mana_where(&mut engine, P0, |id| id == lands[1]);
    activate(&mut engine, P0, snow_fortress(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, fortress), (1, 5));

    // Advance to P1's combat: P1 attacks P0 with Grizzly Bears (no flying) and Wind Drake (flying),
    // leaving Gray Ogre back (not attacking).
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let drake = on_battlefield(&engine, P1, wind_drake()).unwrap();
    let ogre = on_battlefield(&engine, P1, gray_ogre()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P0)), (drake, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    // P0 activates ability 2 ({3}: deal 1 damage to target creature without flying that's attacking you).
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, snow_fortress(), 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Snow Fortress ability 2");
    };
    assert!(
        options.contains(&bears),
        "non-flying creature attacking you is a legal target"
    );
    assert!(
        !options.contains(&drake),
        "flying creature attacking you is excluded"
    );
    assert!(
        !options.contains(&ogre),
        "non-attacking creature is excluded"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).unwrap().damage,
        1,
        "Snow Fortress dealt 1 damage to the attacking non-flying creature"
    );
}
