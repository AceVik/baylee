//! The land drop under a trigger that makes a land play cost life (`land`).

use super::*;

/// Fastbond on the seat's side of the table.
fn fastbond(slot: u32) -> PublicObject {
    carded(
        permanent(obj(slot), PlayerId::new(0), 0),
        "Fastbond",
        TypeSet::ENCHANTMENT,
    )
}

/// A land of the seat's, lying in its graveyard.
fn buried(slot: u32, name: &str) -> PublicObject {
    let mut land = carded(
        permanent(obj(slot), PlayerId::new(0), 0),
        name,
        TypeSet::LAND,
    );
    land.power = None;
    land.toughness = None;
    land
}

/// The seat's own first main phase at `life`, with `battlefield` out.
fn main_phase(life: i32, battlefield: Vec<PublicObject>) -> PlayerView {
    let mut v = view(0, &[life, 40], battlefield);
    v.phase = baylee_view::Phase::FirstMain;
    v.step = baylee_view::Step::Main;
    v
}

/// Priority with exactly these land plays on offer and nothing else.
fn lands_offered(lands: Vec<ObjectId>) -> Pending {
    Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            lands,
            ..Default::default()
        }),
    }
}

/// The game of the feedback of 28.10.2026, at its last land: Fastbond out,
/// five life, and a Petrified Field the graveyard permission offers again.
/// The house played it, and each land after it, until Fastbond had dealt it
/// every point it had.
#[test]
fn fastbond_does_not_play_the_seat_to_death() {
    let mut v = main_phase(5, vec![fastbond(1)]);
    v.graveyards[0].push(buried(2, "Petrified Field"));

    assert_eq!(
        agent().act(&v, &lands_offered(vec![obj(2)])),
        PlayerAction::PassPriority,
        "a land that Fastbond makes cost a point of five was played"
    );
}

/// At twenty life one point is cheap, but a land played from the graveyard
/// at a price is the loop itself: Petrified Field returns a land to the hand,
/// the graveyard permission plays the Field again, and each turn of it is a
/// point more. A land in the hand is bounded by the hand, and is played.
#[test]
fn fastbond_plays_a_land_from_the_hand_and_never_from_the_graveyard() {
    let mut v = main_phase(20, vec![fastbond(1)]);
    v.graveyards[0].push(buried(2, "Petrified Field"));
    v.hand.push(hand_card(3, "Forest"));

    assert_eq!(
        agent().act(&v, &lands_offered(vec![obj(2)])),
        PlayerAction::PassPriority,
        "the graveyard's land was played at a price"
    );
    assert_eq!(
        agent().act(&v, &lands_offered(vec![obj(2), obj(3)])),
        PlayerAction::PlayLand { card: obj(3) },
        "the hand's land was not preferred to the graveyard's"
    );
}

/// Without a trigger that costs life a land from the graveyard is played as
/// any land is: the rule above is a price, not a dislike of the graveyard.
#[test]
fn a_free_land_from_the_graveyard_is_played() {
    let mut v = main_phase(5, vec![]);
    v.graveyards[0].push(buried(2, "Petrified Field"));

    assert_eq!(
        agent().act(&v, &lands_offered(vec![obj(2)])),
        PlayerAction::PlayLand { card: obj(2) }
    );
}

/// An opponent's Ankh of Mishra deals 2 to whoever a land enters under, so a
/// land from the hand is played only above the payment margin: not at seven
/// (2 + 5), at eight.
#[test]
fn an_opponents_ankh_prices_every_land() {
    let ankh = carded(
        permanent(obj(1), PlayerId::new(1), 0),
        "Ankh of Mishra",
        TypeSet::ARTIFACT,
    );
    let mut v = main_phase(7, vec![ankh]);
    v.hand.push(hand_card(3, "Forest"));
    assert_eq!(
        agent().act(&v, &lands_offered(vec![obj(3)])),
        PlayerAction::PassPriority,
        "a land the Ankh makes cost two of seven was played"
    );

    v.seats[0].life = 8;
    assert_eq!(
        agent().act(&v, &lands_offered(vec![obj(3)])),
        PlayerAction::PlayLand { card: obj(3) }
    );
}
