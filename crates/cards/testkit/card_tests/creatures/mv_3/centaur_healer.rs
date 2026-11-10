// Oracle checklist: entry gains exactly three only for entering creature controller; both seats; trigger resolves after entry; death gives no additional life.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("ff435f84-73ab-4a16-8af6-6576af4e4edb")
}

#[test]
fn entry_gains_three_for_each_controller_and_death_does_not_repeat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for caster in [p0, p1] {
        let mut game = Duel::new(SEED, forest())
            .battlefield(0, &[forest(), plains(), plains()])
            .battlefield(1, &[forest(), plains(), plains()])
            .hand(0, &[subject()])
            .hand(1, &[subject()])
            .start();
        keep_mulligans(&mut game);
        if caster == p0 {
            reach_main_phase(&mut game, p0);
        } else {
            reach_their_main_phase(&mut game, p1);
        }
        let life = [game.state().players[0].life, game.state().players[1].life];
        cast_from_hand(&mut game, caster, subject());
        pass_until(&mut game, |e| {
            on_battlefield(e, caster, subject()).is_some()
        });
        assert_eq!(
            [game.state().players[0].life, game.state().players[1].life],
            life
        );
        pass_until(&mut game, stack_is_empty);
        let expected = [
            life[0] + if caster == p0 { 3 } else { 0 },
            life[1] + if caster == p1 { 3 } else { 0 },
        ];
        assert_eq!(
            [game.state().players[0].life, game.state().players[1].life],
            expected
        );
        let source = on_battlefield(&game, caster, subject()).unwrap();
        assert_eq!(pt(&game, source), (3, 3));
        kill(&mut game, source);
        assert_eq!(
            [game.state().players[0].life, game.state().players[1].life],
            expected
        );
    }
}
