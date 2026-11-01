#[allow(clippy::wildcard_imports)]
use super::*;

fn retromancer() -> CardIndex {
    card_index("2c34949d-0765-455d-9ef4-81bc162b48d9")
}

#[test]
fn retromancer_deals_three_damage_to_targeting_spells_controller() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[giant_growth()])
        .battlefield(0, &[forest()])
        .battlefield(1, &[retromancer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let retro_id = on_battlefield(&engine, p1, retromancer()).expect("retromancer on bf");
    assert_eq!(engine.state().players[0].life, 20);
    cast_from_hand(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, retro_id);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 17);
}

#[test]
fn retromancer_also_punishes_targeted_abilities_and_its_own_controllers_spells() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[index::PRODIGAL_SORCERER],
        &[retromancer()],
        &[],
        &[giant_growth()],
    );
    let retro = lk::obj(&e, p1, retromancer());
    lk::use_ability(&mut e, p0, index::PRODIGAL_SORCERER, 0, &[retro], &[]);
    assert_eq!(lk::life(&e, p0), 17);
    assert_eq!(lk::life(&e, p1), 20);
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Green, 1)]);
    lk::cast_at(&mut e, p1, giant_growth(), &[retro], &[]);
    assert_eq!(lk::life(&e, p1), 17);
    assert_eq!(lk::life(&e, p0), 17);
}
