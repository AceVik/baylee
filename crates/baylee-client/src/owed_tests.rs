//! A payment window is told apart from every quiet window in the game, and only by the field the engine added for it. `PlayerView::owed` exists because a CR 605.3a window is an ordinary `Pending::Priority` offering mana abilities and nothing else - so it is shaped exactly like a priority pass over untapped lands with nothing castable, which is most windows most turns. Reading "I owe something" off that shape would light lands in all of them; these tests are what says it does not.

use super::*;
use baylee_client_core::test_support::ViewBuilder;
use baylee_core::generated::subtypes::land;
use baylee_core::mana::ManaCost;
use baylee_core::types::TypeSet;
use baylee_engine::choice::LegalActions;

/// A Forest on the table, untapped, that the engine is offering.
///
/// A card, not a bare object, as a player's Forest is. Two of them pile on
/// any row (#263), so the table's tests read a pile of two.
fn forest(slot: u32) -> baylee_view::PublicObject {
    let mut obj = baylee_client_core::test_support::token(slot, 0, "Forest", 0, 0);
    obj.card = Some(baylee_view::CardIdentity {
        index: baylee_core::generated::index::FOREST,
        print: baylee_core::ids::PrintRef::new(0),
        face: 0,
    });
    obj.rules = obj.card.map(baylee_view::RulesFace::from);
    obj.types = TypeSet::LAND;
    obj.power = None;
    obj.toughness = None;
    obj.subtypes.insert(land::FOREST);
    obj
}

#[test]
fn optional_payment_offers_mana_without_arming_a_fixed_debt_plan() {
    use baylee_client_core::i18n::Lang;
    use baylee_core::mana::ManaPayment;
    let mut duel = seat_with_two_forests(None);
    let mut view = duel.view.clone().unwrap();
    view.owed = Some(ManaPayment::AnyAmount {
        preventable_damage: 2,
    });
    duel.receive_view(view);
    assert!(duel.owed_plan.is_none());
    let legal = duel.interaction.as_ref().unwrap().legal_actions().unwrap();
    assert_eq!(legal.mana_abilities.len(), 2);
    let texts = crate::cardtext::CardTexts::default();
    assert_eq!(
        duel.headline(Lang::En, &texts).unwrap(),
        "You may generate mana to prevent up to 2 damage. Pass to choose the amount."
    );
    assert_eq!(
        duel.headline(Lang::De, &texts).unwrap(),
        "Erzeuge bei Bedarf Mana gegen bis zu 2 Schaden. Passe zur Wahl des Betrags."
    );
}

/// A seat holding priority with two Forests it may tap, fed through the two
/// real edges rather than built as a struct literal.
///
/// That is not tidiness. `Duel { .. ..Default::default() }` leaves
/// `owed_plan` at `None` and every land unlit, so a test that assembled one
/// by hand would pass whatever the feature did — including nothing at all.
/// These go in through [`Duel::receive_view`] and [`Duel::receive_choice`],
/// which is where the plan is worked out.
///
/// `pub(crate)` for `table::offer_tests`, which asks the same question one
/// surface further out — whether the table *draws* what this works out — and
/// must not answer it with a second, differently-wrong harness.
pub(crate) fn seat_with_two_forests(owed: Option<ManaCost>) -> Duel {
    let lands: Vec<_> = (1..=2).map(forest).collect();
    let ids: Vec<ObjectId> = lands.iter().map(|o| o.id).collect();
    let mut view = ViewBuilder::new(2).with_battlefield(0, lands).build();
    view.awaiting = Some(view.seat);
    view.owed = owed.map(baylee_core::mana::ManaPayment::Fixed);
    let legal = LegalActions {
        can_pass: true,
        mana_abilities: ids,
        ..LegalActions::default()
    };
    let mut duel = Duel::default();
    duel.receive_view(view);
    duel.receive_choice(Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(legal),
    });
    duel
}

/// Every land this seat has, for asking whether any of them lit.
fn lands(duel: &Duel) -> Vec<ObjectId> {
    duel.view
        .as_ref()
        .expect("the view")
        .battlefield
        .iter()
        .filter(|o| o.types.contains(TypeSet::LAND))
        .map(|o| o.id)
        .collect()
}

/// **The test this whole field exists for.** A priority window offering mana
/// abilities and nothing else, with nothing owed, lights no land.
///
/// This is not an edge case, it is the ordinary state of the game: every
/// quiet window in every turn is a `Pending::Priority` over untapped lands,
/// and it is the exact shape of a payment window. If this is ever green for
/// the wrong reason the feature is a permanent glow on every land a player
/// owns, all game.
///
/// It is also the house-AI bug written from the client's side. The agent was
/// handed a window like this, found nothing castable, passed, and its own
/// spell was countered — because nothing in the window said a payment was
/// owed. The answer was to add the field, not to guess from the shape, and
/// this is the assertion that the guessing did not come back.
#[test]
fn a_quiet_priority_window_over_untapped_lands_lights_nothing() {
    let duel = seat_with_two_forests(None);
    assert!(
        matches!(duel.proposing(), Proposing::Nothing),
        "a window with nothing owed is proposing something"
    );
    for land in lands(&duel) {
        let offer = cardmat::Offer::on(duel.proposing(), &[land], false);
        assert!(
            !offer.will_tap,
            "{land:?} is marked to be tapped for a payment nobody asked for"
        );
        assert!(!offer.armed, "and nothing was armed");
    }
}

/// And a window that *does* owe something lights the lands that would pay it.
///
/// The counter-test of the one above, and the one that says it is green for
/// the right reason: the same board, the same pending, the same mana
/// abilities, one field different.
#[test]
fn an_open_payment_window_lights_the_lands_that_would_settle_it() {
    let duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    assert!(
        matches!(duel.proposing(), Proposing::Owed(_)),
        "a window owing {{1}} is proposing nothing"
    );
    let lit = lands(&duel)
        .into_iter()
        .filter(|land| cardmat::Offer::on(duel.proposing(), &[*land], false).will_tap)
        .count();
    assert_eq!(
        lit, 1,
        "one Forest pays {{1}}, and exactly one should be lit"
    );
}

/// Nothing is *armed* by a payment window, however much is owed.
///
/// The arming contract is for deeds that cannot be taken back, and it exempts
/// mana abilities by name — one tap, because floating mana is the cheap
/// mistake. A payment window is made of nothing but mana abilities, so
/// arming here would be the contract growing an exception for the one case it
/// was written to exclude. The glow is a suggestion; the tap is still the
/// player's, and it is still one tap.
#[test]
fn a_payment_window_arms_nothing() {
    let duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    assert!(duel.armed.is_none(), "a window armed a deed by itself");
    for land in lands(&duel) {
        assert!(
            !cardmat::Offer::on(duel.proposing(), &[land], false).armed,
            "{land:?} is drawn as armed"
        );
    }
}

/// The plan is worked out on **both** edges, because either may arrive last.
///
/// A view carries `owed` and the pool; a pending carries the mana abilities
/// that could pay it. Neither alone is enough to plan with, and the engine
/// sends them in whichever order it likes. A refresh on one edge only fails
/// silently — the lands simply never light — which is why this asserts the
/// order rather than trusting it.
#[test]
fn the_plan_survives_either_edge_arriving_last() {
    let forward = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    assert!(forward.owed_plan.is_some(), "view then pending");

    // The other way round: the pending first, against a view that has not
    // arrived, and then the view.
    let lands_: Vec<_> = (1..=2).map(forest).collect();
    let ids: Vec<ObjectId> = lands_.iter().map(|o| o.id).collect();
    let mut view = ViewBuilder::new(2).with_battlefield(0, lands_).build();
    view.awaiting = Some(view.seat);
    view.owed = Some(baylee_core::mana::ManaPayment::Fixed(ManaCost::parse(
        "{1}",
    )));
    let mut duel = Duel::default();
    duel.receive_choice(Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: ids,
            ..LegalActions::default()
        }),
    });
    assert!(
        duel.owed_plan.is_none(),
        "a pending with no view yet cannot have planned anything"
    );
    duel.receive_view(view);
    assert!(duel.owed_plan.is_some(), "pending then view");
}

/// "Pay" in a payment window taps what is still owed and only then settles
/// it: the engine hears the tap first and the pass after the mana is up, so
/// a miracle is cast with its cost floating and not passed away short.
#[test]
fn paying_taps_what_is_owed_and_then_settles_the_window() {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    assert!(duel.paying(), "a window owing {{1}} is a payment window");
    assert!(duel.pay_owed(), "a plan stood, so paying starts a run");
    let first = duel.take_outbox();
    assert!(
        matches!(first.as_slice(), [PlayerAction::ActivateManaAbility { .. }]),
        "the tap goes first, alone: {first:?}"
    );

    // The engine answers: the Forest's mana floats, the window is still open.
    let mut view = duel.view.clone().expect("the view");
    view.seats[0].mana_pool.green = 1;
    duel.receive_view(view);
    duel.receive_choice(Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(LegalActions {
            can_pass: true,
            ..LegalActions::default()
        }),
    });
    crate::advance_mana_run(&mut duel);
    assert_eq!(
        duel.take_outbox(),
        [PlayerAction::PassPriority],
        "with the mana up, the window is settled"
    );
    assert!(duel.mana_run.is_none(), "and the run is over");
}

/// Mana the player tapped by hand counts: with the pool already paying,
/// there is nothing left to tap, so paying is the plain pass, and nothing
/// the player floated is tapped for twice.
#[test]
fn a_window_the_pool_already_pays_is_settled_without_a_tap() {
    let mut duel = seat_with_two_forests(None);
    let mut view = duel.view.clone().expect("the view");
    view.owed = Some(baylee_core::mana::ManaPayment::Fixed(ManaCost::parse(
        "{1}",
    )));
    view.seats[0].mana_pool.green = 1;
    duel.receive_view(view);
    assert!(duel.paying());
    assert!(
        duel.owed_plan
            .as_ref()
            .is_some_and(baylee_client_core::manaplan::Plan::is_empty),
        "the plan is for the remainder, and nothing remains"
    );
    assert!(!duel.pay_owed(), "nothing to tap: the confirm passes");
    assert!(duel.take_outbox().is_empty());
}

/// Paying is only for the window: a quiet window over the same lands starts
/// no run, and its confirm is the ordinary pass.
#[test]
fn outside_a_payment_window_paying_does_nothing() {
    let mut duel = seat_with_two_forests(None);
    assert!(!duel.paying());
    assert!(!duel.pay_owed());
    assert!(duel.mana_run.is_none() && duel.take_outbox().is_empty());
}
