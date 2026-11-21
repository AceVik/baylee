//! `cards/instants/mv_1/vigorous_charge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "b8ce3f84-2053-4d16-ab3d-e552bd8d0e8c"
fn vigorous_charge() -> CardIndex {
    card_index("b8ce3f84-2053-4d16-ab3d-e552bd8d0e8c")
}

/// Vigorous Charge ({G}, Kicker {W}): unkicked grants trample until end of turn
/// without any combat-damage lifegain trigger, and trample wears off next turn.
#[test]
fn vigorous_charge_unkicked_grants_trample_without_lifegain() {
    let mut engine = lk::game(
        SEED,
        &[index::HILL_GIANT],
        &[index::LLANOWAR_ELVES],
        &[vigorous_charge()],
        &[],
    );
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let elf = lk::obj(&engine, P1, index::LLANOWAR_ELVES);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Green, 1), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, vigorous_charge());
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::Kicker,
            ..
        }
    ));
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::aim(&mut engine, &[giant], &[]);
    lk::settle(&mut engine);

    assert!(keywords(&engine, giant).contains(KeywordSet::TRAMPLE));
    let _ = lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[(elf, giant)]);
    lk::through_combat(&mut engine);

    assert_eq!(lk::life(&engine, P1), 18, "2 trample damage hits P1");
    assert_eq!(lk::life(&engine, P0), 20, "unkicked grants no lifegain");

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !keywords(&engine, giant).contains(KeywordSet::TRAMPLE),
        "trample wears off at cleanup"
    );
}

/// Kicked for {W}: grants trample AND gains life equal to all combat damage
/// the creature deals this turn (to both blocker and defending player), and
/// expires next turn.
#[test]
fn vigorous_charge_kicked_gains_life_equal_to_combat_damage_dealt() {
    let mut engine = lk::game(
        SEED,
        &[index::HILL_GIANT],
        &[index::LLANOWAR_ELVES],
        &[vigorous_charge()],
        &[],
    );
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let elf = lk::obj(&engine, P1, index::LLANOWAR_ELVES);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Green, 1), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, vigorous_charge());
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::aim(&mut engine, &[giant], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::pool_total(&engine, P0), 0, "{{G}}{{W}} spent");
    let _ = lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[(elf, giant)]);
    lk::through_combat(&mut engine);

    assert_eq!(lk::life(&engine, P1), 18, "2 trample damage hits P1");
    assert_eq!(
        lk::life(&engine, P0),
        23,
        "1 damage to Elf + 2 trample to P1 = 3 life gained"
    );

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let _ = lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(
        lk::life(&engine, P1),
        15,
        "unblocked Giant deals 3 next turn"
    );
    assert_eq!(
        lk::life(&engine, P0),
        23,
        "combat-damage lifegain trigger ended after the first turn"
    );
}
