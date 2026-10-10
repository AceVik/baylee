//! `cards/lands/deserts/desert.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

/// Desert: "{T}: Add {C}." / "{T}: This land deals 1 damage to target attacking creature. Activate only during the end of combat step."
/// Activating Desert produces {C} and leaves the land tapped.
#[test]
fn desert_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(126, forest()).battlefield(0, &[desert()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, desert()).expect("Desert deployed");
    activate(&mut engine, p0, desert(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// A 2/1 Cat: it dies to one damage and deals two.
fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Declares `attacker` and walks the combat one priority at a time until the
/// end of combat step, recording for every priority `p0` receives whether
/// Desert's damage line is offered. `ChooseBlockers` is answered with nobody.
#[track_caller]
fn attack_and_watch_the_desert(
    engine: &mut Engine<RegistryLookup>,
    desert: ObjectId,
    attacker: ObjectId,
) -> Vec<(Step, bool)> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .expect("the Lions may attack");
    let mut seen = Vec::new();
    for _ in 0..40 {
        let step = engine.state().turn.step;
        match engine.pending().clone() {
            Pending::Priority { player, legal } => {
                if player == p0 {
                    seen.push((step, legal.abilities.contains(&(desert, 1))));
                }
                if step == Step::CombatEnd && player == p0 {
                    return seen;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected on the way to the end of combat: {other:?}"),
        }
    }
    panic!("never reached the end of combat step: {seen:?}")
}

/// "{T}: This land deals 1 damage to target attacking creature. Activate only
/// during the end of combat step."
///
/// The Lions (2/1) attack and are unblocked: at the declare attackers, declare
/// blockers and combat damage steps the attacking creature is there to be
/// targeted and the line is still not offered; in the end of combat step it
/// is. Then the Lions have dealt their two combat damage, and the Desert's
/// one kills them (toughness 1). Before combat, in the main phase, the line
/// is not offered either. A Llanowar Elves of the opponent's, not attacking,
/// is not among the targets, and naming it is refused.
#[test]
fn desert_pings_an_attacker_only_in_the_end_of_combat_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[desert(), savannah_lions()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, desert()).expect("the Desert");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority expected")
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "not in the main phase"
    );
    assert!(legal.abilities.contains(&(land, 0)), "the mana line is");

    let seen = attack_and_watch_the_desert(&mut engine, land, lions);
    let steps: Vec<Step> = seen.iter().map(|(s, _)| *s).collect();
    for needed in [Step::DeclareAttackers, Step::CombatDamage, Step::CombatEnd] {
        assert!(steps.contains(&needed), "{needed:?} was visited: {steps:?}");
    }
    for (step, offered) in &seen {
        assert_eq!(
            *offered,
            *step == Step::CombatEnd,
            "offered exactly at the end of combat step, not at {step:?}: {seen:?}"
        );
    }
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the Lions dealt combat damage"
    );
    assert!(
        on_battlefield(&engine, p0, savannah_lions()).is_some(),
        "and are still alive"
    );

    activate(&mut engine, p0, desert(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("target an attacking creature, got {:?}", engine.pending())
    };
    assert_eq!(options, vec![lions], "only the attacker: {options:?}");
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![elf],
                    players: vec![],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![lions],
                players: vec![],
            },
        )
        .expect("the attacker is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, land), "{{T}} is the cost");
    assert!(
        in_graveyard(&engine, p0, savannah_lions()).is_some(),
        "one damage kills the 1-toughness attacker"
    );
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_some());
}
