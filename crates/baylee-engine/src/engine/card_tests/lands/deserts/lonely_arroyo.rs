//! `cards/lands/deserts/lonely_arroyo.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lonely Arroyo — a Desert that "enters tapped", "deals 1 damage to target
/// opponent" as it enters, and taps for {W} or {U}. The tap is read the moment
/// the land lands and *before* the trigger is answered, so an entry that
/// forgot `EnterModifier::Tapped` cannot hide behind the damage; the trigger's
/// menu is read for the seat it leaves out, because "target opponent"
/// (CR 115.4) is not "target player". The mana half needs a whole turn cycle —
/// a land that arrived tapped has no `{T}` to pay with until its own untap
/// step — so the ability is activated on p0's *next* main phase, where the
/// Arroyo is the only source on the board and the colour that reaches the pool
/// can only have come off its own tap.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn lonely_arroyo_enters_tapped_shocks_the_opponent_and_taps_for_white_or_blue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[lonely_arroyo()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    let land = play_land(&mut engine, p0, lonely_arroyo());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — read while the enters-trigger is still \
         unanswered, so nothing else can be blamed for the tap"
    );

    // The trigger is put on the stack the moment the land is on the
    // battlefield, so its target is asked before anybody holds priority
    // (CR 603.3d). A target that is only a player arrives as one of the two
    // choice forms; both are the same answer.
    match engine.pending().clone() {
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the land's controller chooses");
            assert!(
                player_options.contains(&p1) && !player_options.contains(&p0),
                "\"target opponent\" names the other seat and never its own: {player_options:?}"
            );
            assert_eq!(player_options.len(), 1, "a duel has one opponent");
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
            assert_eq!(player, p0, "the land's controller chooses");
            assert!(
                options.contains(&p1) && !options.contains(&p0),
                "\"target opponent\": {options:?}"
            );
            assert_eq!(options.len(), 1, "a duel has one opponent");
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        other => panic!("expected the enters-trigger's target choice, got {other:?}"),
    }

    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage is on the stack, not in the life total yet"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"it deals 1 damage to target opponent\""
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the seat the land named"
    );

    // A land that entered tapped gives nothing in the turn it arrived, so the
    // {T} line is read on p0's next main phase, past its own untap step.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "p0 holds priority in its own main phase: {:?}",
            engine.pending()
        )
    };
    // A printed `{T}: Add …` on a nonbasic is a mana ability the card names,
    // so it may arrive on either list (#159); the land is untapped and its
    // whole price is its own tap, so one of them carries it.
    let route = if legal.mana_abilities.contains(&land) {
        PlayerAction::ActivateManaAbility { source: land }
    } else {
        let (source, ability_index) = legal
            .abilities
            .iter()
            .copied()
            .find(|(id, _)| *id == land)
            .expect("an untapped land offers the {T} ability it prints");
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        }
    };
    engine.apply(p0, route).unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Blue),
        "the two colours the land prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and colorless is no colour at all (CR 105.4): {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named"
    );
    assert_eq!(
        pool.total(),
        1,
        "the Arroyo is the only source on this board"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "{{T}} paid for it");
}
