//! `cards/creatures/mv_3/dominating_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn dominating_licid() -> CardIndex {
    card_index("ae2804c6-fb38-4845-ae21-7f41a766cbba")
}

/// Dominating Licid (`ae2804c6-fb38-4845-ae21-7f41a766cbba`) — {1}{U}{U} Creature — Licid (Partial):
/// "{1}{U}{U}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {U} to end this effect. You control enchanted creature."
#[test]
fn dominating_licid_casts_and_enters_as_1_1() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[dominating_licid()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    tap_all_mana(&mut engine, P0);
    cast_from_hand(&mut engine, P0, dominating_licid());
    pass_until(&mut engine, stack_is_empty);

    let licid = on_battlefield(&engine, P0, dominating_licid()).unwrap();
    assert_eq!(pt(&engine, licid), (1, 1));
}
