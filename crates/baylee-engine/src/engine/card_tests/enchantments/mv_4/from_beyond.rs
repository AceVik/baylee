//! `cards/enchantments/mv_4/from_beyond.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn from_beyond() -> CardIndex {
    card_index("31b27775-196b-4022-bff5-25b34290ff7f")
}

fn drowner_of_truth() -> CardIndex {
    card_index("db19a27a-ee22-4931-ae3c-0ce21f456ea6")
}

/// "Devoid" and "At the beginning of your upkeep, create a 1/1 colorless
/// Eldrazi Scion creature token. It has 'Sacrifice this token: Add {C}.'"
#[test]
fn from_beyond_is_colorless_and_makes_a_scion_each_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[from_beyond()])
        .start();
    keep_mulligans(&mut engine);
    let beyond = on_battlefield(&engine, p0, from_beyond()).expect("seated");
    assert!(
        engine
            .state()
            .object(beyond)
            .expect("object")
            .characteristics()
            .colors
            .is_empty(),
        "Devoid: no color even with {{G}} in the cost"
    );

    walk_to_step_trigger(&mut engine, p0, Step::Upkeep);
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Scion");
    assert_eq!(pt(&engine, tokens[0]), (1, 1), "a 1/1");

    reach_main_phase(&mut engine, p0);
    let scion = tokens_of(&engine, p0)[0];
    assert!(
        priority_offer(&engine).abilities.contains(&(scion, 0)),
        "the Scion offers its sacrifice ability"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: scion,
                ability_index: 0,
            },
        )
        .expect("the Scion sacrifices itself for {C}");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "sacrificing the Scion adds one mana"
    );
    assert!(tokens_of(&engine, p0).is_empty(), "and the Scion is gone");
}

/// "{1}{G}, Sacrifice this enchantment: Search your library for an Eldrazi
/// card, reveal it, put it into your hand, then shuffle.": the only Eldrazi
/// card in a library of Forests is the one on the menu.
#[test]
fn from_beyond_sacrifices_to_fetch_an_eldrazi_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[drowner_of_truth()])
        .battlefield(0, &[from_beyond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);
    let drowner = hand_to_library_top(&mut engine, p0, drowner_of_truth());
    float(&mut engine, p0, ManaColor::Green, 2);

    activate(&mut engine, p0, from_beyond(), 1);
    assert!(
        in_graveyard(&engine, p0, from_beyond()).is_some(),
        "the sacrifice is a cost"
    );
    pass_until_choice(&mut engine);
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the search asks which card, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![drowner], "the Eldrazi card, and nothing else");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![drowner],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, drowner_of_truth()).is_some(),
        "the found card is in my hand"
    );
}

/// Passes until the search question is on the table.
#[track_caller]
fn pass_until_choice(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
}
