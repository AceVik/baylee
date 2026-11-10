//! The tests of the cards under `cards/creatures/artifacts/mv_1/`, one module per card.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

mod augmenting_automaton;
mod chronomaton;
mod clockwork_beetle;
mod clockwork_drawbridge;
mod cogwork_wrestler;
mod emergency_medical_hologram;
mod esper_sentinel;
mod excavated_wall;
mod gene_pollinator;
mod goldhound;
mod haywire_mite;
mod hullcarver;
mod jawbone_skulkin;
mod lightning_core_excavator;
mod locthwain_gargoyle;
mod metallic_sliver;
mod monoist_sentry;
mod myr_mindservant;
mod myr_moonvessel;
mod myr_scrapling;
mod network_disruptor;
mod phyrexian_dreadnought;
mod rabbit_battery;
mod roterothopter;
mod sarpadian_simulacrum;
mod silver_raven;
mod soldevi_sentry;
mod sparring_construct;
mod sphinx_s_herald;
mod steel_wall;
mod straw_golem;
mod swooping_lookout;
mod universal_automaton;

/// Floats `amount` mana of `color` in seat 0's pool, as a card's own
/// mana would, and refreshes the offer so abilities weigh it.
fn float_mana(engine: &mut Engine<RegistryLookup>, color: ManaColor, amount: u32) {
    engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up")
        .players[0]
        .mana_pool
        .add(color, amount);
    engine.refresh_offer();
}

/// Whether printed ability `index` of `source` is on the current offer.
fn is_offered(engine: &Engine<RegistryLookup>, source: ObjectId, index: u32) -> bool {
    matches!(
        engine.pending(),
        Pending::Priority { legal, .. } if legal.abilities.contains(&(source, index))
    )
}

/// Seat 0's first main phase with `mine` and seat 1's `theirs` on the
/// battlefield and nothing in either hand.
fn main_with(mine: &[CardIndex], theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    engine
}

/// Seat 0 attacks seat 1 with `attacker`; seat 1 blocks with `blocker` when
/// given. Stops in the second main phase with an empty stack.
fn attack_with(engine: &mut Engine<RegistryLookup>, attacker: ObjectId, blocker: Option<ObjectId>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .expect("the attack is legal");
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: blocker.map(|b| (b, attacker)).into_iter().collect(),
            },
        )
        .expect("the block is legal");
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
}

/// Passes to the declare-attackers question and returns who may attack.
fn attackers_offered(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    let p0 = PlayerId::new(0);
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on ChooseAttackers");
    };
    attackers
}
