//! `cards/artifacts/mv_4/barl_s_cage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn barl_s_cage() -> CardIndex {
    card_index("eb260cd3-9060-4a5e-bc60-a8ca13d542cc")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// The tapped creatures of p0 (Bears) and p1 (Giant and Lions), the Cage and
/// three Forests on p0's side, p0's main phase.
fn a_board_of_tapped_creatures() -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[barl_s_cage(), grizzly_bears(), forest(), forest(), forest()],
        )
        .battlefield(1, &[hill_giant(), savannah_lions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("Bears");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("Giant");
    let lions = on_battlefield(&engine, p1, savannah_lions()).expect("Lions");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        for id in [bears, giant, lions] {
            state.set_tapped(id, true);
        }
    }
    engine.refresh_offer();
    assert!(is_tapped(&engine, bears) && is_tapped(&engine, giant) && is_tapped(&engine, lions));
    (engine, bears, giant, lions)
}

/// Pays {3} and aims the Cage; the Cage itself stays on the table.
#[track_caller]
fn cage_at(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    let p0 = PlayerId::new(0);
    let cage = on_battlefield(engine, p0, barl_s_cage()).expect("the Cage");
    assert_eq!(tap_all_mana(engine, p0), 3, "three Forests pay {{3}}");
    activate(engine, p0, barl_s_cage(), 0);
    aim_at(engine, p0, target);
    pass_until(engine, stack_is_empty);
    assert!(!is_tapped(engine, cage), "the Cage's price is mana alone");
}

/// Barl's Cage (eb260cd3-9060-4a5e-bc60-a8ca13d542cc) — "{3}: Target creature
/// doesn't untap during its controller's next untap step."
///
/// The Giant is aimed at: it stays down through its controller's next untap
/// step, the bystander Lions untap as usual, and the Giant untaps at the
/// untap step after that — "next", once.
#[test]
fn barl_s_cage_holds_a_creature_down_for_one_untap_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, _bears, giant, lions) = a_board_of_tapped_creatures();
    cage_at(&mut engine, giant);

    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, giant), "its next untap step missed it");
    assert!(!is_tapped(&engine, lions), "the untargeted Lions untapped");

    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !is_tapped(&engine, giant),
        "the step after that is an ordinary one"
    );
}

/// "Target creature": nothing says an opponent's. Aimed at p0's own Bears, the
/// Bears miss p0's next untap step and not the one after.
#[test]
fn barl_s_cage_works_on_the_controllers_own_creature() {
    let p0 = PlayerId::new(0);
    let (mut engine, bears, giant, _lions) = a_board_of_tapped_creatures();
    cage_at(&mut engine, bears);

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(!is_tapped(&engine, giant), "the Giant untapped normally");
    reach_their_main_phase(&mut engine, p0);
    assert!(
        is_tapped(&engine, bears),
        "p0's next untap step missed the Bears"
    );
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, bears), "and the one after did not");
}
