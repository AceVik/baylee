//! Oracle: {T}: Add {R}, {G}, or {W}.
#[allow(clippy::wildcard_imports)] // Shared behavioral harness.
use super::*;

fn subject() -> CardIndex {
    card_index("02e81bb8-ba80-4483-9b9f-973dcae0fc9d")
}

// Checklist: actually produce each printed color, exactly one; immediate mana
// ability has no stack; taps source and cannot repeat while tapped; no colorless.
#[test]
fn each_color_is_one_mana_and_tap_is_paid() {
    let p0 = PlayerId::new(0);
    let colors = [ManaColor::Red, ManaColor::Green, ManaColor::White];
    for color in colors {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[subject()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let source = on_battlefield(&e, p0, subject()).unwrap();
        activate(&mut e, p0, subject(), 0);
        let Pending::ChooseColor { options, .. } = e.pending() else {
            panic!("mana color choice")
        };
        assert_eq!(options.len(), colors.len());
        for allowed in colors {
            assert!(options.contains(&allowed));
        }
        assert!(!options.contains(&ManaColor::Colorless));
        e.apply(p0, PlayerAction::ChooseColor(color)).unwrap();
        assert!(stack_is_empty(&e));
        assert!(is_tapped(&e, source));
        for check in [
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
            ManaColor::Colorless,
        ] {
            assert_eq!(
                e.state().players[0].mana_pool.available(check),
                u32::from(check == color)
            );
        }
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("mana resolved immediately")
        };
        assert!(!legal.abilities.iter().any(|(id, _)| *id == source));
    }
}
