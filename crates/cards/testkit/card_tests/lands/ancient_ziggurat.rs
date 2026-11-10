//! `cards/lands/ancient_ziggurat.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ancient Ziggurat prints one line: "{T}: Add one mana of any color. Spend
/// this mana only to cast a creature spell." Both halves are played, because
/// each is what keeps the other honest — a restriction never taken up proves
/// nothing about the mana, and mana never refused proves nothing about the
/// word after the period. The two cards in hand cost the same {B}: Festering
/// Goblin is a creature spell and Dark Ritual is not, and nothing else on the
/// board could pay for either, so one tapping of the land separates what the
/// restricted black mana may buy from what it may not.
#[test]
fn ancient_ziggurat_pays_for_a_creature_spell_and_refuses_an_instant() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ancient_ziggurat()])
        .hand(0, &[festering_goblin(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // "any color" is a question and not a default: the five colors are on the
    // menu, and colorless is not one of them (CR 105.4).
    activate(&mut engine, p0, ancient_ziggurat(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "the activating seat names the color");
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
        "the five colors of the game, and colorless is no color at all: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    let land = on_battlefield(&engine, p0, ancient_ziggurat()).expect("the Ziggurat is out");
    assert!(is_tapped(&engine, land), "the {{T}} paid for it");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0,
        "the mana is a *restricted* entry — `available` reads the simple pool \
         and finds none of it; the creature spell below is what shows it exists"
    );

    // Both spells cost {B} and the only mana in the game is the one the
    // Ziggurat just made, so what is on this list is the restriction speaking.
    let goblin = in_hand(&engine, p0, festering_goblin()).expect("the Goblin is in hand");
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.castable.contains(&goblin),
        "the {{B}} creature spell the mana is for is castable: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&ritual),
        "\"spend this mana only to cast a creature spell\" — the {{B}} instant \
         is refused with that very mana floating: {:?}",
        legal.castable
    );

    // And the mana really is spent: the creature arrives and the pool empties.
    cast_with_floating(&mut engine, p0, festering_goblin());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some(),
        "the black mana the Ziggurat made paid for the creature"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and there is nothing left over in the pool"
    );
}
