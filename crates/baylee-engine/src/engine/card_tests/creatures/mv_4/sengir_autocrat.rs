//! Sengir Autocrat — `{3}{B}` 2/2 Creature — Human (Homelands)
//!
//! When this creature enters, create three 0/1 black Serf creature tokens.
//! When this creature leaves the battlefield, exile all Serf tokens.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sengir_autocrat() -> CardIndex {
    card_index("0561043a-3896-4bb1-8ff5-ee674ddfea2a")
}

#[test]
fn sengir_autocrat_creates_three_serfs_on_etb_and_exiles_all_serfs_on_ltb() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[sengir_autocrat(), lightning_bolt()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let autocrat_card = in_hand(&engine, p0, sengir_autocrat()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: autocrat_card,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let serfs_after_etb = tokens_of(&engine, p0);
    assert_eq!(
        serfs_after_etb.len(),
        3,
        "created three 0/1 black Serf tokens"
    );

    // Destroy Sengir Autocrat with Lightning Bolt -> all three Serf tokens are exiled.
    let autocrat_bf = on_battlefield(&engine, p0, sengir_autocrat()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![autocrat_bf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        tokens_of(&engine, p0).len(),
        0,
        "all Serf tokens were exiled when Sengir Autocrat left the battlefield"
    );
}
