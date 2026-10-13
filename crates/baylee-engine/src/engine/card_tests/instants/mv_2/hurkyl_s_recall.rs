//! `cards/instants/mv_2/hurkyl_s_recall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

// oracle_id = "ed1e5d24-c8a8-48fe-a88f-1003ad432477"
fn hurkyl_s_recall() -> CardIndex {
    card_index("ed1e5d24-c8a8-48fe-a88f-1003ad432477")
}

// oracle_id = "cd8ae9f2-edac-473a-8846-c08219e617c3"
fn steal_artifact() -> CardIndex {
    card_index("cd8ae9f2-edac-473a-8846-c08219e617c3")
}

/// What the table holds when Hurkyl's Recall is cast.
struct Table {
    /// The Sol Ring player 1 owns and player 0 controls (Steal Artifact).
    stolen: ObjectId,
    /// Player 1's own Pendant, which they control.
    pendant: ObjectId,
    /// Player 1's own artifact creature.
    gargoyle: ObjectId,
    /// Player 1's second Sol Ring, already in their graveyard.
    dead_ring: ObjectId,
    /// Player 1's Elves, not an artifact.
    elves: ObjectId,
    /// Player 0's own Mox Opal, which they control.
    opal: ObjectId,
    /// The Steal Artifact that makes the ring player 0's to control.
    aura: ObjectId,
}

/// A table where player 0 holds Hurkyl's Recall with `{1}{U}` floating, after
/// stealing player 1's Sol Ring with Steal Artifact: so that each player owns
/// something the other controls, and the spell is cast into a board where
/// "owns" and "controls" disagree.
///
/// Player 1 owns the stolen Sol Ring, a Pendant and a Gargoyle (an artifact
/// creature) on the battlefield, a second Sol Ring in their graveyard and an
/// Elves that is no artifact; player 0 owns a Mox Opal.
#[track_caller]
fn the_table_where_owners_and_controllers_differ() -> (Engine<RegistryLookup>, Table) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                mox_opal(),
            ],
        )
        .hand(0, &[steal_artifact(), hurkyl_s_recall()])
        .battlefield(
            1,
            &[
                sol_ring(),
                sol_ring(),
                darksteel_pendant(),
                darksteel_gargoyle(),
                llanowar_elves(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rings = all_on_battlefield(&engine, p1, sol_ring());
    assert_eq!(rings.len(), 2, "two Sol Rings on player 1's side");
    let (stolen, dead_ring) = (rings[0], rings[1]);
    let pendant = on_battlefield(&engine, p1, darksteel_pendant()).expect("their Pendant");
    let gargoyle = on_battlefield(&engine, p1, darksteel_gargoyle()).expect("their Gargoyle");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    let opal = on_battlefield(&engine, p0, mox_opal()).expect("my Opal");

    // A ring in a graveyard is an artifact card, and no artifact: the spell
    // returns what is on the battlefield.
    bury(&mut engine, &[dead_ring]);
    assert_eq!(
        kit::zone_of(&engine, dead_ring),
        Zone::Graveyard,
        "the second Sol Ring is a card in player 1's graveyard"
    );

    // Steal Artifact first, off the islands; two are left floating for the
    // spell this table is built for.
    cast_from_hand(&mut engine, p0, steal_artifact());
    aim_at(&mut engine, p0, stolen);
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, steal_artifact()).expect("the Aura is attached");
    assert_eq!(
        kit::controller_of(&engine, stolen),
        p0,
        "player 0 controls the ring"
    );
    assert_eq!(
        kit::owner_of(&engine, stolen),
        p1,
        "player 1 still owns it, which is the whole of the point"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two of the six islands are still floating for {{1}}{{U}}"
    );

    (
        engine,
        Table {
            stolen,
            pendant,
            gargoyle,
            dead_ring,
            elves,
            opal,
            aura,
        },
    )
}

/// Casts Hurkyl's Recall off the floating mana and names `target`, reading the
/// menu it offers on the way: "target player" means every player, the caster
/// included, and no object.
#[track_caller]
fn cast_it_at(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    let p0 = PlayerId::new(0);
    cast_with_floating(engine, p0, hurkyl_s_recall());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target player\" asks the caster for a target; the spell is on the stack \
             ({}) but the question is {:?}",
            !stack_is_empty(engine),
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster names the target");
    assert_eq!((min, max), (1, 1), "one target");
    assert!(
        options.is_empty(),
        "no object is a legal target: {options:?}"
    );
    assert!(
        player_options.contains(&PlayerId::new(0)) && player_options.contains(&PlayerId::new(1)),
        "any player, the caster too: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .expect("the named player was on the menu");
    pass_until(engine, stack_is_empty);
}

/// "Return all artifacts target player owns to their hand."
///
/// Player 1 is the target. Everything they own that is an artifact on the
/// battlefield goes to *their* hand — the Pendant, the artifact creature, and
/// the Sol Ring that player 0 has been controlling since Steal Artifact
/// resolved (ruling 2004-10-04: "regardless of who controls them") — and
/// nothing else does: the Elves is no artifact, player 0's own Mox Opal is not
/// owned by the target, and the Sol Ring in the graveyard is no longer on the
/// battlefield.
#[test]
fn hurkyl_s_recall_returns_every_artifact_the_target_owns_even_one_an_opponent_controls() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, table) = the_table_where_owners_and_controllers_differ();

    cast_it_at(&mut engine, p1);

    assert_eq!(
        kit::zone_of(&engine, table.stolen),
        Zone::Hand,
        "the stolen Sol Ring is player 1's, so it goes back to player 1"
    );
    assert_eq!(
        in_hand(&engine, p1, sol_ring()),
        Some(table.stolen),
        "to the hand of its owner, not of the player who controlled it"
    );
    assert!(
        in_hand(&engine, p0, sol_ring()).is_none(),
        "and player 0 gets nothing"
    );
    assert!(
        in_hand(&engine, p1, darksteel_pendant()).is_some(),
        "the Pendant player 1 owns and controls returns"
    );
    assert!(
        in_hand(&engine, p1, darksteel_gargoyle()).is_some(),
        "an artifact creature is an artifact"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        3,
        "exactly those three"
    );
    assert!(
        kit::is_on_battlefield(&engine, table.elves),
        "the Elves are no artifact and stay"
    );
    assert!(
        kit::is_on_battlefield(&engine, table.opal),
        "player 0's own artifact is not owned by the target and stays"
    );
    assert_eq!(
        kit::zone_of(&engine, table.dead_ring),
        Zone::Graveyard,
        "an artifact card in a graveyard is not on the battlefield and stays"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "lands are no artifacts"
    );
    assert!(
        in_graveyard(&engine, p0, hurkyl_s_recall()).is_some(),
        "the spell is spent"
    );
    assert!(
        kit::zone_of(&engine, table.aura) == Zone::Graveyard,
        "with nothing left to enchant the Aura goes to its owner's graveyard \
         (CR 704.5m)"
    );
    // Nothing of what returned is still anyone's to control.
    assert!(on_battlefield(&engine, p0, sol_ring()).is_none());
}

/// "Return all artifacts target player owns to their hand" — read from the
/// other side: the target is player 0, who *controls* player 1's Sol Ring and
/// *owns* the Mox Opal.
///
/// Their own Opal comes back to their hand (a player may target themselves);
/// the ring they merely control stays where it is, under their control, and
/// so does the Pendant that player 1 owns (ruling 2004-10-04: "ignores
/// artifacts owned by other players even if target player has control of
/// them").
#[test]
fn hurkyl_s_recall_leaves_an_artifact_the_target_controls_but_does_not_own() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, table) = the_table_where_owners_and_controllers_differ();

    cast_it_at(&mut engine, p0);

    assert_eq!(
        kit::zone_of(&engine, table.opal),
        Zone::Hand,
        "the Opal player 0 owns returns"
    );
    assert_eq!(
        in_hand(&engine, p0, mox_opal()),
        Some(table.opal),
        "to its owner's hand"
    );
    assert!(
        kit::is_on_battlefield(&engine, table.stolen),
        "the ring player 0 controls but player 1 owns stays on the battlefield"
    );
    assert_eq!(
        kit::controller_of(&engine, table.stolen),
        p0,
        "and under the same control"
    );
    assert!(
        kit::is_on_battlefield(&engine, table.aura),
        "which is why the Aura is still attached"
    );
    assert!(
        kit::is_on_battlefield(&engine, table.pendant)
            && kit::is_on_battlefield(&engine, table.gargoyle),
        "player 1's other artifacts are not the target's"
    );
    assert!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).is_empty(),
        "player 1 gets nothing back"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "exactly the Opal"
    );
}

/// "Return all artifacts target player owns to their hand" with no artifact
/// to return: the spell resolves and nothing moves (it is not a reason to
/// fizzle, since the target player is legal).
#[test]
fn hurkyl_s_recall_with_nothing_to_return_changes_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[hurkyl_s_recall()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);

    cast_it_at(&mut engine, p1);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some()
            && on_battlefield(&engine, p1, forest()).is_some(),
        "no artifact, so no permanent moved"
    );
    assert!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).is_empty(),
        "and nobody's hand grew"
    );
    assert!(
        in_graveyard(&engine, p0, hurkyl_s_recall()).is_some(),
        "the spell resolved and went to the graveyard"
    );
}
