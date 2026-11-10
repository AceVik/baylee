//! `cards/creatures/mv_2/carrionette.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn carrionette() -> CardIndex {
    card_index("24315f60-731a-40bb-85ba-eedba1b011ff")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// p0 has Carrionette and six Swamps; p1 has Grizzly Bears and two Swamps.
/// p0's first main phase, Carrionette still on the battlefield.
fn a_carrionette_with_mana() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                carrionette(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Sends the Carrionette on the battlefield to its owner's graveyard and
/// returns the card there.
#[track_caller]
fn bury_the_carrionette(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    let skeleton = on_battlefield(engine, P0, carrionette()).unwrap();
    bury(engine, &[skeleton]);
    engine.refresh_offer();
    in_graveyard(engine, P0, carrionette()).expect("Carrionette is in the graveyard")
}

/// Pays {2}{B}{B} with all of p0's Swamps, aims at `target` and passes, so p1
/// holds priority with the ability on the stack.
#[track_caller]
fn activate_aimed_at(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    tap_all_mana(engine, P0);
    activate(engine, P0, carrionette(), 0);
    aim_at(engine, P0, target);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
}

/// Passes until the "pay {2}" question and returns who is asked.
#[track_caller]
fn the_payer(engine: &mut Engine<RegistryLookup>) -> PlayerId {
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert!(
        matches!(prompt, YesNoPrompt::PayTax { mana: 2 }),
        "the price is {{2}}: {prompt:?}"
    );
    player
}

/// Carrionette (24315f60-731a-40bb-85ba-eedba1b011ff) — {1}{B} Creature — Skeleton:
/// "{2}{B}{B}: Exile this card and target creature unless that creature's controller pays {2}. Activate only if this card is in your graveyard."
#[test]
fn carrionette_exiles_itself_and_the_target_when_its_controller_does_not_pay() {
    let mut engine = a_carrionette_with_mana();
    let gy_skeleton = bury_the_carrionette(&mut engine);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    activate_aimed_at(&mut engine, bears);
    assert_eq!(
        the_payer(&mut engine),
        P1,
        "the creature's controller is asked, not the activator"
    );
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(gy_skeleton).unwrap().zone,
        Zone::Exile
    );
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Exile);
}

#[test]
fn carrionette_exiles_nothing_when_the_targets_controller_pays_two() {
    let mut engine = a_carrionette_with_mana();
    let gy_skeleton = bury_the_carrionette(&mut engine);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    activate_aimed_at(&mut engine, bears);
    // p1 holds priority with the ability on the stack: make {2} to pay with.
    tap_all_mana(&mut engine, P1);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert_eq!(the_payer(&mut engine), P1);
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(gy_skeleton).unwrap().zone,
        Zone::Graveyard,
        "paid: this card is not exiled either"
    );
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "the {{2}} left p1's pool"
    );
}

#[test]
fn carrionette_asks_the_controller_of_a_creature_of_its_own_side_too() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                carrionette(),
                grizzly_bears(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let gy_skeleton = bury_the_carrionette(&mut engine);
    let my_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    // Six Swamps float; the ability takes four and leaves {2} for the price.
    activate_aimed_at(&mut engine, my_bears);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        the_payer(&mut engine),
        P0,
        "the target is p0's, so p0 is the one who may pay"
    );
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(gy_skeleton).unwrap().zone,
        Zone::Graveyard
    );
    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_some());
}

#[test]
fn carrionette_can_be_activated_only_from_the_graveyard() {
    let mut engine = a_carrionette_with_mana();
    let skeleton = on_battlefield(&engine, P0, carrionette()).unwrap();
    tap_all_mana(&mut engine, P0);

    assert!(
        !ability_offered(&engine, skeleton, 0),
        "on the battlefield, with {{2}}{{B}}{{B}} to spare, the ability is not offered"
    );

    let gy_skeleton = bury_the_carrionette(&mut engine);
    assert!(
        ability_offered(&engine, gy_skeleton, 0),
        "in the graveyard it is"
    );
}
