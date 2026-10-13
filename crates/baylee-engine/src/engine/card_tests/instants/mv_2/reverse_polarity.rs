//! `cards/instants/mv_2/reverse_polarity.rs`, played.
//!
//! "You gain X life, where X is twice the damage dealt to you so far this turn
//! by artifacts." The caster is always the seat that was hit (p1), so the
//! numbers are read off its life total: 20 at the start, minus what it took,
//! plus twice what artifacts dealt it.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use crate::turn::Step;
use baylee_core::generated::index;

// oracle_id = "c7076e2c-81f1-44ad-a4dd-45a01802d364"
fn reverse_polarity() -> CardIndex {
    card_index("c7076e2c-81f1-44ad-a4dd-45a01802d364")
}

/// Casts Reverse Polarity as p1 off the two Plains it has untapped and lets it
/// resolve.
#[track_caller]
fn cast_reverse_polarity(engine: &mut Engine<RegistryLookup>) {
    tap_all_mana(engine, kit::P1);
    let mut answers = kit::Answers::new();
    kit::cast(engine, kit::P1, reverse_polarity(), &mut answers);
    kit::settle(engine, kit::P1, &mut answers);
    assert!(
        in_graveyard(engine, kit::P1, reverse_polarity()).is_some(),
        "an instant that resolved is in its owner's graveyard"
    );
}

/// Both halves of "damage dealt to you by artifacts" — combat and not — add
/// up, and a non-artifact's damage on the same turn is left out.
///
/// p0 pings p1 for 1 with Rod of Ruin (a noncombat artifact source), then
/// attacks with Juggernaut (an artifact creature, 5) and Grizzly Bears (not an
/// artifact, 2). p1 is at 20 - 1 - 7 = 12 when it casts Reverse Polarity in the
/// combat damage step. Artifacts dealt 1 + 5, so it gains 12 and ends at 24.
/// Counting the Bears as well would end at 28; counting only combat damage,
/// at 22.
#[test]
fn reverse_polarity_doubles_the_damage_artifacts_dealt_in_combat_and_out_of_it() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                kit::juggernaut(),
                kit::bears(),
                index::ROD_OF_RUIN,
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[plains(), plains()])
        .hand(1, &[reverse_polarity()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let rod = on_battlefield(&engine, kit::P0, index::ROD_OF_RUIN).expect("the Rod is out");
    let juggernaut = on_battlefield(&engine, kit::P0, kit::juggernaut()).expect("Juggernaut");
    let bears = on_battlefield(&engine, kit::P0, kit::bears()).expect("the Bears");

    // Noncombat damage from an artifact: the Rod's own {3},{T} ability.
    tap_all_mana(&mut engine, kit::P0);
    let mut answers = kit::Answers::new().aim_players(&[kit::P1]);
    kit::activate_ability(&mut engine, kit::P0, rod, 0, &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    assert_eq!(kit::life(&engine, kit::P1), 19, "the Rod dealt 1 to p1");

    kit::attack(&mut engine, kit::P0, &[juggernaut, bears], kit::P1);
    kit::walk_to_quiet_priority(
        &mut engine,
        &mut kit::Answers::new(),
        kit::P1,
        Step::CombatDamage,
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        12,
        "5 from Juggernaut and 2 from the Bears on top of the Rod's 1"
    );

    cast_reverse_polarity(&mut engine);
    assert_eq!(
        kit::life(&engine, kit::P1),
        24,
        "gain twice (1 from the Rod + 5 from Juggernaut): 12, and nothing for \
         the Bears"
    );
}

/// "Damage dealt … by artifacts" — the artifact as it was when it dealt the
/// damage. Mishra's Factory animates as a 2/2 artifact creature, connects, and
/// is then killed with Lightning Bolt: in the graveyard it is a land card and
/// no longer an artifact, but the 2 damage it dealt was dealt by an artifact
/// (CR 608.2h: an effect that needs information from an object that has left
/// uses its last known information). p1 ends at 20 - 2 + 4 = 22.
#[test]
fn reverse_polarity_counts_damage_from_a_creature_that_was_an_artifact_when_it_dealt_it() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::MISHRA_S_FACTORY, mountain()])
        .battlefield(1, &[plains(), plains(), mountain()])
        .hand(1, &[reverse_polarity(), kit::bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let factory = on_battlefield(&engine, kit::P0, index::MISHRA_S_FACTORY).expect("Factory");
    let mountain_0 = on_battlefield(&engine, kit::P0, mountain()).expect("the Mountain");
    assert!(
        !types(&engine, factory).contains(TypeSet::ARTIFACT),
        "a plain land to begin with"
    );

    // {1}: this land becomes a 2/2 Assembly-Worker artifact creature.
    tap_mana_where(&mut engine, kit::P0, |id| id == mountain_0);
    let mut answers = kit::Answers::new();
    kit::activate_ability(&mut engine, kit::P0, factory, 1, &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    assert!(
        (types(&engine, factory).contains(TypeSet::ARTIFACT)
            && types(&engine, factory).contains(TypeSet::CREATURE)),
        "animated: an artifact creature until end of turn"
    );

    kit::attack(&mut engine, kit::P0, &[factory], kit::P1);
    kit::walk_to_quiet_priority(
        &mut engine,
        &mut kit::Answers::new(),
        kit::P1,
        Step::CombatDamage,
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        18,
        "the Factory connected for 2"
    );

    // Lightning Bolt on the Factory, off p1's Mountain alone.
    let their_mountain = on_battlefield(&engine, kit::P1, mountain()).expect("p1's Mountain");
    tap_mana_where(&mut engine, kit::P1, |id| id == their_mountain);
    let mut answers = kit::Answers::new().aim(&[factory]);
    kit::cast(&mut engine, kit::P1, kit::bolt(), &mut answers);
    kit::settle(&mut engine, kit::P1, &mut answers);
    assert!(
        in_graveyard(&engine, kit::P0, index::MISHRA_S_FACTORY).is_some(),
        "the Factory died, and a land card in a graveyard is not an artifact"
    );

    cast_reverse_polarity(&mut engine);
    assert_eq!(
        kit::life(&engine, kit::P1),
        22,
        "the Factory was an artifact when it dealt 2: gain 4"
    );
}

/// The inverse of the test above: a creature that is not an artifact when it
/// deals its damage does not count because it *later* became one. Bears and
/// Juggernaut connect; p1 then taps Liquimetal Coating to make the Bears an
/// artifact too, and checks that it is one. p1 ends at 20 - 7 + 10 = 23: the
/// Bears' 2 is not counted (27 if it were), and the sibling tests are the
/// control that the spell gains at all.
#[test]
fn reverse_polarity_ignores_damage_from_a_creature_that_only_became_an_artifact_afterwards() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::juggernaut(), kit::bears()])
        .battlefield(1, &[plains(), plains(), liquimetal_coating()])
        .hand(1, &[reverse_polarity()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let juggernaut = on_battlefield(&engine, kit::P0, kit::juggernaut()).expect("Juggernaut");
    let bears = on_battlefield(&engine, kit::P0, kit::bears()).expect("the Bears");
    let coating = on_battlefield(&engine, kit::P1, liquimetal_coating()).expect("the Coating");

    kit::attack(&mut engine, kit::P0, &[juggernaut, bears], kit::P1);
    kit::walk_to_quiet_priority(
        &mut engine,
        &mut kit::Answers::new(),
        kit::P1,
        Step::CombatDamage,
    );
    assert_eq!(kit::life(&engine, kit::P1), 13, "5 and 2 were dealt");
    assert!(
        !types(&engine, bears).contains(TypeSet::ARTIFACT),
        "the Bears are not an artifact yet"
    );

    // {T}: target permanent becomes an artifact in addition to its other types.
    let mut answers = kit::Answers::new().aim(&[bears]);
    kit::activate_ability(&mut engine, kit::P1, coating, 0, &mut answers);
    kit::settle(&mut engine, kit::P1, &mut answers);
    assert!(
        types(&engine, bears).contains(TypeSet::ARTIFACT),
        "the Bears are an artifact now, after the damage"
    );

    cast_reverse_polarity(&mut engine);
    assert_eq!(
        kit::life(&engine, kit::P1),
        23,
        "only Juggernaut's 5 was dealt by an artifact: gain 10, not 14"
    );
}

/// Prevented damage is not dealt (CR 615.6). p1 shields itself with Healing
/// Salve's second mode ("prevent the next 3 damage that would be dealt to any
/// target this turn") before Juggernaut connects: 3 of its 5 are prevented, 2
/// are dealt, and Reverse Polarity gains twice the 2. p1 ends at 20 - 2 + 4 =
/// 22; counting the prevented damage would end at 28.
#[test]
fn reverse_polarity_ignores_damage_that_was_prevented() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::juggernaut()])
        .battlefield(1, &[plains(), plains(), plains()])
        .hand(1, &[index::HEALING_SALVE, reverse_polarity()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let juggernaut = on_battlefield(&engine, kit::P0, kit::juggernaut()).expect("Juggernaut");
    let plains_objects = kit::objects_of(&engine, kit::P1, plains());

    kit::attack(&mut engine, kit::P0, &[juggernaut], kit::P1);
    kit::walk_to_quiet_priority(
        &mut engine,
        &mut kit::Answers::new(),
        kit::P1,
        Step::DeclareBlockers,
    );

    // The shield, off one Plains, before the damage.
    tap_mana_where(&mut engine, kit::P1, |id| id == plains_objects[0]);
    let mut answers = kit::Answers::new()
        .mode(CastModeKind::Mode(1))
        .aim_players(&[kit::P1]);
    kit::cast(&mut engine, kit::P1, index::HEALING_SALVE, &mut answers);
    kit::settle(&mut engine, kit::P1, &mut answers);

    kit::walk_to_quiet_priority(
        &mut engine,
        &mut kit::Answers::new(),
        kit::P1,
        Step::CombatDamage,
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        18,
        "Juggernaut's 5, less the 3 the shield absorbed"
    );

    cast_reverse_polarity(&mut engine);
    assert_eq!(
        kit::life(&engine, kit::P1),
        22,
        "gain twice the 2 that was dealt, not the 5 that was aimed"
    );
}

/// "So far this turn": Juggernaut's 5 on turn 1 is not part of turn 2's count.
/// On turn 2 p0 pings p1 with the Rod once, and Reverse Polarity — cast in p1's
/// own main phase — gains twice that 1 and nothing for the earlier 5. p1 ends
/// at 20 - 5 - 1 + 2 = 16; counting the earlier turn would end at 26, and the
/// Rod's 1 being counted at all is the control that the spell reads this turn.
#[test]
fn reverse_polarity_counts_this_turns_damage_and_not_an_earlier_turns() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                kit::juggernaut(),
                index::ROD_OF_RUIN,
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[plains(), plains()])
        .hand(1, &[reverse_polarity()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let juggernaut = on_battlefield(&engine, kit::P0, kit::juggernaut()).expect("Juggernaut");
    let rod = on_battlefield(&engine, kit::P0, index::ROD_OF_RUIN).expect("the Rod is out");

    kit::attack(&mut engine, kit::P0, &[juggernaut], kit::P1);
    kit::walk_to_next_own_main(&mut engine, kit::P1);
    assert_eq!(
        kit::life(&engine, kit::P1),
        15,
        "Juggernaut's 5 on turn 1 is already on the books"
    );

    // p1, the active player, passes; p0 answers with the Rod.
    engine
        .apply(kit::P1, PlayerAction::PassPriority)
        .expect("p1 passes in its own main phase");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == kit::P0),
        "p0 gets priority next, got {:?}",
        engine.pending()
    );
    tap_all_mana(&mut engine, kit::P0);
    let mut answers = kit::Answers::new().aim_players(&[kit::P1]);
    kit::activate_ability(&mut engine, kit::P0, rod, 0, &mut answers);
    kit::settle(&mut engine, kit::P1, &mut answers);
    assert_eq!(kit::life(&engine, kit::P1), 14, "the Rod dealt 1 this turn");

    cast_reverse_polarity(&mut engine);
    assert_eq!(
        kit::life(&engine, kit::P1),
        16,
        "gain twice this turn's 1 — the 5 from the turn before is not counted"
    );
}
