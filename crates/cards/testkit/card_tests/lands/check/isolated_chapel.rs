//! `cards/lands/check/isolated_chapel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Isolated Chapel is a check land: "This land enters tapped unless you
/// control a Plains or a Swamp" and "{T}: Add {W} or {B}". The two games
/// differ by exactly one permanent — the first has a Forest alone, the second
/// adds a Swamp — so the change from tapped to untapped is attributable to
/// the check and to nothing else, and the Forest standing in both is what
/// says the check is *read* rather than assumed. Both lands are played as
/// real land drops, because `starting_battlefield` places a permanent with no
/// entry to replace and would arrive untapped whatever the card prints. The
/// second game then taps for **black** while its own check named white and
/// the lands beside it are green and untapped, so one black mana is the only
/// thing that could have produced it.
#[test]
fn isolated_chapel_enters_tapped_without_its_check_land_and_taps_for_either_colour() {
    let p0 = PlayerId::new(0);

    // Control: a Forest satisfies neither half of the check, so the land
    // arrives tapped — and a tapped permanent cannot pay a {T}.
    let mut alone = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[isolated_chapel()])
        .start();
    keep_mulligans(&mut alone);
    assert!(walk_to_own_main(&mut alone, p0), "p0 reaches its own main");
    let tapped_land = play_land(&mut alone, p0, isolated_chapel());
    assert!(
        entered_tapped(&alone, tapped_land),
        "a Forest is neither a Plains nor a Swamp, so the check fails and the \
         land enters tapped"
    );
    let Pending::Priority { legal, .. } = alone.pending().clone() else {
        panic!("expected priority, got {:?}", alone.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == tapped_land)
            && !legal.mana_abilities.contains(&tapped_land),
        "and it is an untap step away from being usable: {{T}} is a price a \
         tapped permanent cannot pay (CR 118.3)"
    );
    assert_eq!(
        alone.state().players[0].mana_pool.total(),
        0,
        "so nothing was made"
    );

    // The same board plus one Swamp: the check passes and the land is untapped.
    let mut checked = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), swamp()])
        .hand(0, &[isolated_chapel()])
        .start();
    keep_mulligans(&mut checked);
    assert!(
        walk_to_own_main(&mut checked, p0),
        "p0 reaches its own main"
    );
    let land = play_land(&mut checked, p0, isolated_chapel());
    assert!(
        !entered_tapped(&checked, land),
        "the one card the two boards differ by is a Swamp, so the check's \
         other arm is what let it in untapped"
    );

    // Ability 0 is the printed "{T}: Add {W} or {B}." Two colours are two
    // colours, so the engine asks the player rather than picking for them.
    activate(&mut checked, p0, isolated_chapel(), 0);
    let Pending::ChooseColor { player, options } = checked.pending().clone() else {
        panic!("\"or\" is a question, got {:?}", checked.pending())
    };
    assert_eq!(player, p0, "the seat that taps names the colour");
    assert_eq!(options.len(), 2, "two colours and no more: {options:?}");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "{{W}} and {{B}}, the two the card prints: {options:?}"
    );
    checked
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &checked.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not the one its own check was about"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap: the pool was empty before it, so this is exact"
    );
    assert!(
        stack_is_empty(&checked),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&checked, land), "the land paid its own {{T}}");
    let swamp_out = on_battlefield(&checked, p0, swamp()).expect("the Swamp is out");
    let forest_out = on_battlefield(&checked, p0, forest()).expect("the Forest is out");
    assert!(
        !is_tapped(&checked, swamp_out) && !is_tapped(&checked, forest_out),
        "and neither land beside it moved, so the black mana has no other \
         source on this board"
    );
}
