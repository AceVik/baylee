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
