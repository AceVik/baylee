//! `cards/creatures/mv_2/tangled_florahedron.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tangled Florahedron, the front face: `{1}{G}` Creature — Elemental 1/1
/// whose whole printed text is "{T}: Add {G}."
///
/// A mana creature and a mana land print the same sentence and differ by one
/// rule, and that rule is what this test is about: CR 302.6's second
/// sentence forbids activating an ability with `{T}` in its cost unless the
/// creature has been under its controller's control since their turn began,
/// so the Florahedron makes no mana on the turn it is cast and makes it on
/// the next. Both halves are needed — the silence alone would also be true
/// of a card whose mana ability nobody wrote.
///
/// The board is read once before the cast for the other printed fact: one
/// card, two ways to play it (CR 712.12), so the engine offers the same
/// object as a creature spell *and* as a land drop in one priority window.
#[test]
fn a_florahedron_cast_as_a_creature_waits_a_turn_before_it_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[tangled_florahedron()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Forests first: `castable` is answered against the *pool*, so a
    // spell nobody has floated mana for is not on the list at all.
    let card = in_hand(&engine, p0, tangled_florahedron()).expect("the card is in hand");
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a main phase grants priority");
    };
    assert!(
        legal.castable.contains(&card),
        "the front face is a {{1}}{{G}} creature spell",
    );
    assert!(
        legal.lands.contains(&card),
        "and the same card in the same window is a land drop (CR 712.12): \
         one card, two ways to play it",
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("an offered cast is castable");
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && on_battlefield(e, p0, tangled_florahedron()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let flora = on_battlefield(&engine, p0, tangled_florahedron()).expect("the spell resolved");
    assert_eq!(
        engine
            .state()
            .object(flora)
            .expect("it is in play")
            .face_index,
        0,
        "a cast reaches the front face, never the land on the back",
    );
    assert_eq!(pt(&engine, flora), (1, 1), "Tangled Florahedron is a 1/1");
    assert!(
        types(&engine, flora).contains(TypeSet::CREATURE),
        "and it is a creature, which is what puts CR 302.6 over it",
    );

    // `pass_until` stopped on p0's own priority, so this is p0's offer.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the resolution hands priority back");
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == flora),
        "CR 302.6: \"{{T}}: Add {{G}}\" is not offered the turn it arrived",
    );

    florahedron_to_next_main(&mut engine, p0);
    assert!(
        !is_tapped(&engine, flora),
        "nothing has tapped it in the meantime, so {{T}} is a cost it can \
         still pay",
    );
    activate(&mut engine, p0, tangled_florahedron(), 0);
    assert!(is_tapped(&engine, flora), "paying {{T}} left it tapped");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "\"{{T}}: Add {{G}}\" — and it is green",
    );
}

/// Tangled Vale, the back face: "Land. This land enters tapped. {T}: Add
/// {G}."
///
/// Only one of this card's two faces is a land, so CR 712.12's face choice
/// is not a *question* here — the engine switches to that face and plays it,
/// which is Glasspool Shore's shape and not a pathway's. What the scenario
/// is really about is the two sentences printed under the type line: the
/// land arrives **tapped**, so its `{T}` is not offered on the turn it was
/// played, and once it untaps the same ability adds {G} without waiting the
/// turn the creature face owes — CR 302.6 speaks of creatures, and this face
/// is not one.
///
/// Both negative halves are read off the priority the land drop itself hands
/// back, and that priority is pinned to p0 before it is read: `legal`
/// belonging to the other seat would be empty of this land whatever the card
/// said, and the assertion would pass on a Vale that came down untapped.
#[test]
fn a_florahedron_played_as_tangled_vale_enters_tapped_and_taps_for_green_next_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(37, forest())
        .hand(0, &[tangled_florahedron()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vale = play_land(&mut engine, p0, tangled_florahedron());
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "one land face, so nothing is asked (CR 712.12): the engine takes the \
         face the card prints, plays it, and hands the same seat its priority \
         back — got {:?}",
        engine.pending(),
    );
    let obj = engine.state().object(vale).expect("the land is in play");
    assert_eq!(
        obj.face_index, 1,
        "a land drop reaches the back face (CR 712.12)",
    );
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Tangled Vale",
        "and it is that face's own name on the battlefield",
    );
    assert!(
        obj.characteristics().types.contains(TypeSet::LAND),
        "Tangled Vale is a Land",
    );
    assert!(
        !obj.characteristics().types.contains(TypeSet::CREATURE),
        "and nothing of the Elemental on the other side came with it",
    );
    assert!(
        entered_tapped(&engine, vale),
        "\"This land enters tapped\" — and this is a real land drop, which is \
         the only way a replacement effect gets to look at it",
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the land play hands priority back");
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == vale),
        "a land that entered tapped cannot pay {{T}}, so the mana ability is \
         not offered on the turn it was played",
    );

    florahedron_to_next_main(&mut engine, p0);
    assert!(
        !is_tapped(&engine, vale),
        "the untap step gives it back, which is what \"enters tapped\" costs: \
         one turn and no more",
    );
    activate(&mut engine, p0, tangled_florahedron(), 0);
    assert!(is_tapped(&engine, vale), "paying {{T}} left it tapped");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one activation, one mana");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "\"{{T}}: Add {{G}}\" on the back face too",
    );
}
