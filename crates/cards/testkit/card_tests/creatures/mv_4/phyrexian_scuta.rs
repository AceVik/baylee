//! `cards/creatures/mv_4/phyrexian_scuta.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn phyrexian_scuta() -> CardIndex {
    card_index("af2845cf-8770-4efa-8a83-bf9eb4122b78")
}

fn answer_kicker(engine: &mut Engine<RegistryLookup>, kicked: bool) {
    match engine.pending().clone() {
        Pending::YesNo {
            prompt: YesNoPrompt::Kicker,
            ..
        } => {
            engine.apply(P0, PlayerAction::YesNo(kicked)).unwrap();
        }
        Pending::ChooseCastMode { options, .. } => {
            let want = if kicked {
                CastModeKind::Kicked
            } else {
                CastModeKind::Normal
            };
            let idx = options.iter().find(|o| o.kind == want).unwrap().index;
            engine
                .apply(P0, PlayerAction::ChooseMode(usize::try_from(idx).unwrap()))
                .unwrap();
        }
        _ => assert!(
            !kicked,
            "expected kicker prompt, got {:?}",
            engine.pending()
        ),
    }
}

/// Phyrexian Scuta (af2845cf-8770-4efa-8a83-bf9eb4122b78) — `{3}{B}` 3/3
/// Creature — Phyrexian Zombie:
/// - Kicker—Pay 3 life.
/// - If this creature was kicked, it enters with two +1/+1 counters on it.
#[test]
fn phyrexian_scuta_kicked_pays_three_life_for_two_counters_and_unkicked_is_three_three() {
    let mut kicked_game = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[phyrexian_scuta()])
        .start();
    keep_mulligans(&mut kicked_game);
    reach_main_phase(&mut kicked_game, P0);

    cast_from_hand(&mut kicked_game, P0, phyrexian_scuta());
    answer_kicker(&mut kicked_game, true);
    pass_until(&mut kicked_game, stack_is_empty);

    let scuta = on_battlefield(&kicked_game, P0, phyrexian_scuta()).expect("Scuta entered");
    assert_eq!(
        kicked_game.state().players[0].life,
        17,
        "paid 3 life for kicker"
    );
    assert_eq!(
        kicked_game
            .state()
            .object(scuta)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert_eq!(pt(&kicked_game, scuta), (5, 5));

    // Unkicked cast pays 0 life and enters as 3/3; at 2 life kicker cannot be paid (CR 119.4).
    let mut low_life = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[phyrexian_scuta()])
        .life(0, 2)
        .start();
    keep_mulligans(&mut low_life);
    reach_main_phase(&mut low_life, P0);

    cast_from_hand(&mut low_life, P0, phyrexian_scuta());
    answer_kicker(&mut low_life, false);
    pass_until(&mut low_life, stack_is_empty);

    let unkicked = on_battlefield(&low_life, P0, phyrexian_scuta()).unwrap();
    assert_eq!(low_life.state().players[0].life, 2);
    assert_eq!(pt(&low_life, unkicked), (3, 3));
}
