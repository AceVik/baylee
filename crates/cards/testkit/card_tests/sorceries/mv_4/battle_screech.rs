//! `cards/sorceries/mv_4/battle_screech.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_cards_dsl::{Color, ColorSet};
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);

fn battle_screech() -> CardIndex {
    card_index("e73131cd-b454-405b-9539-9d777e232b9e")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Battle Screech (e73131cd-b454-405b-9539-9d777e232b9e) — {2}{W}{W} Sorcery:
/// "Create two 1/1 white Bird creature tokens with flying. Flashback—Tap three
/// untapped white creatures you control."
#[test]
fn battle_screech_creates_two_birds_and_flashes_back_by_tapping_three_white_creatures() {
    let mut engine = lk::game(
        SEED,
        &[
            plains(),
            plains(),
            plains(),
            plains(),
            savannah_lions(),
            aurochs(),
        ],
        &[],
        &[battle_screech()],
        &[],
    );
    let lions = lk::obj(&engine, P0, savannah_lions());
    let green_beast = lk::obj(&engine, P0, aurochs());

    cast_from_hand(&mut engine, P0, battle_screech());
    lk::settle(&mut engine);

    let first_birds = tokens_of(&engine, P0);
    assert_eq!(first_birds.len(), 2);
    for bird in &first_birds {
        assert_eq!(pt(&engine, *bird), (1, 1));
        assert!(keywords(&engine, *bird).contains(KeywordSet::FLYING));
        let ch = engine.state().object(*bird).unwrap().characteristics();
        assert_eq!(ch.colors, ColorSet::from_slice(&[Color::White]));
        assert!(ch.subtypes.contains(subtypes::creature::BIRD));
    }

    // Flashback for 0 mana by tapping Lions + the two new white Bird tokens!
    let gy_screech = in_graveyard(&engine, P0, battle_screech()).unwrap();
    engine
        .apply(P0, PlayerAction::CastSpell { card: gy_screech })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let fb = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(P0, PlayerAction::ChooseMode(fb)).unwrap();
    }
    for white_creature in [lions, first_birds[0], first_birds[1]] {
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!("expected TapN prompt, got {:?}", engine.pending());
        };
        assert!(!options.contains(&green_beast), "nonwhite excluded");
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![white_creature],
                },
            )
            .unwrap();
    }
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, gy_screech), Zone::Exile);
    assert_eq!(tokens_of(&engine, P0).len(), 4);
    assert!(lk::tapped(&engine, lions));
}
