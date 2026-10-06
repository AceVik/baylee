//! `cards/lands/unclaimed_territory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Unclaimed Territory prints three sentences: it asks for a creature type as
/// it enters, taps for `{C}`, and taps for one mana of any color that may be
/// spent only on a creature spell of the chosen type. The land is played out
/// of hand rather than seeded onto the battlefield, so the entry choice is the
/// printing's own first line and not a placement that never ran it, and the
/// second ability is then spent on Giant Growth — a green spell that costs
/// exactly what the pool holds and is still refused, because it is no
/// creature. A Forest tapped in the same phase pays the same `{G}` without the
/// rider, which is what separates the printed restriction from a spell that
/// was never affordable.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn unclaimed_territory_chooses_a_type_and_its_colored_mana_only_pays_creature_spells() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(193, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .hand(0, &[unclaimed_territory(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The entry itself asks, so the question is the proof that the first
    // printed line ran.
    let card = in_hand(&engine, p0, unclaimed_territory()).expect("the land is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card })
        .expect("a land drop on a board that has played none yet");
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!(
            "\"as this land enters, choose a creature type\", got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "its controller chooses the type");
    assert!(!options.is_empty(), "the creature types are the menu");
    engine
        .apply(p0, PlayerAction::ChooseSubtype(options[0]))
        .expect("the first creature type is a legal choice");
    let land =
        on_battlefield(&engine, p0, unclaimed_territory()).expect("the land is on the battlefield");

    // Both mana abilities on the still-untapped land. A nonbasic land's own
    // {T} has a printed ability and an index, so it is an ordinary entry in
    // `abilities` rather than the CR 305.6 shortcut.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "the seat holds priority after its own land drop: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert!(
        legal.abilities.contains(&(land, 0)),
        "\"{{T}}: Add {{C}}\" is offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(land, 1)),
        "\"{{T}}: Add one mana of any color\" is offered: {:?}",
        legal.abilities
    );

    // Ability 1 is the coloured one; both want the same {T}, so only one of
    // the two can be read off this land.
    activate(&mut engine, p0, unclaimed_territory(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the color");
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );
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
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colors it offered");
    // Not `available`, which reads the plain pool: this half of the card
    // prints "spend this mana only to cast a creature spell of the chosen
    // type", so what it makes is `RestrictedMana` and a plain read finds
    // nothing at all. The restriction is the card, not an implementation
    // detail beside it.
    let restricted = engine.state().players[0].mana_pool.restricted().to_vec();
    assert_eq!(
        restricted
            .iter()
            .filter(|m| m.color == ManaColor::Green)
            .map(|m| u32::from(m.amount))
            .sum::<u32>(),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana, off one tap"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );

    // The printed rider. Giant Growth is green and the pool holds exactly the
    // green it costs — and it is still not castable, because the only green
    // there may be spent on a *creature* spell.
    let growth = in_hand(&engine, p0, giant_growth()).expect("the Growth is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the seat holds priority after the mana ability: {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.castable.contains(&growth),
        "\"spend this mana only to cast a creature spell\" refuses a spell \
         that is not one: {:?}",
        legal.castable
    );

    // The control: an untapped Forest makes the same {G} without the rider,
    // so the refusal above is the restriction and not the cost. The Elves are
    // kept back — they are the creature the spell would aim at, and a source
    // tapped for mana is mana this reading does not want.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the restricted green and the Forest's unrestricted green"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again: {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&growth),
        "the same spell, castable once a source without the rider is in the \
         pool: {:?}",
        legal.castable
    );
}
