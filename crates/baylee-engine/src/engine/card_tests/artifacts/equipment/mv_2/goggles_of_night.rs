//! `cards/artifacts/equipment/mv_2/goggles_of_night.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn goggles_of_night() -> CardIndex {
    card_index("af27761d-88df-4d4f-a366-d55ed62d1d33")
}

/// Goggles of Night: "Whenever equipped creature deals combat damage to a
/// player, scry 1, then draw a card. Equip {2}". The scried card goes to the
/// bottom, so the card drawn after it is a different one.
#[test]
fn goggles_of_night_scry_then_draw_on_combat_damage() {
    let p0 = PlayerId::new(0);
    let mut r = rig(goggles_of_night(), llanowar_elves(), 2);
    r.equip(2);
    let library_before = library_size(&r.engine, p0);
    assert_eq!(r.hand_size(), 0);
    r.attack_with_host();
    pass_until(&mut r.engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        prompt,
        ..
    } = r.engine.pending().clone()
    else {
        unreachable!("pass_until stops on the scry")
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(cards.len(), 1, "scry 1 looks at one card");
    assert_eq!(r.hand_size(), 0, "the draw comes after the scry");
    let looked_at = cards[0];
    r.engine
        .apply(p0, look_answer(&cards, &[looked_at]))
        .unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(r.hand_size(), 1, "one card drawn");
    assert_eq!(library_size(&r.engine, p0), library_before - 1);
    assert!(
        !r.engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&looked_at),
        "the card sent to the bottom is not the one drawn"
    );
    assert_eq!(r.engine.state().players[1].life, 19, "1 combat damage");
}
