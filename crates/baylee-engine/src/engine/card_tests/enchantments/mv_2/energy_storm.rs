//! `cards/enchantments/mv_2/energy_storm.rs`, played.
//!
//! Oracle:
//! - "Cumulative upkeep {1}"
//! - "Prevent all damage that would be dealt by instant and sorcery spells."
//! - "Creatures with flying don't untap during their controllers' untap steps."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn energy_storm() -> CardIndex {
    card_index("567d3de7-8d56-4b6d-a59a-f8674172f595")
}

fn cast_funded(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    lk::float(
        engine,
        seat,
        &[
            (ManaColor::Colorless, 10),
            (ManaColor::White, 2),
            (ManaColor::Red, 5),
        ],
    );
    lk::cast_at(engine, seat, card, objects, players);
}

#[test]
fn energy_storm_prevents_either_players_instant_damage_to_players_and_creatures() {
    for caster in [P0, P1] {
        for target_creature in [false, true] {
            let mut engine = lk::game(
                SEED,
                &[index::GRIZZLY_BEARS],
                &[index::GRIZZLY_BEARS],
                &[energy_storm(), index::LIGHTNING_BOLT],
                &[index::LIGHTNING_BOLT],
            );
            cast_funded(&mut engine, P0, energy_storm(), &[], &[]);
            let defender = if caster == P0 { P1 } else { P0 };
            let bears = lk::obj(&engine, defender, index::GRIZZLY_BEARS);
            let (objects, players) = if target_creature {
                (vec![bears], vec![])
            } else {
                (vec![], vec![defender])
            };
            cast_funded(
                &mut engine,
                caster,
                index::LIGHTNING_BOLT,
                &objects,
                &players,
            );

            assert_eq!(lk::life(&engine, defender), 20, "spell damage is prevented");
            assert_eq!(lk::zone_of(&engine, bears), Zone::Battlefield);
            assert_eq!(
                lk::damage_on(&engine, bears),
                0,
                "all three damage prevented"
            );
            assert!(in_graveyard(&engine, caster, index::LIGHTNING_BOLT).is_some());
        }
    }
}

#[test]
fn energy_storm_prevents_sorcery_damage_to_both_players_and_both_boards() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS],
        &[energy_storm(), index::FLAME_RIFT, index::PYROCLASM],
        &[],
    );
    let mine = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let theirs = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    cast_funded(&mut engine, P0, energy_storm(), &[], &[]);
    cast_funded(&mut engine, P0, index::FLAME_RIFT, &[], &[]);
    assert_eq!(lk::life(&engine, P0), 20);
    assert_eq!(lk::life(&engine, P1), 20);

    cast_funded(&mut engine, P0, index::PYROCLASM, &[], &[]);
    for bears in [mine, theirs] {
        assert_eq!(lk::zone_of(&engine, bears), Zone::Battlefield);
        assert_eq!(lk::damage_on(&engine, bears), 0);
    }
}

#[test]
fn energy_storm_does_not_prevent_activated_triggered_or_combat_damage() {
    let mut engine = lk::game(
        SEED,
        &[
            index::PRODIGAL_SORCERER,
            index::MOGG_FANATIC,
            index::GRIZZLY_BEARS,
        ],
        &[],
        &[energy_storm(), index::SPELLSHOCK, index::LIGHTNING_BOLT],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    cast_funded(&mut engine, P0, energy_storm(), &[], &[]);
    lk::use_ability(&mut engine, P0, index::PRODIGAL_SORCERER, 0, &[], &[P1]);
    assert_eq!(
        lk::life(&engine, P1),
        19,
        "a permanent's activated damage is dealt"
    );

    activate(&mut engine, P0, index::MOGG_FANATIC, 0);
    lk::aim(&mut engine, &[], &[P1]);
    assert!(in_graveyard(&engine, P0, index::MOGG_FANATIC).is_some());
    lk::settle(&mut engine);
    assert_eq!(
        lk::life(&engine, P1),
        18,
        "the sacrificed source still deals damage"
    );

    cast_funded(&mut engine, P0, index::SPELLSHOCK, &[], &[]);
    cast_funded(&mut engine, P0, index::LIGHTNING_BOLT, &[], &[P1]);
    assert_eq!(
        lk::life(&engine, P0),
        18,
        "Spellshock's triggered damage is dealt"
    );
    assert_eq!(
        lk::life(&engine, P1),
        18,
        "the instant's damage is prevented"
    );

    lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P1), 16, "combat damage is dealt");
}

#[test]
fn energy_storm_cannot_prevent_damage_made_unpreventable_by_whippoorwill() {
    let mut engine = lk::game(
        SEED,
        &[index::WHIPPOORWILL],
        &[index::HILL_GIANT],
        &[energy_storm(), index::LIGHTNING_BOLT, index::LIGHTNING_BOLT],
        &[],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    cast_funded(&mut engine, P0, energy_storm(), &[], &[]);
    cast_funded(&mut engine, P0, index::LIGHTNING_BOLT, &[giant], &[]);
    assert_eq!(
        lk::damage_on(&engine, giant),
        0,
        "ordinary spell damage is prevented"
    );

    lk::float(&mut engine, P0, &[(ManaColor::Green, 2)]);
    lk::use_ability(&mut engine, P0, index::WHIPPOORWILL, 0, &[giant], &[]);
    cast_funded(&mut engine, P0, index::LIGHTNING_BOLT, &[giant], &[]);
    assert_eq!(lk::zone_of(&engine, giant), Zone::Exile);
    assert!(on_battlefield(&engine, P0, energy_storm()).is_some());
}

#[test]
fn energy_storm_prevention_ends_when_it_leaves_the_battlefield() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[
            energy_storm(),
            index::LIGHTNING_BOLT,
            index::LIGHTNING_BOLT,
            index::DISENCHANT,
        ],
        &[],
    );
    cast_funded(&mut engine, P0, energy_storm(), &[], &[]);
    cast_funded(&mut engine, P0, index::LIGHTNING_BOLT, &[], &[P1]);
    assert_eq!(lk::life(&engine, P1), 20);
    let storm = lk::obj(&engine, P0, energy_storm());
    cast_funded(&mut engine, P0, index::DISENCHANT, &[storm], &[]);
    assert!(in_graveyard(&engine, P0, energy_storm()).is_some());
    cast_funded(&mut engine, P0, index::LIGHTNING_BOLT, &[], &[P1]);
    assert_eq!(
        lk::life(&engine, P1),
        17,
        "the source's static prevention ended"
    );
}

#[test]
fn energy_storm_stops_creatures_with_flying_from_untapping() {
    let mut engine = lk::game(
        SEED,
        &[index::AIR_ELEMENTAL, index::GRIZZLY_BEARS],
        &[index::AIR_ELEMENTAL, index::GRIZZLY_BEARS],
        &[energy_storm()],
        &[],
    );
    let flyer = lk::obj(&engine, P1, index::AIR_ELEMENTAL);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let my_flyer = lk::obj(&engine, P0, index::AIR_ELEMENTAL);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, energy_storm());
    lk::settle(&mut engine);

    for id in [flyer, bears, my_flyer, my_bears] {
        engine
            .dev_state_mut(P0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });

    assert!(
        lk::tapped(&engine, flyer),
        "Air Elemental has flying and does not untap during its controller's untap step"
    );
    assert!(
        !lk::tapped(&engine, bears),
        "Grizzly Bears without flying untaps normally"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    assert!(
        lk::tapped(&engine, my_flyer),
        "the controller's flyer also stays tapped"
    );
    assert!(
        !lk::tapped(&engine, my_bears),
        "the controller's ground creature untaps"
    );
}

/// "Cumulative upkeep {1}": pay one, then two, and decline three.
#[test]
fn energy_storm_cumulative_upkeep_grows_and_is_sacrificed_when_unpaid() {
    let mut engine = lk::game_with(SEED, &[energy_storm()], &[], &[], &[], |e| {
        assert!(matches!(
            e.pending(),
            Pending::YesNo { player, prompt: YesNoPrompt::PayTax { mana: 1 }, .. } if *player == P0
        ));
        lk::float(e, P0, &[(ManaColor::Colorless, 1)]);
        lk::yes_no(e, true);
        assert_eq!(lk::pool_total(e, P0), 0, "the first upkeep costs one mana");
    });
    let storm = lk::obj(&engine, P0, energy_storm());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, P0, "asked of Energy Storm's controller");
    assert_eq!(engine.state().turn.active, P0, "at its own upkeep");
    assert_eq!(
        engine
            .state()
            .object(storm)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );
    assert_eq!(
        prompt,
        YesNoPrompt::PayTax { mana: 2 },
        "{{1}} for each of the two age counters"
    );
    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::settle(&mut engine);
    assert_eq!(
        lk::pool_total(&engine, P0),
        0,
        "both mana pay the second upkeep"
    );
    assert!(on_battlefield(&engine, P0, energy_storm()).is_some());

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 3 },
                ..
            }
        )
    });
    assert_eq!(
        engine.state().turn.active,
        P0,
        "never asks during the opponent's upkeep"
    );
    assert_eq!(
        engine
            .state()
            .object(storm)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        3
    );
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::settle(&mut engine);
    assert!(
        in_graveyard(&engine, P0, energy_storm()).is_some(),
        "unpaid, Energy Storm is sacrificed"
    );
}
