//! Molten Rain destroys a land and damages only for a nonbasic one.

use super::*;

fn molten_rain() -> CardIndex {
    card_index("e9774695-b2f1-4014-8504-04b2339be568")
}

fn rain_on(land: CardIndex) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 3])
        .battlefield(1, &[land])
        .hand(0, &[molten_rain()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let target = on_battlefield(&e, p1, land).unwrap();
    cast_from_hand(&mut e, p0, molten_rain());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p1, land).is_some());
    e
}

#[test]
fn molten_rain_damages_for_nonbasic_but_not_basic_land() {
    let e = rain_on(badlands());
    assert_eq!(e.state().players[1].life, 18);
    let e = rain_on(forest());
    assert_eq!(e.state().players[1].life, 20);
}
