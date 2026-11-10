//! `cards/artifacts/mv_2/thran_lens.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn thran_lens() -> CardIndex {
    card_index("0ac723bf-ce86-4028-a693-0ef498e4c82d")
}

#[test]
fn thran_lens_makes_permanents_colorless() {
    use baylee_core::color::{Color, ColorSet};

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), llanowar_elves()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[thran_lens(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(elves)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Green])
    );
    assert_eq!(
        engine
            .state()
            .object(ogre)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Red])
    );

    cast_from_hand(&mut engine, p0, thran_lens());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(elves)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::EMPTY
    );
    assert_eq!(
        engine
            .state()
            .object(ogre)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::EMPTY
    );
    let hand_card = in_hand(&engine, p0, gray_ogre()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(hand_card)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::from_slice(&[Color::Red])
    );
}
