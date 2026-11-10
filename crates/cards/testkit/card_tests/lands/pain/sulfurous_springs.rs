//! `cards/lands/pain/sulfurous_springs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sulfurous Springs prints two mana abilities and the difference between
/// them is the whole card: `{T}: Add {C}` asks nothing but the tap, while
/// `{T}: Add {B} or {R}` makes its controller lose 1 life. Both are pressed
/// on one board — each on its own copy of the land, because a permanent that
/// has already tapped for one of them cannot pay the other in the same turn
/// — and the colour question is read as it arrives, since `{B} or {R}` is the
/// whole of what the second ability enumerates. The two life totals are
/// asserted together at the end: "you" on this card is the land's controller,
/// so a damage clause that had lost `PlayerRel::You` would show up as p1 at
/// 19 rather than p0.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn sulfurous_springs_taps_for_colorless_for_free_and_for_black_at_one_life() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sulfurous_springs()])
        .hand(0, &[sulfurous_springs()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    play_land(&mut engine, p0, sulfurous_springs());
    let springs = all_on_battlefield(&engine, p0, sulfurous_springs());
    assert_eq!(
        springs.len(),
        2,
        "the copy the board started with and the one played this turn"
    );
    let (colorless, taxed) = (springs[0], springs[1]);
    assert!(
        !is_tapped(&engine, colorless) && !is_tapped(&engine, taxed),
        "and both are standing untapped"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(colorless, 0)) && legal.abilities.contains(&(taxed, 1)),
        "each printed line is an ordinary (source, index) entry, offered on \
         the board alone: the whole price of either is the land's own {{T}}, \
         so the pool plays no part in the offer: {:?}",
        legal.abilities
    );

    // Ability 0 — "{T}: Add {C}." The tap is the whole price.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: colorless,
                ability_index: 0,
            },
        )
        .expect("the colourless line costs the land's own {T} and nothing else");
    assert!(is_tapped(&engine, colorless), "the tap is paid");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "one colourless mana, and no question was asked to get it"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the damage is printed on the other ability and nowhere else"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );

    // Ability 1 — "{T}: Add {B} or {R}. This land deals 1 damage to you."
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: taxed,
                ability_index: 1,
            },
        )
        .expect("the coloured line costs the same {T} on the untapped copy");
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "the two colours the card prints — colourless belongs to the *other* \
         ability and is not an option here"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two it offered");
    assert!(is_tapped(&engine, taxed), "the second copy paid its {{T}}");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — the price of the coloured half"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and it is dealt to the land's controller, never to the opponent"
    );
    assert!(
        stack_is_empty(&engine),
        "the damage arrives inside a mana ability, so it used no stack either"
    );
}
