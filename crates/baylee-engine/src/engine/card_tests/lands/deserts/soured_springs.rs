//! `cards/lands/deserts/soured_springs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Soured Springs prints three lines and one turn cycle reads all three: it
/// enters tapped, it deals 1 damage to target opponent as it arrives, and it
/// taps for `{U}` or `{B}`.
///
/// The tap is why the scenario needs a second turn. The land arrives tapped,
/// so its `{T}` is not even on the offer in the turn it was played — which is
/// also what shows `EnterModifier::Tapped` reached the rules rather than only
/// the status line — and only after the untap step does pressing the ability
/// put its two-colour question.
///
/// The damage line is read as a question about *players*: it names one
/// opponent, and the seat that played the land is not on its own menu, so a
/// filter that had lost `AnyOpponent` would be caught here.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn soured_springs_shocks_an_opponent_on_entry_and_taps_for_blue_or_black_a_turn_later() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(2077, forest())
        .hand(0, &[soured_springs()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, soured_springs());
    assert_eq!(
        engine.state().players[1].life,
        20,
        "nothing has been dealt yet: the trigger is still asking"
    );
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the seat that played the land chooses");
            assert!(
                options.contains(&p1),
                "the opponent across the table is a legal target: {options:?}"
            );
            assert!(
                !options.contains(&p0),
                "\"target opponent\" does not offer the controller their own \
                 trigger: {options:?}"
            );
            engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
        }
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the seat that played the land chooses");
            assert!(
                player_options.contains(&p1) && !player_options.contains(&p0),
                "one opponent, and never the controller: {player_options:?}"
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
        other => panic!("the enters trigger asks who takes the damage, got {other:?}"),
    }
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"when this land enters, it deals 1 damage to target opponent\""
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the opponent, not to the land's controller"
    );

    // Tapped on arrival, so the {T} has nothing to pay with yet.
    assert!(is_tapped(&engine, land), "the land entered tapped");
    let mana_offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. }
                if legal.abilities.iter().any(|(source, _)| *source == land)
        )
    };
    assert!(
        !mana_offered(&engine),
        "a land that entered tapped gives nothing this turn: its {{T}} \
         ability is not even offered"
    );

    // A turn cycle later the untap step stands it back up.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn comes back round to the land's controller"
    );
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");

    // Ability 0 is the enters trigger; the mana ability is the line after it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == land)
        .expect("an untapped land with a printed mana ability is offered now");
    assert_eq!(
        ability_index, 1,
        "the trigger took index 0, so the {{T}} line is the second ability"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("no mana and no target stand between the land and its own tap");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`{{U}} or {{B}}` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        2,
        "two colours, and not the five of \"any colour\": {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "the two the card prints, {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one land");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
