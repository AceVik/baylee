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
        "Du kannst Mana erzeugen, um bis zu 2 Schaden zu verhindern. Passe, um den Betrag zu wählen."
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
        matches!(duel.proposing(), Proposing::Owed { .. }),
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

/// "Pay now" in a payment window stages what is still owed and sends
/// nothing; the next press is Go, and Go is one `PayWindow` carrying the
/// staged taps — the engine hears the whole payment at once.
#[test]
fn paying_stages_first_and_then_sends_one_pay_window() {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    assert!(duel.paying(), "a window owing {{1}} is a payment window");
    duel.confirm_payment();
    assert!(duel.take_outbox().is_empty(), "Pay now sends nothing");
    assert_eq!(duel.pay_staged.steps().len(), 1, "one Forest is staged");
    assert!(duel.staged_covers(), "and it covers {{1}}");
    let forest = duel.pay_staged.steps()[0].source;

    duel.confirm_payment();
    assert_eq!(
        duel.take_outbox(),
        [PlayerAction::PayWindow {
            steps: vec![PlayerAction::ActivateManaAbility { source: forest }],
        }],
        "Go sends the staged tap and the settlement as one action"
    );
    assert!(
        duel.mana_run.is_none(),
        "and no run taps anything one by one"
    );
}

/// Mana the player floated counts: with the pool already paying, nothing is
/// staged and Go is a `PayWindow` with no steps, which pays from the pool.
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
    assert!(duel.staged_covers());
    duel.confirm_payment();
    assert_eq!(
        duel.take_outbox(),
        [PlayerAction::PayWindow { steps: vec![] }]
    );
}

/// Paying is only for the window: a quiet window over the same lands stages
/// nothing, and its confirm is the ordinary pass, not this one's.
#[test]
fn outside_a_payment_window_paying_does_nothing() {
    use baylee_client_core::manaplan::{Step, Tap};
    let mut duel = seat_with_two_forests(None);
    assert!(!duel.paying());
    duel.confirm_payment();
    assert!(!duel.stage(Step {
        source: ObjectId::new(1, 0),
        tap: Tap::Intrinsic,
        color: None,
    }));
    assert!(duel.pay_staged.is_empty());
    assert!(duel.mana_run.is_none() && duel.take_outbox().is_empty());
}

/// Report 01: `confirm_payment` with no plan and a short pool sends nothing
/// and says why; the pass it used to be is the "Don't pay" button's.
#[test]
fn confirming_a_short_payment_sends_nothing_and_says_the_mana_is_short() {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{G}{G}{G}")));
    assert!(duel.paying());
    assert!(
        duel.owed_plan.is_none(),
        "two Forests cannot pay three green"
    );
    duel.confirm_payment();
    assert!(duel.take_outbox().is_empty());
    assert!(duel.mana_run.is_none());
    assert_eq!(
        duel.last_error,
        Some(baylee_client_core::i18n::Refusal::Said(
            baylee_client_core::i18n::Phrase::PaymentShort
        ))
    );
}

/// A voluntary amount is not this kind of window: it is settled by passing,
/// whatever floats, and nothing is staged for it.
#[test]
fn a_voluntary_amount_is_not_a_staged_payment() {
    let mut duel = seat_with_two_forests(None);
    let mut view = duel.view.clone().expect("the view");
    view.owed = Some(baylee_core::mana::ManaPayment::AnyAmount {
        preventable_damage: 2,
    });
    duel.receive_view(view);
    assert!(!duel.paying());
    assert!(duel.payment_remaining().is_none());
}

/// The two Forests of [`seat_with_two_forests`], in the order the table
/// holds them.
fn two_forests(duel: &Duel) -> (ObjectId, ObjectId) {
    let ids = lands(duel);
    assert_eq!(ids.len(), 2, "the fixture's two Forests");
    (ids[0], ids[1])
}

/// A tap of a Forest the way the planner names one.
fn forest_tap(source: ObjectId) -> baylee_client_core::manaplan::Step {
    baylee_client_core::manaplan::Step {
        source,
        tap: baylee_client_core::manaplan::Tap::Intrinsic,
        color: None,
    }
}

/// The window's priority question over `mana` as the abilities on offer.
fn priority_over(mana: Vec<ObjectId>) -> Pending {
    Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: mana,
            ..LegalActions::default()
        }),
    }
}

/// The seat's view with `owed` set to `cost` (or cleared), delivered through
/// the real edge.
fn owe(duel: &mut Duel, cost: Option<&str>) {
    let mut view = duel.view.clone().expect("the view");
    view.owed = cost.map(|c| baylee_core::mana::ManaPayment::Fixed(ManaCost::parse(c)));
    duel.receive_view(view);
}

/// Clicking an untapped land in the seat's own window stages it and sends
/// nothing; the board draws it as a staged proposal; a second click takes
/// it back, again without sending anything. The old click tapped for real
/// the moment it landed, so a land tapped by mistake stayed tapped.
#[test]
fn a_click_in_a_window_stages_the_land_and_a_second_click_takes_it_back() {
    use baylee_client_core::board::Proposal;
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    let (first, second) = two_forests(&duel);

    crate::input::activate_card(&mut duel, first);
    assert!(duel.pay_staged.holds(first), "the click staged the Forest");
    assert!(!duel.pay_staged.holds(second));
    assert!(duel.take_outbox().is_empty(), "staging sends nothing");
    assert!(duel.mana_run.is_none(), "and starts no run");
    let proposed = crate::proposals(&duel);
    assert_eq!(proposed.get(&first), Some(&Proposal::Staged));
    assert_ne!(proposed.get(&second), Some(&Proposal::Staged));

    crate::input::activate_card(&mut duel, first);
    assert!(duel.pay_staged.is_empty(), "the second click un-staged it");
    assert!(duel.take_outbox().is_empty(), "and sent nothing either");
    assert_ne!(
        crate::proposals(&duel).get(&first),
        Some(&Proposal::Staged),
        "no longer drawn as staged"
    );
}

/// A staged land is drawn tapped (the preview of the payment) while the
/// land beside it is not, and the view the engine sent never changed: the
/// tap is only on the board model.
#[test]
fn a_staged_land_is_drawn_tapped_and_the_other_is_not() {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    let (first, second) = two_forests(&duel);
    crate::rebuild_board(&mut duel);
    let board = duel.board.as_ref().expect("the board");
    assert!(
        !board
            .group(first)
            .expect("the Forest's group")
            .status
            .is_tapped(),
        "untapped before anything is staged"
    );

    assert!(duel.stage(forest_tap(first)));
    crate::rebuild_board(&mut duel);
    let board = duel.board.as_ref().expect("the board");
    assert!(
        board
            .group(first)
            .expect("the staged Forest's group")
            .status
            .is_tapped(),
        "the staged Forest is drawn tapped"
    );
    assert!(
        !board
            .group(second)
            .expect("the other Forest's group")
            .status
            .is_tapped(),
        "the other Forest is not"
    );
    assert!(
        duel.take_outbox().is_empty(),
        "and the preview sent nothing"
    );
    assert!(
        duel.view
            .as_ref()
            .expect("the view")
            .battlefield
            .iter()
            .all(|o| !o.status.is_tapped()),
        "the engine's own view is untouched"
    );
}

/// A payment the engine refuses comes back as the same view and the same
/// question; the staging must survive it so the player corrects it rather
/// than building it again.
#[test]
fn a_refused_payment_keeps_the_staging() {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    let (first, _) = two_forests(&duel);
    assert!(duel.stage(forest_tap(first)));
    let view = duel.view.clone().expect("the view");
    let pending = duel
        .interaction
        .as_ref()
        .expect("the question")
        .pending()
        .clone();

    duel.confirm_payment();
    assert_eq!(
        duel.take_outbox(),
        [PlayerAction::PayWindow {
            steps: vec![PlayerAction::ActivateManaAbility { source: first }],
        }]
    );

    // The host's re-ask after a refusal: the same view, the same question.
    duel.receive_view(view);
    duel.receive_choice(pending);
    assert!(duel.paying(), "the window is still open");
    assert!(duel.pay_staged.holds(first), "and the Forest still staged");
    assert_eq!(duel.pay_staged.steps().len(), 1);
    assert!(duel.staged_covers());
}

/// The staging belongs to one window: it goes when the window closes, and
/// when a window owing something else opens in its place.
#[test]
fn the_staging_does_not_outlive_its_window() {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{1}")));
    let (first, _) = two_forests(&duel);
    assert!(duel.stage(forest_tap(first)));

    owe(&mut duel, None);
    assert!(!duel.paying());
    assert!(duel.pay_staged.is_empty(), "the window closed");

    owe(&mut duel, Some("{1}"));
    assert!(duel.stage(forest_tap(first)));
    owe(&mut duel, Some("{2}"));
    assert!(duel.paying(), "a window owing {{2}} is open");
    assert!(
        duel.pay_staged.is_empty(),
        "a window owing something else starts with nothing staged"
    );

    // And the same cost delivered again is the same window.
    assert!(duel.stage(forest_tap(first)));
    owe(&mut duel, Some("{2}"));
    assert!(duel.pay_staged.holds(first), "the same window keeps it");
}

/// A staged land the engine stops offering (it left, or something else
/// tapped it) is dropped, and only that one.
#[test]
fn a_land_the_engine_no_longer_offers_leaves_the_staging() {
    let mut duel = seat_with_two_forests(Some(ManaCost::parse("{2}")));
    let (first, second) = two_forests(&duel);
    assert!(duel.stage(forest_tap(first)));
    assert!(duel.stage(forest_tap(second)));
    assert_eq!(duel.pay_staged.steps().len(), 2);

    duel.receive_choice(priority_over(vec![second]));
    assert!(
        !duel.pay_staged.holds(first),
        "Forest 1 is no longer offered"
    );
    assert!(duel.pay_staged.holds(second), "Forest 2 still is");
    assert_eq!(duel.pay_staged.steps().len(), 1);

    duel.receive_choice(priority_over(vec![]));
    assert!(duel.pay_staged.is_empty(), "and then nothing is offered");
}

/// Declining a payment names what it costs, by what the seat said yes to
/// just before the window: a pact's debt loses the game, a taken-back cast
/// is taken back, a tax names nothing the client can know.
#[test]
fn declining_names_its_price_by_what_opened_the_window() {
    use baylee_client_core::i18n::Phrase;
    use baylee_engine::choice::YesNoPrompt;

    let yes_to = |prompt: YesNoPrompt| {
        let mut duel = seat_with_two_forests(None);
        let ids: Vec<ObjectId> = lands(&duel);
        duel.receive_choice(Pending::YesNo {
            player: PlayerId::new(0),
            prompt,
            source: None,
        });
        duel.submit(PlayerAction::YesNo(true));
        assert_eq!(duel.take_outbox(), [PlayerAction::YesNo(true)]);
        // The window the yes opens.
        owe(&mut duel, Some("{1}"));
        duel.receive_choice(priority_over(ids));
        duel
    };

    let pact = ManaCost::parse("{1}");
    let duel = yes_to(YesNoPrompt::PayPact { cost: pact });
    assert!(duel.paying());
    assert_eq!(duel.declining_costs(), Some(Phrase::DeclinePact));

    // A cast named by the view comes first: declining takes it back.
    let mut cast = yes_to(YesNoPrompt::PayPact { cost: pact });
    let mut view = cast.view.clone().expect("the view");
    view.casting = Some(ObjectId::new(90, 0));
    cast.receive_view(view);
    assert_eq!(cast.declining_costs(), Some(Phrase::DeclinePaymentCast));

    let tax = yes_to(YesNoPrompt::PayTax { mana: 1 });
    assert!(tax.paying());
    assert_eq!(
        tax.declining_costs(),
        None,
        "a tax says nothing of its price"
    );

    // Outside a window there is nothing to decline.
    let quiet = seat_with_two_forests(None);
    assert_eq!(quiet.declining_costs(), None);
}

/// What a pact's yes remembered goes with its window, and a "no" never
/// remembers anything: a later, unrelated window must not claim that
/// declining it loses the game.
#[test]
fn the_pact_memory_goes_with_its_window_and_a_no_never_makes_one() {
    use baylee_engine::choice::YesNoPrompt;
    let mut duel = seat_with_two_forests(None);
    let ids: Vec<ObjectId> = lands(&duel);
    let pact = YesNoPrompt::PayPact {
        cost: ManaCost::parse("{1}"),
    };

    duel.receive_choice(Pending::YesNo {
        player: PlayerId::new(0),
        prompt: pact,
        source: None,
    });
    duel.submit(PlayerAction::YesNo(false));
    duel.take_outbox();
    owe(&mut duel, Some("{1}"));
    duel.receive_choice(priority_over(ids.clone()));
    assert_eq!(duel.declining_costs(), None, "a no opened no pact window");

    // A yes, its window, the window closing: the memory goes with it.
    duel.receive_choice(Pending::YesNo {
        player: PlayerId::new(0),
        prompt: pact,
        source: None,
    });
    duel.submit(PlayerAction::YesNo(true));
    duel.take_outbox();
    owe(&mut duel, Some("{1}"));
    duel.receive_choice(priority_over(ids.clone()));
    assert!(duel.declining_costs().is_some());
    owe(&mut duel, None);
    owe(&mut duel, Some("{1}"));
    duel.receive_choice(priority_over(ids));
    assert_eq!(
        duel.declining_costs(),
        None,
        "the next window is not the pact's"
    );
}
