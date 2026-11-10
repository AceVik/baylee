#[allow(clippy::wildcard_imports)]
use super::*;

fn hibernation() -> CardIndex {
    card_index("633b541b-e37f-451a-be73-3398e61e0f75")
}

#[test]
fn hibernation_returns_all_green_permanents_to_owners_hands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[hibernation()])
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, hibernation());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(in_hand(&engine, p0, llanowar_elves()).is_some());
    assert!(in_hand(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
    assert_eq!(all_on_battlefield(&engine, p0, island()).len(), 3);
}
