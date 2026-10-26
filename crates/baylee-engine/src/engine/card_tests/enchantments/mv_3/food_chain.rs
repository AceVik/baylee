//! Tests for Food Chain.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::hill_giant;

// oracle_id = "5c8e5092-962e-49ef-ab82-8434e475e4e7"
fn food_chain() -> CardIndex {
    card_index("5c8e5092-962e-49ef-ab82-8434e475e4e7")
}

/// Exile a creature you control: Add X mana of any one color, where X is 1 plus
/// the exiled creature's mana value. Spend this mana only to cast creature spells.
#[test]
fn food_chain_exiles_creature_for_one_plus_mana_value_creature_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), gray_ogre()])
        .hand(0, &[food_chain(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, food_chain());
    pass_until(&mut engine, stack_is_empty);

    let chain = on_battlefield(&engine, p0, food_chain()).expect("Food Chain");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("Gray Ogre");

    // Exile Gray Ogre (MV 3) to add 4 Red mana restricted to creature spells, then cast Hill Giant ({3}{R}).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: chain,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseCards {
        prompt: ChoicePrompt::CostExile,
        ..
    } = engine.pending()
    else {
        panic!("expected exile cost prompt, got {:?}", engine.pending());
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .unwrap();
    assert!(matches!(engine.pending(), Pending::ChooseColor { .. }));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    cast_with_floating(&mut engine, p0, hill_giant());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, hill_giant()).is_some());
}
