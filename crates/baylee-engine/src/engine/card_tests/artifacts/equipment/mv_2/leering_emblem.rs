//! `cards/artifacts/equipment/mv_2/leering_emblem.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn leering_emblem() -> CardIndex {
    card_index("a5cfd1e4-cd14-454a-9734-b5f6952d6a50")
}

/// Leering Emblem: "Whenever you cast a spell, equipped creature gets +2/+2
/// until end of turn. Equip {2}". A Giant Growth on the creature we did not
/// equip still pumps the equipped one, and the pump ends with the turn.
#[test]
fn leering_emblem_pumps_the_equipped_creature_when_you_cast_a_spell() {
    let p1 = PlayerId::new(1);
    let p0 = PlayerId::new(0);
    let mut r = rig_vs(
        leering_emblem(),
        llanowar_elves(),
        3,
        &[giant_growth()],
        &[llanowar_elves()],
        &[],
    );
    r.equip_keeping(2, 1);
    assert_eq!(pt(&r.engine, r.host), (1, 1), "nothing without a spell");
    cast_with_floating(&mut r.engine, p0, giant_growth());
    let bystander = r.bystander;
    r.engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bystander],
            },
        )
        .unwrap();
    pass_until(&mut r.engine, stack_is_empty);
    assert_eq!(pt(&r.engine, r.host), (3, 3), "the cast pumped the host");
    assert_eq!(pt(&r.engine, r.bystander), (5, 4), "the spell's own +3/+3");
    reach_their_main_phase(&mut r.engine, p1);
    assert_eq!(pt(&r.engine, r.host), (1, 1), "until end of turn");
}
