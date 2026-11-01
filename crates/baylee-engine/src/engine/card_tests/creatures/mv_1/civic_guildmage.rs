//! `cards/creatures/mv_1/civic_guildmage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn civic_guildmage() -> CardIndex {
    card_index("6e93cb9a-44c8-49fb-b6a7-bfdf7bd725fe")
}

/// Civic Guildmage (6e93cb9a-44c8-49fb-b6a7-bfdf7bd725fe) — {W} 1/1:
/// - "{G}, {T}: Target creature gets +0/+1 until end of turn."
/// - "{U}, {T}: Put target creature you control on top of its owner's library."
///
/// The second ability is left off (no effect puts a permanent on top of its
/// owner's library with no choice, and the owner choosing the bottom is not
/// printed), so the {G} ability is ability 0 and there is no ability 1.
/// "Target creature" is any creature, an opponent's included, and the +0/+1
/// ends with the turn.
#[test]
fn civic_guildmage_gives_any_creature_plus_zero_plus_one_until_end_of_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[civic_guildmage(), GRIZZLY_BEARS, forest(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mage = on_battlefield(&engine, P0, civic_guildmage()).unwrap();
    let mine = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    let theirs = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    assert!(
        !ability_offered(&engine, mage, 1),
        "the {{U}}, {{T}} library ability is left off, with {{U}} floating"
    );
    activate(&mut engine, P0, civic_guildmage(), 0);
    let menu = aim_at(&mut engine, P0, theirs);
    assert!(
        menu.contains(&mine) && menu.contains(&theirs),
        "any creature is a target: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, theirs), (2, 3), "their Bears get +0/+1");
    assert_eq!(pt(&engine, mine), (2, 2), "mine does not");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, theirs), (2, 2), "until end of turn");
}
