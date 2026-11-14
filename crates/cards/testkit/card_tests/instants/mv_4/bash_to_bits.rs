//! `cards/2001/src/ody/bash_to_bits.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn bash_to_bits() -> CardIndex {
    card_index("894b8375-5d55-4138-8448-c38f83a17e4f")
}

fn start_flashback(engine: &mut Engine<RegistryLookup>, player: PlayerId, card: ObjectId) {
    engine
        .apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let slot = choose_cast_kind(engine, CastModeKind::Flashback);
        engine
            .apply(player, PlayerAction::ChooseMode(slot))
            .unwrap();
    }
}

/// Destroys target artifact from hand (rejecting creatures and enchantments),
/// requires {4}{R}{R} for Flashback (5 mana is insufficient), and exiles on
/// flashback resolution.
#[test]
fn bash_to_bits_destroys_artifact_and_flashback_exiles() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 9])
        .battlefield(
            1,
            &[
                quiet_artifact(),
                sol_ring(),
                grizzly_bears(),
                their_enchantment(),
            ],
        )
        .hand(0, &[bash_to_bits()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let arts = all_on_battlefield(&engine, P1, quiet_artifact());
    let (art1, art2) = (arts[0], arts[1]);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let chant = on_battlefield(&engine, P1, their_enchantment()).unwrap();

    cast_from_hand(&mut engine, P0, bash_to_bits());
    let options = aim_at(&mut engine, P0, art1);
    assert!(options.contains(&art1) && options.contains(&art2));
    assert!(!options.contains(&bears) && !options.contains(&chant));
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(art1).unwrap().zone, Zone::Graveyard);
    assert!(on_battlefield(&engine, P1, sol_ring()).is_some());
    let bash = in_graveyard(&engine, P0, bash_to_bits()).expect("Bash to Bits in graveyard");

    // Only 5 mana left from 9 Mountains after spending 4 -> Flashback {4}{R}{R} refuses.
    tap_all_mana(&mut engine, P0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority");
    };
    assert!(!legal.castable.contains(&bash));
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: bash })
            .is_err()
    );

    // Next P0 main with 6+ mana untapping: Flashback {4}{R}{R} destroys art2 and exiles Bash.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    start_flashback(&mut engine, P0, bash);
    let fb_options = aim_at(&mut engine, P0, art2);
    assert_eq!(fb_options, vec![art2]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(art2).unwrap().zone, Zone::Graveyard);
    assert!(in_graveyard(&engine, P0, bash_to_bits()).is_none());
    assert_eq!(engine.state().object(bash).unwrap().zone, Zone::Exile);
}
