//! `cards/enchantments/mv_2/righteous_aura.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn righteous_aura() -> CardIndex {
    card_index("5bf93d4f-be59-4aa7-83d5-7c35d1f07fae")
}

/// P0 casts Righteous Aura on turn 1 with three Plains; P1 has a Grizzly Bears
/// and a Hill Giant. Returns the engine at P0's turn-1 end of main phase.
fn board() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears(), HILL_GIANT])
        .hand(0, &[righteous_aura()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, righteous_aura());
    pass_until(&mut engine, stack_is_empty);
    let bear = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();
    (engine, bear, giant)
}

/// P0 pays {W} and 2 life and names `source` as the one to prevent; stops after
/// the choice.
fn prevent_from(engine: &mut Engine<RegistryLookup>, source: ObjectId) {
    tap_all_mana(engine, P0);
    activate(engine, P0, righteous_aura(), 0);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    let source = options
        .into_iter()
        .find(|s| s.object == source)
        .expect("the creature is a damage source option");
    engine
        .apply(P0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
}

/// P1 attacks P0 with `attackers` and play runs to the second main phase.
fn attack_with(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(P0)))
                    .collect(),
            },
        )
        .unwrap();
    pass_until(engine, |e| e.state().turn.phase == Phase::SecondMain);
}

/// `Righteous Aura` ({1}{W}, Enchantment, `Coverage::Implemented`):
/// `{W}, Pay 2 life: The next time a source of your choice would deal damage to
/// you this turn, prevent that damage.` The chosen Bears' damage is prevented,
/// the 2 life paid is gone, and the Giant, which was not chosen, still hits.
#[test]
fn righteous_aura_pays_white_and_2_life_to_prevent_next_damage_from_chosen_source() {
    let (mut engine, bear, giant) = board();

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    prevent_from(&mut engine, bear);
    assert_eq!(engine.state().players[0].life, 18, "2 life paid");

    attack_with(&mut engine, &[bear, giant]);

    assert_eq!(
        engine.state().players[0].life,
        15,
        "the Bears' 2 was prevented, the unchosen Giant's 3 was not"
    );
}

/// "This turn": a shield made in P0's own turn is gone by P1's, so the Bears'
/// damage then is dealt.
#[test]
fn righteous_aura_prevention_does_not_outlast_the_turn() {
    let (mut engine, bear, _giant) = board();

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    prevent_from(&mut engine, bear);
    assert_eq!(engine.state().players[0].life, 18);

    reach_their_main_phase(&mut engine, P1);
    attack_with(&mut engine, &[bear]);

    assert_eq!(
        engine.state().players[0].life,
        16,
        "the shield ended with P0's turn, the Bears hit for 2"
    );
}
