//! Oracle scenarios for Filthy Cur.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn filthy_cur() -> CardIndex {
    card_index("4aa9764a-39ab-4d55-bc71-45f83035fb4c")
}

#[test]
fn filthy_cur_makes_you_lose_life_equal_to_damage_dealt_to_it() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &[index::SWAMP, index::SWAMP, index::MOUNTAIN])
        .hand(0, &[filthy_cur(), index::SHOCK])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    cast_from_hand(&mut e, p0, filthy_cur());
    pass_until(&mut e, stack_is_empty);

    let cur = on_battlefield(&e, p0, filthy_cur()).unwrap();
    cast_with_floating(&mut e, p0, index::SHOCK);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![cur],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, p0, filthy_cur()).is_some());
    assert_eq!(e.state().players[0].life, 18);
}
