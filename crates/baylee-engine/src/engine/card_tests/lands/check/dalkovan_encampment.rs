//! `cards/lands/check/dalkovan_encampment.rs`, played.
//!
//! Dalkovan Encampment: "This land enters tapped unless you control a Swamp
//! or a Mountain. / {T}: Add {W}. / {2}{W}, {T}: Whenever you attack this
//! turn, create two 1/1 red Warrior creature tokens that are tapped and
//! attacking. Sacrifice them at the beginning of the next end step."
//!
//! The attack ability is the ability after the mana ability. It is found by
//! what the Encampment is offered, so that a test does not guess an index.
//! The attacker is a Gray Ogre (no mana ability, so tapping the lands for
//! the cost leaves it untapped), and a Righteous Cause is the bystander that
//! hears "a creature attacks" and so counts what was declared (CR 508.3a).
//! The game has no card that gives an extra combat, so the ruling's second
//! combat is not played here.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature;

/// Player 0 in the first main phase with the Encampment, Plains, a Swamp, a
/// Mountain, a Gray Ogre and a Righteous Cause on the battlefield (so the
/// Encampment came in untapped on its own) and `extra` besides. Returns the
/// engine, the Encampment and the Ogre.
#[track_caller]
fn the_encampment(
    extra: &[CardIndex],
    hand: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut board = vec![
        dalkovan_encampment(),
        plains(),
        swamp(),
        mountain(),
        gray_ogre(),
        righteous_cause(),
    ];
    board.extend_from_slice(extra);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let encampment =
        on_battlefield(&engine, kit::P0, dalkovan_encampment()).expect("the Encampment");
    let ogre = on_battlefield(&engine, kit::P0, gray_ogre()).expect("the Ogre");
    (engine, encampment, ogre)
}

/// Activates the Encampment's attack ability, `{2}{W}, {T}`, off mana that is
/// already floating, and lets it resolve. The ability is the last one offered,
/// after the mana ability.
#[track_caller]
fn arm(engine: &mut Engine<RegistryLookup>, encampment: ObjectId) {
    let offered = kit::offered(engine, encampment);
    let attack = offered.iter().copied().max().expect("something is offered");
    assert!(
        attack >= 1,
        "an ability beyond the mana ability is offered: {offered:?}"
    );
    kit::activate_object(engine, kit::P0, encampment, attack);
    pass_until(engine, stack_is_empty);
}

/// The Warrior tokens player 0 controls.
fn warriors(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    tokens_of(engine, kit::P0)
}

/// Dalkovan Encampment: "This land enters tapped unless you control a Swamp or
/// a Mountain." / "{T}: Add {W}."
///
/// Controlling a Swamp allows Dalkovan Encampment to enter untapped and
/// immediately tap for white mana. (The first test of the card, kept as it
/// was: only its comment, which named the unfinished card, is rewritten.)
#[test]
fn dalkovan_encampment_enters_untapped_with_swamp_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(304, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[dalkovan_encampment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, dalkovan_encampment());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, dalkovan_encampment(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

/// "This land enters tapped unless you control a Swamp or a Mountain."
///
/// Untapped for a Swamp, for a Mountain and for a nonbasic land that has both
/// types (Badlands), tapped for no lands, for a Forest and for a Plains with
/// an Island: the tapped cases are what make the untapped ones count.
#[test]
fn dalkovan_encampment_enters_tapped_unless_you_control_a_swamp_or_a_mountain() {
    let cases: [(&str, Vec<CardIndex>, bool); 6] = [
        ("a Swamp", vec![swamp()], false),
        ("a Mountain", vec![mountain()], false),
        ("a Badlands (Swamp Mountain)", vec![badlands()], false),
        ("no land", vec![], true),
        ("a Forest", vec![forest()], true),
        ("a Plains and an Island", vec![plains(), island()], true),
    ];
    for (name, board, tapped) in cases {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .hand(0, &[dalkovan_encampment()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, kit::P0);

        let land = play_land(&mut engine, kit::P0, dalkovan_encampment());
        pass_until(&mut engine, stack_is_empty);

        assert_eq!(
            entered_tapped(&engine, land),
            tapped,
            "controlling {name}, the Encampment enters {}",
            if tapped { "tapped" } else { "untapped" }
        );
    }
}

/// "{2}{W}, {T}: Whenever you attack this turn, create two 1/1 red Warrior
/// creature tokens that are tapped and attacking."
///
/// Armed in the first main phase: nothing is made until the attack. The Ogre
/// attacks player 1 and two Warriors arrive, each a tapped, attacking 1/1 red
/// Warrior (CR 508.4), and they deal combat damage with the Ogre. The
/// Righteous Cause gained one life and not three: the tokens never "attacked"
/// (CR 508.3a).
#[test]
fn dalkovan_encampment_makes_two_red_warriors_when_you_attack() {
    let (mut engine, encampment, ogre) = the_encampment(&[], &[]);
    let (mine, theirs) = (kit::life(&engine, kit::P0), kit::life(&engine, kit::P1));
    tap_mana_except(&mut engine, kit::P0, encampment);
    arm(&mut engine, encampment);
    assert!(is_tapped(&engine, encampment), "{{T}} is part of the cost");
    assert!(
        warriors(&engine).is_empty(),
        "nothing is made before anyone attacks"
    );

    kit::declare_attack(&mut engine, &[ogre], kit::P1);
    kit::resolve_the_stack(&mut engine, |_, _| None);

    let made = warriors(&engine);
    assert_eq!(made.len(), 2, "two tokens: {made:?}");
    for token in made {
        assert!(is_tapped(&engine, token), "tapped");
        assert!(engine.state().combat.is_attacking(token), "and attacking");
        assert_eq!(
            engine.state().combat.defender_of(token),
            Some(Defender::Player(kit::P1))
        );
        assert_eq!(pt(&engine, token), (1, 1));
        assert_eq!(kit::colors_of(&engine, token), ColorSet::of(Color::Red));
        assert!(kit::has_subtype(&engine, token, creature::WARRIOR));
    }
    assert_eq!(
        kit::life(&engine, kit::P0),
        mine + 1,
        "Righteous Cause heard one creature attack: the Ogre"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        kit::life(&engine, kit::P1),
        theirs - 4,
        "the Ogre's 2 and the Warriors' 1 each"
    );
}

/// "Sacrifice them at the beginning of the next end step.": the Warriors are
/// there through the second main phase and gone once the end step has begun.
#[test]
fn dalkovan_encampment_warriors_are_sacrificed_at_the_beginning_of_the_next_end_step() {
    let (mut engine, encampment, ogre) = the_encampment(&[], &[]);
    tap_mana_except(&mut engine, kit::P0, encampment);
    arm(&mut engine, encampment);
    kit::declare_attack(&mut engine, &[ogre], kit::P1);
    kit::resolve_the_stack(&mut engine, |_, _| None);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        warriors(&engine).len(),
        2,
        "still there in the second main phase"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.step, crate::turn::Step::End)
    });
    kit::resolve_the_stack(&mut engine, |_, _| None);
    assert!(
        warriors(&engine).is_empty(),
        "sacrificed at the beginning of the end step"
    );
}

/// Ruling (2025-04-04): the delayed trigger "won't trigger if you don't declare
/// any creatures as attackers." Armed and then not attacking, player 0 makes
/// no Warrior. (The test above is the twin: the same board and the same
/// arming, with the Ogre sent.)
#[test]
fn dalkovan_encampment_makes_nothing_when_no_creature_attacks() {
    let (mut engine, encampment, _) = the_encampment(&[], &[]);
    tap_mana_except(&mut engine, kit::P0, encampment);
    arm(&mut engine, encampment);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert!(
        warriors(&engine).is_empty(),
        "no attacker was declared, so no trigger"
    );
}

/// "Whenever you attack **this turn**": the watch lapses with the turn. Armed
/// on turn one and not used, the Encampment's delayed trigger does nothing on
/// turn three, when the Ogre attacks.
#[test]
fn dalkovan_encampment_does_nothing_on_a_later_turn() {
    let (mut engine, encampment, ogre) = the_encampment(&[], &[]);
    tap_mana_except(&mut engine, kit::P0, encampment);
    arm(&mut engine, encampment);

    kit::walk_to_next_own_main(&mut engine, kit::P0);
    assert_eq!(engine.state().turn.number, 3, "player 0's next turn");
    kit::declare_attack(&mut engine, &[ogre], kit::P1);
    kit::resolve_the_stack(&mut engine, |_, _| None);

    assert!(
        engine.state().combat.is_attacking(ogre),
        "the Ogre did attack"
    );
    assert!(
        warriors(&engine).is_empty(),
        "and the trigger is gone with the turn it was made in"
    );
}

/// Ruling (2025-04-04): "This ability will trigger even if Dalkovan
/// Encampment is no longer on the battlefield." Armed, then destroyed by a
/// Vindicate of player 0's own, the Encampment is in the graveyard when the
/// Ogre attacks and the two Warriors still arrive.
#[test]
fn dalkovan_encampment_still_triggers_once_it_has_left_the_battlefield() {
    let (mut engine, encampment, ogre) =
        the_encampment(&[plains(), swamp(), mountain(), mountain()], &[vindicate()]);
    // {2}{W} for the ability off one Plains and two Mountains, and {1}{W}{B}
    // for the Vindicate off the second Plains and the two Swamps: the lands
    // are tapped in two batches so that the white is not spent twice.
    let lands = lands_of(&engine, kit::P0);
    let of = |card: CardIndex, nth: usize| -> ObjectId {
        *lands
            .iter()
            .filter(|id| engine_card(&engine, **id) == Some(card))
            .nth(nth)
            .expect("that land")
    };
    let first = [of(plains(), 0), of(mountain(), 0), of(mountain(), 1)];
    let second = [of(plains(), 1), of(swamp(), 0), of(swamp(), 1)];
    tap_mana_where(&mut engine, kit::P0, |id| first.contains(&id));
    arm(&mut engine, encampment);

    tap_mana_where(&mut engine, kit::P0, |id| second.contains(&id));
    cast_with_floating(&mut engine, kit::P0, vindicate());
    aim_at(&mut engine, kit::P0, encampment);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, kit::P0, dalkovan_encampment()).is_some(),
        "the Encampment is gone"
    );

    kit::declare_attack(&mut engine, &[ogre], kit::P1);
    kit::resolve_the_stack(&mut engine, |_, _| None);
    assert_eq!(
        warriors(&engine).len(),
        2,
        "the delayed trigger does not need its source"
    );
}

/// The printing a battlefield object is.
fn engine_card(engine: &Engine<RegistryLookup>, id: ObjectId) -> Option<CardIndex> {
    engine
        .state()
        .object(id)
        .and_then(|o| o.card)
        .map(|c| c.index)
}
