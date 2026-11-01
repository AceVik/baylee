//! The tests of the cards under `cards/enchantments/mv_6/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod a_realm_reborn;
mod aysen_highway;
mod collective_blessing;
mod dragon_roost;
mod dragonback_assault;
mod fable_of_wolf_and_owl;
mod forced_fruition;
mod hidden_path;
mod mind_over_matter;
mod primeval_bounty;
mod rage_reflection;
mod rite_of_the_dragoncaller;
mod ritual_of_subdual;
mod storm_of_saruman;
mod territorial_dispute;
mod true_conviction;

/// Declares `attackers` for `seat` against `at` and stops once the
/// declaration has been accepted.
#[track_caller]
fn declare_attack(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    attackers: &[ObjectId],
    at: PlayerId,
) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(at)))
                    .collect(),
            },
        )
        .expect("the attack was on the offer");
}

/// How many cards `seat` holds.
fn hand_len(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Nobody blocks; the game runs on to the second main phase.
#[track_caller]
fn finish_combat(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

/// After the attack was declared: the blocks the defender is offered, or
/// `None` when combat went by without asking (nothing could block).
#[track_caller]
fn blocks_offered(engine: &mut Engine<RegistryLookup>) -> Option<Vec<crate::choice::BlockOption>> {
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => return Some(blockers),
            Pending::Priority { player, .. } => {
                if matches!(
                    engine.state().turn.step,
                    Step::CombatDamageFirst | Step::CombatDamage | Step::CombatEnd
                ) || engine.state().turn.phase == Phase::SecondMain
                {
                    return None;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to the blockers: {other:?}"),
        }
    }
    panic!("never reached blockers or damage")
}

/// Whether `blocker` is offered the block of `attacker`.
fn may_block(
    offer: Option<&Vec<crate::choice::BlockOption>>,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    offer.is_some_and(|o| {
        o.iter()
            .any(|b| b.blocker == blocker && b.attackers.contains(&attacker))
    })
}

/// Whether seat 1's Gray Ogre is offered the block of seat 0's `attacker`,
/// with `p0_board` standing beside the attacker and `p1_land` under the Ogre.
#[track_caller]
fn ogre_may_block(p0_board: &[CardIndex], attacker: CardIndex, p1_land: CardIndex) -> bool {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = p0_board.to_vec();
    board.push(attacker);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[p1_land, gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker_id = on_battlefield(&engine, p0, attacker).expect("the attacker");
    let blocker = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    declare_attack(&mut engine, p0, &[attacker_id], p1);
    let offer = blocks_offered(&mut engine);
    may_block(offer.as_ref(), blocker, attacker_id)
}

mod earthlink;
mod nature_s_wrath;
