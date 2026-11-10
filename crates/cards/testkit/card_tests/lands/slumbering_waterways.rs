//! `cards/lands/slumbering_waterways.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn slumbering_waterways() -> CardIndex {
    card_index("7a82c183-1561-4276-9a75-6bbb9c2bee2e")
}

#[test]
fn slumbering_waterways_enters_tapped_carries_keywords_and_taps_for_green_or_blue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[slumbering_waterways()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, slumbering_waterways());
    assert!(
        is_tapped(&engine, land),
        "Slumbering Waterways enters tapped"
    );
    let kw = engine
        .state()
        .object(land)
        .unwrap()
        .characteristics()
        .keywords;
    assert!(kw.contains(KeywordSet::FLYING));
    assert!(kw.contains(KeywordSet::VIGILANCE));
    assert!(kw.contains(KeywordSet::TRAMPLE));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, slumbering_waterways(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "expected ChooseColor for {{G}} or {{U}}, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0);
    assert_eq!(options, vec![ManaColor::Green, ManaColor::Blue]);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, land));
}
