#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn pygmy_pyrosaur() -> CardIndex {
    card_index("667c39bc-92d6-4fee-bced-95f0e1f21455")
}

#[test]
fn pygmy_pyrosaur_pumps_power_for_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), pygmy_pyrosaur()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pyro = on_battlefield(&engine, p0, pygmy_pyrosaur()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pyro,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, pyro), (2, 1));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert_eq!(pt(&engine, pyro), (1, 1));
}

#[test]
fn pygmy_pyrosaur_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(SEED, &[pygmy_pyrosaur()], &[gray_ogre()], &[], &[]);
    reach_their_main_phase(&mut engine, p1);
    let pyro = lk::obj(&engine, p0, pygmy_pyrosaur());
    let ogre = lk::obj(&engine, p1, gray_ogre());
    let blocks = lk::attack(&mut engine, &[ogre]);
    assert!(!lk::may_block(&blocks, pyro, ogre));
}
