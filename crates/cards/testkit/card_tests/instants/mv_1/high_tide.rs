//! `cards/instants/mv_1/high_tide.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn high_tide() -> CardIndex {
    card_index("dc671205-f2fa-454f-9957-921a6069ad53")
}

/// Taps `land` for mana (ability 0 of a basic land).
#[track_caller]
fn tap_for_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId, land: ObjectId) {
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("the land taps for mana");
}

fn blue(engine: &Engine<RegistryLookup>, seat: PlayerId) -> u32 {
    engine.state().players[usize::from(seat.get())]
        .mana_pool
        .available(ManaColor::Blue)
}

fn green(engine: &Engine<RegistryLookup>, seat: PlayerId) -> u32 {
    engine.state().players[usize::from(seat.get())]
        .mana_pool
        .available(ManaColor::Green)
}

/// High Tide (dc671205-f2fa-454f-9957-921a6069ad53) — "Until end of turn,
/// whenever a player taps an Island for mana, that player adds an additional
/// {U}."
///
/// An Island tapped by me gives {U}{U}, a Forest gives only its {G}, an
/// Island the opponent taps in my turn gives them {U}{U} too, and on the
/// opponent's own turn an Island is back to one {U}.
#[test]
fn high_tide_doubles_island_mana_for_every_player_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[high_tide()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let mine = all_on_battlefield(&engine, p0, island());
    let wood = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    let their_island = on_battlefield(&engine, p1, island()).expect("their Island is out");

    tap_for_mana(&mut engine, p0, mine[0]);
    cast_with_floating(&mut engine, p0, high_tide());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(blue(&engine, p0), 0, "the first Island paid for High Tide");

    tap_for_mana(&mut engine, p0, mine[1]);
    assert_eq!(blue(&engine, p0), 2, "an Island now adds {{U}}{{U}}");

    tap_for_mana(&mut engine, p0, wood);
    assert_eq!(green(&engine, p0), 1, "a Forest adds its own {{G}}");
    assert_eq!(blue(&engine, p0), 2, "and no extra {{U}}");

    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("p0 passes");
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "the opponent holds priority next");
    tap_for_mana(&mut engine, p1, their_island);
    assert_eq!(
        blue(&engine, p1),
        2,
        "\"a player\": the opponent's Island adds {{U}}{{U}} as well"
    );

    reach_their_main_phase(&mut engine, p1);
    tap_for_mana(&mut engine, p1, their_island);
    assert_eq!(
        blue(&engine, p1),
        1,
        "next turn the Island is back to a single {{U}}"
    );
}
