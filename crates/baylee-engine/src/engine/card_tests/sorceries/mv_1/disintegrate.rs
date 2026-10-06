//! `cards/sorceries/mv_1/disintegrate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Disintegrate: "deals X damage to any target. If it's a creature, it
/// can't be regenerated this turn, and if it would die this turn, exile it
/// instead." A shielded creature dies anyway, and lands in exile, not the
/// graveyard.
#[test]
fn disintegrate_exiles_a_shielded_creature_instead_of_letting_it_die() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), quiet_creature()])
        .hand(0, &[disintegrate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(elf)
        .expect("seated")
        .regeneration_shields = 1;

    cast_from_hand(&mut engine, p0, disintegrate());
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_creature()).is_none(),
        "2 damage kills a 1-toughness Elf"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_creature()).is_none(),
        "\"exile it instead\" — not the graveyard"
    );
    let exiled = engine
        .state()
        .zones
        .list(ZoneLocation::Exile(p0))
        .iter()
        .any(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == quiet_creature()))
        });
    assert!(exiled, "the Elf is in exile");
}
