//! `cards/instants/mv_1/syncopate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn syncopate() -> CardIndex {
    card_index("318f7f70-e374-40ef-8afb-3389c10461d8")
}

/// Casts Syncopate with `x` at `target`, answering its X announcement and
/// target question in whichever order the engine asks them.
#[track_caller]
fn cast_syncopate_at(
    engine: &mut Engine<RegistryLookup>,
    caster: PlayerId,
    x: u32,
    target: ObjectId,
) -> Vec<ObjectId> {
    let spell = in_hand(engine, caster, syncopate()).expect("Syncopate in hand");
    engine
        .apply(caster, PlayerAction::CastSpell { card: spell })
        .expect("Syncopate is castable");
    let mut announced_x = false;
    let mut targeted = false;
    let mut menu = Vec::new();
    for _ in 0..4 {
        if announced_x && targeted {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert!(
                    min <= x && x <= max,
                    "X = {x} must be offered in {min}..={max}"
                );
                engine.apply(player, PlayerAction::ChooseNumber(x)).unwrap();
                announced_x = true;
            }
            Pending::ChooseTargets {
                player, options, ..
            } => {
                menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![target],
                        },
                    )
                    .expect("the named spell was on the target menu");
                targeted = true;
            }
            other => panic!("unexpected Syncopate question: {other:?}"),
        }
    }
    assert!(announced_x && targeted);
    menu
}

/// With X = 2, Syncopate costs {2}{U}. The tax question belongs to the
/// targeted spell's controller; declining counters that spell into exile,
/// while Syncopate itself goes to its owner's graveyard.
#[test]
fn syncopate_counters_into_exile_when_the_controller_declines_to_pay_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(); 4])
        .hand(1, &[syncopate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let elves = in_hand(&engine, p0, llanowar_elves()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: elves })
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let menu = cast_syncopate_at(&mut engine, p1, 2, elves);
    assert_eq!(menu, vec![elves], "the spell on the stack is the target");
    assert_eq!(engine.state().players[1].mana_pool.total(), 1);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass stopped on the tax question")
    };
    assert_eq!(player, p0, "the targeted spell's controller is asked");
    assert_eq!(mana, 2, "the announced X determines the tax");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert_eq!(engine.state().object(elves).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_none());
    assert!(in_graveyard(&engine, p1, syncopate()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// With X = 1, paying that one mana prevents the counter effect. The targeted
/// creature resolves normally, and Syncopate's own {1}{U} cost is separate.
#[test]
fn syncopate_lets_the_spell_resolve_when_its_controller_pays_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[island(); 3])
        .hand(1, &[syncopate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let elves = in_hand(&engine, p0, llanowar_elves()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: elves })
        .unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_syncopate_at(&mut engine, p1, 1, elves);
    assert_eq!(engine.state().players[1].mana_pool.total(), 1);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass stopped on the tax question")
    };
    assert_eq!(player, p0);
    assert_eq!(mana, 1);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(on_battlefield(&engine, p0, llanowar_elves()), Some(elves));
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_none());
    assert_ne!(engine.state().object(elves).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().players[1].mana_pool.total(), 1);
    assert!(in_graveyard(&engine, p1, syncopate()).is_some());
}
