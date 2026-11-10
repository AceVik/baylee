//! `cards/creatures/mv_2/voice_of_resurgence.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Whenever an opponent casts a spell during your turn and when this
/// creature dies, create a green and white Elemental creature token with
/// 'This token's power and toughness are each equal to the number of
/// creatures you control.'" An opponent's Giant Growth on Voice's turn makes
/// one Elemental (2/2 beside Voice); the same spell on the opponent's own
/// turn makes none; Voice dying makes a second, and each Elemental is then a
/// 2/2 counting the two of them.
#[test]
fn voice_of_resurgence_answers_a_spell_on_its_turn_and_its_own_death() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[voice_of_resurgence()])
        .battlefield(1, &[forest(), forest(), llanowar_elves()])
        .hand(1, &[giant_growth(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    // Voice's turn: its controller passes, the opponent answers with a spell.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, giant_growth());
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let made = tokens_of(&engine, p0);
    assert_eq!(made.len(), 1, "one spell during your turn, one Elemental");
    assert_eq!(pt(&engine, made[0]), (2, 2), "Voice and the token itself");

    // The opponent's own turn: the same spell makes nothing.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, giant_growth());
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(tokens_of(&engine, p0).len(), 1, "not during your turn");

    // "and when this creature dies".
    let voice = on_battlefield(&engine, p0, voice_of_resurgence()).unwrap();
    kill(&mut engine, voice);
    let made = tokens_of(&engine, p0);
    assert_eq!(made.len(), 2, "its death makes the second");
    for token in made {
        assert_eq!(pt(&engine, token), (2, 2), "two creatures now, Voice gone");
    }
}
