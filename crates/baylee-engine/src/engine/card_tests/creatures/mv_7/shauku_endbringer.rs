//! `cards/creatures/mv_7/shauku_endbringer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shauku_endbringer() -> CardIndex {
    card_index("49478593-29ff-4e5a-b1e8-6462c2d6176d")
}

/// Shauku, Endbringer (`{5}{B}{B}`, `Coverage::Implemented`): 5/5 flying
/// Vampire that can't attack if there's another creature on the battlefield,
/// makes you lose 3 life at your upkeep, and taps to exile target creature and
/// put a +1/+1 counter on Shauku.
#[test]
fn shauku_endbringer_upkeep_life_loss_tap_exile_and_attack_restriction() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[shauku_endbringer()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shauku = on_battlefield(&engine, p0, shauku_endbringer()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(keywords(&engine, shauku).contains(KeywordSet::FLYING));
    assert!(
        keywords(&engine, shauku).contains(KeywordSet::CANT_ATTACK),
        "Shauku can't attack while Grizzly Bears is on the battlefield"
    );

    // Turn 1 upkeep already lost 3 life (20 -> 17); advancing to p0's next turn loses 3 more (17 -> 14).
    assert_eq!(engine.state().players[0].life, 17);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().players[0].life, 14);

    // Tap Shauku to exile Grizzly Bears and put a +1/+1 counter on Shauku.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: shauku,
                ability_index: 2,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert_eq!(pt(&engine, shauku), (6, 6));
    assert!(
        !keywords(&engine, shauku).contains(KeywordSet::CANT_ATTACK),
        "Shauku can attack once no other creature is on the battlefield"
    );
}

/// "Can't attack if there's another creature on the battlefield": the attack
/// menu leaves Shauku out beside a creature of its own controller, and has it
/// when Shauku is alone.
#[test]
fn shauku_endbringer_attacks_only_when_no_other_creature_is_on_the_battlefield() {
    let mut crowded = lk::game(
        SEED,
        &[shauku_endbringer(), index::GRIZZLY_BEARS],
        &[],
        &[],
        &[],
    );
    let shauku = lk::obj(&crowded, P0, shauku_endbringer());
    let bears = lk::obj(&crowded, P0, index::GRIZZLY_BEARS);
    let able = lk::attackers_offered(&mut crowded);
    assert!(able.contains(&bears));
    assert!(
        !able.contains(&shauku),
        "another creature is on the battlefield"
    );

    let mut alone = lk::game(SEED, &[shauku_endbringer()], &[], &[], &[]);
    let shauku = lk::obj(&alone, P0, shauku_endbringer());
    let able = lk::attackers_offered(&mut alone);
    assert!(
        able.contains(&shauku),
        "no other creature: Shauku may attack"
    );
}

/// "At the beginning of **your** upkeep, you lose 3 life": the opponent's
/// upkeep costs the controller nothing, and the life loss is the controller's.
#[test]
fn shauku_endbringer_upkeep_costs_only_its_controller_life() {
    let mut engine = lk::game(SEED, &[shauku_endbringer()], &[], &[], &[]);
    assert_eq!(lk::life(&engine, P0), 17, "the first upkeep already cost 3");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        lk::life(&engine, P0),
        17,
        "the opponent's upkeep costs nothing"
    );
    assert_eq!(lk::life(&engine, P1), 20);
}
