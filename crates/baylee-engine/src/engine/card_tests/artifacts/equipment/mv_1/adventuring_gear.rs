//! `cards/artifacts/equipment/mv_1/adventuring_gear.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn adventuring_gear() -> CardIndex {
    card_index("17042684-7af4-43a6-88c7-885032cfb27c")
}

fn play_forest(engine: &mut Engine<RegistryLookup>) {
    let land = in_hand(engine, PlayerId::new(0), forest()).expect("a Forest in hand");
    engine
        .apply(PlayerId::new(0), PlayerAction::PlayLand { card: land })
        .expect("the land drop");
    pass_until(engine, stack_is_empty);
}

/// Adventuring Gear — "Landfall — Whenever a land you control enters,
/// equipped creature gets +2/+2 until end of turn." and "Equip {1}". A land
/// entering while the Gear holds nobody changes nothing; once it holds the
/// Elf, the landfall makes it a 3/3, the bystander stays 1/1, and the bonus
/// is gone when the turn is.
#[test]
fn adventuring_gear_pumps_its_host_on_landfall_until_end_of_turn() {
    let elf = llanowar_elves();
    let p0 = PlayerId::new(0);

    // Unattached: nobody is "equipped creature".
    let mut engine = rig(0, &[forest()], &[adventuring_gear(), elf, elf], &[]);
    play_forest(&mut engine);
    for e in all_on_battlefield(&engine, p0, elf) {
        assert_eq!(pt(&engine, e), (1, 1), "an unattached Gear pumps no one");
    }

    // Attached: the land drop is the trigger.
    let mut engine = rig(1, &[forest()], &[adventuring_gear(), elf, elf], &[]);
    let elves = all_on_battlefield(&engine, p0, elf);
    let (host, other) = (elves[0], elves[1]);
    equip(&mut engine, adventuring_gear(), 1, host);
    assert_eq!(pt(&engine, host), (1, 1), "Equip alone gives no bonus");
    play_forest(&mut engine);
    assert_eq!(pt(&engine, host), (3, 3), "landfall: +2/+2");
    assert_eq!(pt(&engine, other), (1, 1), "only the equipped creature");

    pass_until(&mut engine, |e| {
        e.state().turn.number > 1 && e.state().turn.step == Step::Upkeep
    });
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "until end of turn, and no longer"
    );
}
