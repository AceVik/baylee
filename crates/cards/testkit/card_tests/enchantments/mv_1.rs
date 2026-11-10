//! The tests of the cards under `cards/enchantments/mv_1/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// A duel whose seat 0 holds `hand` and `mine`, seat 1 holds `theirs`,
/// at seat 0's first main phase with both pools full of mana.
fn table(
    seed: u64,
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(seed, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    fill_pool(&mut e);
    e
}

/// Fills both pools, so a test about an ability's effect does not depend on
/// the lands the table happens to hold.
fn fill_pool(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

/// Answers every question until priority comes back: targets from
/// `objects`/`players`, card choices from `picks` (else the first the menu
/// allows), yes/no with `yes`.
#[track_caller]
fn drive(
    e: &mut Engine<RegistryLookup>,
    objects: &[ObjectId],
    players: &[PlayerId],
    picks: &[ObjectId],
    yes: bool,
) {
    for _ in 0..16 {
        let action = match e.pending().clone() {
            Pending::Priority { .. } => return,
            Pending::ChooseTargets { player, .. } => (
                player,
                PlayerAction::ChooseTargets {
                    objects: objects.to_vec(),
                    players: players.to_vec(),
                },
            ),
            Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } => {
                let mut chosen: Vec<ObjectId> = picks
                    .iter()
                    .copied()
                    .filter(|o| options.contains(o))
                    .collect();
                if chosen.is_empty() {
                    chosen = options
                        .iter()
                        .copied()
                        .take(usize::from(min).max(1))
                        .collect();
                }
                (player, PlayerAction::ChooseObjects { objects: chosen })
            }
            Pending::YesNo { player, .. } => (player, PlayerAction::YesNo(yes)),
            other => panic!("unexpected question {other:?}"),
        };
        e.apply(action.0, action.1).expect("the answer is legal");
    }
    panic!("still asking: {:?}", e.pending());
}

fn hand_size(e: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    e.state().zones.list(ZoneLocation::Hand(seat)).len()
}

fn life(e: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    e.state().players[seat.get() as usize].life
}

mod ajani_s_welcome;
mod barrage_of_expendables;
mod burgeoning;
mod concordant_crossroads;
mod darkest_hour;
mod dread_of_night;
mod eladamri_s_vineyard;
mod elephant_grass;
mod elven_fortress;
mod elven_palisade;
mod exploration;
mod fastbond;
mod fasting;
mod field_of_dreams;
mod flame_blitz;
mod flood;
mod font_of_fertility;
mod gerrard_s_battle_cry;
mod graf_harvest;
mod hidden_gibbons;
mod hidden_guerrillas;
mod hidden_predators;
mod hidden_spider;
mod high_ground;
mod hopeless_nightmare;
mod illness_in_the_ranks;
mod inheritance;
mod land_tax;
mod legion_s_landing;
mod mass_hysteria;
mod mirri_s_guile;
mod molten_vortex;
mod mystic_remora;
mod need_for_speed;
mod omen_of_the_dead;
mod onslaught;
mod opal_caryatid;
mod phyrexian_reclamation;
mod quest_for_the_gravelord;
mod reconnaissance;
mod revelation;
mod root_maze;
mod sarcomancy;
mod seal_of_fire;
mod seal_of_removal;
mod seal_of_strength;
mod serra_s_hymn;
mod storm_front;
mod storm_world;
mod sunspring_expedition;
mod thassa_s_ire;
mod tidal_flats;
mod tortured_existence;
mod twists_and_turns;
mod vampiric_rites;
mod veil_of_birds;
mod veiled_sentry;
mod war_dance;
mod weaponize_the_monsters;
