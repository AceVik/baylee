// Oracle checklist: either player casts matching spell; optional accept/decline; exact one life to source controller before spell resolves; green nonartifact cast excluded.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("ee38377f-8d3a-402e-a5d7-a01cfe3a5322")
}

#[test]
fn matching_cast_from_either_player_is_optional_and_benefits_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for caster in [p0, p1] {
        for accept in [false, true] {
            let mut game = Duel::new(SEED, forest())
                .battlefield(0, &[subject(), swamp(), swamp()])
                .battlefield(1, &[swamp(), swamp()])
                .hand(0, &[dark_ritual()])
                .hand(1, &[dark_ritual()])
                .start();
            keep_mulligans(&mut game);
            if caster == p0 {
                reach_main_phase(&mut game, p0);
            } else {
                reach_their_main_phase(&mut game, p1);
            }
            let life = [game.state().players[0].life, game.state().players[1].life];
            cast_from_hand(&mut game, caster, dark_ritual());
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
                on_stack(&game, dark_ritual()).is_some(),
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
