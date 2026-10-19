//! `cards/enchantments/mv_2/fungal_bloom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn bloom_card() -> CardIndex {
    card_index("fa76ee16-ff48-418b-819a-69f1139b36f3")
}
fn thallid_card() -> CardIndex {
    card_index("46abaabe-7015-4d82-a5aa-2706b1f51bed")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn spores(e: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    e.state()
        .object(id)
        .expect("on the battlefield")
        .counters
        .get(baylee_cards_dsl::counters::SPORE)
}

/// Fungal Bloom (fa76ee16-ff48-418b-819a-69f1139b36f3) — "{G}{G}: Put a spore
/// counter on target Fungus." Only Fungi are on the target menu, and each
/// activation adds one counter.
#[test]
fn fungal_bloom_puts_a_spore_counter_on_a_fungus_each_time() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                bloom_card(),
                thallid_card(),
                bears_card(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let thallid = on_battlefield(&e, P0, thallid_card()).expect("the Thallid");
    let bears = on_battlefield(&e, P0, bears_card()).expect("the Bears");
    let start = spores(&e, thallid);

    tap_all_mana(&mut e, P0);
    activate(&mut e, P0, bloom_card(), 0);
    let menu = aim_at(&mut e, P0, thallid);
    assert!(menu.contains(&thallid), "a Fungus is a legal target");
    assert!(!menu.contains(&bears), "a non-Fungus is not");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(spores(&e, thallid), start + 1);

    activate(&mut e, P0, bloom_card(), 0);
    aim_at(&mut e, P0, thallid);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(spores(&e, thallid), start + 2, "every activation adds one");
    assert_eq!(spores(&e, bears), 0, "nothing landed on the Bears");
}
