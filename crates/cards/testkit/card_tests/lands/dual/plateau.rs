//! `cards/lands/dual/plateau.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Plateau prints no rules text at all — the whole card is the type line
/// "Land — Mountain Plains", so both halves of it are about how the engine
/// reads those two basic land types: the land has to be a real land drop that
/// arrives **untapped** (it prints no entry modifier), and one activation has
/// to offer *both* colors, which is what `{T}: Add {R} or {W}` means once
/// CR 305.6 supplies the text from the types. Two copies stand on the table
/// so that a single turn pays for a white and a red out of the same card,
/// and an untapped land that produces one fixed color could not produce the
/// pair.
#[test]
fn plateau_is_a_mountain_and_a_plains_that_tap_for_either_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4821, forest())
        .battlefield(0, &[plateau()])
        .hand(0, &[plateau()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real land drop: `starting_battlefield` places a permanent without an
    // entry (`Cause::Setup`), which is exactly what a land that entered tapped
    // would slip past unnoticed.
    let played = play_land(&mut engine, p0, plateau());
    assert!(
        !entered_tapped(&engine, played),
        "Plateau prints no entry modifier, so the land drop leaves it ready"
    );
    let copies = all_on_battlefield(&engine, p0, plateau());
    assert_eq!(copies.len(), 2, "the played copy and the seeded one");
    let seeded = *copies
        .iter()
        .find(|id| **id != played)
        .expect("the copy that was dealt, not played");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, so both mana below come off these two lands"
    );

    for (land, color) in [(played, ManaColor::White), (seeded, ManaColor::Red)] {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        // Both lists carry mana abilities (#159): a basic land type is the
        // CR 305.6 shortcut, while a card that printed its own `{T}: Add …`
        // would be an ordinary `(source, index)` entry.
        let tap = if legal.mana_abilities.contains(&land) {
            PlayerAction::ActivateManaAbility { source: land }
        } else {
            let (source, ability_index) = legal
                .abilities
                .iter()
                .copied()
                .find(|(src, _)| *src == land)
                .expect("the tap is offered in one of the two lists");
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            }
        };
        engine
            .apply(p0, tap)
            .expect("the offer named this land's tap, so it is affordable");

        let Pending::ChooseColor { player, options } = engine.pending().clone() else {
            panic!(
                "`Add {{R}} or {{W}}` is a choice while both are producible, \
                 got {:?}",
                engine.pending()
            )
        };
        assert_eq!(player, p0, "the seat that activated names the color");
        assert!(
            options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
            "the Mountain's red and the Plains' white: {options:?}"
        );
        assert_eq!(
            options.len(),
            2,
            "two printed basic types, two colors and no third: {options:?}"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(color))
            .expect("the answer came out of the question");
    }

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "one tap said white");
    assert_eq!(pool.available(ManaColor::Red), 1, "the other said red");
    assert_eq!(pool.total(), 2, "two taps, two mana, nothing else");
    assert!(
        is_tapped(&engine, played) && is_tapped(&engine, seeded),
        "each tap paid its own {{T}}"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
