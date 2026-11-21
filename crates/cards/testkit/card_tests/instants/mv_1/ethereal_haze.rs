//! `cards/instants/mv_1/ethereal_haze.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "0cdbd9e7-4941-46ac-99d8-ea227181bdf8"
fn ethereal_haze() -> CardIndex {
    card_index("0cdbd9e7-4941-46ac-99d8-ea227181bdf8")
}

/// Ethereal Haze ({W}): prevents ALL damage dealt by creatures this turn
/// (combat damage, noncombat activated damage, AND damage from a haste creature
/// entering later this turn), does NOT prevent spell damage, and wears off
/// next turn.
#[test]
fn ethereal_haze_prevents_all_creature_damage_but_not_spell_damage() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .hand(0, &[ethereal_haze()])
        .battlefield(
            1,
            &[
                mountain(),
                mountain(),
                index::HILL_GIANT,
                index::PRODIGAL_SORCERER,
            ],
        )
        .hand(1, &[index::RAGING_GOBLIN, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let tim = lk::obj(&engine, P1, index::PRODIGAL_SORCERER);

    // P0 casts Ethereal Haze on P1's main phase.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ethereal_haze());
    pass_until(&mut engine, stack_is_empty);

    // 1. Noncombat creature damage (Prodigal Sorcerer) is prevented.
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[], &[P0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(lk::life(&engine, P0), 20, "creature ping damage prevented");

    // 2. Noncreature spell damage (Lightning Bolt) is NOT prevented.
    let mtns = all_on_battlefield(&engine, P1, mountain());
    tap_mana_where(&mut engine, P1, |id| id == mtns[0]);
    lk::cast_at(&mut engine, P1, lightning_bolt(), &[], &[P0]);
    assert_eq!(lk::life(&engine, P0), 17, "spell damage is not prevented");

    // 3. A haste creature entering AFTER Ethereal Haze resolves AND existing
    //    Hill Giant both have their combat damage prevented.
    tap_mana_where(&mut engine, P1, |id| id == mtns[1]);
    cast_with_floating(&mut engine, P1, index::RAGING_GOBLIN);
    pass_until(&mut engine, stack_is_empty);
    let goblin = lk::obj(&engine, P1, index::RAGING_GOBLIN);

    let _ = lk::attack(&mut engine, &[giant, goblin]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(
        lk::life(&engine, P0),
        17,
        "combat damage from both existing Giant and new Raging Goblin prevented"
    );

    // 4. Next turn prevention wears off.
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    let _ = lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P0), 14, "next turn Hill Giant deals 3");
}
