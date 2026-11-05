//! An activation a card allows in one step alone is never passed by the
//! rail in that step.

use super::*;

/// Codex attempt 191, q215: this seat's upkeep, Ashen Ghoul in its
/// graveyard with three creature cards above it and a Swamp untapped. Its
/// "{B}: Return this card from your graveyard to the battlefield. Activate
/// only during your upkeep" is offered as the engine offers an activation
/// whose mana is not floating yet (`unpaid_abilities`).
fn ashen_ghoul_upkeep() -> (PlayerView, LegalActions) {
    let (mut view, mut legal) = table(&["Swamp", "Swamp"], &[]);
    view.graveyards[0] = vec![
        permanent(30, "Ashen Ghoul"),
        permanent(31, "Grizzly Bears"),
        permanent(32, "Grizzly Bears"),
        permanent(33, "Grizzly Bears"),
    ];
    view.phase = Phase::Beginning;
    view.step = Step::Upkeep;
    legal
        .unpaid_abilities
        .push((o(30), 0, "{B}".parse().unwrap()));
    (view, legal)
}

/// G001-D: the default rail has no upkeep row, and the window was passed
/// as quiet every turn the Ghoul could have come back.
#[test]
fn an_upkeep_only_activation_wakes_the_seat_in_its_upkeep() {
    let (view, legal) = ashen_ghoul_upkeep();
    assert!(!WakeFilter::default().stops_at(RailSide::Mine, RailRow::Upkeep));
    assert!(offering(&view, &legal), "the Swamps pay the {{B}}");
    assert_eq!(judge(&view, legal), Verdict::Wake(Why::RailStop));
}

/// An activation any window allows is still passed where the rail passes.
#[test]
fn an_activation_of_any_step_is_passed_in_the_upkeep() {
    let (mut view, mut legal) = ashen_ghoul_upkeep();
    view.graveyards[0].clear();
    legal.unpaid_abilities.clear();
    view.battlefield.push(permanent(40, "Drudge Skeletons"));
    legal.abilities.retain(|&(source, _)| source != o(40));
    legal
        .unpaid_abilities
        .push((o(40), 0, "{B}".parse().unwrap()));
    assert!(offering(&view, &legal));
    assert_eq!(judge(&view, legal), passes(Standing::QuietWindow));
}
