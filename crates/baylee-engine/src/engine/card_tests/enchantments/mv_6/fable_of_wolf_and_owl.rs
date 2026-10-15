//! `cards/enchantments/mv_6/fable_of_wolf_and_owl.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn fable_of_wolf_and_owl() -> CardIndex {
    card_index("24fe8bc3-7e3a-435b-a232-7f80787aba1f")
}

/// Casts `spell` under `lands`, runs to the "you may" question and hands
/// the engine back on it.
fn spell_cast_asks(spell: CardIndex, lands: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut board = vec![fable_of_wolf_and_owl()];
    board.extend_from_slice(lands);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[spell])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, spell);
    for _ in 0..10 {
        if matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        ) {
            return engine;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("the trigger never asked");
}

/// "Whenever you cast a green spell, you may create a 2/2 green Wolf
/// creature token." Accepted: a Wolf.
#[test]
fn fable_of_wolf_and_owl_green_spell_makes_a_wolf() {
    let p0 = PlayerId::new(0);
    let mut engine = spell_cast_asks(llanowar_elves(), &[forest()]);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
    assert!(!keywords(&engine, tokens[0]).contains(KeywordSet::FLYING));
}

/// "You may": declining makes nothing.
#[test]
fn fable_of_wolf_and_owl_may_decline() {
    let p0 = PlayerId::new(0);
    let mut engine = spell_cast_asks(llanowar_elves(), &[forest()]);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(tokens_of(&engine, p0).is_empty());
}

/// "Whenever you cast a blue spell, you may create a 1/1 blue Bird creature
/// token with flying."
#[test]
fn fable_of_wolf_and_owl_blue_spell_makes_a_bird() {
    let p0 = PlayerId::new(0);
    let mut engine = spell_cast_asks(
        killer_whale(),
        &[island(), island(), island(), island(), island()],
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (1, 1));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::FLYING));
}
