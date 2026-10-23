//! `cards/creatures/mv_3/zhalfirin_knight.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn zhalfirin_knight() -> CardIndex {
    card_index("a534f06e-dae4-4d1a-96e9-7693bbb21e12")
}

/// Zhalfirin Knight (`{2}{W}`, `Coverage::Implemented`): 2/2 Human Knight with
/// flanking and `{W}{W}: This creature gains first strike until end of turn.`
#[test]
fn zhalfirin_knight_gains_first_strike_and_flanks_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[zhalfirin_knight(), plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let knight = on_battlefield(&engine, p0, zhalfirin_knight()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(keywords(&engine, knight).contains(KeywordSet::FLANKING));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: knight,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, knight).contains(KeywordSet::FIRST_STRIKE));

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
    pass_until(&mut engine, stack_is_empty);

    // Grizzly Bears gets -1/-1 from flanking (1/1).
    assert_eq!(pt(&engine, bear), (1, 1));
    // First strike: the 1/1 Bears dies before it can deal its damage.
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(
        engine.state().object(knight).unwrap().damage,
        0,
        "the Knight struck first and took nothing"
    );
    // Until end of turn.
    reach_their_main_phase(&mut engine, p1);
    assert!(!keywords(&engine, knight).contains(KeywordSet::FIRST_STRIKE));
}
