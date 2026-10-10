//! `cards/artifacts/mv_4/life_chisel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

fn life_chisel() -> CardIndex {
    card_index("2493fe65-2ecb-418f-8e4e-797f83475c73")
}

/// A 2/1 Cat: power and toughness differ, so the life gained says which
/// of the two numbers was read.
fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Life Chisel: "Sacrifice a creature: You gain life equal to the sacrificed
/// creature's toughness. Activate only during your upkeep."
///
/// A creature stands beside it the whole time, so a missing offer is the
/// timing restriction and not an empty board. Every priority `p0` receives
/// from its first main phase to its next upkeep is inspected: the ability is
/// offered at none of them (main phase, combat, end step and the
/// opponent's whole turn, upkeep included), and at that next upkeep it is.
/// There the Lions (2/1) are first made 5/4 with Giant Growth, so 4 is the
/// toughness the creature last had, not its printed 1 or its power 5.
#[test]
fn life_chisel_is_offered_only_in_your_upkeep_and_gains_the_pumped_toughness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[life_chisel(), forest(), savannah_lions()])
        .hand(0, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let chisel = on_battlefield(&engine, p0, life_chisel()).expect("the Chisel is out");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions");

    let mut looked = 0;
    let mut saw_opponents_upkeep = false;
    for _ in 0..200 {
        let turn = engine.state().turn;
        match engine.pending().clone() {
            Pending::Priority { player, legal } => {
                let offered = legal.abilities.contains(&(chisel, 0));
                if player == p0 {
                    if turn.active == p0 && turn.step == Step::Upkeep && turn.number > 1 {
                        assert!(offered, "offered in your own upkeep");
                        break;
                    }
                    assert!(
                        !offered,
                        "not offered in {:?} / {:?} of turn {} (active {:?})",
                        turn.phase, turn.step, turn.number, turn.active
                    );
                    looked += 1;
                    saw_opponents_upkeep |= turn.active == p1 && turn.step == Step::Upkeep;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
    assert!(looked >= 3, "several priorities were inspected: {looked}");
    assert!(saw_opponents_upkeep, "the opponent's upkeep was among them");
    assert_eq!(engine.state().turn.step, Step::Upkeep);
    assert_eq!(engine.state().turn.active, p0);

    let forest_id = on_battlefield(&engine, p0, forest()).expect("a Forest");
    tap_mana_where(&mut engine, p0, |id| id == forest_id);
    cast_with_floating(&mut engine, p0, giant_growth());
    aim_at(&mut engine, p0, lions);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, lions), (5, 4), "Giant Growth: +3/+3");

    let life = engine.state().players[0].life;
    activate(&mut engine, p0, life_chisel(), 0);
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        assert!(options.contains(&lions));
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![lions],
                },
            )
            .expect("sacrifice the Lions");
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, savannah_lions()).is_some());
    assert_eq!(
        engine.state().players[0].life,
        life + 4,
        "the toughness the Lions last had"
    );
    assert!(
        on_battlefield(&engine, p0, life_chisel()).is_some(),
        "Life Chisel itself stays"
    );
}
