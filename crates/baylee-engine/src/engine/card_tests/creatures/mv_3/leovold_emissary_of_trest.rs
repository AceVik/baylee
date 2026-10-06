//! `cards/creatures/mv_3/leovold_emissary_of_trest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Leovold, Emissary of Trest: "Whenever you or a permanent you control
/// becomes the target of a spell or ability an opponent controls, you may
/// draw a card."
///
/// Three pieces of an opponent's work, one board. A Mesa tapping two of
/// Leovold's lands fires it twice (the ruling: one trigger per target, not
/// per ability); a second Mesa tapping one of his lands and one of its own
/// controller's fires it once, because the opponent's land is not "a
/// permanent you control"; and a Lightning Bolt at Leovold's controller
/// fires it once more, through the player half of the sentence. Four cards,
/// each asked for with "you may". A trigger reading `BecomesTarget` would
/// never fire at all (it watches Leovold alone), and one counting abilities
/// would draw three.
#[test]
fn leovold_draws_once_for_each_of_yours_an_opponent_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(385, island())
        .battlefield(0, &[leovold_emissary_of_trest(), forest(), forest()])
        .battlefield(
            1,
            &[
                wintermoon_mesa(),
                wintermoon_mesa(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let forests = all_of(&engine, p0, forest());
    let their_mountain = on_battlefield(&engine, p1, mountain()).expect("a Mountain");
    let mesas = all_of(&engine, p1, wintermoon_mesa());
    let before = hand_size(&engine, p0);
    tap_mana_where(&mut engine, p1, |id| !mesas.contains(&id));

    mesa_at(&mut engine, p1, [forests[0], forests[1]]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, p0),
        before + 2,
        "two of Leovold's lands targeted by one ability: two triggers, two cards"
    );

    mesa_at(&mut engine, p1, [forests[0], their_mountain]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, p0),
        before + 3,
        "the opponent's own Mountain is not a permanent Leovold's controller controls"
    );

    let bolt = in_hand(&engine, p1, lightning_bolt()).expect("the Bolt is in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .expect("the Bolt is castable off the floating red");
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("Leovold's controller is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, p0),
        before + 4,
        "\"you\" becoming the target is the other half of the sentence"
    );
}

/// Leovold's own spell at his own controller is nobody's opponent's, and
/// "you may" means no: a declined trigger draws nothing.
#[test]
fn leovold_ignores_his_own_targeting_and_may_be_declined() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(386, island())
        .battlefield(0, &[leovold_emissary_of_trest(), mountain()])
        .hand(0, &[lightning_bolt()])
        .hand(1, &[lightning_bolt()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let leovold = on_battlefield(&engine, p0, leovold_emissary_of_trest()).expect("Leovold");

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("a player may aim at themselves");
    let after_cast = hand_size(&engine, p0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, p0),
        after_cast,
        "his own Bolt fired nothing"
    );

    reach_their_main_phase(&mut engine, p1);
    let before = hand_size(&engine, p0);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![leovold],
                players: vec![],
            },
        )
        .expect("Leovold is a legal target");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the predicate above matched a question")
    };
    assert_eq!(player, p0, "Leovold's controller is asked");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, p0),
        before,
        "declined, so nothing was drawn"
    );
}

/// Leovold's first line: each **opponent** can't draw more than one card each
/// turn, and his controller can. The opponent draws in their draw step, so a
/// "draw two cards" in their main phase draws nothing at all.
#[test]
fn leovold_stops_an_opponents_second_draw_and_not_his_controllers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(387, island())
        .battlefield(0, &[leovold_emissary_of_trest()])
        .battlefield(1, &[island(), island(), island()])
        .hand(1, &[counsel_of_the_soratami()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().draw_limit(p1),
        Some(1),
        "the opponent is limited"
    );
    assert_eq!(
        engine.state().draw_limit(p0),
        None,
        "Leovold's controller is not"
    );
    assert_eq!(engine.state().per_turn.draws[1], 1, "the draw step's card");

    let before = hand_size(&engine, p1);
    cast_from_hand(&mut engine, p1, counsel_of_the_soratami());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, p1),
        before - 1,
        "the spell left the hand and neither of its cards arrived"
    );
}
