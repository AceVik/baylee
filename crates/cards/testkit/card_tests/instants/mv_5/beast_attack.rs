//! `cards/2001/src/ody/beast_attack.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;

fn beast_attack() -> CardIndex {
    card_index("3f5acd90-199c-4ed3-9c8a-eeac4670d876")
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

fn assert_green_beast_token(engine: &Engine<RegistryLookup>, id: ObjectId) {
    assert_eq!(pt(engine, id), (4, 4));
    let chars = engine.state().object(id).unwrap().characteristics();
    assert_eq!(chars.colors, ColorSet::of(Color::Green));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.subtypes.contains(subtypes::creature::BEAST));
}

/// Casts for {2}{G}{G}{G} to create a 4/4 green Beast token, refuses Flashback
/// with 4 mana, and casts Flashback {2}{G}{G}{G} for a second token while
/// exiling Beast Attack.
#[test]
fn beast_attack_creates_beast_tokens_from_hand_and_flashback() {
    // 9 Forests: 5 spent on {2}{G}{G}{G} from hand leaves 4 untapped for the
    // 4-mana Flashback refusal boundary; all 9 untap next P0 main.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 9])
        .hand(0, &[beast_attack()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, me()));

    cast_from_hand(&mut engine, me(), beast_attack());
    settle(&mut engine);

    let tokens = my_tokens(&engine);
    assert_eq!(tokens.len(), 1);
    assert_green_beast_token(&engine, tokens[0]);
    let card = in_graveyard(&engine, me(), beast_attack()).expect("in graveyard");

    // 4 floating mana, 0 untapped Forests -> Flashback {2}{G}{G}{G} (5) refuses.
    tap_all_mana(&mut engine, me());
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.castable.contains(&card),
        "4 mana cannot pay {{2}}{{G}}{{G}}{{G}}"
    );
    assert!(
        engine
            .apply(me(), PlayerAction::CastSpell { card })
            .is_err()
    );

    reach_their_main_phase(&mut engine, them());
    reach_their_main_phase(&mut engine, me());
    tap_all_mana(&mut engine, me());
    start_flashback(&mut engine, me(), card);
    settle(&mut engine);

    let tokens = my_tokens(&engine);
    assert_eq!(tokens.len(), 2);
    assert_green_beast_token(&engine, tokens[1]);
    assert!(in_graveyard(&engine, me(), beast_attack()).is_none());
    assert!(exiled(&engine, me(), beast_attack()));
}
