//! Assigning no combat damage overrides CR 510.1a for blockers too. It is
//! neither damage prevention nor a restriction on noncombat damage.

use super::*;
use baylee_cards_dsl::{Amount, Duration, Filter, Modifier, PlayerRel, TargetSpec};
use baylee_core::ids::Defender;

const PACIFIST: u32 = 7390;
const RAIDER: u32 = 7391;

static ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::continuous(
            &Filter::This,
            Modifier::AssignsNoCombatDamage,
            Duration::UntilEndOfTurn,
        )],
        None,
    ),
    free(
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::Player(PlayerRel::EachOpponent),
        }],
        None,
    ),
];

fn fixture(keywords: KeywordSet) -> (Bench, ObjectId, ObjectId) {
    let mut engine = bench(
        7390,
        vec![
            card(
                PACIFIST,
                creature_face("Pacifist", "{0}", 3, 12),
                keywords,
                ABILITIES,
            ),
            card(
                RAIDER,
                creature_face("Raider", "{0}", 1, 12),
                KeywordSet::EMPTY,
                &[],
            ),
        ],
        [Seat::with(&[RAIDER]), Seat::with(&[PACIFIST])],
    );
    let pacifist = the(&engine, ZoneLocation::Battlefield, PACIFIST);
    let raider = the(&engine, ZoneLocation::Battlefield, RAIDER);
    to_main(&mut engine, me());
    (engine, pacifist, raider)
}

fn fight(engine: &mut Bench, raider: ObjectId, blocker: ObjectId) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(raider, Defender::Player(them()))],
            },
        )
        .unwrap();
    walk_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, raider)],
            },
        )
        .unwrap();
    walk_until(engine, |e| e.state().turn.step == Step::CombatEnd);
}

#[test]
fn blocker_assigns_nothing_in_either_strike_step_and_resumes_next_turn() {
    for (keywords, later_damage) in [
        (KeywordSet::EMPTY, 3),
        (KeywordSet::FIRST_STRIKE, 3),
        (KeywordSet::DOUBLE_STRIKE, 6),
    ] {
        let (mut engine, blocker, raider) = fixture(keywords);
        walk_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == them()),
        );
        activate(&mut engine, them(), blocker, 0);
        settle(&mut engine, them());
        // Noncombat damage still happens while the restriction applies.
        activate(&mut engine, them(), blocker, 1);
        settle(&mut engine, them());
        assert_eq!(life(&engine, me()), 19);
        fight(&mut engine, raider, blocker);
        assert_eq!(engine.state().object(raider).unwrap().damage, 0);
        assert_eq!(engine.state().object(blocker).unwrap().damage, 1);

        // Cleanup ends the restriction. The same creature deals its normal
        // damage when it blocks again, including both double-strike steps.
        to_main(&mut engine, them());
        to_main(&mut engine, me());
        fight(&mut engine, raider, blocker);
        assert_eq!(engine.state().object(raider).unwrap().damage, later_damage);
        assert_eq!(engine.state().object(blocker).unwrap().damage, 1);
    }
}
