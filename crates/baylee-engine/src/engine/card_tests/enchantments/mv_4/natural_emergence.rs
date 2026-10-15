//! `cards/enchantments/mv_4/natural_emergence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn natural_emergence() -> CardIndex {
    card_index("ce19d2dd-4b2c-493e-8963-ad97649248e2")
}

fn aether_sting() -> CardIndex {
    card_index("61105cb5-d7a1-4021-a006-dd1b947dfa68")
}

fn builder_s_blessing() -> CardIndex {
    card_index("33a64de1-dc78-43d3-b89e-b4b13049e174")
}

/// "When this enchantment enters, return a red or green enchantment you
/// control to its owner's hand.": the menu holds the red Sting and the
/// Emergence itself (red and green) and not the white Blessing; returning
/// the Sting puts it in hand.
#[test]
fn natural_emergence_returns_a_red_or_green_enchantment_of_mine() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[aether_sting(), builder_s_blessing()])
        .hand(0, &[natural_emergence()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Red, 2);
    float(&mut engine, p0, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p0, natural_emergence());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    let sting = on_battlefield(&engine, p0, aether_sting()).expect("seated");
    let blessing = on_battlefield(&engine, p0, builder_s_blessing()).expect("seated");
    let emergence = on_battlefield(&engine, p0, natural_emergence()).expect("resolved");
    assert!(options.contains(&sting), "red: eligible");
    assert!(options.contains(&emergence), "red and green: eligible");
    assert!(!options.contains(&blessing), "white: not eligible");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![sting],
            },
        )
        .expect("the Sting was on the menu");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, aether_sting()).is_some(),
        "back in hand"
    );
    assert!(on_battlefield(&engine, p0, aether_sting()).is_none());
}

/// "Lands you control are 2/2 creatures with first strike. They're still
/// lands.": mine are, the opponent's are plain lands.
#[test]
fn natural_emergence_makes_my_lands_two_two_first_strike_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[natural_emergence(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, forest()).expect("seated");
    let theirs = on_battlefield(&engine, p1, forest()).expect("seated");

    let t = types(&engine, mine);
    assert!(t.contains(TypeSet::LAND), "still a land");
    assert!(t.contains(TypeSet::CREATURE), "and a creature");
    assert_eq!(pt(&engine, mine), (2, 2), "2/2");
    assert!(keywords(&engine, mine).contains(baylee_cards_dsl::KeywordSet::FIRST_STRIKE));
    assert!(
        !types(&engine, theirs).contains(TypeSet::CREATURE),
        "the opponent's land is no creature"
    );
}
