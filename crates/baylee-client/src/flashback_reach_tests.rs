//! A card in the seat's own graveyard that it may flash back is a card this
//! client taps lands for, priced at the flashback cost (#242).
//!
//! The engine offers such a card in `castable` only once its cost is
//! floating, so without this the Opt that Snapcaster Mage made castable was a
//! card nothing could click on: it ended the turn in the graveyard beside the
//! Island that could have paid for it. The view says which cards and at what
//! price (`PublicObject::flashback`); these hold what the client does with it.

use super::*;
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_engine::choice::{GRANTED_ABILITY, LegalActions};
use baylee_view::PublicObject;

/// Where the graveyard card lies in every fixture below.
pub(crate) const BURIED: ObjectId = ObjectId::new(60, 0);

/// `n` lands that each make one mana of any colour.
///
/// Any colour for `commander_reach_tests`' reason: what is under test is
/// whether the graveyard is *looked at* and at which price, and a test that
/// also had to get the colours right would fail for two reasons and say one.
fn lands(n: usize) -> Vec<PublicObject> {
    (0..n)
        .map(|slot| {
            let mut land = token(100 + slot as u32, 0, "Wastes", 0, 0);
            land.types = baylee_core::types::TypeSet::LAND;
            land.power = None;
            land.toughness = None;
            land.granted_mana = Some(baylee_view::GrantedMana {
                slot: 0,
                colors: vec![
                    ManaColor::White,
                    ManaColor::Blue,
                    ManaColor::Black,
                    ManaColor::Red,
                    ManaColor::Green,
                ],
                amount: 1,
            });
            land
        })
        .collect()
}

/// `name` out of the registry, lying at [`BURIED`] in `seat`'s graveyard with
/// the flashback the view gives it.
///
/// The types come off the printed face because `reachable` asks the timing
/// question of the *projected* types, and `printed` leaves every object a
/// 2/2 creature.
pub(crate) fn buried(seat: u8, name: &str, flashback: Option<&str>) -> PublicObject {
    let mut card = crate::registry_printed(BURIED.slot(), seat, name);
    let def = card
        .card
        .and_then(|c| baylee_cards::by_index(c.index))
        .expect("the card is in the registry");
    card.types = def.faces[0].types;
    card.power = None;
    card.toughness = None;
    card.flashback = flashback.map(ManaCost::parse);
    card
}

/// Seat 0 at priority in its own main phase over `lands` untapped lands, with
/// `mine` in its graveyard and `theirs` in seat 1's, the engine offering the
/// lands' mana and `castable` as castable.
///
/// Filled through `receive_view` and `receive_choice`, as a frame is, and then
/// `rebuild_board`, which is what writes `Duel::reachable` — the set the
/// click path reads.
pub(crate) fn table_with(
    lands_up: usize,
    mine: Vec<PublicObject>,
    theirs: Vec<PublicObject>,
    castable: Vec<ObjectId>,
) -> Duel {
    let lands = lands(lands_up);
    let ids: Vec<_> = lands.iter().map(|o| o.id).collect();
    let view = ViewBuilder::new(2)
        .with_battlefield(0, lands)
        .with_graveyard(0, mine)
        .with_graveyard(1, theirs)
        .build();
    let legal = LegalActions {
        can_pass: true,
        castable,
        abilities: ids.iter().map(|id| (*id, GRANTED_ABILITY)).collect(),
        mana_abilities: ids,
        ..LegalActions::default()
    };
    let mut duel = Duel::default();
    duel.receive_view(view);
    duel.receive_choice(Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(legal),
    });
    crate::rebuild_board(&mut duel);
    duel
}

/// [`table_with`] with nothing in seat 1's graveyard and nothing castable.
pub(crate) fn table(lands_up: usize, mine: Vec<PublicObject>) -> Duel {
    table_with(lands_up, mine, Vec::new(), Vec::new())
}

/// The reported board: Opt in the graveyard with Snapcaster's `{U}` on it,
/// and one land that makes blue.
#[test]
fn a_card_it_may_flash_back_is_reached_for() {
    let duel = table(1, vec![buried(0, "Opt", Some("{U}"))]);
    assert!(
        duel.reachable.contains(&BURIED),
        "one land pays {{U}} and Opt was not offered: {:?}",
        duel.reachable
    );
    assert_eq!(
        mana_for(&duel, BURIED).map(|plan| plan.steps.len()),
        Some(1),
        "and the click has a run of one tap to start"
    );
    assert_eq!(duel.reach_of(BURIED), Some(Reach::Taps));
}

/// The counter-test: the same card with no flashback is only a card in a
/// graveyard.
#[test]
fn a_card_it_may_not_flash_back_is_not() {
    let duel = table(1, vec![buried(0, "Opt", None)]);
    assert!(duel.reachable.is_empty(), "{:?}", duel.reachable);
    assert!(mana_for(&duel, BURIED).is_none());
    assert_eq!(duel.reach_of(BURIED), None);
}

/// Priced at the flashback cost and never at the card's own.
///
/// Opt costs `{U}`. A flashback of `{2}{U}` is Think Twice's shape — a card
/// whose two costs differ — and one land pays the first and not the second.
/// Both halves are asserted, because a client pricing at the mana cost
/// passes the three-land half.
#[test]
fn it_is_priced_at_the_flashback_cost_and_not_its_own() {
    let short = table(1, vec![buried(0, "Opt", Some("{2}{U}"))]);
    assert!(
        !short.reachable.contains(&BURIED),
        "one land pays Opt's own {{U}}, not {{2}}{{U}}"
    );
    assert!(mana_for(&short, BURIED).is_none());

    let paid = table(3, vec![buried(0, "Opt", Some("{2}{U}"))]);
    assert!(paid.reachable.contains(&BURIED));
    assert_eq!(
        mana_for(&paid, BURIED).map(|plan| plan.steps.len()),
        Some(3),
        "the run taps for the flashback cost, all three"
    );
}

/// Only the seat's own graveyard is read.
///
/// The view never says `flashback` of another seat's card — this is the
/// client not relying on that, because the one graveyard it may cast from
/// is its own.
#[test]
fn only_its_own_graveyard_is_read() {
    let duel = table_with(
        1,
        Vec::new(),
        vec![buried(1, "Opt", Some("{U}"))],
        Vec::new(),
    );
    assert!(duel.reachable.is_empty(), "{:?}", duel.reachable);
    assert!(mana_for(&duel, BURIED).is_none());
}

/// A sorcery is flashed back when a sorcery could be cast (CR 702.34a casts
/// it; CR 307.1 times it), and not otherwise.
///
/// Faithless Looting prints flashback `{2}{R}`; the price is set by hand
/// because the view carries granted flashback only, so far.
#[test]
fn a_sorcery_is_flashed_back_only_when_a_sorcery_could_be_cast() {
    let own_turn = table(3, vec![buried(0, "Faithless Looting", Some("{2}{R}"))]);
    assert!(
        own_turn.reachable.contains(&BURIED),
        "its own main phase, with an empty stack"
    );

    let mut theirs = table(3, vec![buried(0, "Faithless Looting", Some("{2}{R}"))]);
    theirs.view.as_mut().expect("the view").active = PlayerId::new(1);
    assert!(
        !reachable(&theirs).contains(&BURIED),
        "not on somebody else's turn"
    );
}

/// What the engine already offers is the engine's, and is not offered again
/// as a run: the mana is up, and a run would float a second lot.
#[test]
fn what_the_engine_already_offers_is_its_own() {
    let duel = table_with(
        1,
        vec![buried(0, "Opt", Some("{U}"))],
        Vec::new(),
        vec![BURIED],
    );
    assert!(!duel.reachable.contains(&BURIED));
    assert_eq!(duel.reach_of(BURIED), Some(Reach::Offered));
}

/// A tap on it arms the run, as a tap on a hand card does, and does not
/// open the pile it lies on.
///
/// Before #242 the click fell through every branch of `activate_card` to
/// the last, which opens the pile — a player tapping Opt got a panel about
/// their graveyard. The second tap is what sends: it starts the run.
#[test]
fn a_tap_on_it_arms_the_run_and_the_second_starts_it() {
    let mut duel = table(1, vec![buried(0, "Opt", Some("{U}"))]);
    crate::input::activate_card(&mut duel, BURIED);
    assert!(!duel.browser.is_open(), "the tap opened the graveyard");
    assert!(
        matches!(
            duel.armed.as_ref(),
            Some(Armed { object, deed: Deed::Run { then: RunEnd::Cast, .. } }) if *object == BURIED
        ),
        "armed {:?}",
        duel.armed
    );

    crate::input::activate_card(&mut duel, BURIED);
    assert!(duel.mana_run.is_some(), "the second tap started no run");
}
