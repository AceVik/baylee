//! `cards/artifacts/equipment/mv_3/umbral_mantle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn umbral_mantle() -> CardIndex {
    card_index("44f3241f-af82-4f2e-b15d-7f279bf6e816")
}

/// Umbral Mantle: "Equipped creature has '{3}, {Q}: This creature gets +2/+2
/// until end of turn.' Equip {0}". The Elf has tapped itself for mana, so the
/// untap symbol can be paid.
#[test]
fn umbral_mantle_grants_the_untap_pump() {
    let p0 = PlayerId::new(0);
    let mut r = rig(umbral_mantle(), llanowar_elves(), 3);
    tap_all_mana(&mut r.engine, p0);
    assert_eq!(r.pool(), 4, "three Forests and the Elf");
    let ability = crate::choice::granted_ability(0);
    let offered = |r: &Rig| {
        let Pending::Priority { legal, .. } = r.engine.pending().clone() else {
            panic!("expected priority, got {:?}", r.engine.pending())
        };
        legal.abilities.contains(&(r.host, ability))
    };
    assert!(!offered(&r), "the Elf has no such ability yet");
    r.equip_now();
    assert_eq!(r.pool(), 4, "the Equip cost is {{0}}");
    assert!(offered(&r), "the equipped Elf has it");
    assert!(is_tapped(&r.engine, r.host));
    r.engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: r.host,
                ability_index: ability,
            },
        )
        .unwrap();
    pass_until(&mut r.engine, stack_is_empty);
    assert_eq!(pt(&r.engine, r.host), (3, 3), "+2/+2");
    assert!(!is_tapped(&r.engine, r.host), "{{Q}} untapped it");
    assert_eq!(r.pool(), 1, "{{3}} came out of the pool");
}
