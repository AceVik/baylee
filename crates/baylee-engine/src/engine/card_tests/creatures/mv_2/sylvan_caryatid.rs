//! `cards/creatures/mv_2/sylvan_caryatid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sylvan Caryatid prints a {1}{G} 0/3 Plant with defender and hexproof and
/// one ability: "{T}: Add one mana of any color." Neither half is a static a
/// card file could be trusted to have — the mana arrives as a question the
/// engine asks as the {T} is paid, and the defender only means anything in
/// the declaration the plant stands untapped for. So the scenario plays both
/// in one turn: the attack step first, where the only creature its controller
/// has is refused while it is untapped and otherwise able, and then the tap,
/// where the color named is one nothing else on the board can make — the two
/// Forests that cast it are tapped and green.
#[test]
fn sylvan_caryatid_may_not_attack_and_taps_for_a_color_nobody_else_can_make() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[sylvan_caryatid()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{G} out of the two Forests, which is everything the pool had.
    cast_from_hand(&mut engine, p0, sylvan_caryatid());
    pass_until(&mut engine, stack_is_empty);
    let plant = on_battlefield(&engine, p0, sylvan_caryatid()).expect("the Caryatid resolved");

    assert_eq!(pt(&engine, plant), (0, 3), "the body it prints");
    let printed = keywords(&engine, plant);
    assert!(printed.contains(KeywordSet::DEFENDER), "Defender");
    assert!(printed.contains(KeywordSet::HEXPROOF), "hexproof");

    // Defender, read where it decides something: the plant is untapped, on
    // the battlefield, and its controller is the one attacking, so nothing is
    // left to account for its absence but the keyword (CR 702.3b).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the Caryatid's controller is the active player");
    assert!(
        attackers.is_empty(),
        "the only creature on this board may not attack: {attackers:?}"
    );

    // Back to a priority the untapped plant is still worth spending: a mana
    // ability wants nothing but its own {T} and a moment to use it in.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Priority { player, legal }
                if *player == p0 && legal.abilities.iter().any(|(id, _)| *id == plant)
        )
    });
    assert!(
        !is_tapped(&engine, plant),
        "it stood through the combat step it was not allowed to join"
    );

    let before = engine.state().players[0].mana_pool.total();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!("the predicate above matched a priority")
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == plant)
        .expect("the printed {{T}} is the whole of its text");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("a mana ability needs no stack and no permission");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that tapped names the color");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, on a board whose own lands only make green"
    );
    assert_eq!(pool.total(), before + 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, plant), "the Caryatid paid its own {{T}}");
}
