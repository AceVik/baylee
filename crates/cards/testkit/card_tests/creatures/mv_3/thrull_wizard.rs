//! `cards/creatures/mv_3/thrull_wizard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thrull_wizard() -> CardIndex {
    card_index("3cae0e4b-1827-4ed2-832f-98f8e79941a5")
}

fn mindstab_thrull() -> CardIndex {
    card_index("43abe1f3-7b92-411a-bf93-e158f949685c")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn float(engine: &mut Engine<RegistryLookup>, seat: usize, mana: &[(ManaColor, u32)]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for (color, n) in mana {
        state.players[seat].mana_pool.add(*color, *n);
    }
    engine.refresh_offer();
}

fn offered(engine: &Engine<RegistryLookup>, card: CardIndex, index: u32) -> bool {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.iter().any(|(id, ai)| {
        *ai == index
            && engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
    })
}

/// p0 holds the Wizard; p1, on their own turn, casts `spell` off `mana`, and
/// passes so that p0 has priority with the spell on the stack.
fn p1_casts(spell: CardIndex, mana: &[(ManaColor, u32)]) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[thrull_wizard()])
        .hand(1, &[spell])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    float(&mut engine, 1, mana);
    cast_with_floating(&mut engine, p1, spell);
    assert!(
        on_stack(&engine, spell).is_some(),
        "the spell is on the stack"
    );
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("p1 passes");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "p0 has priority with the spell on the stack, got {:?}",
        engine.pending()
    );
    engine
}

/// Counters `spell` with the Wizard and runs until its controller is asked
/// whether to pay {B}.
fn counter_attempt(engine: &mut Engine<RegistryLookup>, spell: CardIndex) {
    let p0 = PlayerId::new(0);
    float(
        engine,
        0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    activate(engine, p0, thrull_wizard(), 0);
    let object = on_stack(engine, spell).expect("the spell is still on the stack");
    let options = aim_at(engine, p0, object);
    assert_eq!(options, vec![object], "the black spell is the only target");
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(
        player,
        PlayerId::new(1),
        "the spell's controller, not the Wizard's, is asked"
    );
}

/// Thrull Wizard (3cae0e4b-1827-4ed2-832f-98f8e79941a5) — "{1}{B}: Counter
/// target black spell unless that spell's controller pays {B} or {3}."
#[test]
fn thrull_wizard_counters_a_black_spell_whose_controller_pays_nothing() {
    let p1 = PlayerId::new(1);
    let mut engine = p1_casts(mindstab_thrull(), &[(ManaColor::Black, 3)]);
    counter_attempt(&mut engine, mindstab_thrull());
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declines {B}");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declines {3}");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, mindstab_thrull()).is_none(),
        "the countered creature never arrives"
    );
    assert!(
        in_graveyard(&engine, p1, mindstab_thrull()).is_some(),
        "it is in its owner's graveyard"
    );
}

/// Paying {B} is enough.
#[test]
fn thrull_wizard_does_not_counter_when_the_controller_pays_black() {
    let p1 = PlayerId::new(1);
    let mut engine = p1_casts(mindstab_thrull(), &[(ManaColor::Black, 5)]);
    counter_attempt(&mut engine, mindstab_thrull());
    engine
        .apply(p1, PlayerAction::YesNo(true))
        .expect("pays {B}");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, mindstab_thrull()).is_some(),
        "the spell resolved"
    );
    assert!(
        in_graveyard(&engine, p1, mindstab_thrull()).is_none(),
        "and was not countered"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "five black floated, three for the spell and one for the tax"
    );
}

/// Paying {3} is enough as well, with no black left to pay {B} with.
#[test]
fn thrull_wizard_does_not_counter_when_the_controller_pays_three() {
    let p1 = PlayerId::new(1);
    let mut engine = p1_casts(
        mindstab_thrull(),
        &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
    );
    counter_attempt(&mut engine, mindstab_thrull());
    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declines {B}");
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
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(mana, 3, "the second price is {{3}}");
    engine
        .apply(p1, PlayerAction::YesNo(true))
        .expect("pays {3}");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, mindstab_thrull()).is_some(),
        "the spell resolved"
    );
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
}

/// "Target black spell": a white spell is not a target, so there is nothing
/// to activate the ability on.
#[test]
fn thrull_wizard_cannot_target_a_white_spell() {
    let mut engine = p1_casts(savannah_lions(), &[(ManaColor::White, 1)]);
    float(
        &mut engine,
        0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    assert!(
        !offered(&engine, thrull_wizard(), 0),
        "no black spell on the stack, no ability"
    );
}
