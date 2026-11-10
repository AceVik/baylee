//! `cards/lands/check/dragonskull_summit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dragonskull Summit is a check land: "This land enters tapped unless you
/// control a Swamp or a Mountain", and "{T}: Add {B} or {R}".
///
/// Both boards are *played* rather than seeded, because the replacement is
/// the whole card: `starting_battlefield` places a permanent without an
/// entry, so a Summit set down that way would arrive untapped whatever the
/// sentence says. The negative board holds a Forest — a land you control and
/// neither of the two types the card names, so a check that had degraded to
/// "unless you control a land" would flip it — and the tapped arrival has its
/// own consequence read off the pool: the Forest pays and the Summit pays
/// nothing until its controller's next untap step. The positive board differs
/// by exactly one permanent, the Mountain, and the Summit that arrives
/// untapped then taps for the colour its controller names.
#[test]
fn dragonskull_summit_arrives_tapped_without_a_swamp_or_mountain_and_untapped_with_one() {
    let p0 = PlayerId::new(0);

    let mut bare = Duel::new(11, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[dragonskull_summit()])
        .start();
    keep_mulligans(&mut bare);
    assert!(walk_to_own_main(&mut bare, p0), "p0 reaches its own main");
    let tapped = play_land(&mut bare, p0, dragonskull_summit());
    assert!(
        entered_tapped(&bare, tapped),
        "a Forest is a land you control and neither a Swamp nor a Mountain"
    );
    tap_all_mana(&mut bare, p0);
    assert_eq!(
        bare.state().players[0].mana_pool.total(),
        1,
        "the Forest paid {{G}} and the Summit, still tapped, paid nothing"
    );

    let mut checked = Duel::new(12, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[dragonskull_summit()])
        .start();
    keep_mulligans(&mut checked);
    assert!(
        walk_to_own_main(&mut checked, p0),
        "p0 reaches its own main"
    );
    let land = play_land(&mut checked, p0, dragonskull_summit());
    assert!(
        !entered_tapped(&checked, land),
        "the Mountain beside it is exactly the condition the card prints"
    );

    // Ability 0 is the printed "{T}: Add {B} or {R}" — a real question, and
    // the answer is the colour that lands in the pool.
    activate(&mut checked, p0, dragonskull_summit(), 0);
    let Pending::ChooseColor { player, options } = checked.pending().clone() else {
        panic!(
            "`Add {{B}} or {{R}}` is a question, got {:?}",
            checked.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat that tapped the Summit names the colour"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "the two colours the card prints: {options:?}"
    );
    assert_eq!(options.len(), 2, "and no third: {options:?}");

    checked
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    let mountain_land = on_battlefield(&checked, p0, mountain()).expect("the Mountain stands");
    let pool = &checked.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named"
    );
    assert_eq!(pool.total(), 1, "one land tapped, one mana");
    assert!(is_tapped(&checked, land), "the Summit paid its own {{T}}");
    assert!(
        !is_tapped(&checked, mountain_land),
        "and the Mountain beside it never moved, so the black mana has no \
         other source on this board"
    );
}
