//! `cards/lands/gain/tranquil_cove.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tranquil Cove prints three sentences: it enters tapped, it gains its
/// controller 1 life as it arrives, and `{T}` adds `{W}` or `{U}`. All three
/// are read in one game, and each keeps the others honest — the life total
/// only moves if the enters-trigger really resolved, and the colour question
/// is only asked once the land has stood back up, which is a turn later than
/// the turn it arrived tapped in (CR 502.3). The mana line is activated
/// through the offer rather than guessed at, because a nonbasic land's own
/// tap is an ordinary `(source, index)` ability and not the CR 305.6
/// shortcut (`LegalActions::mana_abilities`), which this land has no basic
/// land type to qualify for.
#[test]
fn tranquil_cove_enters_tapped_gains_a_life_and_taps_for_white_or_blue() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[tranquil_cove()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, tranquil_cove());
    // Walked until the life total moves, which is the enters-trigger
    // resolving rather than merely being put on the stack behind the walk.
    pass_until(&mut engine, |e| e.state().players[0].life == 21);
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" — the permanent is down and already spent"
    );

    // A permanent that entered tapped untaps in its controller's next untap
    // step and nowhere earlier, so the mana line is read a turn on.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Cove's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran, so the land is standing again"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "an untapped land in a main phase holds priority, got {:?}",
            engine.pending()
        )
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == land)
        .expect("`{T}: Add {W} or {U}` is a printed mana ability (CR 605.1)");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the activation the offer itself named");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{W}} or {{U}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert_eq!(options.len(), 2, "two colours and no more: {options:?}");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Blue),
        "the two colours the land prints: {options:?}"
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
    assert_eq!(pool.total(), 1, "one mana off one tap, and nothing else");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
