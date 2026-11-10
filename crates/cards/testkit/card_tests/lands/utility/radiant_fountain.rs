//! `cards/lands/utility/radiant_fountain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Radiant Fountain prints two lines: "When this land enters, you gain 2 life"
/// and "{T}: Add {C}". The land is played as a real land drop rather than
/// seated with `starting_battlefield`, because a seeded permanent is *placed*
/// and no entry is ever run — a board built that way could not tell the
/// printed life gain from a card that has none.
///
/// The colourless is the half worth reading on the board: the card prints no
/// basic land type, so the engine has no CR 305.6 shortcut to give it and the
/// "{{C}}" can only come off the land's own printed `{T}` ability. Nothing
/// else on the battlefield makes mana at all, so the one mana in the pool
/// afterwards is exact.
#[test]
fn radiant_fountain_gains_two_life_as_it_enters_and_taps_for_one_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[radiant_fountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the land is played"
    );

    let fountain = play_land(&mut engine, p0, radiant_fountain());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "\"When this land enters, you gain 2 life\" — the entry trigger fired, \
         which a `starting_battlefield` placement would never have done"
    );
    assert!(
        on_battlefield(&engine, p0, radiant_fountain()).is_some(),
        "the land reached the battlefield"
    );
    assert!(
        !is_tapped(&engine, fountain),
        "and it enters untapped, so its {{T}} is still there to pay"
    );

    // A mana ability a card prints carries an index to name — it is not the
    // CR 305.6 shortcut a basic land uses — so the ability is taken out of
    // the offer rather than guessed at.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == fountain)
        .expect("the one line the land prints is offered: {:?}");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ability that was offered is affordable");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\" — one colourless, off the land's own tap"
    );
    assert_eq!(
        pool.total(),
        1,
        "and nothing else: the board holds no other mana source at all"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
    assert!(is_tapped(&engine, fountain), "the land paid its own {{T}}");
}
