//! `cards/instants/mv_5/stir_the_pride.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

// oracle_id = "7e46d2c7-59ae-4b0e-a2ec-d0ac1b68d594"
fn stir_the_pride() -> CardIndex {
    card_index("7e46d2c7-59ae-4b0e-a2ec-d0ac1b68d594")
}

/// Stir the Pride ({4}{W}, Entwine {1}{W}): entwined spends {5}{W}{W}, gives
/// your creatures +2/+2 EOT AND grants them a lifegain trigger on ANY damage
/// (combat and noncombat), leaving opponent's creatures untouched and expiring
/// next turn.
#[test]
fn stir_the_pride_entwined_pumps_and_gains_life_on_combat_and_noncombat_damage() {
    let mut engine = table(
        SEED,
        &[stir_the_pride()],
        &[],
        &[index::GRIZZLY_BEARS, index::PRODIGAL_SORCERER],
        &[index::GRIZZLY_BEARS],
    );
    let my_bears = on_battlefield(&engine, me(), index::GRIZZLY_BEARS).unwrap();
    let tim = on_battlefield(&engine, me(), index::PRODIGAL_SORCERER).unwrap();
    let their_bears = on_battlefield(&engine, them(), index::GRIZZLY_BEARS).unwrap();

    lk::float(&mut engine, me(), &[(ManaColor::White, 7)]);
    cast_with_floating(&mut engine, me(), stir_the_pride());
    let both = choose_cast_kind(&engine, CastModeKind::Modes(3));
    engine.apply(me(), PlayerAction::ChooseMode(both)).unwrap();
    settle(&mut engine);

    assert_eq!(
        lk::pool_total(&engine, me()),
        0,
        "all {{5}}{{W}}{{W}} spent"
    );
    assert_eq!(pt(&engine, my_bears), (4, 4));
    assert_eq!(pt(&engine, tim), (3, 3));
    assert_eq!(pt(&engine, their_bears), (2, 2));

    // Noncombat damage from Tim gains 1 life (20 -> 21).
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[], &[them()]);
    settle(&mut engine);
    assert_eq!(life_of(&engine, them()), 19);
    assert_eq!(
        life_of(&engine, me()),
        21,
        "noncombat creature damage gained 1 life"
    );

    // Combat damage from 4/4 Bears gains 4 life (21 -> 25).
    let _ = lk::attack(&mut engine, &[my_bears]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(life_of(&engine, them()), 15);
    assert_eq!(life_of(&engine, me()), 25, "combat damage gained 4 life");

    // Next turn both pump and lifegain trigger have worn off.
    reach_their_main_phase(&mut engine, them());
    reach_their_main_phase(&mut engine, me());
    assert_eq!(pt(&engine, my_bears), (2, 2));
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[], &[them()]);
    settle(&mut engine);
    assert_eq!(
        life_of(&engine, me()),
        25,
        "lifegain trigger wore off at EOT"
    );
}
