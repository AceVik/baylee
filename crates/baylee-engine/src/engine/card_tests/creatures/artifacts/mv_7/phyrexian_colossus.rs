#[allow(clippy::wildcard_imports)]
use super::*;

fn phyrexian_colossus() -> CardIndex {
    card_index("caab555c-7852-418a-bdc3-f908af7e0932")
}

#[test]
fn phyrexian_colossus_pays_eight_life_to_untap_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[phyrexian_colossus()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let colossus_id = on_battlefield(&engine, p0, phyrexian_colossus()).expect("on battlefield");
    engine
        .state
        .object_mut(colossus_id)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();

    assert_eq!(engine.state().players[0].life, 20);
    activate(&mut engine, p0, phyrexian_colossus(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 12);
    assert!(
        !engine
            .state()
            .object(colossus_id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn phyrexian_colossus_stays_tapped_through_its_untap_step() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[phyrexian_colossus()], &[], &[], &[]);
    let colossus = lk::obj(&e, p0, phyrexian_colossus());
    lk::attack(&mut e, &[colossus]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 12);
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert!(is_tapped(&e, colossus));
    lk::use_ability(&mut e, p0, phyrexian_colossus(), 1, &[], &[]);
    assert!(!is_tapped(&e, colossus));
    assert_eq!(lk::life(&e, p0), 12);
}
