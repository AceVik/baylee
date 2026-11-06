//! Triggers going on the stack at once (CR 603.3b), as a model reads the
//! question: each ability's own words, which goes first, and one list back.

use super::*;
use baylee_core::ids::AbilityRef;
use baylee_engine::choice::{ArrangePile, ArrangePlace, ArrangePrompt};
use baylee_view::{OrderedTrigger, StackItem};

const OPPONENT: PlayerId = PlayerId::new(1);

/// `name`'s first ability about what `about` says, as the host tells it.
fn row(place: u32, card: &str, twin: u32, about: (&str, PlayerId)) -> OrderedTrigger {
    let def = baylee_cards::all()
        .find(|def| def.faces[0].name == card)
        .expect("a card of the pool");
    OrderedTrigger {
        handle: ObjectId::queued_trigger(place),
        item: StackItem::Ability {
            source: id(30),
            ability: Some(AbilityRef::new(def.index, 0)),
            text: None,
            rules: Some(RulesFace {
                card: def.index,
                face: 0,
            }),
            token: None,
        },
        name: card.to_owned(),
        twin,
        about_object: None,
        about_name: Some(about.0.to_owned()),
        about_player: Some(about.1),
        amount: None,
        asks_targets: false,
        modal: false,
    }
}

/// Three Dingus Egg triggers and a Flametongue Kavu's: the Egg about the
/// opponent's land, two about the seat's own, and a target to come.
fn question() -> Request {
    let (mut view, _) = board();
    let mut kavu = row(3, "Flametongue Kavu", 3, ("Flametongue Kavu", ME));
    kavu.asks_targets = true;
    view.trigger_order = vec![
        kavu,
        row(2, "Dingus Egg", 2, ("Forest", OPPONENT)),
        row(1, "Dingus Egg", 0, ("Mountain", ME)),
        row(0, "Dingus Egg", 0, ("Mountain", ME)),
    ];
    let cards: Vec<ObjectId> = view.trigger_order.iter().map(|t| t.handle).collect();
    let n = u32::try_from(cards.len()).expect("a few");
    let pending = Pending::Arrange {
        player: ME,
        cards,
        piles: vec![ArrangePile::all_of(ArrangePlace::Stack, n)],
        prompt: ArrangePrompt::TriggerOrder,
    };
    request(
        view,
        pending,
        LogTail {
            from: 0,
            entries: Vec::new(),
        },
    )
}

fn order(ids: &[&str]) -> Decision {
    Decision {
        ask: Some("q12".into()),
        piles: vec![ids.iter().map(ToString::to_string).collect()],
        ..Decision::default()
    }
}

#[test]
fn a_trigger_order_reads_as_its_golden_text() {
    let request = question();
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    golden("trigger_order.txt", &wake.text);
    assert!(wake.text.contains("resolves FIRST"), "{}", wake.text);
    assert!(wake.text.contains("the same as #T2"), "{}", wake.text);
}

/// The list goes back as written: the first named is the first to resolve,
/// which is the first of the pile (`docs/protocol.md`).
#[test]
fn the_list_a_model_gives_is_the_answer_in_its_order() {
    let request = question();
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    assert_eq!(
        wake.menu
            .resolve(&order(&["#T3", "#T4", "#T2", "#T1"]))
            .expect("an order")
            .act,
        Act::Now(PlayerAction::Arrange {
            piles: vec![vec![
                ObjectId::queued_trigger(2),
                ObjectId::queued_trigger(3),
                ObjectId::queued_trigger(1),
                ObjectId::queued_trigger(0),
            ]]
        })
    );
}

#[test]
fn a_list_that_is_not_every_ability_once_is_refused() {
    let request = question();
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    for wrong in [
        &["#T1", "#T2", "#T3"][..],
        &["#T1", "#T1", "#T2", "#T3"],
        &["#T1", "#T2", "#T3", "#T9"],
        &["#T1", "#T2", "#T3", "#4294967295"],
    ] {
        assert!(wake.menu.resolve(&order(wrong)).is_err(), "{wrong:?}");
    }
}
