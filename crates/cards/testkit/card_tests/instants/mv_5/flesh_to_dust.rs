//! `cards/instants/mv_5/flesh_to_dust.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "2f05f3ed-e3b7-4225-b3f2-c12eda335237"
fn flesh_to_dust() -> CardIndex {
    card_index("2f05f3ed-e3b7-4225-b3f2-c12eda335237")
}

/// Flesh to Dust: "Destroy target creature. It can't be regenerated."
///
/// The Troll buys a shield from its own `{B}` ability before the spell is cast,
/// so a plain destroy would leave it standing; that it is in the graveyard is
/// the second sentence. The Elf beside it is a legal target too, and is left
/// alone.
#[test]
fn flesh_to_dust_destroys_a_creature_that_has_a_shield() {
    let mut engine = table(
        908,
        &[flesh_to_dust()],
        &[swamp(); 6],
        &[lotleth_troll()],
        &[llanowar_elves()],
    );
    let troll = on_battlefield(&engine, me(), lotleth_troll()).expect("my Troll");
    let elf = on_battlefield(&engine, them(), llanowar_elves()).expect("their Elf");
    tap_all_mana(&mut engine, me());
    raise_a_shield(&mut engine, me(), troll, 1);
    cast_with_floating(&mut engine, me(), flesh_to_dust());
    let menu = aim_at(&mut engine, me(), troll);
    assert!(menu.contains(&troll) && menu.contains(&elf), "{menu:?}");
    settle(&mut engine);
    assert!(
        in_graveyard(&engine, me(), lotleth_troll()).is_some(),
        "the shield did not save it"
    );
    assert!(on_battlefield(&engine, them(), llanowar_elves()).is_some());
}
