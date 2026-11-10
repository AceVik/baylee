//! Aether Storm — `{3}{U}` Enchantment (Homelands)
//!
//! Creature spells can't be cast.
//! Pay 4 life: Destroy Aether Storm. It can't be regenerated. Any player may
//! activate this ability.

#[allow(clippy::wildcard_imports)]
use super::*;

fn aether_storm() -> CardIndex {
    card_index("ff4297d3-3d96-4bd6-a606-1bdc20a6df2b")
}

fn dryad_arbor() -> CardIndex {
    card_index("e996cd67-739c-40f4-b276-0042acf26c71")
}

#[test]
fn aether_storm_forbids_creature_spells_until_opponent_pays_four_life_to_destroy_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[aether_storm(), grizzly_bears(), dryad_arbor()])
        .battlefield(
            0,
            &[island(), forest(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Aether Storm ({3}{U}).
    let storm_hand = in_hand(&engine, p0, aether_storm()).unwrap();
    let bears_hand = in_hand(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: storm_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let storm = on_battlefield(&engine, p0, aether_storm()).unwrap();

    // With Aether Storm on the battlefield and {G}{G} floating, Grizzly Bears cannot be cast.
    assert!(
        !priority_offer(&engine).castable.contains(&bears_hand),
        "Aether Storm prevents creature spells from being cast"
    );

    // A land creature is played and not cast, so "creature spells can't be
    // cast" leaves Dryad Arbor alone.
    let arbor = in_hand(&engine, p0, dryad_arbor()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: arbor })
        .expect("Dryad Arbor is a land play, not a creature spell");
    assert!(on_battlefield(&engine, p0, dryad_arbor()).is_some());

    // Pass priority to p1; p1 (the non-controller) activates Aether Storm's Pay 4 life ability.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        priority_offer(&engine).abilities.contains(&(storm, 1)),
        "any player may activate Aether Storm's Pay 4 life ability"
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: storm,
                ability_index: 1,
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[1].life,
        16,
        "p1 paid 4 life as the activation cost"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(storm).unwrap().zone,
        Zone::Graveyard,
        "Aether Storm is destroyed"
    );

    // Once Aether Storm is gone, p0 can cast Grizzly Bears again.
    assert!(
        priority_offer(&engine).castable.contains(&bears_hand),
        "creature spells can be cast again after Aether Storm leaves the battlefield"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: bears_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}
