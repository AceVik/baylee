//! `cards/creatures/mv_3/searing_spear_askari.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn searing_spear_askari() -> CardIndex {
    card_index("d3c68573-95b1-4db5-bbea-363b86f713c1")
}

/// Searing Spear Askari (`{2}{R}`, `Coverage::Implemented`): 2/2 Human Knight
/// with flanking and `{1}{R}` to gain menace until end of turn.
#[test]
fn searing_spear_askari_flanking_and_menace_activation() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[searing_spear_askari(), mountain(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let askari = on_battlefield(&engine, p0, searing_spear_askari()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(keywords(&engine, askari).contains(KeywordSet::FLANKING));

    // Activate {1}{R}: gains menace until end of turn.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: askari,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, askari).contains(KeywordSet::MENACE));

    // With menace, a single blocker (Grizzly Bears) cannot block Searing Spear Askari;
    // on the next turn without activating menace, Grizzly Bears blocks and gets -1/-1 from flanking.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!keywords(&engine, askari).contains(KeywordSet::MENACE));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(askari, Defender::Player(p1))],
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
                blockers: vec![(bear, askari)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (1, 1));
}

/// "{1}{R}: gains menace": a single blocker is refused, and the attack goes
/// through unblocked.
#[test]
fn searing_spear_askari_with_menace_cannot_be_blocked_by_one_creature() {
    let mut engine = lk::game(
        SEED,
        &[searing_spear_askari()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let askari = lk::obj(&engine, P0, searing_spear_askari());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
    );
    lk::use_ability(&mut engine, P0, searing_spear_askari(), 0, &[], &[]);
    lk::attack(&mut engine, &[askari]);

    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, askari)],
                },
            )
            .is_err(),
        "menace: one creature alone cannot block"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P1), 18);
}

/// "Whenever a creature without flanking blocks this creature": a blocker
/// with flanking is not shrunk.
#[test]
fn searing_spear_askari_does_not_shrink_a_blocker_with_flanking() {
    let mut engine = lk::game(
        SEED,
        &[searing_spear_askari()],
        &[index::KNIGHT_OF_VALOR],
        &[],
        &[],
    );
    let askari = lk::obj(&engine, P0, searing_spear_askari());
    let knight = lk::obj(&engine, P1, index::KNIGHT_OF_VALOR);

    let offer = lk::attack(&mut engine, &[askari]);
    assert!(lk::may_block(&offer, knight, askari));
    let before = pt(&engine, knight);
    lk::block(&mut engine, &[(knight, askari)]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, knight),
        before,
        "a flanking blocker gets no -1/-1"
    );
}
