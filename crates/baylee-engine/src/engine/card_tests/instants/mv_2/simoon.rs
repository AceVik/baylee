//! `cards/instants/mv_2/simoon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MOUNTAIN};

fn simoon() -> CardIndex {
    card_index("128e201a-f520-4917-a1a4-2f3836f1f92d")
}

/// `Simoon` ({R}{G}, Instant, `Coverage::Partial`) targets an opponent and
/// deals 1 damage to each creature opponents control.
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
    assert_eq!(engine.state().object(own_elf).unwrap().zone, Zone::Battlefield);
    assert_eq!(engine.state().object(own_elf).unwrap().damage, 0);
}
