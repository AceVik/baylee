//! The tests of the cards under `cards/enchantments/mv_5/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

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

/// Plays on until `done`, passing priority, declaring no attackers or
/// blockers, and answering every other question out of what it enumerated.
#[track_caller]
fn run_until(engine: &mut Engine<RegistryLookup>, done: impl Fn(&Engine<RegistryLookup>) -> bool) {
    for _ in 0..200 {
        if done(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => {
                let (player, action) = answer_one(engine)
                    .unwrap_or_else(|rest| panic!("{other:?} could not be answered: {rest:?}"));
                engine.apply(player, action).unwrap();
            }
        }
    }
    panic!("never reached the wanted state; at {:?}", engine.pending());
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
mod asceticism;
mod berserkers_onslaught;
mod black_market;
mod blockbuster;
mod blood_rites;
mod bulwark;
mod burning_sands;
mod call_to_the_grave;
mod centaur_glade;
mod confusion_in_the_ranks;
mod curse_of_marit_lage;
mod death_pits_of_rath;
mod dictate_of_heliod;
mod dire_undercurrents;
mod discordant_dirge;
mod dispersing_orb;
mod duplicity;
mod enchanted_evening;
mod energy_vortex;
mod festering_evil;
mod gravitational_shift;
mod intimidation;
mod larceny;
mod liliana_s_mastery;
mod midsummer_revel;
mod moldervine_reclamation;
mod nature_s_revolt;
mod noble_purpose;
mod noble_stand;
mod opal_archangel;
mod open_the_graves;
mod painful_quandary;
mod polluted_bonds;
mod precognition;
mod psychic_allergy;
mod putrefaction;
mod recantation;
mod righteous_cause;
mod rumbling_crescendo;
mod safeguard;
mod screams_of_the_damned;
mod sigil_of_the_empty_throne;
mod subversion;
mod sunken_hope;
mod teferi_s_moat;
mod tornado;
mod where_ancients_tread;
mod worms_of_the_earth;
mod wrath_of_marit_lage;
mod zendikar_s_roil;
