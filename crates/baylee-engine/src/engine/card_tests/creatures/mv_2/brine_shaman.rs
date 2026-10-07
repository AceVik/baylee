//! `cards/creatures/mv_2/brine_shaman.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Brine Shaman` prints an activated ability with cost `{{T}}, Sacrifice a creature` under `Coverage::Implemented`.
/// In accordance with CR 601.2c and CR 601.2h, targeting occurs before paying activation costs.
/// Targeting the Shaman itself and then sacrificing another creature verifies that `Pending::ChooseTargets`
/// precedes `Pending::ChooseCards` with `ChoicePrompt::CostSacrifice`, pumping the Shaman by +2/+2.
#[test]
fn brine_shaman_sacrifices_creature_to_pump_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1602, forest())
        .battlefield(0, &[brine_shaman(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shaman = on_battlefield(&engine, p0, brine_shaman()).expect("shaman is on battlefield");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf is on battlefield");
    assert_eq!(pt(&engine, shaman), (1, 1));

    activate(&mut engine, p0, brine_shaman(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Brine Shaman pump ability");
    };
    assert!(options.contains(&shaman));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![shaman],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        prompt: ChoicePrompt::CostSacrifice,
        options: sacrifice_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice cost prompt for Brine Shaman");
    };
    assert!(sacrifice_options.contains(&elf));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, shaman), (3, 3));
    assert!(is_tapped(&engine, shaman));
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
}

/// Brine Shaman: "{1}{U}{U}, Sacrifice a creature: Counter target creature
/// spell." The opponent's creature spell never arrives, and the sacrifice
/// is a cost: the Elves are in the graveyard.
#[test]
fn brine_shaman_sacrifices_a_creature_to_counter_a_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(2201, forest())
        .battlefield(0, &[forest(), plains()])
        .hand(0, &[ondu_cleric()])
        .battlefield(
            1,
            &[
                brine_shaman(),
                llanowar_elves(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, ondu_cleric());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("elf");
    tap_mana_where(&mut engine, p1, |id| id != elf);
    activate(&mut engine, p1, brine_shaman(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the spell target, got {:?}", engine.pending())
    };
    let spell = *options.first().expect("the creature spell is a target");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .unwrap();
    let Pending::ChooseCards {
        prompt: ChoicePrompt::CostSacrifice,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the sacrifice, got {:?}", engine.pending())
    };
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, ondu_cleric()).is_none());
    assert!(
        in_graveyard(&engine, p0, ondu_cleric()).is_some(),
        "countered"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the cost"
    );
    assert!(on_battlefield(&engine, p1, brine_shaman()).is_some());
}
