// Oracle checklist: either player casts matching spell; optional accept/decline; exact one life to source controller before spell resolves; green nonartifact cast excluded.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("650890bc-aba6-45fb-8096-3a36b0a015b3")
}

#[test]
fn matching_cast_from_either_player_is_optional_and_benefits_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for caster in [p0, p1] {
        for accept in [false, true] {
            let mut game = Duel::new(SEED, forest())
                .battlefield(0, &[subject(), forest(), forest()])
                .battlefield(1, &[forest(), forest()])
                .hand(0, &[sol_ring()])
                .hand(1, &[sol_ring()])
                .start();
            keep_mulligans(&mut game);
            if caster == p0 {
                reach_main_phase(&mut game, p0);
            } else {
                reach_their_main_phase(&mut game, p1);
            }
            let life = [game.state().players[0].life, game.state().players[1].life];
            cast_from_hand(&mut game, caster, sol_ring());
            pass_until(&mut game, |e| {
                matches!(
                    e.pending(),
                    Pending::YesNo {
                        prompt: YesNoPrompt::MayDo,
                        ..
                    }
                )
            });
            let Pending::YesNo { player, .. } = game.pending().clone() else {
                unreachable!()
            };
            assert_eq!(player, p0);
            assert!(
                on_stack(&game, sol_ring()).is_some(),
                "trigger resolves before the spell"
            );
            assert_eq!(game.state().players[0].life, life[0]);
            game.apply(p0, PlayerAction::YesNo(accept)).unwrap();
            pass_until(&mut game, stack_is_empty);
            assert_eq!(game.state().players[0].life, life[0] + i32::from(accept));
            assert_eq!(game.state().players[1].life, life[1]);
        }
    }
}

#[test]
fn a_green_nonartifact_spell_does_not_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for caster in [p0, p1] {
        let mut game = Duel::new(SEED, forest())
            .battlefield(0, &[subject(), forest()])
            .battlefield(1, &[forest()])
            .hand(0, &[llanowar_elves()])
            .hand(1, &[llanowar_elves()])
            .start();
        keep_mulligans(&mut game);
        if caster == p0 {
            reach_main_phase(&mut game, p0);
        } else {
            reach_their_main_phase(&mut game, p1);
        }
        let life = [game.state().players[0].life, game.state().players[1].life];
        cast_from_hand(&mut game, caster, llanowar_elves());
        assert_eq!(game.state().zones.list(ZoneLocation::Stack).len(), 1);
        pass_until(&mut game, stack_is_empty);
        assert_eq!(
            [game.state().players[0].life, game.state().players[1].life],
            life
        );
        assert!(on_battlefield(&game, caster, llanowar_elves()).is_some());
    }
}
