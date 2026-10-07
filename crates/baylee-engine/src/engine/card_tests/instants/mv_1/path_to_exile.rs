//! `cards/instants/mv_1/path_to_exile.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Path to Exile: "Exile target creature. **Its controller** may search their
/// library for a basic land card…"
///
/// The ramp is the half of the card that is not the removal, and it is asked
/// of the seat whose creature just died — never of the seat who cast the
/// spell. `PlayerRel::ControllerOfTarget` is how the card says that, and a
/// site that resolved it through `eval::players` got no seats at all and
/// returned early, so the card shipped as a strictly better Swords to
/// Plowshares that also gave the opponent nothing.
///
/// The assertion therefore names the seat, not just the question: an
/// implementation that offered the search to the *caster* would be exactly as
/// wrong and would pass a test that only counted a `ChooseCards`.
#[test]
fn path_to_exile_offers_the_ramp_to_the_creatures_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[plains()])
        .hand(1, &[path_to_exile()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let victim = on_battlefield(&engine, p0, quiet_creature()).expect("p0's creature");
    let lands_before = lands_of(&engine, p0).len();

    cast_from_hand(&mut engine, p1, path_to_exile());
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!("Path asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "their spell, their target");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("Path points at the creature");

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "the ramp never asked — got {:?}. `OptionalBasicLandSearchFor` \
                 resolves `ControllerOfTarget`, which only `players_of` can \
                 answer.",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the basic-land search, got {:?}", engine.pending())
    };
    assert_eq!(
        player, p0,
        "the search belongs to the creature's controller, not to the caster"
    );
    assert_eq!((min, max), (0, 1), "\"may search\" — one card at most");
    assert!(
        !options.is_empty(),
        "p0's library is sixty Forests and every one of them is basic"
    );

    // The removal half happened too, and on the right card.
    assert_eq!(
        engine.state().object(victim).map(|o| o.zone),
        Some(Zone::Exile),
        "the creature is exiled, not destroyed"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("p0 takes the land");
    pass_until(&mut engine, |e| lands_of(e, p0).len() > lands_before);
    let fetched = *lands_of(&engine, p0).first().expect("the fetched land");
    assert!(
        engine
            .state()
            .object(fetched)
            .is_some_and(|o| o.status.contains(Status::TAPPED)),
        "\"put that card onto the battlefield tapped\""
    );
}

/// "…search their library for a basic land card, put that card onto the
/// battlefield tapped, then shuffle": the library shuffled is the one that
/// was searched. The resumed search shuffled the *resolving spell's
/// controller's* library, so Path's caster had their library shuffled for
/// nothing and the victim kept theirs in the order they had just looked
/// through.
///
/// The libraries are read as orders of object ids: the caster's must be
/// exactly as it was, and the victim's, less the land they took, must not.
#[test]
fn path_to_exile_shuffles_the_library_its_victim_searched() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[plains()])
        .hand(1, &[path_to_exile()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let victim = on_battlefield(&engine, p0, quiet_creature()).unwrap();
    let order = |e: &Engine<RegistryLookup>, seat: PlayerId| {
        e.state().zones.list(ZoneLocation::Library(seat)).clone()
    };
    let (searched_before, caster_before) = (order(&engine, p0), order(&engine, p1));

    cast_from_hand(&mut engine, p1, path_to_exile());
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    let Pending::ChooseCards {
        player, options, ..
    } = pass_to_card_choice(&mut engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, p0);
    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        order(&engine, p1),
        caster_before,
        "the caster searched nothing and is not shuffled"
    );
    let unshuffled: Vec<ObjectId> = searched_before
        .iter()
        .copied()
        .filter(|id| *id != found)
        .collect();
    let searched_after: Vec<ObjectId> = order(&engine, p0);
    assert_eq!(searched_after.len(), unshuffled.len());
    assert_ne!(
        searched_after, unshuffled,
        "\"then shuffle\": the searched library is not left in the order its \
         owner just saw"
    );
}

/// Misdirection on Path to Exile, with a Plains on each side of the table
/// (#247). "Change the target" means another **legal** target (CR 115.7a),
/// and a legal target for Path is a creature: the redirect used to offer
/// every permanent on the battlefield, so the Plains were offered, the AI
/// took one, and Path fizzled. It also offered the Elves Path already names,
/// which is not "another" target, and a phased-out Raptor, which does not
/// exist (CR 702.26b): the walk read the raw battlefield.
#[test]
fn misdirection_offers_path_another_creature_and_never_a_plains() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), llanowar_elves()])
        .hand(0, &[path_to_exile()])
        .battlefield(1, &[plains(), llanowar_elves(), umara_raptor()])
        .hand(1, &[misdirection(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let raptor = on_battlefield(&engine, p1, umara_raptor()).expect("the Raptor is out");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(raptor)
        .expect("the Raptor exists")
        .status
        .insert(crate::object::Status::PHASED_OUT);
    engine.refresh_offer();

    let path = aimed(&mut engine, p0, path_to_exile(), &[theirs], &[]);
    let Some(Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    }) = misdirected(&mut engine, p1, path)
    else {
        panic!("another creature stands, so the new target is asked for")
    };
    assert_eq!(player, p1, "the seat that cast Misdirection re-aims");
    assert_eq!(
        options,
        vec![mine],
        "\"exile target creature\": the other creature, and neither Plains, \
         nor the Elves Path already names, nor the phased-out Raptor"
    );
    assert!(
        player_options.is_empty(),
        "a creature target is never a player: {player_options:?}"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "\"change the target\" is not a may: with another legal target, it moves"
    );
}
