//! `cards/instants/mv_7/hunting_pack.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::{index, subtypes};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Hunting Pack — {5}{G}{G} Instant:
/// "Create a 4/4 green Beast creature token.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn.)"
#[test]
fn hunting_pack_creates_one_4_4_green_beast_token_per_storm_copy_plus_original() {
    let pack = card_index("d3e1848a-c236-4352-a7be-b0b4699af968");
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[index::DARK_RITUAL, pack])
        .hand(1, &[index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool.add(ManaColor::Black, 1);
        state.players[0].mana_pool.add(ManaColor::Green, 7);
        state.players[1].mana_pool.add(ManaColor::Black, 1);
    }
    engine.refresh_offer();

    // Two prior spells (one from each player) -> Storm creates 2 copies -> 3 tokens total.
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut engine, P1, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, pack);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().per_turn.spells_cast,
        vec![2, 1],
        "Storm copies do not count as cast spells"
    );
    let tokens = tokens_of(&engine, P0);
    assert_eq!(tokens.len(), 3, "original + 2 Storm copies create 3 Beasts");
    for token in tokens {
        let obj = engine.state().object(token).unwrap();
        let ch = obj.characteristics();
        assert_eq!(pt(&engine, token), (4, 4));
        assert_eq!(ch.colors, ColorSet::from_slice(&[Color::Green]));
        assert!(ch.types.contains(TypeSet::CREATURE));
        assert!(ch.subtypes.contains(subtypes::creature::BEAST));
    }
}
