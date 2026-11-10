//! Sengir Autocrat — `{3}{B}` 2/2 Creature — Human (Homelands)
//!
//! When this creature enters, create three 0/1 black Serf creature tokens.
//! When this creature leaves the battlefield, exile all Serf tokens.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sengir_autocrat() -> CardIndex {
    card_index("0561043a-3896-4bb1-8ff5-ee674ddfea2a")
}

fn pest_summoning() -> CardIndex {
    card_index("f6e0cb77-4115-4948-8e1e-fdb774b1b254")
}

/// The ids of the permanents `seat` controls that are copies of `card`.
fn permanents_named(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
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

/// "Exile all Serf tokens" is every Serf token, whoever controls it, and nothing else: the
/// opponent's Serfs go too, a black-and-green Pest token of the caster's stays, and the opponent's
/// own Sengir Autocrat (a Serf-maker, not a Serf) stays.
#[test]
fn sengir_autocrat_exiles_every_players_serf_tokens_and_no_other_token() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[sengir_autocrat(), pest_summoning(), lightning_bolt()])
        .hand(1, &[sengir_autocrat()])
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
            ],
        )
        .battlefield(1, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);

    // Turn two: P1 makes three Serfs of its own.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, sengir_autocrat());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(tokens_of(&engine, p1).len(), 3);

    // Turn three: P0 makes three Serfs and two Pests, paying each from its own lands so that the
    // red mana is left for the Bolt.
    reach_their_main_phase(&mut engine, p0);
    let swamps = permanents_named(&engine, p0, swamp());
    let mountain_id = on_battlefield(&engine, p0, mountain()).unwrap();
    tap_mana_where(&mut engine, p0, |id| swamps[..4].contains(&id));
    cast_with_floating(&mut engine, p0, sengir_autocrat());
    pass_until(&mut engine, stack_is_empty);
    tap_mana_where(&mut engine, p0, |id| swamps[4..].contains(&id));
    cast_with_floating(&mut engine, p0, pest_summoning());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(tokens_of(&engine, p0).len(), 5, "three Serfs and two Pests");

    let autocrat = on_battlefield(&engine, p0, sengir_autocrat()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == mountain_id);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![autocrat],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "P0's three Serfs are exiled and its two Pests are not"
    );
    assert_eq!(
        tokens_of(&engine, p1).len(),
        0,
        "P1's Serfs are exiled too: the sentence says all Serf tokens"
    );
    assert!(
        on_battlefield(&engine, p1, sengir_autocrat()).is_some(),
        "P1's Sengir Autocrat is a creature card, not a Serf token"
    );
}
