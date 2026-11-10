//! `cards/creatures/mv_3/nissa_resurgent_animist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Nissa, Resurgent Animist`:
/// "Landfall — Whenever a land you control enters, add one mana of any color. Then if this
/// is the second time this ability has resolved this turn, reveal cards from the top of your
/// library until you reveal an Elf or Elemental card. Put that card into your hand and the rest
/// on the bottom of your library in a random order."
///
/// The mana half, one landfall a turn: the test plays a forest under p0's control, answers the
/// resulting `Pending::ChooseColor` prompt with `ManaColor::Blue`, confirms that one blue mana
/// enters the pool, and verifies that an opponent playing a land does not trigger it. The reveal
/// on the second resolution is the test after this one.
#[test]
fn nissa_resurgent_animist_adds_chosen_mana_on_landfall_and_ignores_opponents_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(308, forest())
        .battlefield(0, &[nissa_resurgent_animist()])
        .hand(0, &[forest()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nissa =
        on_battlefield(&engine, p0, nissa_resurgent_animist()).expect("Nissa on battlefield");
    assert_eq!(pt(&engine, nissa), (3, 3), "Nissa has 3/3 stats");

    // p0 plays a land; Nissa triggers and asks for a color choice upon resolution.
    play_land(&mut engine, p0, forest());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseColor");
    };
    assert_eq!(player, p0, "p0 chooses the color");
    assert!(
        options.contains(&ManaColor::Blue),
        "blue is among all five colors"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "Nissa added one blue mana on landfall"
    );

    // Advance to p1's turn and have p1 play a land. Filter::YOUR_LAND must not trigger for opponent's land.
    reach_their_main_phase(&mut engine, p1);
    play_land(&mut engine, p1, forest());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "opponent playing a land does not trigger Nissa"
    );
}

/// "Then if this is the second time this ability has resolved this turn,
/// reveal cards from the top of your library until you reveal an Elf or
/// Elemental card. Put that card into your hand and the rest on the bottom of
/// your library in a random order."
///
/// Two Explorations give three land drops. The first landfall reveals
/// nothing; the second turns over Forest, Forest, Llanowar Elves, takes the
/// Elves and puts both Forests on the bottom; the third — not the second —
/// leaves a second Elves on top of the library where it is.
#[test]
fn nissa_resurgent_animist_reveals_an_elf_on_the_second_landfall_only() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(308, forest())
        .battlefield(
            0,
            &[nissa_resurgent_animist(), exploration(), exploration()],
        )
        .hand(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = hand_to_library_top(&mut engine, p0, llanowar_elves());
    let covers = [
        hand_to_library_top(&mut engine, p0, forest()),
        hand_to_library_top(&mut engine, p0, forest()),
    ];
    let library = ZoneLocation::Library(p0);

    landfall_for_nissa(&mut engine, p0);
    assert_eq!(
        engine.state().zones.list(library).last(),
        Some(&covers[1]),
        "the first resolution reveals nothing"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&elves)
    );

    let journal_from = engine.state().journal.len();
    landfall_for_nissa(&mut engine, p0);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&elves),
        "the second resolution puts the Elf into the hand"
    );
    let revealed: Vec<ObjectId> = engine.state().journal.entries()[journal_from..]
        .iter()
        .find_map(|e| match &e.event {
            GameEvent::Revealed { cards, .. } => Some(cards.clone()),
            _ => None,
        })
        .expect("the cards turned over are revealed");
    assert_eq!(
        revealed,
        vec![covers[1], covers[0], elves],
        "top down, until the Elf"
    );
    let bottom = &engine.state().zones.list(library)[..2];
    assert!(
        bottom.contains(&covers[0]) && bottom.contains(&covers[1]),
        "the rest go on the bottom"
    );

    let second = hand_to_library_top(&mut engine, p0, llanowar_elves());
    landfall_for_nissa(&mut engine, p0);
    assert_eq!(
        engine.state().zones.list(library).last(),
        Some(&second),
        "the third resolution is not the second, so nothing is revealed"
    );
}
