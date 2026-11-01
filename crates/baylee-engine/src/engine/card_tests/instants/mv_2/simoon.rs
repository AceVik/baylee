//! `cards/instants/mv_2/simoon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MOUNTAIN};

fn simoon() -> CardIndex {
    card_index("128e201a-f520-4917-a1a4-2f3836f1f92d")
}

/// `Simoon` ({R}{G}, Instant, `Coverage::Partial`) deals 1 damage to each
/// creature target opponent controls: in a duel, every creature of the opponent
/// takes 1 and the caster's own creature is untouched.
#[test]
fn simoon_deals_one_damage_to_each_opponent_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[MOUNTAIN, forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), GRIZZLY_BEARS])
        .hand(0, &[simoon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let own_elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let opp_bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, simoon());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, llanowar_elves()).is_some());
    assert_eq!(engine.state().object(opp_bear).unwrap().damage, 1);
    assert_eq!(
        engine.state().object(own_elf).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().object(own_elf).unwrap().damage, 0);
    // The damage is to creatures only.
    assert_eq!(engine.state().players[1].life, 20);
}

/// At a table of three the spell cannot be aimed at one opponent's creatures
/// alone, so it deals no damage rather than hitting the other opponent's
/// creatures too (which "target opponent" does not allow).
#[test]
fn simoon_deals_no_damage_with_two_opponents() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[MOUNTAIN, forest()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .battlefield(2, &[GRIZZLY_BEARS])
        .hand(0, &[simoon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear1 = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let bear2 = on_battlefield(&engine, p2, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, simoon());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(bear1).unwrap().damage,
        0,
        "the targeted opponent's creature is not damaged without the scope"
    );
    assert_eq!(
        engine.state().object(bear2).unwrap().damage,
        0,
        "the other opponent's creature is not damaged"
    );
}
