// Oracle checklist: own enchantment cast draws one before spell resolves; opposing enchantment and own creature casts do not; exact hand/library deltas and recipient.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("aa321138-b1a7-4b8e-a2ca-b9ce65704e92")
}

#[test]
fn only_own_enchantment_cast_draws_and_it_draws_before_resolution() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (caster, card, draws) in [
        (p0, knighthood(), true),
        (p1, knighthood(), false),
        (p0, llanowar_elves(), false),
    ] {
        let mut game = Duel::new(SEED, forest())
            .battlefield(0, &[subject(), plains(), plains(), forest()])
            .battlefield(1, &[plains(), plains(), forest()])
            .hand(0, &[card])
            .hand(1, &[card])
            .start();
        keep_mulligans(&mut game);
        if caster == p0 {
            reach_main_phase(&mut game, p0);
        } else {
            reach_their_main_phase(&mut game, p1);
        }
        let hand = game.state().zones.list(ZoneLocation::Hand(p0)).len();
        let library = game.state().zones.list(ZoneLocation::Library(p0)).len();
        let theirs = game.state().zones.list(ZoneLocation::Library(p1)).len();
        cast_from_hand(&mut game, caster, card);
        if draws {
            pass_until(&mut game, |e| {
                e.state().zones.list(ZoneLocation::Library(p0)).len() < library
            });
            assert!(on_stack(&game, card).is_some());
        }
        pass_until(&mut game, stack_is_empty);
        assert_eq!(
            game.state().zones.list(ZoneLocation::Library(p0)).len(),
            library - usize::from(draws)
        );
        assert_eq!(
            game.state().zones.list(ZoneLocation::Hand(p0)).len(),
            hand - usize::from(caster == p0) + usize::from(draws)
        );
        assert_eq!(
            game.state().zones.list(ZoneLocation::Library(p1)).len(),
            theirs
        );
    }
}
