//! `cards/lands/shock/temple_garden.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The two shocklands this pool has, on both answers to the question they
/// ask.
///
/// A classifier is untested until both branches fire, and this one is
/// written as a single `EnterModifier::TappedOrPayLife(2)` whose two outcomes
/// are decided by one `PlayerAction::YesNo`. Paying and declining are asked
/// of the same card in the same test, and the life is read on both — because
/// "enters untapped" and "costs two life" are two claims and a land that did
/// the first without the second would look right from the battlefield.
#[test]
fn a_shockland_pays_two_life_to_enter_untapped_and_costs_nothing_to_decline() {
    for (i, card) in [temple_garden(), watery_grave()].into_iter().enumerate() {
        let name = baylee_cards::by_index(card).expect("a compiled card").faces[0].name;
        for (pass, pay) in [true, false].into_iter().enumerate() {
            let p0 = PlayerId::new(0);
            let seed = 4_800 + u64::try_from(i * 2 + pass).expect("four passes");
            let mut engine = Duel::new(seed, forest()).hand(0, &[card]).start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, p0);

            let before = engine.state().players[0].life;
            let land = play_land(&mut engine, p0, card);
            let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
                panic!("{name} asks on the way in, got {:?}", engine.pending())
            };
            assert_eq!(player, p0, "the controller is asked");
            assert_eq!(
                prompt,
                crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount: 2 },
                "{name} prints two life"
            );

            engine
                .apply(p0, PlayerAction::YesNo(pay))
                .expect("both answers are legal");
            pass_until(&mut engine, stack_is_empty);

            assert_eq!(
                is_tapped(&engine, land),
                !pay,
                "{name}: paying is the whole of what keeps it untapped"
            );
            assert_eq!(
                engine.state().players[0].life,
                if pay { before - 2 } else { before },
                "{name}: the life moves with the answer and not with the land"
            );
        }
    }
}
