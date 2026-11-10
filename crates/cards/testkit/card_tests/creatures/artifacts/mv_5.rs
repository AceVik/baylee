//! The tests of the cards under `cards/creatures/artifacts/mv_5/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A seat's life total.
fn life_of(engine: &Engine<RegistryLookup>, seat: usize) -> i32 {
    engine.state().players[seat].life
}

/// Declares `attacker` alone against `defender`, stopping at the declaration.
#[track_caller]
fn attack_alone(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    attacker: ObjectId,
    defender: PlayerId,
) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(defender))],
            },
        )
        .expect("the attack is legal");
}

/// Plays the rest of the combat out, nobody blocking, to the second main.
#[track_caller]
fn through_combat(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

/// Casts `card` from `seat`'s hand off every land it has and lets the spell
/// and whatever it triggered resolve; answers the permanent it became.
#[track_caller]
fn cast_resolved(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> ObjectId {
    cast_from_hand(engine, seat, card);
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, seat, card).expect("the permanent spell resolved")
}

/// Whether the permanent is, as the layers read it, an artifact creature.
fn is_artifact_creature(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    let types = engine
        .state()
        .object(id)
        .expect("on the table")
        .characteristics()
        .types;
    types.contains(TypeSet::ARTIFACT) && types.contains(TypeSet::CREATURE)
}
mod adarkar_sentinel;
mod anodet_lurker;
mod bastion_mastodon;
mod boulderborn_dragon;
mod clockwork_avian;
mod clockwork_vorrac;
mod cloudheath_drake;
mod coal_golem;
mod copper_leaf_angel;
mod crosis_s_attendant;
mod dalek_drone;
mod darigaaz_s_attendant;
mod darksteel_juggernaut;
mod dromar_s_attendant;
mod etched_monstrosity;
mod ethersworn_adjudicator;
mod gearsmith_guardian;
mod glass_golem;
mod glassdust_hulk;
mod henge_guardian;
mod igneous_golem;
mod iron_man_tony_stark;
mod karn_silver_golem;
mod lead_golem;
mod leveler;
mod mantis_engine;
mod marble_priest;
mod matopi_golem;
mod myr_prototype;
mod narstad_scrapper;
mod necropolis;
mod pewter_golem;
mod prizefighter_construct;
mod razormane_masticore;
mod rith_s_attendant;
mod saberclaw_golem;
mod scarecrow;
mod slash_panther;
mod snow_fortress;
mod soldevi_steam_beast;
mod soliton;
mod stone_golem;
mod tangle_hulk;
mod teferi_s_sentinel;
mod tek;
mod thopter_squadron;
mod thran_golem;
mod titanium_golem;
mod treva_s_attendant;
mod ultron_machine_overlord;
mod urza_s_engine;
mod vectis_agents;
mod venser_s_sliver;
