//! `cards/instants/mv_2/remedy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LIGHTNING_BOLT;

fn remedy() -> CardIndex {
    card_index("c644f190-63e5-4496-8612-bbe7be91afc9")
}

/// Two Plains and three Mountains, a Remedy and two Lightning Bolts in P0's
/// hand, at P0's first main phase.
fn board() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), mountain(), mountain(), mountain()])
        .hand(0, &[remedy(), LIGHTNING_BOLT, LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
}

/// Casts `card` from P0's hand at `target` and resolves it.
fn cast_at_player(engine: &mut Engine<RegistryLookup>, card: CardIndex, target: PlayerId) {
    let p0 = PlayerId::new(0);
    cast_from_hand(engine, p0, card);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// `Remedy` ({1}{W}, Instant, `Coverage::Partial`) prevents the next 5 damage
/// that would be dealt to its target this turn: a 3-damage Bolt is prevented
/// entirely and a second only down to what is left of the 5 (3 + 3 - 5 = 1
/// dealt).
#[test]
fn remedy_prevents_next_5_damage_to_target() {
    let p0 = PlayerId::new(0);
    let mut engine = board();

    cast_at_player(&mut engine, remedy(), p0);
    cast_at_player(&mut engine, LIGHTNING_BOLT, p0);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the first Bolt's 3 prevented"
    );

    cast_at_player(&mut engine, LIGHTNING_BOLT, p0);
    assert_eq!(
        engine.state().players[0].life,
        19,
        "2 of the second Bolt's 3 prevented, 1 dealt"
    );
}

/// The shield is on the chosen target and nobody else: aimed at the opponent it
/// does not stop a Bolt at the caster.
#[test]
fn remedy_does_not_protect_a_player_it_was_not_aimed_at() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();

    cast_at_player(&mut engine, remedy(), p1);
    cast_at_player(&mut engine, LIGHTNING_BOLT, p0);

    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(engine.state().players[1].life, 20);
}
