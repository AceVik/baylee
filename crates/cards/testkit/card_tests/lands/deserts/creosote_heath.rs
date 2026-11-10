//! `cards/lands/deserts/creosote_heath.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Creosote Heath prints three sentences and all three are played here: it
/// enters tapped, it deals 1 damage to target opponent as it arrives, and it
/// taps for {G} or {W}. The land is p0's only permanent, so the point of
/// damage has no other source on the board, and the tapped entry is what
/// makes the mana line unreachable until a whole turn cycle has stood the
/// land back up (CR 502.3) — a `{T}` on a permanent that came in tapped is
/// not payable in the turn it arrived.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn creosote_heath_arrives_tapped_burns_an_opponent_and_taps_for_colored_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[creosote_heath()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_life = engine.state().players[1].life;
    let land = play_land(&mut engine, p0, creosote_heath());
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\" — so this turn it produces nothing"
    );

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseTargets { .. } | Pending::ChoosePlayer { .. }
        )
    });
    match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the land's controller aims its own trigger");
            assert_eq!(
                player_options,
                vec![p1],
                "the one opponent is the whole menu"
            );
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .unwrap();
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the land's controller aims its own trigger");
            assert_eq!(options, vec![p1], "the one opponent is the whole menu");
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        other => panic!("the enters-trigger never asked for an opponent: {other:?}"),
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        their_life - 1,
        "\"it deals 1 damage to target opponent\""
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the point belongs to the seat the controller named, not to the controller"
    );

    // A turn cycle, because the land came in tapped and its `{T}` is not
    // payable until its own untap step has stood it back up (CR 502.3).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step ran");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == land)
        .expect("a printed mana ability is an ordinary entry in `abilities`");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{W}}\" is two producible colours, so it is a question: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names the colour");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "both of the colours the land prints: {options:?}"
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
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap — the land is the only permanent on the board"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
