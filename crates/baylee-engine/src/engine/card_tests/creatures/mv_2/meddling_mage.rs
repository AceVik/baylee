//! `cards/creatures/mv_2/meddling_mage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn meddling_mage() -> CardIndex {
    card_index("a5d02339-9f6d-486e-87d8-5ec2274cf680")
}

#[track_caller]
fn cast_mage_waiting_for_name(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    cast_from_hand(engine, seat, meddling_mage());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
}

#[track_caller]
fn choose_name(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) {
    engine
        .apply(seat, PlayerAction::ChooseCardName { card, face: 0 })
        .expect("a nonland card of the pool may be named");
}

#[track_caller]
fn named_mage(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> ObjectId {
    cast_mage_waiting_for_name(engine, seat);
    choose_name(engine, seat, card);
    on_battlefield(engine, seat, meddling_mage()).expect("the Mage entered")
}

/// Meddling Mage: "As this creature enters, choose a nonland card name.
/// Spells with the chosen name can't be cast."
///
/// Forest is neither a permissible answer nor a spell the lock stops. The
/// named Sol Ring is refused to either player, both in the offered actions
/// and at the engine's action boundary, while Brainstorm is another name and
/// remains castable.
#[test]
fn meddling_mage_names_a_nonland_and_stops_that_spell_for_both_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(604, forest())
        .battlefield(0, &[plains(), island(), island()])
        .hand(0, &[meddling_mage(), sol_ring(), brainstorm(), forest()])
        .battlefield(1, &[island()])
        .hand(1, &[sol_ring(), brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_mage_waiting_for_name(&mut engine, p0);
    assert!(
        matches!(engine.pending(), Pending::ChooseCardName {
            player, types, excluded_types
        } if *player == p0 && types.is_empty() && *excluded_types == TypeSet::LAND),
        "the Mage's controller chooses any printed nonland face: {:?}",
        engine.pending()
    );

    for (seat, card, face, what) in [
        (p0, forest(), 0, "a land name"),
        (p1, sol_ring(), 0, "the other player's answer"),
        (p0, sol_ring(), 1, "a face the card does not print"),
    ] {
        let before = engine.state().snapshot_hash();
        assert!(
            engine
                .apply(seat, PlayerAction::ChooseCardName { card, face })
                .is_err(),
            "{what} is refused"
        );
        assert_eq!(engine.state().snapshot_hash(), before, "nothing changes");
        assert!(
            matches!(engine.pending(), Pending::ChooseCardName { player, .. } if *player == p0),
            "the same question remains open after {what}"
        );
    }
    choose_name(&mut engine, p0, sol_ring());
    let mage = on_battlefield(&engine, p0, meddling_mage()).expect("the Mage is out");
    assert_eq!(
        engine
            .state()
            .object(mage)
            .and_then(|o| o.chosen_name)
            .map(|name| (name.card(), name.face())),
        Some((sol_ring(), 0)),
        "the choice belongs to the entering permanent"
    );

    for (seat, step) in [(p0, "its controller"), (p1, "its opponent")] {
        if seat == p1 {
            reach_their_main_phase(&mut engine, p1);
            tap_all_mana(&mut engine, p1);
        }
        let named = in_hand(&engine, seat, sol_ring()).expect("the named card is in hand");
        let other = in_hand(&engine, seat, brainstorm()).expect("another spell is in hand");
        let legal = priority_offer(&engine);
        assert!(
            !legal.castable.contains(&named),
            "the named spell is not offered to {step}"
        );
        assert!(
            legal.castable.contains(&other),
            "another name remains offered to {step}"
        );
        let before = engine.state().snapshot_hash();
        assert!(
            engine
                .apply(seat, PlayerAction::CastSpell { card: named })
                .is_err(),
            "a direct cast of the named spell is refused to {step}"
        );
        assert_eq!(
            engine.state().snapshot_hash(),
            before,
            "a refused cast changes nothing"
        );
        if seat == p0 {
            play_land(&mut engine, p0, forest());
            assert_eq!(
                engine.state().players[0].lands_played_this_turn,
                1,
                "a land is played, not cast, and the Mage does not stop it"
            );
        }
    }
}

/// The name is not a property of the card forever. Boomerang returns the Mage
/// to hand, ending its static lock and forgetting the name; the next Mage is
/// a new object and asks again, this time naming Brainstorm instead.
#[test]
fn meddling_mage_leaving_and_reentering_forgets_the_old_name() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(605, forest())
        .battlefield(0, &[plains(), plains(), island(), island(), forest()])
        .hand(0, &[meddling_mage(), sol_ring(), brainstorm()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[boomerang()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mage = named_mage(&mut engine, p0, sol_ring());
    let ring = in_hand(&engine, p0, sol_ring()).expect("Sol Ring is in hand");
    assert!(!priority_offer(&engine).castable.contains(&ring));

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, boomerang());
    aim_at(&mut engine, p1, mage);
    pass_until(&mut engine, stack_is_empty);
    let returned = in_hand(&engine, p0, meddling_mage()).expect("the Mage is back in hand");
    assert_eq!(
        engine.state().object(returned).and_then(|o| o.chosen_name),
        None,
        "the card in hand is a new object with no chosen name"
    );
    assert!(
        priority_offer(&engine).castable.contains(&ring),
        "the old Mage no longer locks Sol Ring"
    );

    cast_mage_waiting_for_name(&mut engine, p0);
    choose_name(&mut engine, p0, brainstorm());
    let legal = priority_offer(&engine);
    assert!(
        legal.castable.contains(&ring),
        "the new name leaves Sol Ring free"
    );
    let brainstorm = in_hand(&engine, p0, brainstorm()).expect("Brainstorm is in hand");
    assert!(
        !legal.castable.contains(&brainstorm),
        "and locks Brainstorm instead"
    );
}
