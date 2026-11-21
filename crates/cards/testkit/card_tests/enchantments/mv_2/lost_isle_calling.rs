//! Lost Isle Calling's scry trigger and its own exiled counters, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn lost_isle_calling_draws_and_awards_the_turn_at_seven_verse_counters() {
    let player = PlayerId::new(0);
    let calling = card_index("0efbf29c-09a0-4472-83ad-8d692c0232a5");
    let opt = instants::opt();
    let mut battlefield = vec![calling];
    battlefield.extend([island(); 7]);
    for verses in [6, 7] {
        let mut engine = Duel::new(SEED, island())
            .battlefield(0, &battlefield)
            .hand(0, &[opt])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, player);
        let source = on_battlefield(&engine, player, calling).unwrap();
        engine
            .dev_state_mut(player)
            .unwrap()
            .object_mut(source)
            .unwrap()
            .counters
            .add(counters::VERSE, verses - 1);

        cast_from_hand(&mut engine, player, opt);
        pass_until(&mut engine, |engine| {
            matches!(
                engine.pending(),
                Pending::Arrange {
                    prompt: ArrangePrompt::Scry,
                    ..
                }
            )
        });
        let Pending::Arrange { cards, .. } = engine.pending().clone() else {
            panic!("expected Opt's scry, got {:?}", engine.pending());
        };
        engine.apply(player, look_answer(&cards, &[])).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine
                .state()
                .object(source)
                .unwrap()
                .counters
                .get(counters::VERSE),
            verses
        );

        let hand = ZoneLocation::Hand(player);
        let before = engine.state().zones.list(hand).len();
        activate(&mut engine, player, calling, 1);
        assert_eq!(engine.state().object(source).unwrap().zone, Zone::Exile);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().zones.list(hand).len() - before,
            usize::from(verses)
        );
        assert_eq!(
            engine.state().extra_turns.front().copied(),
            (verses == 7).then_some(player),
            "extra turn with {verses} verse counters"
        );
    }
}
