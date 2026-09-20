//! What combat *offers*, as opposed to what it accepts.
//!
//! Attacker and blocker legality used to live only in `apply`: the choice
//! said "declare attackers" and left the asking side to work out which
//! creatures those could be. The house AI filtered the battlefield with
//! `combat::can_attack`, the Bevy client filtered its board model by
//! "untapped and not summoning sick", and the two disagreed with each other
//! and with the rules — a client cannot re-derive evasion.
//!
//! Both choices now carry the enumeration, and these are the tests that it
//! is the rules' answer and not a plausible-looking subset of it.

use super::testkit::{Duel, card_index, keep_mulligans, pass_until, reach_main_phase};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
/// 1/1 flier.
fn baleful_strix() -> CardIndex {
    card_index("37688720-03de-4eca-a82d-a0afe8d58adc")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
/// 1/2 ground creature.
fn halimar_excavator() -> CardIndex {
    card_index("fd3e37c9-93bf-4f3e-a279-22afbffd8d43")
}

/// Runs the game until `seat` is asked to declare attackers.
#[track_caller]
fn reach_attackers(engine: &mut Engine<super::testkit::RegistryLookup>) -> Pending {
    for _ in 0..80 {
        if matches!(engine.pending(), Pending::ChooseAttackers { .. }) {
            return engine.pending().clone();
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected: {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("never reached the declare-attackers step");
}

/// A creature that entered this turn cannot attack (CR 302.6), so it must
/// not be offered — the client would otherwise draw an affordance the
/// engine rejects. A turn later the same creature is offered, so the
/// enumeration tracks the rules rather than being cautious by construction.
#[test]
fn a_creature_cast_this_turn_is_not_offered_until_the_next_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(21, island())
        .battlefield(0, &[island(), swamp()])
        .hand(0, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert!(
        attackers.is_empty(),
        "the Strix entered this turn and may not attack"
    );

    // Round the table once: the next declare-attackers step that belongs to
    // seat 0 is a turn later, and by then the Strix has settled in.
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseAttackers { player, .. } if *player == p0
        )
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(
        attackers.len(),
        1,
        "one turn later the same creature may attack"
    );
}

/// Evasion is a pairing question. A ground creature is a perfectly legal
/// blocker and still may not block a flier (CR 702.9b), so the offer is
/// per attacker and not one flat list of "creatures that may block".
#[test]
fn a_ground_creature_is_not_offered_against_a_flier() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(22, island())
        .battlefield(0, &[baleful_strix()])
        .battlefield(1, &[halimar_excavator()])
        .start();
    keep_mulligans(&mut engine);

    // Seat 0 passes its sick first combat; seat 1 does the same; on seat 0's
    // second turn the Strix attacks.
    let attackers = loop {
        let Pending::ChooseAttackers {
            player, attackers, ..
        } = reach_attackers(&mut engine)
        else {
            unreachable!()
        };
        if player == p0 && !attackers.is_empty() {
            break attackers;
        }
        engine
            .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
            .unwrap();
    };
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attackers[0], baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    let blockers = loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => break blockers,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    };
    assert!(
        blockers.is_empty(),
        "a 1/2 without flying or reach was offered against a flier: {blockers:?}"
    );
}

/// An empty attack asks nobody to block (CR 508.8).
///
/// The declare blockers and combat damage steps do not happen at all on a
/// turn where nothing was declared as an attacker, and the engine used to
/// walk through them both anyway. That is not an invisible extra step: the
/// blockers step *asks a question*, so every turn where neither seat swung
/// stopped both players on "Declare blockers" over an empty board. It was
/// reported from the client as being asked to block on turn two with no
/// creature anywhere.
#[test]
fn declaring_no_attackers_skips_the_blockers_step_entirely() {
    let mut engine = Duel::new(21, island())
        .battlefield(0, &[island(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    let Pending::ChooseAttackers { player, .. } = reach_attackers(&mut engine) else {
        unreachable!()
    };
    engine
        .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    // Walk the rest of the combat phase. Nothing here may be a question
    // about blocks, and the phase has to end somewhere other than in one.
    let mut steps = Vec::new();
    for _ in 0..40 {
        steps.push(engine.state().turn.step);
        assert!(
            !matches!(engine.pending(), Pending::ChooseBlockers { .. }),
            "asked to declare blockers with nothing attacking, after {steps:?}"
        );
        if engine.state().turn.phase != Phase::Combat {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected: {:?} after {steps:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    assert!(
        !steps.contains(&Step::DeclareBlockers) && !steps.contains(&Step::CombatDamage),
        "both steps are skipped, not merely answered: {steps:?}"
    );
    assert!(
        steps.contains(&Step::CombatEnd),
        "and the phase still ends properly: {steps:?}"
    );
}

/// 3/2 whose whole printed text is menace.
fn viashino_runner() -> CardIndex {
    card_index("ac7e317b-387c-403f-bcf1-403f96302f21")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// Every creature `seat` controls on the battlefield, in zone order.
fn creatures_of(
    engine: &Engine<super::testkit::RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Attacks with seat 0's only creature and stops on the blockers question.
#[track_caller]
fn attack_and_reach_blockers(
    engine: &mut Engine<super::testkit::RegistryLookup>,
    p0: PlayerId,
    p1: PlayerId,
) -> Vec<crate::choice::BlockOption> {
    let attackers = loop {
        let Pending::ChooseAttackers {
            player, attackers, ..
        } = reach_attackers(engine)
        else {
            unreachable!()
        };
        if player == p0 && !attackers.is_empty() {
            break attackers;
        }
        engine
            .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
            .unwrap();
    };
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attackers[0], baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();
    loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => break blockers,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}

/// Menace is a restriction on the **declaration** and not on a pair
/// (CR 702.111b, checked where CR 509.1b puts it), and this is the test that
/// says so from all three sides at once: the offer names the attacker to
/// each blocker, one blocker is refused, and two are taken.
///
/// It fails against the code before #156 on its *first* assertion, and that
/// is the point. `combat::can_block` asked `state.combat.blockers_of(attacker)`
/// and answered `false` while that list was empty — which it always was,
/// because both callers ask before anything is recorded: `progress_step`
/// while it builds this offer, and `declare_blockers` in a per-pair loop that
/// runs to completion before the first `declare_block`. So the offer was
/// empty, a pair was refused along with a lone blocker, and menace read as
/// plain unblockable. The declaration-wide count had been written with the
/// rest of the rule and was unreachable by any legal answer.
#[test]
fn a_menace_attacker_takes_two_blockers_or_none() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, mountain())
        .battlefield(0, &[viashino_runner()])
        .battlefield(1, &[halimar_excavator(), halimar_excavator()])
        .start();
    keep_mulligans(&mut engine);

    let guards = creatures_of(&engine, p1, halimar_excavator());
    assert_eq!(guards.len(), 2, "two creatures are there to block with");

    let blockers = attack_and_reach_blockers(&mut engine, p0, p1);
    let runner = engine.state().combat.attackers[0].creature;
    assert_eq!(
        blockers.len(),
        2,
        "both 1/2s are offered — neither flying nor protection nor a tap is \
         in the way, and menace is not a pairing question: {blockers:?}"
    );
    assert!(
        blockers.iter().all(|o| o.attackers.contains(&runner)),
        "each of them is paired with the menace attacker, because either may \
         be one of the two CR 702.111b asks for: {blockers:?}"
    );

    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(guards[0], runner)],
                },
            )
            .is_err(),
        "\"can't be blocked except by two or more creatures\": one is not two"
    );

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(guards[0], runner), (guards[1], runner)],
            },
        )
        .expect("two is exactly what the sentence's escape clause names");
    let declared = engine.state().combat.blockers_of(runner);
    assert_eq!(
        declared.len(),
        2,
        "and both of them are recorded as blockers: {declared:?}"
    );
}

/// The half of menace that *is* answerable one attacker at a time.
///
/// A defender with one legal blocker has no legal declaration that blocks a
/// menace attacker at all, so naming that pairing in the offer would name a
/// block `declare_blockers` must refuse — and this engine's contract is that
/// a client cannot name an option the engine did not offer.
///
/// The two boards are one test because the empty half proves nothing alone:
/// before #156 the offer was empty for *every* menace attacker, so "one
/// blocker is not offered" was green for the wrong reason. The two-blocker
/// board beside it is what makes the assertion mean the rule rather than the
/// defect.
#[test]
fn a_menace_attacker_is_offered_only_where_two_could_block_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut lonely = Duel::new(32, mountain())
        .battlefield(0, &[viashino_runner()])
        .battlefield(1, &[halimar_excavator()])
        .start();
    keep_mulligans(&mut lonely);
    assert_eq!(
        creatures_of(&lonely, p1, halimar_excavator()).len(),
        1,
        "one creature, which is one short of the restriction's escape clause"
    );
    let offered = attack_and_reach_blockers(&mut lonely, p0, p1);
    assert!(
        offered.is_empty(),
        "the lone 1/2 is offered nothing: it may block, and no declaration \
         containing it is legal, so the pairing is not published: {offered:?}"
    );

    let mut paired = Duel::new(32, mountain())
        .battlefield(0, &[viashino_runner()])
        .battlefield(1, &[halimar_excavator(), halimar_excavator()])
        .start();
    keep_mulligans(&mut paired);
    let offered = attack_and_reach_blockers(&mut paired, p0, p1);
    assert_eq!(
        offered.len(),
        2,
        "the same attacker over the same board plus one creature is offered \
         to both of them — which is what makes the empty offer above a \
         statement about the count and not about menace: {offered:?}"
    );
}
