//! `cards/enchantments/mv_2/righteous_aura.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn righteous_aura() -> CardIndex {
    card_index("5bf93d4f-be59-4aa7-83d5-7c35d1f07fae")
}

/// `Righteous Aura` ({1}{W}, Enchantment, `Coverage::Implemented`):
/// `{W}, Pay 2 life: The next time a source of your choice would deal damage to
/// you this turn, prevent that damage.`
#[test]
fn righteous_aura_pays_white_and_2_life_to_prevent_next_damage_from_chosen_source() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[righteous_aura()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, righteous_aura());
    pass_until(&mut engine, stack_is_empty);
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, righteous_aura(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    assert_eq!(engine.state().players[0].life, 18);

    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    let source = options
        .into_iter()
        .find(|s| s.object == bear)
        .expect("Grizzly Bears is a damage source option");
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[0].life, 18);
}
