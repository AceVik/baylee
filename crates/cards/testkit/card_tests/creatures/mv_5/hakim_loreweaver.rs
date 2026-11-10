//! `cards/creatures/mv_5/hakim_loreweaver.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HOLY_STRENGTH};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hakim_loreweaver() -> CardIndex {
    card_index("43e99cba-a6e3-4abc-a92b-aaf5839c72d2")
}

/// Hakim, Loreweaver (43e99cba-a6e3-4abc-a92b-aaf5839c72d2) — {3}{U}{U} 2/4:
/// "Flying. {U}{U}: Return target Aura card from your graveyard to the
/// battlefield attached to Hakim. Activate only during your upkeep and only if
/// Hakim isn't enchanted. {U}{U}, {T}: Destroy all Auras attached to Hakim."
///
/// The first ability is left off (no effect returns a graveyard card to the
/// battlefield attached to the source), so the destroy ability is ability 0.
/// "All Auras attached to Hakim" is Hakim's own: an Aura on another creature
/// stays.
#[test]
fn hakim_loreweaver_destroys_only_the_auras_attached_to_it() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[hakim_loreweaver(), plains(), plains(), island(), island()],
        )
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[HOLY_STRENGTH, HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hakim = on_battlefield(&engine, P0, hakim_loreweaver()).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, HOLY_STRENGTH);
    aim_at(&mut engine, P0, hakim);
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, P0, HOLY_STRENGTH);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, hakim), (3, 6));
    assert_eq!(pt(&engine, bears), (3, 4));
    assert!(
        !ability_offered(&engine, hakim, 1),
        "the graveyard-return ability is left off, so there is no second ability"
    );

    activate(&mut engine, P0, hakim_loreweaver(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, hakim), (2, 4), "Hakim's Aura is destroyed");
    assert_eq!(
        pt(&engine, bears),
        (3, 4),
        "an Aura attached to another creature stays"
    );
}
