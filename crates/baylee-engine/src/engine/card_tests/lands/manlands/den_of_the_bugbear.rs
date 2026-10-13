//! `cards/lands/manlands/den_of_the_bugbear.rs`, played.
//!
//! Den of the Bugbear: "If you control two or more other lands, this land
//! enters tapped. / {T}: Add {R}. / {3}{R}: Until end of turn, this land
//! becomes a 3/2 red Goblin creature with 'Whenever this creature attacks,
//! create a 1/1 red Goblin creature token that's tapped and attacking.' It's
//! still a land."
//!
//! Ability 0 is the mana ability and ability 1 the animation. The Den is
//! placed on the starting battlefield for the attack tests: a land that has
//! been under its controller's control since the turn began may attack
//! (CR 302.6), which a land played this turn may not. The bystander for the
//! "attacked" half of CR 508.3a is Righteous Cause, "Whenever a creature
//! attacks, you gain 1 life."

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature;

/// A duel in player 0's first main phase: the Den, a Righteous Cause and
/// `others` Mountains on the battlefield. Returns the engine and the Den.
#[track_caller]
fn the_den_and(others: usize) -> (Engine<RegistryLookup>, ObjectId) {
    let mut board = vec![den_of_the_bugbear(), righteous_cause()];
    board.extend(vec![mountain(); others]);
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let den = on_battlefield(&engine, kit::P0, den_of_the_bugbear()).expect("the Den");
    (engine, den)
}

/// Animates the Den once, off mana already floating, and lets the ability
/// resolve.
#[track_caller]
fn animate(engine: &mut Engine<RegistryLookup>) {
    activate(engine, kit::P0, den_of_the_bugbear(), 1);
    pass_until(engine, stack_is_empty);
}

/// Den of the Bugbear: "If you control two or more other lands, this land
/// enters tapped." / "{T}: Add {R}." / "{3}{R}: Until end of turn, this land
/// becomes a 3/2 red Goblin creature…"
///
/// Playing this land while controlling two or more other lands enters tapped,
/// and paying `{3}{R}` after untapping animates it into a 3/2 Goblin creature
/// that is still a land. (The first test of the card, kept as it was: only
/// its comment, which named the unfinished card, is rewritten.)
#[test]
fn den_of_the_bugbear_animates_into_goblin() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(310, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[den_of_the_bugbear()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, den_of_the_bugbear());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, den_of_the_bugbear(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 2));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

/// "If you control two or more other lands, this land enters tapped."
///
/// The boundary is the count of *other* lands: with none or one the Den
/// enters untapped, with two or three it enters tapped (it does not count
/// itself). One game per count, because there is one land drop per turn.
#[test]
fn den_of_the_bugbear_enters_tapped_only_with_two_or_more_other_lands() {
    for (others, tapped) in [(0_usize, false), (1, false), (2, true), (3, true)] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &vec![mountain(); others])
            .hand(0, &[den_of_the_bugbear()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, kit::P0);

        let land = play_land(&mut engine, kit::P0, den_of_the_bugbear());
        pass_until(&mut engine, stack_is_empty);

        assert_eq!(
            entered_tapped(&engine, land),
            tapped,
            "with {others} other land(s) the Den enters {}",
            if tapped { "tapped" } else { "untapped" }
        );
    }
}

/// "{T}: Add {R}."
///
/// The Den, untapped and alone, taps for exactly one red mana and is then
/// tapped.
#[test]
fn den_of_the_bugbear_taps_for_one_red_mana() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[den_of_the_bugbear()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let den = on_battlefield(&engine, kit::P0, den_of_the_bugbear()).expect("the Den");

    activate(&mut engine, kit::P0, den_of_the_bugbear(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one mana");
    assert_eq!(pool.available(ManaColor::Red), 1, "and it is red");
    assert!(is_tapped(&engine, den), "tapping it is the cost");
}

/// "…this land becomes a 3/2 red Goblin creature … It's still a land",
/// "until end of turn": its colours are exactly red (CR 105.3: an effect that
/// gives an object a colour replaces the colours it had), it is a Goblin, a
/// land and a creature; and when the turn is over it is the colourless land
/// again.
#[test]
fn den_of_the_bugbear_is_a_red_goblin_creature_land_until_end_of_turn() {
    let (mut engine, den) = the_den_and(4);
    assert_eq!(
        kit::colors_of(&engine, den),
        ColorSet::EMPTY,
        "a land is colourless before the animation"
    );
    tap_mana_except(&mut engine, kit::P0, den);

    animate(&mut engine);

    let animated = types(&engine, den);
    assert!(animated.contains(TypeSet::LAND), "it is still a land");
    assert!(animated.contains(TypeSet::CREATURE), "and now a creature");
    assert_eq!(pt(&engine, den), (3, 2));
    assert_eq!(
        kit::colors_of(&engine, den),
        ColorSet::of(Color::Red),
        "exactly red"
    );
    assert!(kit::has_subtype(&engine, den, creature::GOBLIN), "a Goblin");

    kit::walk_to_next_own_main(&mut engine, kit::P0);
    let after = types(&engine, den);
    assert!(
        !after.contains(TypeSet::CREATURE),
        "until end of turn: next turn it is a land again"
    );
    assert!(after.contains(TypeSet::LAND));
    assert_eq!(
        kit::colors_of(&engine, den),
        ColorSet::EMPTY,
        "and colourless again"
    );
}

/// "Whenever this creature attacks, create a 1/1 red Goblin creature token
/// that's tapped and attacking."
///
/// The animated Den attacks player 1. The token it makes is a 1/1 red Goblin,
/// tapped, attacking player 1 (CR 508.4), and deals combat damage with the
/// Den: 3 + 1. Righteous Cause gains one life, not two: the token was put onto
/// the battlefield attacking and never "attacked" (CR 508.3a).
#[test]
fn den_of_the_bugbear_attacks_with_a_tapped_attacking_goblin_token() {
    let (mut engine, den) = the_den_and(4);
    let (mine, theirs) = (kit::life(&engine, kit::P0), kit::life(&engine, kit::P1));
    tap_mana_except(&mut engine, kit::P0, den);
    animate(&mut engine);

    kit::declare_attack(&mut engine, &[den], kit::P1);
    kit::resolve_the_stack(&mut engine, |_, _| None);

    let tokens = tokens_of(&engine, kit::P0);
    assert_eq!(tokens.len(), 1, "one token: {tokens:?}");
    let token = tokens[0];
    assert!(is_tapped(&engine, token), "it enters tapped");
    assert!(engine.state().combat.is_attacking(token), "and attacking");
    assert_eq!(
        engine.state().combat.defender_of(token),
        Some(Defender::Player(kit::P1)),
        "the only opponent"
    );
    assert_eq!(pt(&engine, token), (1, 1));
    assert_eq!(kit::colors_of(&engine, token), ColorSet::of(Color::Red));
    assert!(kit::has_subtype(&engine, token, creature::GOBLIN));
    assert_eq!(
        kit::life(&engine, kit::P0),
        mine + 1,
        "Righteous Cause heard the Den attack and not the token (CR 508.3a)"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        kit::life(&engine, kit::P1),
        theirs - 4,
        "the Den's 3 and the token's 1"
    );
}

/// Ruling (2021-07-23): "If Den of the Bugbear's last ability is activated
/// multiple times in one turn, it will have multiple instances of the
/// triggered ability that it grants. For example, if you activate it twice and
/// attack, you will create two 1/1 red Goblin creature tokens that are tapped
/// and attacking."
#[test]
fn den_of_the_bugbear_activated_twice_makes_two_tokens() {
    let (mut engine, den) = the_den_and(8);
    let theirs = kit::life(&engine, kit::P1);
    tap_mana_except(&mut engine, kit::P0, den);
    animate(&mut engine);
    animate(&mut engine);

    kit::declare_attack(&mut engine, &[den], kit::P1);
    kit::resolve_the_stack(&mut engine, |_, _| None);

    let tokens = tokens_of(&engine, kit::P0);
    assert_eq!(tokens.len(), 2, "two instances of the trigger, two tokens");
    for token in tokens {
        assert!(is_tapped(&engine, token));
        assert!(engine.state().combat.is_attacking(token));
    }
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        kit::life(&engine, kit::P1),
        theirs - 5,
        "the Den's 3 and the two tokens' 1 each"
    );
}

/// CR 508.4: "its controller chooses which defending player … it's attacking
/// as it enters". At a table of three the Den attacks player 1 and the token
/// is sent at player 2, chosen by its controller among both opponents:
/// player 1 takes the Den's 3 and player 2 the token's 1.
#[test]
fn den_of_the_bugbear_token_attacks_the_opponent_its_controller_picks() {
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(
            0,
            &[
                den_of_the_bugbear(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let den = on_battlefield(&engine, kit::P0, den_of_the_bugbear()).expect("the Den");
    let (first, second) = (kit::life(&engine, kit::P1), kit::life(&engine, kit::P2));
    tap_mana_except(&mut engine, kit::P0, den);
    animate(&mut engine);

    kit::declare_attack(&mut engine, &[den], kit::P1);
    let mut asked = 0;
    kit::resolve_the_stack(&mut engine, |_, pending| match pending {
        Pending::ChoosePlayer { player, options } => {
            asked += 1;
            assert_eq!(*player, kit::P0, "the token's controller chooses");
            assert_eq!(
                options,
                &vec![kit::P1, kit::P2],
                "either opponent, whom the Den attacks or not"
            );
            Some(PlayerAction::ChoosePlayer(kit::P2))
        }
        _ => None,
    });
    assert_eq!(asked, 1, "asked once, for the one token");

    let token = tokens_of(&engine, kit::P0)[0];
    assert_eq!(
        engine.state().combat.defender_of(token),
        Some(Defender::Player(kit::P2)),
        "the token attacks the player its controller named"
    );
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(kit::life(&engine, kit::P1), first - 3, "the Den's 3");
    assert_eq!(kit::life(&engine, kit::P2), second - 1, "the token's 1");
}
