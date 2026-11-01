//! `cards/creatures/mv_5/sawback_manticore.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
use crate::engine::card_tests::partials_w2c_kit::{damage_on, hill_giant};

fn sawback_manticore() -> CardIndex {
    card_index("9360d30c-4556-4b41-bd50-08b4367e5efb")
}

/// Sawback Manticore (`{3}{R}{G}`, `Coverage::Implemented`): `{4}` grants
/// flying until end of turn, and `{1}` deals 2 damage to target attacking or
/// blocking creature only while Sawback Manticore is attacking or blocking and
/// only once each turn.
#[test]
fn sawback_manticore_flying_and_once_per_turn_combat_damage_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                sawback_manticore(),
                mountain(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let manticore = on_battlefield(&engine, p0, sawback_manticore()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let my_lands = lands_of(&engine, p0);

    // Outside combat, ability 1 ({1} deal 2 damage) is not offered.
    tap_mana_where(&mut engine, p0, |id| my_lands[..4].contains(&id));
    assert!(
        !priority_offer(&engine).abilities.contains(&(manticore, 1)),
        "ability 1 cannot be activated outside combat"
    );

    // Ability 0 ({4}) grants flying until end of turn.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: manticore,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, manticore).contains(KeywordSet::FLYING));

    // Advance to p1's turn and block Hill Giant with Sawback Manticore.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(manticore, giant)],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    assert!(
        priority_offer(&engine).abilities.contains(&(manticore, 1)),
        "ability 1 is offered while blocking"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: manticore,
                ability_index: 1,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(damage_on(&engine, giant), 2);
    assert!(
        !priority_offer(&engine).abilities.contains(&(manticore, 1)),
        "once-per-turn limit prevents a second activation"
    );
}

/// "{4}: gains flying until end of turn": the keyword is gone in the
/// opponent's turn.
#[test]
fn sawback_manticore_flying_ends_with_the_turn() {
    let mut engine = lk::game(SEED, &[sawback_manticore()], &[], &[], &[]);
    let manticore = lk::obj(&engine, P0, sawback_manticore());

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 4)]);
    lk::use_ability(&mut engine, P0, sawback_manticore(), 0, &[], &[]);
    assert!(keywords(&engine, manticore).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, P1);
    assert!(!keywords(&engine, manticore).contains(KeywordSet::FLYING));
}

/// "Activate only if this creature is attacking or blocking": while another
/// creature does the blocking, the Manticore's own ping is not offered.
#[test]
fn sawback_manticore_cannot_ping_while_another_creature_blocks() {
    let mut engine = lk::game(
        SEED,
        &[sawback_manticore(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    reach_their_main_phase(&mut engine, P1);
    lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[(bears, giant)]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 1)]);

    assert!(
        !lk::offered(&engine, sawback_manticore(), P0, 1),
        "the Manticore itself is not blocking"
    );
}
