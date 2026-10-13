//! `cards/creatures/mv_4/floral_spuzzem.rs`, played.
//!
//! "Whenever this creature attacks and isn't blocked, you may destroy target
//! artifact defending player controls. If you do, this creature assigns no
//! combat damage this turn." A 2/2.
//!
//! The artifacts are Rod of Ruin (destroyable) and Darksteel Pendant
//! (indestructible), and every board has at least two on the defending side, so
//! the target question is a real menu with something to choose.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use baylee_core::generated::index;

// oracle_id = "994de451-14f9-466f-a56e-da052b4666e5"
fn floral_spuzzem() -> CardIndex {
    card_index("994de451-14f9-466f-a56e-da052b4666e5")
}

/// A two-seat table: p0 has the Spuzzem, p1 has two artifacts and `defenders`.
fn two_seats(defenders: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut seat_one = vec![index::ROD_OF_RUIN, index::DARKSTEEL_PENDANT];
    seat_one.extend_from_slice(defenders);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[floral_spuzzem()])
        .battlefield(1, &seat_one)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );
    engine
}

/// Sends the Spuzzem at p1, declares no blockers, and plays out the combat
/// with `answers`; returns when p0 holds priority in its second main phase.
#[track_caller]
fn attack_unblocked(engine: &mut Engine<RegistryLookup>, answers: &mut kit::Answers) -> ObjectId {
    let spuzzem = on_battlefield(engine, kit::P0, floral_spuzzem()).expect("the Spuzzem is out");
    kit::attack(engine, kit::P0, &[spuzzem], kit::P1);
    kit::block(engine, kit::P1, &[]);
    kit::drive_with(engine, answers, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P0)
    });
    spuzzem
}

/// "Whenever this creature attacks and isn't blocked, you may destroy target
/// artifact defending player controls. If you do, this creature assigns no
/// combat damage this turn." Nobody blocks; p0 aims the trigger at the Rod of
/// Ruin of two artifacts and says yes. The Rod is destroyed, the Pendant next
/// to it is not, and the Spuzzem's 2 damage is never dealt: p1 stays at 20.
#[test]
fn floral_spuzzem_unblocked_may_destroy_an_artifact_and_then_deals_no_combat_damage() {
    let mut engine = two_seats(&[]);
    let rod = on_battlefield(&engine, kit::P1, index::ROD_OF_RUIN).expect("the Rod is out");
    let pendant =
        on_battlefield(&engine, kit::P1, index::DARKSTEEL_PENDANT).expect("the Pendant is out");

    let mut answers = kit::Answers::new().aim(&[rod]).may(true);
    let spuzzem = attack_unblocked(&mut engine, &mut answers);

    assert!(
        answers
            .menus
            .iter()
            .any(|menu| menu.contains(&rod) && menu.contains(&pendant)),
        "the trigger offered both of the defending player's artifacts: {:?}",
        answers.menus
    );
    assert_eq!(
        kit::zone_of(&engine, rod),
        Zone::Graveyard,
        "the artifact it was aimed at is destroyed"
    );
    assert!(
        kit::on_the_table(&engine, pendant),
        "and only that one: the other artifact is untouched"
    );
    assert!(
        kit::on_the_table(&engine, spuzzem),
        "the Spuzzem is still there"
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        20,
        "\"if you do, this creature assigns no combat damage this turn\": p1 lost nothing"
    );
}

/// "You may destroy": saying no leaves the artifact on the table, and the
/// Spuzzem, which did not destroy anything, assigns its combat damage as
/// usual: p1 goes to 18. This is the control of the test above, the same
/// attack with the other answer.
#[test]
fn floral_spuzzem_declining_leaves_the_artifact_and_deals_its_combat_damage() {
    let mut engine = two_seats(&[]);
    let rod = on_battlefield(&engine, kit::P1, index::ROD_OF_RUIN).expect("the Rod is out");

    let mut answers = kit::Answers::new().aim(&[rod]).may(false);
    attack_unblocked(&mut engine, &mut answers);

    assert!(
        answers.menus.iter().any(|menu| menu.contains(&rod)) && answers.may_questions == 1,
        "the trigger fired and the player was asked, and said no: {:?} / {} questions",
        answers.menus,
        answers.may_questions
    );
    assert!(
        kit::on_the_table(&engine, rod),
        "the player said no: the artifact stays"
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        18,
        "nothing was destroyed, so the Spuzzem dealt its 2 damage"
    );
}

/// "Attacks and isn't blocked": a blocked Spuzzem does not trigger. A Wall of
/// Wood blocks it, no target is asked about (an unscripted question would fail
/// the test), both artifacts stay, and the Spuzzem's damage goes to the Wall.
/// The two sibling tests are the positive control: the same attack unblocked
/// does trigger.
#[test]
fn floral_spuzzem_blocked_does_not_trigger() {
    let mut engine = two_seats(&[kit::wall_of_wood()]);
    let spuzzem = on_battlefield(&engine, kit::P0, floral_spuzzem()).expect("the Spuzzem is out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_wood()).expect("the Wall is out");
    let rod = on_battlefield(&engine, kit::P1, index::ROD_OF_RUIN).expect("the Rod is out");
    let pendant =
        on_battlefield(&engine, kit::P1, index::DARKSTEEL_PENDANT).expect("the Pendant is out");

    kit::attack(&mut engine, kit::P0, &[spuzzem], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, spuzzem)]);
    let mut answers = kit::Answers::new();
    kit::drive_with(&mut engine, &mut answers, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P0)
    });

    assert!(
        answers.menus.is_empty(),
        "a blocked creature did not trigger it, yet a target was asked: {:?}",
        answers.menus
    );
    assert!(kit::on_the_table(&engine, rod), "the Rod stays");
    assert!(
        kit::on_the_table(&engine, pendant),
        "and so does the Pendant"
    );
    assert_eq!(
        kit::damage_on(&engine, wall),
        2,
        "the Spuzzem dealt its damage, to the creature that blocked it"
    );
    assert_eq!(kit::life(&engine, kit::P1), 20, "and none to the player");
}

/// "Target artifact defending player controls" in a game of three (CR 508.5,
/// CR 508.5a): the Spuzzem attacks p1, so p1's artifacts are the menu, and
/// neither p2's nor p0's own. p0 destroys one of p1's, and p2's artifact and
/// p0's stand untouched.
#[test]
fn floral_spuzzem_at_a_table_of_three_offers_only_the_attacked_players_artifacts() {
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[floral_spuzzem(), index::ROD_OF_RUIN])
        .battlefield(1, &[index::ROD_OF_RUIN, index::DARKSTEEL_PENDANT])
        .battlefield(2, &[index::ROD_OF_RUIN])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let spuzzem = on_battlefield(&engine, kit::P0, floral_spuzzem()).expect("the Spuzzem is out");
    let own_rod = on_battlefield(&engine, kit::P0, index::ROD_OF_RUIN).expect("p0's Rod");
    let defenders_rod = on_battlefield(&engine, kit::P1, index::ROD_OF_RUIN).expect("p1's Rod");
    let defenders_pendant =
        on_battlefield(&engine, kit::P1, index::DARKSTEEL_PENDANT).expect("p1's Pendant");
    let bystanders_rod = on_battlefield(&engine, kit::P2, index::ROD_OF_RUIN).expect("p2's Rod");

    kit::attack(&mut engine, kit::P0, &[spuzzem], kit::P1);
    kit::block(&mut engine, kit::P1, &[]);
    let mut answers = kit::Answers::new().aim(&[defenders_rod]).may(true);
    kit::drive_with(&mut engine, &mut answers, |e| {
        e.state().turn.phase == Phase::SecondMain && at_rest(e, kit::P0)
    });

    assert!(
        answers
            .menus
            .iter()
            .any(|menu| menu.contains(&defenders_rod) && menu.contains(&defenders_pendant)),
        "the attacked player's artifacts were on the menu: {:?}",
        answers.menus
    );
    for menu in &answers.menus {
        assert!(
            !menu.contains(&bystanders_rod) && !menu.contains(&own_rod),
            "only the artifacts of the player it attacked: {menu:?}"
        );
    }
    assert_eq!(
        kit::zone_of(&engine, defenders_rod),
        Zone::Graveyard,
        "the attacked player's Rod is destroyed"
    );
    assert!(
        kit::on_the_table(&engine, bystanders_rod),
        "the third player's Rod is untouched"
    );
    assert!(
        kit::on_the_table(&engine, own_rod),
        "so is the attacker's own"
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        20,
        "the Spuzzem assigned no combat damage after destroying"
    );
}

/// "If you do": the damage is lost only when something was destroyed
/// (CR 701.8a: to destroy a permanent is to move it from the battlefield to
/// its owner's graveyard). The only artifacts across the table are two Darksteel
/// Pendants, which are indestructible: p0 aims at one and says yes, the Pendant
/// stays where it is, and the Spuzzem, which destroyed nothing, deals its 2
/// combat damage.
#[test]
fn floral_spuzzem_cannot_destroy_an_indestructible_artifact_so_its_damage_is_dealt() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[floral_spuzzem()])
        .battlefield(1, &[index::DARKSTEEL_PENDANT, index::DARKSTEEL_PENDANT])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );
    let pendants = kit::objects_of(&engine, kit::P1, index::DARKSTEEL_PENDANT);
    assert_eq!(pendants.len(), 2, "two Pendants");

    let mut answers = kit::Answers::new().aim(&[pendants[0]]).may(true);
    attack_unblocked(&mut engine, &mut answers);

    assert!(
        answers.menus.iter().any(|menu| menu.contains(&pendants[0])),
        "the trigger was put on the stack aimed at an artifact: {:?}",
        answers.menus
    );
    assert!(
        kit::on_the_table(&engine, pendants[0]),
        "an indestructible artifact is not destroyed"
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        18,
        "nothing was destroyed, so the Spuzzem still assigns its combat damage"
    );
}

/// "If you do, this creature assigns no combat damage *this turn*": the next
/// turn's attack is not affected. The Spuzzem destroys the Rod of Ruin in the
/// first turn and p1 stays at 20; two turns later it attacks again, the player
/// says no to destroying the Pendant, and p1 takes the Spuzzem's 2.
#[test]
fn floral_spuzzem_assigns_no_combat_damage_only_in_the_turn_it_destroyed_something() {
    let mut engine = two_seats(&[]);
    let rod = on_battlefield(&engine, kit::P1, index::ROD_OF_RUIN).expect("the Rod is out");
    let pendant =
        on_battlefield(&engine, kit::P1, index::DARKSTEEL_PENDANT).expect("the Pendant is out");

    let mut first = kit::Answers::new().aim(&[rod]).may(true);
    attack_unblocked(&mut engine, &mut first);
    assert_eq!(
        kit::zone_of(&engine, rod),
        Zone::Graveyard,
        "the Rod was destroyed in the first turn"
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        20,
        "and no damage was assigned"
    );

    kit::walk_to_next_own_main(&mut engine, kit::P0);
    let mut second = kit::Answers::new().aim(&[pendant]).may(false);
    attack_unblocked(&mut engine, &mut second);
    assert!(
        kit::on_the_table(&engine, pendant),
        "the player declined in the later turn"
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        18,
        "and the Spuzzem assigned its 2 damage again: the first turn's restriction ended with it"
    );
}
