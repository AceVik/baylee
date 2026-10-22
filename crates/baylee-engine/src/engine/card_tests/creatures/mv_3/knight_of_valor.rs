//! `cards/creatures/mv_3/knight_of_valor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn knight_of_valor() -> CardIndex {
    card_index("f2edb543-de33-4eed-892c-98038eec96e2")
}

/// `Knight of Valor` ({2}{W}, 2/2 Human Knight, `Coverage::Implemented`) has
/// flanking and `{1}{W}: Each creature without flanking blocking this creature
/// gets -1/-1 until end of turn. Activate only once each turn.`
#[test]
fn knight_of_valor_flanking_and_activated_ability_shrink_blocker_without_flanking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[knight_of_valor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, knight_of_valor());
    pass_until(&mut engine, stack_is_empty);
    let knight = on_battlefield(&engine, p0, knight_of_valor()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(keywords(&engine, knight).contains(KeywordSet::FLANKING));

    // Advance to P0's next turn so Knight of Valor can attack with {1}{W} up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(knight, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, knight)],
            },
        )
        .unwrap();
    // Flanking trigger resolves first (-1/-1 -> 1/1), then P0 activates {1}{W} (-1/-1 -> 0/0 -> dies).
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (1, 1));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, knight_of_valor(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Graveyard);
}
