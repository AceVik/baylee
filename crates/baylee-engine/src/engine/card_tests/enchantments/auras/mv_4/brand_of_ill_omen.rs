//! `cards/enchantments/auras/mv_4/brand_of_ill_omen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn brand_of_ill_omen() -> CardIndex {
    card_index("947bd180-ee95-4db4-b5d9-432428018eb8")
}

fn cast_is_offered(engine: &Engine<RegistryLookup>, player: PlayerId, card: ObjectId) -> bool {
    matches!(
        engine.pending(),
        Pending::Priority { player: holder, legal, .. }
            if *holder == player && legal.castable.contains(&card)
    )
}

/// The prohibition follows the enchanted creature's controller, not the Aura's:
/// P0 may still cast a creature after branding P1's creature, while P1 may not.
#[test]
fn brand_of_ill_omen_locks_the_enchanted_creatures_controller_only() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 7])
        .battlefield(
            1,
            &[gray_ogre(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[brand_of_ill_omen(), gray_ogre()])
        .hand(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let host = on_battlefield(&engine, P1, gray_ogre()).expect("Gray Ogre seated");
    cast_from_hand(&mut engine, P0, brand_of_ill_omen());
    aim_at(&mut engine, P0, host);
    pass_until(&mut engine, stack_is_empty);

    let own_ogre = in_hand(&engine, P0, gray_ogre()).expect("P0's creature is in hand");
    assert!(
        cast_is_offered(&engine, P0, own_ogre),
        "the Aura's controller is not the enchanted creature's controller"
    );
    cast_with_floating(&mut engine, P0, gray_ogre());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, gray_ogre()).is_some());

    reach_their_main_phase(&mut engine, P1);
    tap_all_mana(&mut engine, P1);
    let locked_ogre = in_hand(&engine, P1, gray_ogre()).expect("P1's creature is in hand");
    assert!(!cast_is_offered(&engine, P1, locked_ogre));
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(P1, PlayerAction::CastSpell { card: locked_ogre })
            .is_err(),
        "directly naming the prohibited spell is rejected too"
    );
    assert_eq!(engine.fingerprint(), before, "the rejected cast is atomic");
}

/// Branding one's own creature restricts one's own creature spells; the effect
/// is not an opponent-only lock disguised by the usual Aura use case.
#[test]
fn brand_of_ill_omen_on_own_creature_locks_its_controller() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                gray_ogre(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[brand_of_ill_omen(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let host = on_battlefield(&engine, P0, gray_ogre()).expect("Gray Ogre seated");
    cast_from_hand(&mut engine, P0, brand_of_ill_omen());
    aim_at(&mut engine, P0, host);
    pass_until(&mut engine, stack_is_empty);

    let ogre = in_hand(&engine, P0, gray_ogre()).expect("the second Ogre is in hand");
    assert!(!cast_is_offered(&engine, P0, ogre));
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: ogre })
            .is_err()
    );
}

/// A noncreature spell remains legal. Casting Disenchant on Brand ends the
/// static effect, after which the same player can cast their creature spell.
#[test]
fn brand_of_ill_omen_allows_noncreatures_and_removal_restores_casting() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(); 4])
        .battlefield(
            1,
            &[
                gray_ogre(),
                plains(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[brand_of_ill_omen()])
        .hand(1, &[gray_ogre(), baylee_core::generated::index::DISENCHANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let host = on_battlefield(&engine, P1, gray_ogre()).expect("Gray Ogre seated");
    cast_from_hand(&mut engine, P0, brand_of_ill_omen());
    aim_at(&mut engine, P0, host);
    pass_until(&mut engine, stack_is_empty);
    let brand = on_battlefield(&engine, P0, brand_of_ill_omen()).expect("Brand seated");

    reach_their_main_phase(&mut engine, P1);
    tap_all_mana(&mut engine, P1);
    let ogre = in_hand(&engine, P1, gray_ogre()).expect("creature in hand");
    let disenchant = in_hand(&engine, P1, baylee_core::generated::index::DISENCHANT)
        .expect("Disenchant in hand");
    assert!(!cast_is_offered(&engine, P1, ogre));
    assert!(cast_is_offered(&engine, P1, disenchant));

    cast_with_floating(&mut engine, P1, baylee_core::generated::index::DISENCHANT);
    aim_at(&mut engine, P1, brand);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, brand_of_ill_omen()).is_some());
    assert!(
        cast_is_offered(&engine, P1, ogre),
        "the prohibition ends when Brand leaves the battlefield"
    );
    cast_with_floating(&mut engine, P1, gray_ogre());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .iter()
            .filter(|id| {
                engine.state().object(**id).is_some_and(|object| {
                    object.controller == P1
                        && object.card.is_some_and(|card| card.index == gray_ogre())
                })
            })
            .count(),
        2,
        "the enchanted Ogre and the newly cast Ogre are both on the battlefield"
    );
}

/// Brand of Ill Omen (947bd180-ee95-4db4-b5d9-432428018eb8) — {3}{R} Enchantment — Aura:
/// - "Enchant creature"
/// - "Cumulative upkeep {R}"
/// - "Enchanted creature's controller can't cast creature spells."
#[test]
fn brand_of_ill_omen_enchants_a_creature_and_has_cumulative_upkeep_red() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[brand_of_ill_omen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("Gray Ogre seated");

    cast_from_hand(&mut engine, P0, brand_of_ill_omen());
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);

    let brand = on_battlefield(&engine, P0, brand_of_ill_omen()).expect("Brand seated");
    assert_eq!(
        engine.state().object(brand).unwrap().attached_to,
        Some(ogre)
    );

    // Advance to P0's next upkeep while the cumulative upkeep trigger is on the stack,
    // tap a Mountain for {R}, then resolve the trigger and pay {R}.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: mtn })
        .expect("tap Mountain for {R}");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    assert_eq!(
        counters_on(&engine, brand, baylee_cards_dsl::counters::AGE),
        1
    );
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("pay cumulative upkeep {R}");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, brand_of_ill_omen()).is_some(),
        "paid cumulative upkeep {{R}}, so Brand stays on the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the floating {{R}} paid for the one age counter"
    );
}

/// "Sacrifice it unless you pay its upkeep cost for each age counter on it":
/// declined at the first upkeep, Brand of Ill Omen goes to the graveyard.
#[test]
fn brand_of_ill_omen_is_sacrificed_when_its_cumulative_upkeep_is_not_paid() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[brand_of_ill_omen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("Gray Ogre seated");
    cast_from_hand(&mut engine, P0, brand_of_ill_omen());
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);
    let brand = on_battlefield(&engine, P0, brand_of_ill_omen()).expect("Brand seated");

    reach_their_main_phase(&mut engine, P1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    assert_eq!(
        counters_on(&engine, brand, baylee_cards_dsl::counters::AGE),
        1
    );
    engine
        .apply(P0, PlayerAction::YesNo(false))
        .expect("decline cumulative upkeep");
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, brand_of_ill_omen()).is_none());
    assert!(in_graveyard(&engine, P0, brand_of_ill_omen()).is_some());
}
