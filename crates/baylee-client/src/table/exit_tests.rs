use super::*;
use std::time::Duration;

fn seat() -> SeatSlot {
    SeatSlot {
        player: baylee_core::ids::PlayerId::new(0),
        ring_index: 0,
        angle: 0.0,
        center: Vec2::ZERO,
        facing: 0.0,
        half_extent: Vec2::new(6.0, 3.0),
        reclaimed: 0.0,
        is_local: true,
    }
}

/// A card lying flat on the near seat's board.
fn standing() -> Transform {
    card_transform(&seat(), Vec2::ZERO, false, 0.0)
}

/// Where that seat's graveyard stands, as `placements` puts its top card.
fn grave() -> Transform {
    let slot = seat();
    card_transform(&slot, slot.pile_center(PileKind::Graveyard), false, 0.0)
}

/// The fault this whole pass exists to correct. A pile's top card *is* a
/// placement, so a creature that dies while another dies with it has one
/// of the two glide to the graveyard and the other marked stale — and the
/// two must end up in the same place, because which of them is on top is
/// an accident of sort order and nothing a player can see a reason for.
#[test]
fn a_permanent_that_dies_goes_to_the_pile_the_top_card_glides_to() {
    let at = standing();
    let pile = grave();
    let gone = exit(Some(Place::Graveyard(seat().player)), Some(pile), &at);
    assert!(
        (gone.translation.xz() - pile.translation.xz()).length() < 1e-4,
        "it belongs on the pile, not at its lane: {gone:?}"
    );
    assert_eq!(gone.rotation, pile.rotation, "lying as the pile lies");
    // And behind the card already standing there, or the two fight for
    // the same depth and the pile flickers between them.
    assert!(gone.translation.y < pile.translation.y, "tucked under it");
    assert!(
        pile.translation.y - gone.translation.y < CARD_HEIGHT * 0.01,
        "but only just: it is hidden, not dropped"
    );
}

#[test]
fn a_bounced_permanent_leaves_the_way_a_card_arrives() {
    let at = standing();
    let gone = exit(Some(Place::Hand), None, &at);
    let arriving = entrance(&at);
    assert!(gone.translation.y > arriving.translation.y, "further up");
    assert!(gone.scale.length() < arriving.scale.length(), "and smaller");
    assert!(
        gone.translation.y > at.translation.y && gone.scale.length() < at.scale.length(),
        "which is the entrance run backwards"
    );
}

/// The honest exit. It must be confusable with neither of the others, or
/// a card whose fate is unknown would be reported as buried.
#[test]
fn a_card_this_seat_cannot_follow_leaves_quietly() {
    let at = standing();
    for unknown in [None, Some(Place::Stack)] {
        let gone = exit(unknown, None, &at);
        assert!(
            (gone.translation - at.translation).length() < 1e-4,
            "it goes nowhere: {unknown:?}"
        );
        assert!(gone.scale.length() < at.scale.length() * 0.1, "{unknown:?}");
        // Exactly, not nearly: `Quat::angle_between` is an `acos` and is
        // worth about 7e-4 of noise on two identical rotations, which is
        // more slack than this assertion has to give.
        assert_eq!(gone.rotation, at.rotation, "and does not turn: {unknown:?}");
    }
}

/// A pile place with no pile to send it to is the same unknown fate, and
/// has to read as one: a layout that has not arrived yet must not make a
/// death look like a bounce.
#[test]
fn a_pile_this_table_is_not_drawing_is_no_destination_at_all() {
    let at = standing();
    let nowhere = exit(Some(Place::Graveyard(seat().player)), None, &at);
    assert_eq!(nowhere, exit(None, None, &at));
}

/// Coming back out is going in, run backwards — which is what makes a
/// resurrection read as one rather than as a fresh card being made.
#[test]
fn a_permanent_returning_from_a_pile_comes_off_that_pile() {
    let at = standing();
    let pile = grave();
    let start = entrance_from(Some(pile), &at);
    assert_eq!(start, pile);
    let gone = exit(Some(Place::Graveyard(seat().player)), Some(pile), &at);
    // The tuck apart and no more, compared with a hair of slack: the two
    // differ by exactly `PILE_TUCK`, which an `f32` subtraction does not
    // reproduce to the last bit.
    assert!(
        (start.translation - gone.translation).length() < PILE_TUCK * 1.01,
        "it leaves from where it arrived: {start:?} against {gone:?}"
    );
    assert!(
        (start.scale - at.scale).length() < 1e-4,
        "at full size: it is the card coming back, not a card being made"
    );
}

/// A creature cast from hand arrives from the stack; a token arrives from
/// nowhere. Both of them belong dropping onto their mark, which is what
/// this table has always done.
#[test]
fn every_other_arrival_is_the_one_the_table_has_always_drawn() {
    let at = standing();
    assert_eq!(entrance_from(None, &at), entrance(&at));
}

/// The seat is half the answer: two graveyards are two places, and a card
/// dying under an opponent's control belongs at *their* pile.
#[test]
fn two_seats_piles_are_two_different_places() {
    let mut far = seat();
    far.player = baylee_core::ids::PlayerId::new(1);
    far.center = Vec2::new(0.0, -12.0);
    far.facing = std::f32::consts::PI;
    let theirs = card_transform(&far, far.pile_center(PileKind::Graveyard), false, 0.0);
    assert!(
        (theirs.translation - grave().translation).length() > 1.0,
        "both graveyards drawn at the same point"
    );
}

/// The exits are only worth anything if the card is still there to play
/// them, and only harmless if it eventually is not.
#[test]
fn a_departing_card_is_despawned_when_its_time_is_up_and_not_before() {
    let mut app = App::new();
    app.init_resource::<Time>().add_systems(Update, retire);
    let card = app.world_mut().spawn(Departing { left: EXIT_LIFE }).id();

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(EXIT_LIFE * 0.5));
    app.update();
    assert!(
        app.world().get_entity(card).is_ok(),
        "half way through it is still leaving"
    );

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(EXIT_LIFE));
    app.update();
    assert!(app.world().get_entity(card).is_err(), "and then it is gone");
}

/// A plain card material, as `material` would build one for a card with
/// nothing happening to it.
fn plain(materials: &mut Assets<CardMaterial>) -> Handle<CardMaterial> {
    materials.add(crate::cardmat::material(
        CardLook::flat(
            Color::srgb(0.5, 0.5, 0.5),
            baylee_client_core::images::FinishTreatment::Plain,
        ),
        None,
        Color::srgb(0.5, 0.5, 0.5),
        MOVING,
    ))
}

fn leaving_for(to: Option<Place>) -> zones::Move {
    zones::Move {
        object: ObjectId::new(1, 0),
        from: Some(Place::Battlefield),
        to,
    }
}

/// The claim item 4 is about, and the one nothing else can make: a card
/// on its way out wears the door it is going through.
///
/// An outcome and not a call — the handle changes and the params on the
/// far side of it say which door — because the failure this guards is a
/// dressing that runs and writes nothing, which looks from every other
/// angle exactly like a card that left the ordinary way.
#[test]
fn a_card_leaving_the_table_wears_the_door_it_goes_through() {
    let mut materials = Assets::<CardMaterial>::default();
    let seat = baylee_core::ids::PlayerId::new(0);
    for (to, want) in [
        (Place::Graveyard(seat), crate::cardmat::door::DESTROYED),
        (Place::Exile(seat), crate::cardmat::door::EXILED),
        (Place::Hand, crate::cardmat::door::BOUNCE),
    ] {
        let worn = plain(&mut materials);
        let dressed = dress_the_exit(&mut materials, &worn, leaving_for(Some(to)), 12.0, MOVING)
            .unwrap_or_else(|| panic!("a card going to {to:?} was dressed in nothing"));
        assert_ne!(dressed, worn, "it kept the material it arrived in");
        let params = materials.get(&dressed).expect("the new material").params;
        assert_eq!(params.sweep_door, want, "the wrong door for {to:?}");
        // And it is a real one-shot on the clock it was given, or the
        // shader draws the door at a phase it never leaves.
        assert!(
            (params.sweep_at - 12.0).abs() < 1e-3,
            "started at {} rather than now",
            params.sweep_at
        );
        assert!(
            (params.sweep_rate - 1.0 / EXIT_LIFE).abs() < 1e-3,
            "it does not last exactly as long as the exit it rides"
        );
    }
}

/// The counter-tests, which matter more than the claim: three ways of
/// leaving that must stay undressed.
#[test]
fn a_card_leaving_by_no_door_is_dressed_in_nothing() {
    let mut materials = Assets::<CardMaterial>::default();
    let worn = plain(&mut materials);
    // Somewhere this seat cannot see: an opponent's hand, the bottom of a
    // library. `zones` reports `None` rather than guessing, and a guess
    // here would be a portal drawn over a card that was merely bounced.
    assert!(
        dress_the_exit(&mut materials, &worn, leaving_for(None), 12.0, MOVING).is_none(),
        "a move with one end missing was given a door"
    );
    // The stack. A permanent going there did not leave through any of the
    // five, and a mark on it would be the everywhere-at-once again.
    assert!(
        dress_the_exit(
            &mut materials,
            &worn,
            leaving_for(Some(Place::Stack)),
            12.0,
            MOVING
        )
        .is_none(),
        "leaving for the stack was drawn as a door"
    );
    // And a player who has turned motion off sees none of it — the same
    // decision an arriving card makes, made in the same function.
    let seat = baylee_core::ids::PlayerId::new(0);
    let still = dress_the_exit(
        &mut materials,
        &worn,
        leaving_for(Some(Place::Graveyard(seat))),
        12.0,
        crate::cardmat::STILL,
    )
    .expect("a still card is still dressed, just with nothing happening");
    let params = materials.get(&still).expect("the new material").params;
    assert!(
        params.sweep_rate.abs() < f32::EPSILON,
        "a still card was given a travel"
    );
    assert_eq!(
        params.sweep_door,
        crate::cardmat::door::NONE,
        "a still card was given a door to travel through"
    );
}
