//! `cards/creatures/mv_3/morgue_thrull.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Morgue Thrull is a {2}{B} 2/2 Thrull whose whole printed text is
/// "Sacrifice this creature: Mill three cards."
///
/// The sacrifice is the cost and the mill is the effect, so the two are read
/// off different places: the Thrull must already be in its owner's graveyard
/// when the ability goes on the stack (CR 601.2h), and the three cards can
/// only leave the library when it resolves. The board has a second Thrull
/// under the same seat so the graveyard entry is the one that was sacrificed
/// and not merely a printing that vanished, and the ability costs no mana —
/// so the offer is read off an empty pool rather than off tapped lands.
#[test]
fn morgue_thrull_sacrifices_itself_to_mill_three() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[morgue_thrull(), morgue_thrull()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let thrulls = all_on_battlefield(&engine, p0, morgue_thrull());
    assert_eq!(thrulls.len(), 2, "two copies, one of which stays out");
    let fodder = thrulls[0];

    let library_before = library_size(&engine, p0);
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price is the creature, so no mana is involved"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(fodder, 0)),
        "an untapped Thrull with itself to give up is offered its one line: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, morgue_thrull(), 0);
    assert!(
        in_graveyard(&engine, p0, morgue_thrull()).is_some(),
        "`Sacrifice this creature` is paid on announcement (CR 601.2h)"
    );
    assert!(
        !stack_is_empty(&engine),
        "milling is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        library_before - 3,
        "\"mill three cards\": exactly three off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard_before + 4,
        "the three milled cards, and the Thrull that ate itself to mill them \
         — the count was taken before the sacrifice paid the cost"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, morgue_thrull()).len(),
        1,
        "the printed card was the price: one Thrull ate itself and the other stands"
    );
}
