//! `cards/lands/pain/adarkar_wastes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Adarkar Wastes prints two mana abilities and only the second one bites:
/// "{T}: Add {C}." and "{T}: Add {W} or {U}. This land deals 1 damage to
/// you." A painland *is* that trade — an untapped dual on the turn it lands,
/// paid for in life — so one board buys both halves: the copy played from
/// hand comes in untapped (no enter modifier) and is pressed for blue, which
/// has to cost exactly one life, while the copy already standing beside it is
/// pressed for colourless, which must cost none. Two copies of the same card
/// are the control that puts the damage on the *second* line rather than on
/// tapping the land at all, and both life totals are read so that "you" is
/// the controller and not the table.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn adarkar_wastes_enters_untapped_and_only_the_coloured_line_costs_a_life() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);
    let mut engine = Duel::new(1207, forest())
        .battlefield(0, &[adarkar_wastes()])
        .hand(0, &[adarkar_wastes()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let standing = on_battlefield(&engine, p0, adarkar_wastes()).expect("the seated copy");
    let played = play_land(&mut engine, p0, adarkar_wastes());
    assert_ne!(standing, played, "the land drop put a second copy down");
    assert_eq!(
        all_on_battlefield(&engine, p0, adarkar_wastes()).len(),
        2,
        "the drop landed beside the copy that was already standing"
    );
    assert!(
        !is_tapped(&engine, played),
        "it has no enter modifier, so the drop is usable the moment it lands"
    );

    // Both lines cost nothing but the land's own {T}, so no pool decides
    // whether they are offered; tapping everything else first is what says
    // this board holds no other mana source.
    tap_all_mana_but(&mut engine, p0, Some(adarkar_wastes()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and neither copy has been tapped yet"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a quiet main phase holds priority: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(played, 0)) && legal.abilities.contains(&(played, 1)),
        "the copy that just landed offers both printed lines as \
         `(source, index)`: a printed mana ability is still a mana ability \
         (CR 605.1) and never the CR 305.6 shortcut: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: played,
                ability_index: 1,
            },
        )
        .expect("the coloured line costs only its own tap");
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{W}} or {{U}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated names the colour");
    assert_eq!(
        options.len(),
        2,
        "two colours to choose between, and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Blue),
        "the pair the card prints: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Colorless),
        "the coloured line makes no {{C}} — the land's other ability is the \
         one that does: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, played), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve behind it"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — the price of the coloured half"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"you\" is the land's controller, so the opponent pays nothing"
    );

    // The other printed line, off the copy standing beside it: the same card,
    // the same {T}, and no damage at all.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: standing,
                ability_index: 0,
            },
        )
        .expect("the colourless line is the same price with no bite");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\""
    );
    assert_eq!(pool.total(), 2, "with the blue still floating beside it");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "the {{C}} line deals no damage, so the one life that is gone was the \
         blue"
    );
    assert!(
        is_tapped(&engine, standing),
        "and the copy that made it is tapped too"
    );
}

/// The owner's board (08.10.2026): a Plains and Adarkar Wastes pay a
/// `{W}{U}` creature. The Wastes' coloured line is a mana ability with a
/// rider (CR 605.1a), so it resolves at once (CR 605.3b) and hands priority
/// straight back: nothing is put on the stack, nothing is asked beyond the
/// colour, and the creature is cast with the damage already dealt.
#[test]
fn a_plains_and_adarkar_wastes_cast_a_white_and_blue_creature() {
    let p0 = PlayerId::new(0);
    let sliver = card_index("ba3aa1eb-722a-47d3-83be-96daddb50265");
    let mut engine = Duel::new(1208, forest())
        .battlefield(0, &[plains(), adarkar_wastes()])
        .hand(0, &[sliver])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let wastes = on_battlefield(&engine, p0, adarkar_wastes()).expect("the Wastes");
    let life = engine.state().players[0].life;

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wastes,
                ability_index: 1,
            },
        )
        .expect("the coloured line");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was offered");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "priority comes straight back: {:?}",
        engine.pending()
    );
    assert!(stack_is_empty(&engine), "a mana ability never stacks");
    assert_eq!(engine.state().players[0].life, life - 1);

    tap_mana_except(&mut engine, p0, wastes);
    cast_with_floating(&mut engine, p0, sliver);
    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    assert_eq!(stack.len(), 1, "the creature, and nothing else: {stack:?}");
    assert_eq!(
        engine
            .state()
            .object(stack[0])
            .and_then(|o| o.card)
            .map(|c| c.index),
        Some(sliver)
    );
}
