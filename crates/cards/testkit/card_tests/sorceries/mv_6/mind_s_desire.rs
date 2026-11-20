//! `cards/sorceries/mv_6/mind_s_desire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_engine::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shuffle_count(engine: &Engine<RegistryLookup>, who: PlayerId) -> usize {
    engine
        .journal()
        .entries()
        .iter()
        .filter(|entry| {
            matches!(
                entry.event,
                GameEvent::Shuffled {
                    player,
                    zone: Zone::Library
                } if player == who
            )
        })
        .count()
}

fn exiled_cards_owned_by(engine: &Engine<RegistryLookup>, who: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(who))
        .iter()
        .copied()
        .filter(|id| engine.state().object(*id).is_some_and(|o| o.owner == who))
        .collect()
}

/// Mind's Desire — {4}{U}{U} Sorcery:
/// "Shuffle your library. Then exile the top card of your library. Until end of turn,
/// you may play that card without paying its mana cost.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn.)"
#[test]
fn mind_s_desire_shuffles_exiles_per_storm_casts_free_and_expires_at_eot() {
    let desire = card_index("b3739b5a-5731-4c64-a244-815a363b0d5c");
    let mut engine = Duel::new(SEED, index::GRIZZLY_BEARS)
        .hand(0, &[index::DARK_RITUAL, desire])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let shuffles_before = shuffle_count(&engine, P0);
    let lib_before = library_len(&engine, P0);
    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool.add(ManaColor::Black, 1);
        state.players[0].mana_pool.add(ManaColor::Blue, 6);
    }
    engine.refresh_offer();

    // 1 prior spell -> Storm creates 1 copy -> 2 shuffles and 2 cards exiled.
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, desire);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().per_turn.spells_cast, vec![2, 0]);
    assert_eq!(
        shuffle_count(&engine, P0) - shuffles_before,
        2,
        "each resolution shuffles the caster's library"
    );
    assert_eq!(library_len(&engine, P0), lib_before - 2);
    let exiled = exiled_cards_owned_by(&engine, P0);
    assert_eq!(exiled.len(), 2, "original + 1 Storm copy exile 2 cards");

    // Empty P0's mana pool completely to prove casting from exile is free.
    engine.dev_state_mut(P0).unwrap().players[0].mana_pool = baylee_core::mana::ManaPool::default();
    engine.refresh_offer();

    let first = exiled[0];
    let second = exiled[1];
    engine
        .apply(P0, PlayerAction::CastSpell { card: first })
        .expect("exiled Grizzly Bears can be cast without paying its mana cost this turn");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(first).map(|o| o.zone),
        Some(Zone::Battlefield),
        "free exiled creature resolved onto the battlefield"
    );

    // Leave the second exiled card in exile and move to P0's next turn: permission expires at EOT.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 2);
    engine.refresh_offer();
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: second })
            .is_err(),
        "exiled card from Mind's Desire cannot be cast after end of turn"
    );
}

#[test]
fn mind_s_desire_allows_playing_exiled_land_this_turn_only() {
    let desire = card_index("b3739b5a-5731-4c64-a244-815a363b0d5c");
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[index::DARK_RITUAL, desire])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool.add(ManaColor::Black, 1);
        state.players[0].mana_pool.add(ManaColor::Blue, 6);
    }
    engine.refresh_offer();

    // 1 prior spell -> 2 Forests exiled from library.
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, desire);
    pass_until(&mut engine, stack_is_empty);

    let exiled = exiled_cards_owned_by(&engine, P0);
    assert_eq!(exiled.len(), 2);
    let first_land = exiled[0];
    let second_land = exiled[1];

    // "play that card" includes playing an exiled land (using the normal land drop).
    engine
        .apply(P0, PlayerAction::PlayLand { card: first_land })
        .expect("exiled Forest can be played this turn");
    assert_eq!(
        engine.state().object(first_land).map(|o| o.zone),
        Some(Zone::Battlefield)
    );

    // Next turn, the remaining exiled Forest is no longer playable.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        engine
            .apply(P0, PlayerAction::PlayLand { card: second_land })
            .is_err(),
        "exiled land permission must expire at end of turn"
    );
}
