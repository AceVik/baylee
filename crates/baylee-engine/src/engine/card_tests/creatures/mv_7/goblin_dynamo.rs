//! `cards/creatures/mv_7/goblin_dynamo.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Goblin Dynamo (`381fdb36-2467-421c-8b89-419d14955175`) — {5}{R}{R} Creature — Goblin Mutant 4/4:
/// "{T}: This creature deals 1 damage to any target.
/// {X}{R}, {T}, Sacrifice this creature: It deals X damage to any target."
#[test]
fn goblin_dynamo_pings_for_one_and_sacrifices_for_x_damage() {
    let dynamo_card = card_index("381fdb36-2467-421c-8b89-419d14955175");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                dynamo_card,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // First ability: {T}: deal 1 damage to any target.
    activate(&mut engine, p0, dynamo_card, 0);
    point_at(&mut engine, p0, &[], &[p1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 1), 19);

    // Untap on next turn and activate second ability: {X}{R}, {T}, Sacrifice: deal X=4 damage.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, dynamo_card, 1);
    engine.apply(p0, PlayerAction::ChooseNumber(4)).unwrap();
    point_at(&mut engine, p0, &[], &[p1]);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, dynamo_card).is_some());
    assert_eq!(life_of(&engine, 1), 15);
}
