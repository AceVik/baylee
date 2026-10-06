use super::*;

fn sorted(mut ids: Vec<ObjectId>) -> Vec<ObjectId> {
    ids.sort_by_key(|o| o.slot());
    ids
}

/// A seat's commander line is that seat's, and it is one entry per
/// commander.
///
/// It used to be `GameState::commander_casts` — one number per *seat* —
/// cloned whole into every seat's line, which is why the command-zone
/// panel indexed it by command-zone slot and got away with it: at a duel
/// where both seats have one commander the two shapes coincide. They
/// stop coinciding here, in both directions at once.
#[test]
fn each_seats_commanders_are_its_own_and_are_listed_one_per_commander() {
    let preset = commander_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");

    for seat in [0u8, 1] {
        let view = player_view(
            engine.state(),
            PlayerId::new(seat),
            0,
            None,
            &SeatContext::default(),
            &[],
        );
        assert_eq!(
            view.seats[0].commanders.len(),
            2,
            "seat 0 has partners, and seat {seat}'s view forgot one"
        );
        assert_eq!(
            view.seats[1].commanders.len(),
            1,
            "seat 1 has one commander, and seat {seat}'s view invented another"
        );
        for (i, line) in view.seats.iter().enumerate() {
            assert_eq!(
                sorted(line.commanders.iter().map(|c| c.object).collect()),
                sorted(view.command[i].iter().map(|o| o.id).collect()),
                "seat {i}'s line does not name the cards in seat {i}'s command zone"
            );
            assert!(
                line.commanders.iter().all(|c| c.casts == 0),
                "nothing has been cast yet"
            );
        }
    }

    // Everything above still passes if `casts` is filled from the seat
    // total, because at the start of a game every count is zero. So give
    // the three commanders three different numbers — none of which is a
    // number any *seat* could be holding — and read them back. This is
    // the assertion the old shape could not have satisfied: seat 0's two
    // commanders have to answer 2 and 5, and one number per seat cannot
    // say that.
    let mut state = engine.state().clone();
    state.commanders[0][0].casts = 2;
    state.commanders[0][1].casts = 5;
    state.commanders[1][0].casts = 7;
    state.commander_casts = vec![99, 99];

    for seat in [0u8, 1] {
        let view = player_view(
            &state,
            PlayerId::new(seat),
            0,
            None,
            &SeatContext::default(),
            &[],
        );
        let casts =
            |i: usize| -> Vec<u32> { view.seats[i].commanders.iter().map(|c| c.casts).collect() };
        assert_eq!(casts(0), vec![2, 5], "seat {seat}'s view of the partners");
        assert_eq!(casts(1), vec![7], "seat {seat}'s view of seat 1");
    }
}

/// The marker on the object and the seat's line are the same claim, so
/// they must never disagree. The object carries it only so that a
/// renderer holding one card does not have to carry the seat list down
/// with it — a redundancy that is safe exactly as long as this holds.
#[test]
fn the_marker_on_a_card_agrees_with_the_seat_that_claims_it() {
    let preset = commander_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let view = player_view(
        engine.state(),
        PlayerId::new(0),
        0,
        None,
        &SeatContext::default(),
        &[],
    );

    let named: Vec<ObjectId> = view
        .seats
        .iter()
        .flat_map(|s| s.commanders.iter().map(|c| c.object))
        .collect();
    assert_eq!(named.len(), 3, "two commanders for seat 0, one for seat 1");

    let mut marked = 0;
    for obj in view.command.iter().flatten().chain(&view.battlefield) {
        assert_eq!(
            obj.commander,
            named.contains(&obj.id),
            "the marker on {} disagrees with the seat list",
            obj.name
        );
        marked += usize::from(obj.commander);
    }
    assert_eq!(marked, 3, "the command zones did not carry the marker");
    assert!(
        view.battlefield.iter().all(|o| !o.commander),
        "an Island is not anybody's commander"
    );
}

/// A commander that declined CR 903.9b's replacement sits in its owner's
/// hand, and everything the view says about it has to survive the trip.
///
/// Two claims, and the second is the one with a way to go wrong. The
/// marker stays, because the card is still a commander. And *every* seat
/// keeps being told its identity — CR 903.3 designates a commander
/// openly, so a hand is not a hiding place for the fact that it is one —
/// which means the printing has to be earned by a seat that can no
/// longer see the card in any zone it is sent.
#[test]
fn a_commander_in_its_owners_hand_keeps_its_marker_and_its_printing() {
    let preset = commander_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let mut state = engine.state().clone();
    let katara_obj = state.commanders[0][0].object;
    let print = state
        .object(katara_obj)
        .and_then(|o| o.card)
        .expect("a commander has a card")
        .print;
    state
        .move_object(
            katara_obj,
            ZoneLocation::Hand(PlayerId::new(0)),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .expect("the commander reaches its owner's hand");

    let owner = player_view(
        &state,
        PlayerId::new(0),
        0,
        None,
        &SeatContext::default(),
        &[],
    );
    let held = owner
        .hand
        .iter()
        .find(|o| o.id == katara_obj)
        .expect("it is in the hand it was sent to");
    assert!(held.commander, "it is still a commander in a hand");

    let other = player_view(
        &state,
        PlayerId::new(1),
        0,
        None,
        &SeatContext::default(),
        &[],
    );
    assert!(
        other.command[0].iter().all(|o| o.id != katara_obj),
        "it has left the command zone, so no seat sees it there"
    );
    let named = other.seats[0]
        .commanders
        .iter()
        .find(|c| c.object == katara_obj)
        .expect("seat 0's line still names it");
    assert_eq!(
        named.card.map(|c| c.print),
        Some(print),
        "and still says which printing it is"
    );
    assert!(
        other.prints().any(|p| p == print),
        "a seat told about a printing has to earn it, or it draws a hole"
    );
}

/// CR 302.6 is a rule about creatures, and the projection says so. The
/// field used to answer "did this permanent enter this turn", which is
/// also true of a land the player just played — so every client had to
/// mask it off again to avoid drawing a whole opening board asleep, and
/// the fact itself stayed wrong for anything that read it straight.
#[test]
fn only_a_creature_is_projected_summoning_sick() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let mut state = engine.state().clone();
    let seat = PlayerId::new(0);

    let mut fresh = |name: &str, types| {
        let name = state.names.intern(name);
        let id = state.create_bare(
            seat,
            baylee_engine::object::ObjectKind::Permanent,
            name,
            baylee_engine::zone::ZoneLocation::Battlefield,
        );
        state.object_mut(id).expect("just created").base_mut().types = types;
        id
    };
    let land = fresh("Fresh Land", baylee_core::types::TypeSet::LAND);
    let bear = fresh("Fresh Bear", baylee_core::types::TypeSet::CREATURE);

    let view = player_view(&state, seat, 0, None, &SeatContext::default(), &[]);
    let asleep = |id: ObjectId| {
        view.battlefield
            .iter()
            .find(|o| o.id == id)
            .expect("the permanent is in the view")
            .summoning_sick
    };
    assert!(
        !asleep(land),
        "a land played this turn was projected summoning sick"
    );
    assert!(asleep(bear), "a creature that entered this turn is asleep");
}

/// CR 306.5c: a planeswalker's loyalty is the counters on it, not the
/// number printed on the card. The client draws the plate off this field,
/// so a printed number meant a walker stood at its starting loyalty for
/// the whole game however it was ticked or attacked.
#[test]
fn a_planeswalker_is_projected_at_the_loyalty_it_has() {
    let preset = mixed_print_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let mut state = engine.state().clone();
    let seat = PlayerId::new(0);

    let mut walker = |zone, counters: u16| {
        let name = state.names.intern("Fresh Walker");
        let kind = if matches!(zone, baylee_engine::zone::ZoneLocation::Battlefield) {
            baylee_engine::object::ObjectKind::Permanent
        } else {
            baylee_engine::object::ObjectKind::Card
        };
        let id = state.create_bare(seat, kind, name, zone);
        let obj = state.object_mut(id).expect("just created");
        obj.base_mut().types = baylee_core::types::TypeSet::PLANESWALKER;
        obj.base_mut().loyalty = Some(4);
        obj.counters
            .set(baylee_cards_dsl::CounterKind::Loyalty, counters);
        id
    };
    let ticked = walker(baylee_engine::zone::ZoneLocation::Battlefield, 6);
    let dying = walker(baylee_engine::zone::ZoneLocation::Battlefield, 1);
    let held = walker(baylee_engine::zone::ZoneLocation::Graveyard(seat), 0);

    let view = player_view(&state, seat, 0, None, &SeatContext::default(), &[]);
    let loyalty = |id: ObjectId| {
        view.battlefield
            .iter()
            .chain(view.graveyards.iter().flatten())
            .find(|o| o.id == id)
            .expect("the object is in the view")
            .loyalty
    };
    // Both directions: a printed number would answer 4 for each of them,
    // so one of these alone proves nothing.
    assert_eq!(loyalty(ticked), Some(6), "a walker that ticked up");
    assert_eq!(loyalty(dying), Some(1), "a walker that has been attacked");
    // Off the battlefield there are no counters and the card is what it
    // prints, which is the answer a graveyard panel wants.
    assert_eq!(
        loyalty(held),
        Some(4),
        "a walker card is its printed number"
    );
}

/// The tally is a second life total (CR 903.10a), and it is public: the
/// seat taking the damage is not the only one who needs to see how close
/// twenty-one is.
#[test]
fn commander_damage_reaches_every_seats_view_keyed_by_the_commander() {
    let preset = commander_preset();
    let engine = Engine::new(&preset, Registry).expect("game starts");
    let mut state = engine.state().clone();
    let katara_obj = state.commanders[0][0].object;
    let norn_obj = state.commanders[0][1].object;
    state.players[1].commander_damage.push((katara_obj, 13));
    state.players[1].commander_damage.push((norn_obj, 4));

    for seat in [0u8, 1] {
        let view = player_view(
            &state,
            PlayerId::new(seat),
            0,
            None,
            &SeatContext::default(),
            &[],
        );
        let taken: Vec<(ObjectId, u16)> = view.seats[1]
            .commander_damage
            .iter()
            .map(|d| (d.source, d.amount))
            .collect();
        assert_eq!(
            taken,
            vec![(katara_obj, 13), (norn_obj, 4)],
            "seat {seat} was not told what seat 1 has taken, and from which commander"
        );
        assert!(
            view.seats[0].commander_damage.is_empty(),
            "seat 0 has taken none"
        );
    }
}
