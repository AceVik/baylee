//! `cards/2001/src/ody/elephant_ambush.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn elephant_ambush() -> CardIndex {
    card_index("09386931-59bd-481f-bccc-32e8a8abc0f9")
}

fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, card: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let slot = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(slot))
            .unwrap();
    }
}

fn assert_green_elephant_token(engine: &Engine<RegistryLookup>, id: ObjectId) {
    assert_eq!(pt(engine, id), (3, 3));
    let chars = engine.state().object(id).unwrap().characteristics();
    assert_eq!(chars.colors, ColorSet::of(Color::Green));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.subtypes.contains(subtypes::creature::ELEPHANT));
}

/// Casts for {2}{G}{G} to create a 3/3 green Elephant token; fails Flashback
/// on 7 mana, and resolves Flashback {6}{G}{G} to create a second token and
/// exile itself.
#[test]
fn elephant_ambush_creates_elephant_tokens_from_hand_and_flashback() {
    // 11 Forests: 4 spent on {2}{G}{G} from hand leaves 7 untapped for the
    // Flashback {6}{G}{G} insufficient-mana boundary; all 11 untap next turn.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 11])
        .hand(0, &[elephant_ambush()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, elephant_ambush());
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, P0);
    assert_eq!(tokens.len(), 1);
    assert_green_elephant_token(&engine, tokens[0]);
    let ambush = in_graveyard(&engine, P0, elephant_ambush()).expect("in graveyard");

    // 7 floating mana, 0 untapped Forests -> Flashback {6}{G}{G} (8) refuses.
    tap_all_mana(&mut engine, P0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.castable.contains(&ambush),
        "7 mana cannot pay {{6}}{{G}}{{G}}"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: ambush })
            .is_err()
    );

    // Next P0 main with 11 mana untapping: Flashback {6}{G}{G} creates 2nd token and exiles.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    start_flashback(&mut engine, P0, ambush);
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, P0);
    assert_eq!(tokens.len(), 2);
    assert_green_elephant_token(&engine, tokens[1]);
    assert!(in_graveyard(&engine, P0, elephant_ambush()).is_none());
    assert_eq!(engine.state().object(ambush).unwrap().zone, Zone::Exile);
}
