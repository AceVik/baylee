//! `cards/lands/deserts/festering_gulch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Festering Gulch prints three lines and the scenario walks them in the
/// order a turn puts them in, because each one gates the next. It arrives
/// tapped, so its printed `{T}` is not among the offers for the rest of that
/// turn — the mana ability is read only after the land has been untapped by
/// its controller's own untap step. The middle line is a real trigger on the
/// stack: `settle_aiming_at` reports that a target was asked for and the life
/// total moves on the opponent, never on the land's controller. And the
/// colour question offers exactly the two colours the card prints, so a
/// default or a single-colour reading would be visible.
#[test]
fn festering_gulch_arrives_tapped_burns_an_opponent_and_taps_for_black_or_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[festering_gulch()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, festering_gulch());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");

    // The enters-trigger is the only thing on the stack, and its target is an
    // opponent: a player, not a permanent.
    assert!(
        settle_aiming_at(&mut engine, p1),
        "the arrival asks for a target, so the trigger really reached the stack"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"When this land enters, it deals 1 damage to target opponent\""
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the opponent, not to the land's controller"
    );

    // A land that entered tapped is still down, so a `{T}` cost cannot be
    // paid and the ability is not even in the offer (CR 118.3).
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected a quiet priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the active player's own main phase");
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped permanent offers no {{T}} ability: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back: the untap step is the only thing
    // that can stand the land up, since it arrived tapped.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it up");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing on this board has been tapped for mana yet"
    );

    // The printed `{T}: Add {B} or {G}` is an ordinary entry in `abilities`
    // (CR 605.1), so the index is read out of the offer rather than guessed
    // past the enters-trigger that sits in front of it. No mana is needed to
    // be offered, so nothing has to be floated first.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == land)
        .expect("the land's only activation is its own mana ability");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("an untapped land pays its own {{T}}");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Green],
        "both printed colours, and only those two"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Black), 0, "and not the other one");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
