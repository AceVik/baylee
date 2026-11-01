use super::*;
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::PrintRef;
use baylee_core::types::TypeSet;
use baylee_engine::choice::LegalActions;
use baylee_view::{CardIdentity, HandObject};

/// The first card in the registry that is only a sorcery and costs at
/// most one green mana plus generic.
///
/// Found rather than named, because a test that hard-codes a card breaks
/// when the pool is re-cut and says nothing about what it was testing.
///
/// It has to ask for a *printed* cost out loud. "Cheap" and "mono-green"
/// are both satisfied trivially by a card that prints no mana cost at all
/// — cmc zero, no colours — and Ancestral Vision is a single-faced
/// sorcery at index 4, so this picked it first and every reach test below
/// was built on a card CR 202.1b says can never be cast for mana.
fn a_cheap_sorcery() -> &'static baylee_cards_dsl::CardDef {
    const GREEN: ColorSet = ColorSet::of(Color::Green);
    baylee_cards::all()
        .filter(|def| def.faces.len() == 1)
        .find(|def| {
            let face = &def.faces[0];
            face.types == TypeSet::SORCERY
                && !def
                    .keywords_for_face(0)
                    .contains(baylee_cards_dsl::KeywordSet::FLASH)
                && face.mana_cost.symbols().next().is_some()
                && face.mana_cost.cmc() <= 2
                && face.mana_cost.colors().difference(GREEN) == ColorSet::EMPTY
        })
        .expect("the pool has a cheap mono-green sorcery")
}

/// A Forest on the table, an untapped source the engine is offering.
fn forest(slot: u32) -> baylee_view::PublicObject {
    let mut obj = token(slot, 0, "Forest", 0, 0);
    obj.types = TypeSet::LAND;
    obj.power = None;
    obj.toughness = None;
    obj.subtypes.insert(land::FOREST);
    obj
}

/// A seat holding one sorcery, with two Forests it may tap.
///
/// Built as a struct literal, and #105 named this file for it. It is the
/// case the ticket's third step is about: every claim below is about
/// [`reachable`] the **function**, which reads `view` and `interaction` and
/// nothing else, and the literal sets both. The cached `Duel::reachable` that
/// `rebuild_board` fills is never read here — the tests call the function
/// rather than the field, on purpose, because what is under test is the
/// answer and not the caching.
///
/// The rule the audit came out with, so this does not get re-opened: a
/// literal is only a trap for a field the wire fills **whose empty value is
/// also a legal value**. `owed_plan: None` and `reachable: {}` are answers a
/// working client gives all the time; `board: None` is not, and every system
/// that reads it bails loudly enough for a test to notice.
fn duel_holding_a_sorcery() -> Duel {
    duel_holding(a_cheap_sorcery())
}

/// The same table, holding whichever card is named.
///
/// Split out of [`duel_holding_a_sorcery`] so the instant tests below stand
/// on the *same* board — two Forests, one hand card, the engine offering
/// both lands as mana abilities and nothing as castable. A second fixture
/// would let the two halves drift, and the whole claim is that the only
/// thing differing between them is the card's type.
fn duel_holding(def: &'static baylee_cards_dsl::CardDef) -> Duel {
    let lands: Vec<_> = (1..=2).map(forest).collect();
    let ids: Vec<ObjectId> = lands.iter().map(|o| o.id).collect();
    let mut view = ViewBuilder::new(2).with_battlefield(0, lands).build();
    view.hand = vec![HandObject {
        id: ObjectId::new(50, 0),
        card: CardIdentity {
            index: def.index,
            print: PrintRef::new(0),
            face: 0,
        },
        name: def.faces[0].name.to_string(),
        mana_value: def.faces[0].mana_cost.cmc(),
        colors: def.faces[0].mana_cost.colors(),
        types: def.faces[0].types,
        commander: false,
    }];
    let legal = LegalActions {
        can_pass: true,
        mana_abilities: ids,
        ..LegalActions::default()
    };
    Duel {
        view: Some(view),
        interaction: Some(Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(legal),
            },
            PlayerId::new(0),
        )),
        ..Duel::default()
    }
}

/// The offer this client makes on its own — "I will tap those lands for
/// you" — is only worth making when the spell could actually be cast.
#[test]
fn a_sorcery_is_reached_for_in_an_open_main_phase() {
    let duel = duel_holding_a_sorcery();
    assert_eq!(
        reachable(&duel).len(),
        1,
        "two Forests pay for it and nothing is on the stack"
    );
}

/// The reported fault. Taking the offer taps the lands *first*, so a
/// spell the engine will then refuse spends the turn's mana on nothing —
/// which makes offering it worse than saying nothing at all.
#[test]
fn a_sorcery_is_not_reached_for_over_an_unresolved_stack() {
    let mut duel = duel_holding_a_sorcery();
    duel.view
        .as_mut()
        .expect("the view")
        .stack
        .push(token(90, 1, "Something Resolving", 0, 0));
    assert!(
        reachable(&duel).is_empty(),
        "CR 307.1: not while the stack has anything on it"
    );
}

/// And the other half of the same window.
#[test]
fn nor_on_somebody_elses_turn() {
    let mut duel = duel_holding_a_sorcery();
    duel.view.as_mut().expect("the view").active = PlayerId::new(1);
    assert!(reachable(&duel).is_empty());
}

/// The first card in the registry that is only an instant and costs at
/// most one green mana plus generic.
///
/// Found rather than named, for the reason [`a_cheap_sorcery`] gives.
fn a_cheap_instant() -> &'static baylee_cards_dsl::CardDef {
    const GREEN: ColorSet = ColorSet::of(Color::Green);
    baylee_cards::all()
        .filter(|def| def.faces.len() == 1)
        .find(|def| {
            let face = &def.faces[0];
            face.types == TypeSet::INSTANT
                && face.mana_cost.symbols().next().is_some()
                && face.mana_cost.cmc() <= 2
                && face.mana_cost.colors().difference(GREEN) == ColorSet::EMPTY
        })
        .expect("the pool has a cheap mono-green instant")
}

/// The offer this client makes on its own, for a card the stack does not
/// close the window on.
///
/// Every claim this file made about the stack and the turn was a
/// **negative** — a sorcery is not offered over a stack, not on somebody
/// else's turn — and a file of negatives passes exactly as well when
/// `reachable` has stopped offering anything at all. That is the shape of
/// the fault reported in #112: an instant in hand, an untapped land, no
/// floating mana, so `castable` is empty by design and the card is
/// reachable or it is nothing.
///
/// `timing::allows` already asserts the instant case at the unit. This
/// asserts it through the composition a player actually meets, which is
/// where the cost filter, the source scan and the mana plan are.
#[test]
fn an_instant_is_reached_for_over_an_unresolved_stack() {
    let mut duel = duel_holding(a_cheap_instant());
    duel.view
        .as_mut()
        .expect("the view")
        .stack
        .push(token(90, 1, "Something Resolving", 0, 0));
    assert_eq!(
        reachable(&duel).len(),
        1,
        "CR 117.1b: responding is the whole reason to hold one"
    );
}

/// And on somebody else's turn, which is where a player holds one.
#[test]
fn and_on_somebody_elses_turn_as_well() {
    let mut duel = duel_holding(a_cheap_instant());
    let view = duel.view.as_mut().expect("the view");
    view.active = PlayerId::new(1);
    view.stack.push(token(90, 1, "Something Resolving", 0, 0));
    assert_eq!(
        reachable(&duel).len(),
        1,
        "the reported window: an opponent's spell on the stack, on their turn"
    );
}

/// A basic land of `kind` on the table, an untapped source the engine offers.
fn basic(slot: u32, name: &str, kind: baylee_core::ids::SubtypeId) -> baylee_view::PublicObject {
    let mut obj = token(slot, 0, name, 0, 0);
    // A card, as a player's land is: an object with neither a card nor a
    // token is unreadable to `targeting` and reads as "might be a target".
    obj.card = Some(CardIdentity {
        index: baylee_cards::decks::by_name(name).expect("a basic land"),
        print: PrintRef::new(0),
        face: 0,
    });
    obj.rules = obj.card.map(baylee_view::RulesFace::from);
    obj.types = TypeSet::LAND;
    obj.power = None;
    obj.toughness = None;
    obj.subtypes.insert(kind);
    obj
}

/// An opponent's Grizzly Bears, a card the way a real permanent is.
fn their_bears() -> baylee_view::PublicObject {
    let mut obj = token(90, 1, "Grizzly Bears", 2, 2);
    obj.card = Some(CardIdentity {
        index: baylee_cards::decks::by_name("Grizzly Bears").expect("in the pool"),
        print: PrintRef::new(0),
        face: 0,
    });
    obj.rules = obj.card.map(baylee_view::RulesFace::from);
    obj
}

/// A seat holding `name`, id 50, with `lands` it may tap and `theirs` on the
/// opponent's side of the table.
fn duel_holding_named(
    name: &str,
    lands: Vec<baylee_view::PublicObject>,
    theirs: Vec<baylee_view::PublicObject>,
) -> Duel {
    let def = baylee_cards::by_index(baylee_cards::decks::by_name(name).expect("a card by name"))
        .expect("a registered card");
    let ids: Vec<ObjectId> = lands.iter().map(|o| o.id).collect();
    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, lands)
        .with_battlefield(1, theirs)
        .build();
    view.hand = vec![HandObject {
        id: ObjectId::new(50, 0),
        card: CardIdentity {
            index: def.index,
            print: PrintRef::new(0),
            face: 0,
        },
        name: def.faces[0].name.to_string(),
        mana_value: def.faces[0].mana_cost.cmc(),
        colors: def.faces[0].mana_cost.colors(),
        types: def.faces[0].types,
        commander: false,
    }];
    Duel {
        view: Some(view),
        interaction: Some(Interaction::new(
            Pending::Priority {
                player: PlayerId::new(0),
                legal: Box::new(LegalActions {
                    can_pass: true,
                    mana_abilities: ids,
                    ..LegalActions::default()
                }),
            },
            PlayerId::new(0),
        )),
        ..Duel::default()
    }
}

/// Report 17: Prismatic Ending (`{X}{W}`) beside a Plains is in reach, and the
/// click's plan taps the Plains. An `{X}` cost used to be unreadable for
/// reach (the planner refuses X), so a card the engine offered as payable
/// answered no click. Reach reads X at its least, 0 (CR 107.3).
#[test]
fn an_x_spell_is_reached_for_when_the_rest_of_its_cost_is_paid() {
    let creature = their_bears();
    let duel = duel_holding_named(
        "Prismatic Ending",
        vec![basic(1, "Plains", land::PLAINS)],
        vec![creature.clone()],
    );
    let card = ObjectId::new(50, 0);
    assert!(
        reachable(&duel).contains(&card),
        "one Plains pays {{W}} with X at 0"
    );
    let plan = crate::mana_for(&duel, card).expect("the click has a plan");
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].source, ObjectId::new(1, 0));

    // The counter-test: the same card with no white source is out of reach,
    // so the first half is about the Plains and not about X being waved by.
    let duel = duel_holding_named("Prismatic Ending", vec![forest(1)], vec![creature]);
    assert!(!reachable(&duel).contains(&card));
    assert!(crate::mana_for(&duel, card).is_none());
}

/// Report 00: Cyclonic Rift with no nonland permanent an opponent controls has
/// no legal target for its normal mode, leaving only overload `{6}{U}`. A click
/// the client does not answer (the card is not in `Duel::reachable`, which
/// this harness leaves empty, as `input::tests` documents) says so, not "the
/// resources are not available"; with such a permanent present the refusal is
/// not that sentence.
#[test]
fn a_rift_with_nothing_to_bounce_says_only_the_costlier_mode_is_left() {
    use baylee_client_core::i18n::{Phrase, Refusal};
    let island = |slot| basic(slot, "Island", land::ISLAND);
    let card = ObjectId::new(50, 0);

    let mut bare = duel_holding_named("Cyclonic Rift", vec![island(1), island(2)], vec![]);
    crate::input::activate_card(&mut bare, card);
    assert_eq!(
        bare.last_error,
        Some(Refusal::Said(Phrase::OnlyDearerModeLeft)),
        "nothing to target in the normal mode"
    );
    assert!(bare.armed.is_none() && bare.outbox().is_empty());

    let mut with_target = duel_holding_named(
        "Cyclonic Rift",
        vec![island(1), island(2)],
        vec![their_bears()],
    );
    crate::input::activate_card(&mut with_target, card);
    assert_ne!(
        with_target.last_error,
        Some(Refusal::Said(Phrase::OnlyDearerModeLeft)),
        "a nonland permanent to bounce leaves the normal mode"
    );
}

/// Report 00, the reach half: a Cyclonic Rift with nothing to bounce is out of
/// reach on two Islands (its printed `{1}{U}` mode has no target and the
/// overload `{6}{U}` is not paid), in reach on seven (overload is), and in
/// reach on two beside an opponent's nonland permanent (the normal mode).
/// Before the fix the first case was in reach, so the click armed lands to
/// tap for a cast the engine then opened an unpayable window for.
#[test]
fn a_rift_with_nothing_to_bounce_is_reached_for_only_at_overloads_price() {
    let island = |slot| basic(slot, "Island", land::ISLAND);
    let card = ObjectId::new(50, 0);

    let bare = duel_holding_named("Cyclonic Rift", (1..=2).map(island).collect(), vec![]);
    assert!(!reachable(&bare).contains(&card), "two Islands, no target");

    let seven = duel_holding_named("Cyclonic Rift", (1..=7).map(island).collect(), vec![]);
    assert!(reachable(&seven).contains(&card), "overload is paid");

    let target = duel_holding_named(
        "Cyclonic Rift",
        (1..=2).map(island).collect(),
        vec![their_bears()],
    );
    assert!(reachable(&target).contains(&card), "the normal mode");
}

/// Report 00, the click: with the reach set filled as the frame fills it, the
/// bare two-Island Rift is answered with the refusal that says why, and arms
/// nothing; beside a target the same click arms the cast.
#[test]
fn clicking_an_unreached_rift_says_why_and_arms_nothing_while_a_target_arms_it() {
    use baylee_client_core::i18n::{Phrase, Refusal};
    let island = |slot| basic(slot, "Island", land::ISLAND);
    let card = ObjectId::new(50, 0);

    let mut bare = duel_holding_named("Cyclonic Rift", (1..=2).map(island).collect(), vec![]);
    bare.reachable = reachable(&bare);
    crate::input::activate_card(&mut bare, card);
    assert_eq!(
        bare.last_error,
        Some(Refusal::Said(Phrase::OnlyDearerModeLeft))
    );
    assert!(bare.armed.is_none() && bare.outbox().is_empty());

    let mut target = duel_holding_named(
        "Cyclonic Rift",
        (1..=2).map(island).collect(),
        vec![their_bears()],
    );
    target.reachable = reachable(&target);
    crate::input::activate_card(&mut target, card);
    assert_eq!(target.last_error, None);
    assert!(target.armed.is_some());
}

#[test]
fn foretell_is_reachable_throughout_its_owners_turn_only() {
    let card = ObjectId::new(50, 0);
    let mut duel = duel_holding_named("Doomskar", vec![forest(1), forest(2)], vec![]);
    duel.view.as_mut().expect("the view").phase = baylee_view::Phase::Combat;
    duel.view
        .as_mut()
        .expect("the view")
        .stack
        .push(token(90, 0, "An instant", 0, 0));
    assert!(foretell_reach(&duel).contains(&card));
    assert_eq!(
        crate::foretell_mana_for(&duel, card)
            .expect("a plan")
            .taps(),
        2
    );

    duel.view.as_mut().expect("the view").active = PlayerId::new(1);
    assert!(foretell_reach(&duel).is_empty());
}

#[test]
fn a_castable_foretell_card_offers_both_and_the_choice_arms_the_special_action() {
    let plains = |slot| basic(slot, "Plains", land::PLAINS);
    let card = ObjectId::new(50, 0);
    let mut duel = duel_holding_named("Doomskar", (1..=5).map(plains).collect(), vec![]);
    duel.reachable = reachable(&duel);
    duel.foretell_reach = foretell_reach(&duel);
    crate::input::activate_card(&mut duel, card);
    let menu = duel
        .cast_menu
        .as_ref()
        .expect("cast and foretell are two deeds");
    assert_eq!(menu.modes.len(), 2);
    assert_eq!(
        menu.modes[1].kind,
        baylee_engine::choice::CastModeKind::Foretell
    );

    crate::input::take_cast_row(&mut duel, 1);
    assert!(duel.cast_answer.is_none());
    assert!(matches!(
        duel.armed.as_ref().map(|armed| &armed.deed),
        Some(crate::Deed::Run {
            then: crate::RunEnd::Foretell,
            ..
        })
    ));
    crate::input::fire_armed(&mut duel);
    assert_eq!(
        duel.mana_run.as_ref().expect("the funded run").then(),
        crate::RunEnd::Foretell
    );
}
