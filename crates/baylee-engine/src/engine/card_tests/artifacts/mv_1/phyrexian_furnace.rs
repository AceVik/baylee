//! `cards/artifacts/mv_1/phyrexian_furnace.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, PHYREXIAN_FURNACE};

/// `Phyrexian Furnace` ({1}, Artifact, Partial) taps to exile target card from
/// a graveyard, and pays `{1}` and sacrifices itself to exile target card from
/// a graveyard and draw a card.
#[test]
fn phyrexian_furnace_exiles_cards_from_graveyard_and_draws() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[PHYREXIAN_FURNACE, plains()])
        .battlefield(1, &[GRIZZLY_BEARS, HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    let _ = engine.state.move_object(
        bears,
        ZoneLocation::Graveyard(p1),
        crate::zone::ZonePosition::Top,
        Cause::Effect,
    );
    let _ = engine.state.move_object(
        giant,
        ZoneLocation::Graveyard(p1),
        crate::zone::ZonePosition::Top,
        Cause::Effect,
    );
    engine.refresh_offer();

    activate(&mut engine, p0, PHYREXIAN_FURNACE, 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Exile);

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Main
    });

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, PHYREXIAN_FURNACE, 1);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, PHYREXIAN_FURNACE).is_some());
    assert_eq!(engine.state().object(giant).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
