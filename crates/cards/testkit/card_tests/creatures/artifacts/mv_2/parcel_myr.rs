//! `cards/creatures/artifacts/mv_2/parcel_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn parcel_myr() -> CardIndex {
    card_index("9e660f66-0a94-4694-95e0-25296799b193")
}

/// Parcel Myr: a 2/1 with "{2}, Sacrifice this creature: Draw a card." Two
/// mana and the Myr itself buy one card.
#[test]
fn parcel_myr_sacrifices_itself_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[parcel_myr(), forest(), forest()], &[]);
    let myr = on_battlefield(&engine, p0, parcel_myr()).expect("seated");
    assert_eq!(pt(&engine, myr), (2, 1));
    assert!(
        !priority_offer(&engine).abilities.contains(&(myr, 0)),
        "no mana floating"
    );
    let hand = hand_count(&engine, 0);
    let library = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, parcel_myr(), 0);
    assert!(
        in_graveyard(&engine, p0, parcel_myr()).is_some(),
        "sacrificed as a cost, before the ability resolves"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_count(&engine, 0), hand + 1);
    assert_eq!(library_size(&engine, p0), library - 1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{2}} paid");
}
