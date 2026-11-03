//! A payment owed (CR 605.3a, `PlayerView::owed`): what the window offers
//! and how the seat pays from it, on boards rebuilt from LLM games.

use super::*;
use crate::llm::seatstate::Seat;
use baylee_core::mana::ManaPayment;

/// Codex attempt 62, turn 15, q191: P1's upkeep under the opponent's Energy
/// Flux, with Aeolipile's granted tax trigger on the stack and "yes, pay
/// {2}" answered. Forests #25, #18 and #29 untapped, #11 and #14 tapped;
/// Aeolipile #1 and Skull Catapult #42 on the table.
///
/// `unpaid_abilities` is what the engine published at the opening of such
/// a window before it narrowed its hints: the two artifacts' "{1}, {T}"
/// abilities.
fn energy_flux_tax() -> (PlayerView, Pending) {
    let mut view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            [
                card(25, "Forest"),
                tapped(card(11, "Forest")),
                card(18, "Forest"),
                tapped(card(14, "Forest")),
                card(29, "Forest"),
                card(34, "Fyndhorn Elves"),
                card(1, "Aeolipile"),
                card(42, "Skull Catapult"),
            ],
        )
        .with_battlefield(1, [card(118, "Water Elemental"), card(74, "Energy Flux")])
        .build();
    view.turn = 15;
    view.phase = baylee_view::Phase::Beginning;
    view.step = baylee_view::Step::Upkeep;
    let mut trigger = token(121, 0, "Aeolipile", 0, 0);
    trigger.stack_item = Some(baylee_view::StackItem::Ability {
        source: id(1),
        ability: None,
        text: None,
        rules: None,
        token: None,
    });
    view.stack = vec![trigger];
    view.awaiting = Some(ME);
    view.owed = Some(ManaPayment::Fixed("{2}".parse().unwrap()));
    let pending = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![id(25), id(18), id(29)],
            abilities: vec![(id(34), 0)],
            unpaid_abilities: vec![
                (id(1), 0, "{1}".parse().unwrap()),
                (id(42), 0, "{1}".parse().unwrap()),
            ],
            ..LegalActions::default()
        }),
    };
    (view, pending)
}

/// NEW-06: the window offers the payment and the pass, and no spell or
/// activation the engine would not take there. Before, it offered
/// "Activate Aeolipile (taps Forest #18)": the seat tapped the Forest,
/// passed to let the trigger resolve, and so left the tax unpaid; the
/// Aeolipile was sacrificed with its ability never activated.
#[test]
fn an_owed_payment_offers_only_the_payment_and_the_pass() {
    let (mut view, pending) = energy_flux_tax();
    // A creature in hand the lands could pay for is no option either.
    view.hand = vec![in_hand(60, "Llanowar Elves")];
    let request = request(view, pending, LogTail::default());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let options: Vec<&str> = wake
        .text
        .lines()
        .skip_while(|line| *line != "Options:")
        .skip(1)
        .collect();
    assert!(
        options.iter().any(|line| line.contains("Pay {2} (taps")),
        "{}",
        wake.text
    );
    assert!(
        options.iter().any(|line| line.contains("without paying")),
        "{}",
        wake.text
    );
    for absent in ["Aeolipile", "Skull Catapult", "Cast"] {
        assert!(
            options.iter().all(|line| !line.contains(absent)),
            "{absent} is not legal in a payment window:\n{}",
            wake.text
        );
    }
}

/// NEW-06, the seat's half: taps in flight that meet a payment window do
/// not pass it to let an ability on the stack resolve, because that pass
/// settles the payment unpaid. The model is asked instead.
#[test]
fn taps_in_flight_never_pass_an_owed_window() {
    let (mut view, pending) = energy_flux_tax();
    // The same board in an ordinary window: Aeolipile is within reach.
    view.owed = None;
    let open = request(view.clone(), pending.clone(), LogTail::default());
    let mut seat = Seat::new(&open.context, None, Style::default());
    let mut narrator = Narrator::new(&open.context);
    let wake = narrator.wake(&open, &[], None);
    let pick = wake
        .text
        .lines()
        .find(|line| line.contains("Activate Aeolipile") && line.contains("(taps"))
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or_else(|| panic!("Aeolipile is reachable:\n{}", wake.text))
        .to_string();
    let resolved = wake
        .menu
        .resolve(&Decision {
            ask: Some("q12".into()),
            pick: vec![pick],
            ..Decision::default()
        })
        .expect("the activation resolves");
    let first = seat.keep(&open, narrator, resolved, None);
    assert!(matches!(first, PlayerAction::ActivateManaAbility { .. }));
    // The table now holds a payment window, the trigger still on the
    // stack and the Forest's {G} in the pool.
    view.owed = Some(ManaPayment::Fixed("{2}".parse().unwrap()));
    view.seats[0].mana_pool.green = 1;
    let owed = request(view, pending, LogTail::default());
    assert!(
        seat.begin(&owed, |_| false).is_none(),
        "the pass would settle the window: the model is asked"
    );
}
