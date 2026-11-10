//! `cards/artifacts/equipment/mv_3/barbed_battlegear.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn barbed_battlegear() -> CardIndex {
    card_index("b81fed36-07c4-4903-b5b4-24e62d2ce239")
}

/// Barbed Battlegear:
/// > Equipped creature gets +4/-1.
/// > Equip {2}
///
/// The statics read "equipped creature", so the test pins both halves: the
/// host gets the bonus once the Equip is paid, and the creature we did not
/// equip and the one across the table do not.
#[test]
fn barbed_battlegear_gives_the_equipped_creature_what_it_prints() {
    equipped_gets(
        barbed_battlegear(),
        2,
        jasmine_boreal(),
        (4, -1),
        KeywordSet::EMPTY,
    );
}
