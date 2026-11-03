//! The play against Power Surge: tapping lands for mana that is lost.

use super::*;

/// Codex attempt 184, turn 20, q286: this seat's first main phase with
/// its own Power Surge #45 out and four Mountains untapped, Inferno in
/// hand (more than they make).
fn surge_board(surge: bool) -> Request {
    let mut battlefield = vec![
        card(25, "Mountain"),
        card(26, "Mountain"),
        card(27, "Mountain"),
        card(36, "Mountain"),
        card(54, "The Hive"),
    ];
    if surge {
        battlefield.push(card(45, "Power Surge"));
    }
    let mut view = ViewBuilder::new(2).with_battlefield(0, battlefield).build();
    view.turn = 20;
    view.hand = vec![in_hand(9, "Inferno")];
    let pending = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![id(25), id(26), id(27), id(36)],
            ..LegalActions::default()
        }),
    };
    request(view, pending, LogTail::default())
}

/// G001-C: with Power Surge out the menu offers the taps that keep the
/// lands from counting when the next turn begins, as one answer: the taps,
/// then the pass. The menu offered a tap only as part of a payment, so the
/// model could not make the play.
#[test]
fn power_surge_offers_tapping_the_lands_for_nothing() {
    let request = surge_board(true);
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let line = wake
        .text
        .lines()
        .find(|line| line.contains("Tap your untapped lands for mana"))
        .unwrap_or_else(|| panic!("the option is offered:\n{}", wake.text));
    assert!(line.contains("Power Surge #45"), "{line}");
    let pick = line.split_whitespace().next().expect("an id").to_string();
    let resolved = wake
        .menu
        .resolve(&Decision {
            ask: Some("q12".into()),
            pick: vec![pick],
            ..Decision::default()
        })
        .expect("the option resolves");
    let Act::Taps { steps, then } = resolved.act else {
        panic!("taps: {:?}", resolved.act);
    };
    assert_eq!(then, PlayerAction::PassPriority);
    let mut tapped: Vec<u32> = steps.iter().map(|s| s.source.slot()).collect();
    tapped.sort_unstable();
    assert_eq!(tapped, [25, 26, 27, 36]);
}

/// Without a card that counts untapped lands the option is not there: it
/// would be a line in every menu for nothing.
#[test]
fn no_power_surge_no_option() {
    let request = surge_board(false);
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    assert!(
        !wake.text.contains("Tap your untapped lands"),
        "{}",
        wake.text
    );
}
