//! Oracle checklist: tapping the artifact immediately adds exactly one chosen B, R, or G, with no stack or payment and no other color option.
#[allow(clippy::wildcard_imports)]
use super::*;

fn obelisk_of_jund() -> CardIndex {
    card_index("eb0be178-55f9-4743-8128-34bc86f134ec")
}

#[test]
fn jund_obelisk_offers_and_adds_each_of_its_three_colors() {
    let p0 = PlayerId::new(0);
    for color in [ManaColor::Black, ManaColor::Red, ManaColor::Green] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[obelisk_of_jund()])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let source = on_battlefield(&e, p0, obelisk_of_jund()).unwrap();
        activate(&mut e, p0, obelisk_of_jund(), 0);
        let Pending::ChooseColor { player, options } = e.pending().clone() else {
            panic!(
                "the mana ability asks which printed color: {:?}",
                e.pending()
            );
        };
        assert_eq!(player, p0);
        assert_eq!(options.len(), 3);
        assert!(options.contains(&ManaColor::Black));
        assert!(options.contains(&ManaColor::Red));
        assert!(options.contains(&ManaColor::Green));
        assert!(
            !options.contains(&ManaColor::Blue)
                && !options.contains(&ManaColor::White)
                && !options.contains(&ManaColor::Colorless)
        );
        e.apply(p0, PlayerAction::ChooseColor(color)).unwrap();
        assert_eq!(e.state().players[0].mana_pool.available(color), 1);
        for other in [
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
            ManaColor::Colorless,
        ] {
            if other != color {
                assert_eq!(e.state().players[0].mana_pool.available(other), 0);
            }
        }
        assert!(is_tapped(&e, source));
        assert!(stack_is_empty(&e), "mana is added without a stack object");
    }
}
