//! `cards/artifacts/equipment/mv_1/silver_inlaid_dagger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn silver_inlaid_dagger() -> CardIndex {
    card_index("93e9b0d6-97a8-4357-bcd4-f82d81bad232")
}

/// Silver-Inlaid Dagger — "Equipped creature gets +2/+0. As long as equipped
/// creature is a Human, it gets an additional +1/+0." and "Equip {2}". An
/// Elf gets +2, a Human +3.
#[test]
fn silver_inlaid_dagger_gives_humans_an_extra_power() {
    plain_pump(silver_inlaid_dagger(), 2, 2, 0);

    let (poet, elf) = (tragic_poet(), llanowar_elves());
    let mut engine = rig(2, &[], &[silver_inlaid_dagger(), poet, elf], &[]);
    let p0 = PlayerId::new(0);
    let human = on_battlefield(&engine, p0, poet).expect("the Human");
    equip(&mut engine, silver_inlaid_dagger(), 2, human);
    assert_eq!(pt(&engine, human), (4, 1), "1 + 2 + 1");
}
