//! The tests of the cards under `cards/creatures/artifacts/mv_4/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The top `n` cards of `seat`'s library, the topmost first.
fn library_top(engine: &Engine<RegistryLookup>, seat: PlayerId, n: usize) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .iter()
        .rev()
        .take(n)
        .copied()
        .collect()
}

/// Answers what the engine asks, the first legal way, and passes priority,
/// until somebody holds priority with an empty stack again.
#[track_caller]
fn answer_until_settled(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..60 {
        if matches!(engine.pending(), Pending::Priority { .. }) && stack_is_empty(engine) {
            return;
        }
        if let Pending::Priority { player, .. } = engine.pending().clone() {
            engine
                .apply(player, PlayerAction::PassPriority)
                .expect("passing priority is always legal");
            continue;
        }
        let (player, action) = answer_one(engine)
            .unwrap_or_else(|rest| panic!("the game stopped on a question: {rest:?}"));
        engine
            .apply(player, action)
            .expect("an answer out of the question that enumerated it");
    }
    panic!("never settled: {:?}", engine.pending());
}

/// Whether the Grizzly Bears across the table may block `attacker` when it
/// attacks them: the question flying, fear and their relatives answer.
#[track_caller]
fn bears_may_block(attacker: CardIndex) -> bool {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[attacker])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker = on_battlefield(&engine, p0, attacker).expect("seated");
    attack_and_collect_blocks(&mut engine, attacker, p1)
        .iter()
        .any(|option| option.attackers.contains(&attacker))
}

/// Seat 1's `attacker` attacks seat 0, whose `blocker` blocks it, and combat
/// is played out to the second main phase. Returns the engine, the blocker
/// and the attacker.
#[track_caller]
fn they_attack_and_we_block(
    attacker: CardIndex,
    blocker: CardIndex,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blocker])
        .battlefield(1, &[attacker])
        .start();
    keep_mulligans(&mut engine);
    let blocker_id = on_battlefield(&engine, p0, blocker).expect("seated");
    let attacker_id = on_battlefield(&engine, p1, attacker).expect("seated");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker_id, Defender::Player(p0))],
            },
        )
        .expect("the attacker is offered");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker_id, attacker_id)],
            },
        )
        .expect("the block is offered");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    (engine, blocker_id, attacker_id)
}
mod amaranthine_wall;
mod ancestral_statue;
mod anvilwrought_raptor;
mod architects_of_will;
mod badgey_malicious_glitch;
mod bladed_sentinel;
mod brass_talon_chimera;
mod clay_statue;
mod clockwork_condor;
mod clockwork_gnomes;
mod clockwork_steed;
mod clockwork_swarm;
mod cobalt_golem;
mod complex_automaton;
mod crenellated_wall;
mod crystal_golem;
mod dancing_scimitar;
mod dinotomaton;
mod dovin_s_automaton;
mod dragonloft_idol;
mod dross_ripper;
mod dross_scorpion;
mod dukhara_peafowl;
mod duskworker;
mod energizer;
mod esper_cormorants;
mod extruder;
mod filigree_crawler;
mod filigree_sages;
mod flint_golem;
mod forgotten_sentinel;
mod frogmite;
mod gilded_sentinel;
mod goblin_war_wagon;
mod golden_guardian;
mod grapeshot_catapult;
mod grid_monitor;
mod guardian_automaton;
mod hedron_rover;
mod hematite_golem;
mod iron_heart_chimera;
mod juggernaut;
mod lead_belly_chimera;
mod lodestone_myr;
mod machine_man_model_x_51;
mod masticore;
mod millennial_gargoyle;
mod mindless_automaton;
mod mobile_fort;
mod molten_tail_masticore;
mod needlebug;
mod patagia_golem;
mod peace_strider;
mod pierce_strider;
mod plasmancer;
mod rackling;
mod roaming_throne;
mod rusted_sentinel;
mod rusted_slasher;
mod rusting_golem;
mod rustspore_ram;
mod saheeli_s_silverwing;
mod sanctum_gargoyle;
mod scampering_surveyor;
mod scornful_aether_lich;
mod scrapyard_steelbreaker;
mod sentinel;
mod shell_skulkin;
mod shield_wall_sentinel;
mod signpost_scarecrow;
mod skorpekh_destroyer;
mod snare_thopter;
mod soldevi_golem;
mod soldevi_simulacrum;
mod solemn_simulacrum;
mod su_chi;
mod surgical_metamorph;
mod telethopter;
mod the_vision_and_scarlet_witch;
mod thran_war_machine;
mod tin_wing_chimera;
mod tower_gargoyle;
mod venomcrawler;
mod viseling;
mod wakandan_drone_flock;
mod walking_wall;
mod watchwing_scarecrow;
mod wirecat;
