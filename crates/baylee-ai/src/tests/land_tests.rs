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

/// Simic Growth Chamber with the handle `slot`, on the battlefield, tapped
/// as it enters.
fn karoo_on_board(slot: u32, tapped: bool) -> PublicObject {
    let mut land = carded(
        permanent(obj(slot), PlayerId::new(0), 0),
        "Simic Growth Chamber",
        TypeSet::LAND,
    );
    land.power = None;
    land.toughness = None;
    if tapped {
        land.status = ObjectStatus::TAPPED;
    }
    land
}

/// A basic land of the seat's on the battlefield.
fn basic_on_board(slot: u32, name: &str, tapped: bool) -> PublicObject {
    let mut land = carded(
        permanent(obj(slot), PlayerId::new(0), 0),
        name,
        TypeSet::LAND,
    );
    land.power = None;
    land.toughness = None;
    land.supertypes = SupertypeSet::BASIC;
    if tapped {
        land.status = ObjectStatus::TAPPED;
    }
    land
}

/// The report of 3.11.2026: a Karoo played with no other land out returns
/// itself, and the land drop is gone. A basic beside it in the hand is the
/// better land; alone, the Karoo is not played at all.
#[test]
fn a_bounce_land_is_not_played_with_no_land_to_return() {
    let mut v = main_phase(20, vec![]);
    v.hand.push(hand_card(2, "Simic Growth Chamber"));
    v.hand.push(hand_card(3, "Forest"));
    assert_eq!(
        agent().act(&v, &lands_offered(vec![obj(2), obj(3)])),
        PlayerAction::PlayLand { card: obj(3) },
        "the Karoo was played ahead of a basic with nothing to return"
    );

    v.hand.remove(1);
    assert_eq!(
        agent().act(&v, &lands_offered(vec![obj(2)])),
        PlayerAction::PassPriority,
        "a Karoo that can only return itself was played"
    );
}

/// With a land out the Karoo is worth playing, but after a plain land in the
/// hand and before nothing.
#[test]
fn a_bounce_land_is_played_after_plain_lands_and_when_it_is_the_only_one() {
    let mut v = main_phase(20, vec![basic_on_board(1, "Island", false)]);
    v.hand.push(hand_card(2, "Simic Growth Chamber"));
    v.hand.push(hand_card(3, "Forest"));
    assert_eq!(
        agent().act(&v, &lands_offered(vec![obj(2), obj(3)])),
        PlayerAction::PlayLand { card: obj(3) }
    );
    assert_eq!(
        agent().act(&v, &lands_offered(vec![obj(2)])),
        PlayerAction::PlayLand { card: obj(2) }
    );
}

/// The Karoo's trigger asks which land to return. Offered first, the Karoo
/// itself was returned, and the board was the same as before it was played.
#[test]
fn a_bounce_land_returns_a_tapped_basic_and_not_itself() {
    use baylee_engine::choice::ChoicePrompt;
    let mut trigger = permanent(obj(9), PlayerId::new(0), 0);
    trigger.stack_item = Some(baylee_view::StackItem::Ability {
        source: obj(1),
        ability: None,
        text: None,
        token: None,
        rules: Some(baylee_view::RulesFace::from(
            karoo_on_board(1, true).card.unwrap(),
        )),
    });
    let mut v = main_phase(
        20,
        vec![
            karoo_on_board(1, true),
            basic_on_board(2, "Island", false),
            basic_on_board(3, "Forest", true),
        ],
    );
    v.stack.push(trigger);
    let ask = |v: &PlayerView, options: Vec<ObjectId>| {
        agent().act(
            v,
            &Pending::ChooseCards {
                player: v.seat,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            },
        )
    };
    assert_eq!(
        ask(&v, vec![obj(1), obj(2), obj(3)]),
        PlayerAction::ChooseObjects {
            objects: vec![obj(3)]
        },
        "the tapped basic was not the land returned"
    );
    assert_eq!(
        ask(&v, vec![obj(1), obj(2)]),
        PlayerAction::ChooseObjects {
            objects: vec![obj(2)]
        },
        "the Karoo was returned while another land was there"
    );
    assert_eq!(
        ask(&v, vec![obj(1)]),
        PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        }
    );
}
