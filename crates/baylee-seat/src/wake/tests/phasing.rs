//! Phased-out creatures cannot contribute to a reachable spell's payment.

use super::{permanent, reachable, table};

#[test]
fn a_phased_out_creature_cannot_convoke_the_missing_colored_pip() {
    let (mut view, legal) = table(&["Forest", "Forest"], &["Chord of Calling"]);
    view.battlefield.push(permanent(10, "Grizzly Bears"));
    let reach = reachable(&view, &legal);
    assert_eq!(reach.len(), 1);
    assert_eq!(reach[0].plan.steps.len(), 2);

    view.battlefield[2].status = baylee_view::ObjectStatus::PHASED_OUT;
    assert!(reachable(&view, &legal).is_empty());
}
