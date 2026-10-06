//! `cards/sorceries/mv_3/goblin_offensive.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Offensive is a sorcery reading "Create X 1/1 red Goblin creature
/// tokens." — X *is* the card, so the board is five Mountains, which is
/// exactly `{X}{1}{R}{R}` at X = 2 and nothing more: the spell is answered at
/// its X question (CR 601.2b draws X out of the announcer before any cost is
/// paid), the pool reads empty afterwards, and the tokens are counted on the
/// battlefield. Two Goblins is the only number that reads both halves of the
/// cost — an X of 1 would have left a Mountain's worth of red floating, and a
/// flood of tokens would mean the mana was never spent at all.
#[test]
fn goblin_offensive_creates_one_goblin_token_for_each_point_of_x() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[goblin_offensive()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, goblin_offensive()).expect("the spell is in hand");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing is on the board before the spell resolves"
    );

    // Mana into the pool first: `legal.castable` is filtered through
    // `can_afford`, which reads the pool and not the untapped lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains tapped and the board holds nothing else that makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.castable.contains(&spell),
        "five red pays {{X}}{{1}}{{R}}{{R}} with X = 2: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, goblin_offensive());
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "an X spell asks for its X before its cost is paid, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the seat casting the spell names X");
    assert_eq!(min, 0, "X may be zero");
    assert!(
        max >= 2,
        "five Mountains reach exactly X = 2, so 2 is on the menu: {max}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "CR 601.2h pays last: the mana is untouched while the question stands"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("X = 2 is what the floating mana pays for");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{1}}{{R}}{{R}} with X = 2 spends all five red"
    );
    let goblins = tokens_of(&engine, p0);
    assert_eq!(
        goblins.len(),
        2,
        "one 1/1 Goblin per point of X — and not one per red spent"
    );
    for goblin in goblins {
        let token = engine
            .state()
            .object(goblin)
            .and_then(|o| o.token)
            .expect("a created token knows which token it is");
        assert_eq!((token.power, token.toughness), (Some(1), Some(1)));
        assert!(
            token.colors.contains(baylee_core::color::Color::Red),
            "a 1/1 *red* Goblin"
        );
    }
}
