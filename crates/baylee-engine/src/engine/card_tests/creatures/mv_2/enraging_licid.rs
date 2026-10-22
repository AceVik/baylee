//! `cards/creatures/mv_2/enraging_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn enraging_licid() -> CardIndex {
    card_index("5ec9f317-887a-4e08-a165-e7cf15eb071d")
}

/// Enraging Licid (`5ec9f317-887a-4e08-a165-e7cf15eb071d`) — {1}{R} Creature — Licid (Partial):
/// "{R}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {R} to end this effect. Enchanted creature has haste."
#[test]
fn enraging_licid_grants_target_creature_haste() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[enraging_licid(), index::MOUNTAIN, index::GRIZZLY_BEARS],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert!(!keywords(&engine, bears).contains(KeywordSet::HASTE));

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, enraging_licid(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::HASTE));
}
