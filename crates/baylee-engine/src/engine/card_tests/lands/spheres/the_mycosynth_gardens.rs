//! `cards/lands/spheres/the_mycosynth_gardens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The printed colorless and filtering mana activations pay their tap costs.
#[test]
fn the_mycosynth_gardens_taps_for_colorless_and_spends_one_for_any_color() {
    let p0 = PlayerId::new(0);

    // {T}: Add {C}. Played as the turn's land drop rather than seeded onto
    // the battlefield, so the permanent that pays is the one the card became.
    let mut simple = Duel::new(SEED, forest())
        .hand(0, &[the_mycosynth_gardens()])
        .start();
    keep_mulligans(&mut simple);
    reach_main_phase(&mut simple, p0);
    let garden = play_land(&mut simple, p0, the_mycosynth_gardens());
    let taken = tap_all_mana(&mut simple, p0);
    assert_eq!(taken, 1, "the Gardens is the only mana source on the board");
    let pool = &simple.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&simple, garden), "the printed {{T}} is what paid");

    // {1}, {T}: Add one mana of any color. A Forest floats one green and the
    // Gardens is kept back, so the {1} is a payment out of the pool and the
    // tap that follows buys a colour the board could not have produced.
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[the_mycosynth_gardens()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let garden = play_land(&mut engine, p0, the_mycosynth_gardens());
    tap_all_mana_but(&mut engine, p0, Some(the_mycosynth_gardens()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Forest pays the {{1}}; the Gardens keeps its {{T}}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one green floating, and that is what the {{1}} is spent from"
    );

    // Ability 1 is the "{1}, {T}" line; ability 0 is the printed {T} alone.
    activate(&mut engine, p0, the_mycosynth_gardens(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
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
        "the color that was named — nothing else on this board makes black"
    );
    assert_eq!(
        pool.total(),
        1,
        "the {{1}} took the Forest's mana and the tap bought one back"
    );
    assert!(
        is_tapped(&engine, garden),
        "the {{T}} half of the price was paid too"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}

#[test]
fn gardens_requires_its_chosen_x_and_keeps_the_copy_after_the_turn() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[the_mycosynth_gardens(), index::MIND_STONE, index::SOL_RING],
        )
        .battlefield(1, &[index::MIND_STONE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let garden = on_battlefield(&e, p0, the_mycosynth_gardens()).unwrap();
    let stone = on_battlefield(&e, p0, index::MIND_STONE).unwrap();
    let other = on_battlefield(&e, p1, index::MIND_STONE).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    e.refresh_offer();
    activate(&mut e, p0, the_mycosynth_gardens(), 2);
    assert!(matches!(e.pending(), Pending::ChooseNumber { .. }));
    e.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(options, &vec![stone]);
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![other],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    aim_at(&mut e, p0, stone);
    pass_until(&mut e, stack_is_empty);
    let copied = e.state().object(garden).unwrap().characteristics();
    assert_eq!(copied.types, TypeSet::ARTIFACT);
    assert_eq!(copied.mana_value(), 2);
    assert!(is_tapped(&e, garden));
    reach_their_main_phase(&mut e, p1);
    assert_eq!(
        e.state().object(garden).unwrap().characteristics().types,
        TypeSet::ARTIFACT
    );
}

/// "target nontoken artifact you control with mana value X": a Treasure token
/// is an artifact you control with mana value 0 and still not a target for
/// X = 0, where the nontoken Mox beside it is.
#[test]
fn gardens_never_offers_a_token_artifact_as_the_copy_target() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                the_mycosynth_gardens(),
                index::FOUNTAINPORT,
                index::MOX_OPAL,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let mox = on_battlefield(&e, p0, index::MOX_OPAL).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, p0, index::FOUNTAINPORT, 3);
    pass_until(&mut e, stack_is_empty);
    let tokens = tokens_of(&e, p0);
    assert_eq!(tokens.len(), 1, "Fountainport made its Treasure");
    let treasure = tokens[0];
    let made = e.state().object(treasure).unwrap().characteristics();
    assert!(
        made.types.contains(TypeSet::ARTIFACT) && made.mana_value() == 0,
        "the Treasure is an artifact of mana value 0, a match for everything but \"nontoken\""
    );

    activate(&mut e, p0, the_mycosynth_gardens(), 2);
    assert!(matches!(e.pending(), Pending::ChooseNumber { .. }));
    e.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    let options = aim_at(&mut e, p0, mox);
    assert_eq!(
        options,
        vec![mox],
        "X = 0: the nontoken Mox is the only target, the Treasure token is not"
    );
}
