//! Enchantments and Auras of the Arabian Nights / Antiquities / Legends pass
//! (45c5bbd7), each played from its Oracle text.

#[allow(clippy::wildcard_imports)]
use super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

fn clockwork() -> CardIndex {
    card_index("eb97c8db-ac6c-476c-b14d-87785e9c82f0")
}

fn has(e: &Engine<RegistryLookup>, id: ObjectId, k: KeywordSet) -> bool {
    keywords(e, id).contains(k)
}

/// Blight: "Enchant land. When enchanted land becomes tapped, destroy it."
#[test]
fn blight_destroys_the_land_when_it_is_tapped() {
    let mut e = game(3000, &[forest()], &[], &[ids::blight()], &[]);
    let land = obj(&e, P0, forest());
    float(&mut e, P0, &[(ManaColor::Black, 2)]);
    cast_at(&mut e, P0, ids::blight(), &[land], &[]);
    assert!(
        on_battlefield(&e, P0, ids::blight()).is_some(),
        "the Aura is attached"
    );
    assert!(
        on_battlefield(&e, P0, forest()).is_some(),
        "an untapped land is untouched"
    );
    tap_all_mana(&mut e, P0);
    settle(&mut e);
    assert!(
        on_battlefield(&e, P0, forest()).is_none(),
        "tapping it destroyed it"
    );
    assert!(in_graveyard(&e, P0, forest()).is_some());
    assert!(
        in_graveyard(&e, P0, ids::blight()).is_some(),
        "the Aura fell off"
    );
}

/// Spirit Shackle: "Whenever enchanted creature becomes tapped, put a -0/-2
/// counter on it."
#[test]
fn spirit_shackle_shrinks_the_creature_each_time_it_taps() {
    let mut e = game(
        3010,
        &[ids::moss_monster()],
        &[],
        &[ids::spirit_shackle()],
        &[],
    );
    let moss = obj(&e, P0, ids::moss_monster());
    float(&mut e, P0, &[(ManaColor::Black, 2)]);
    cast_at(&mut e, P0, ids::spirit_shackle(), &[moss], &[]);
    assert_eq!(pt(&e, moss), (3, 6), "no counter before it taps");
    attack(&mut e, &[moss]);
    block(&mut e, &[]);
    settle(&mut e);
    assert_eq!(pt(&e, moss), (3, 4), "tapped to attack: one -0/-2 counter");
}

/// The Brute: "Enchant creature. Enchanted creature gets +1/+0. {R}{R}{R}:
/// Regenerate enchanted creature."
#[test]
fn the_brute_gives_plus_one_power_and_can_regenerate_its_host() {
    let mut e = game(
        3020,
        &[ids::barbary_apes()],
        &[],
        &[ids::the_brute(), lightning_bolt()],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    float(
        &mut e,
        P0,
        &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
    );
    cast_at(&mut e, P0, ids::the_brute(), &[apes], &[]);
    assert_eq!(pt(&e, apes), (3, 2));
    float(&mut e, P0, &[(ManaColor::Red, 3)]);
    use_ability(&mut e, P0, ids::the_brute(), 2, &[], &[]);
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_at(&mut e, P0, lightning_bolt(), &[apes], &[]);
    assert!(
        on_battlefield(&e, P0, ids::barbary_apes()).is_some(),
        "regenerated"
    );
    assert!(tapped(&e, apes));
}

/// Demonic Torment: "Enchant creature. Enchanted creature can't attack.
/// Prevent all combat damage that would be dealt by enchanted creature."
#[test]
fn demonic_torment_stops_the_attack_and_the_damage() {
    let mut e = game(
        3030,
        &[ids::barbary_apes()],
        &[],
        &[ids::demonic_torment()],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    float(
        &mut e,
        P0,
        &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
    );
    cast_at(&mut e, P0, ids::demonic_torment(), &[apes], &[]);
    assert!(
        !attackers_offered(&mut e).contains(&apes),
        "it can't attack"
    );

    // Enchant the blocker: its combat damage is prevented.
    let mut e = game(
        3031,
        &[ids::moss_monster()],
        &[ids::barbary_apes()],
        &[ids::demonic_torment()],
        &[],
    );
    let (moss, apes) = (
        obj(&e, P0, ids::moss_monster()),
        obj(&e, P1, ids::barbary_apes()),
    );
    float(
        &mut e,
        P0,
        &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
    );
    cast_at(&mut e, P0, ids::demonic_torment(), &[apes], &[]);
    attack(&mut e, &[moss]);
    block(&mut e, &[(apes, moss)]);
    to_step(&mut e, P0, Step::CombatEnd);
    assert_eq!(
        damage_on(&e, moss),
        0,
        "the enchanted blocker dealt nothing"
    );
    assert!(
        in_graveyard(&e, P1, ids::barbary_apes()).is_some(),
        "but takes the Monster's 3"
    );
}

/// Gaseous Form: "Prevent all combat damage that would be dealt to and dealt
/// by enchanted creature."
#[test]
fn gaseous_form_prevents_combat_damage_both_ways() {
    let mut e = game(
        3040,
        &[ids::moss_monster()],
        &[ids::barbary_apes()],
        &[ids::gaseous_form()],
        &[],
    );
    let (moss, apes) = (
        obj(&e, P0, ids::moss_monster()),
        obj(&e, P1, ids::barbary_apes()),
    );
    float(
        &mut e,
        P0,
        &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
    );
    cast_at(&mut e, P0, ids::gaseous_form(), &[apes], &[]);
    attack(&mut e, &[moss]);
    block(&mut e, &[(apes, moss)]);
    to_step(&mut e, P0, Step::CombatEnd);
    assert_eq!(
        damage_on(&e, moss),
        0,
        "nothing dealt by the enchanted creature"
    );
    assert_eq!(damage_on(&e, apes), 0, "nothing dealt to it");
    assert!(on_battlefield(&e, P1, ids::barbary_apes()).is_some());
}

/// Puppet Master: "When enchanted creature dies, return that card to its
/// owner's hand. If that card is returned to its owner's hand this way, you
/// may pay {U}{U}{U}. If you do, return this card to its owner's hand."
fn puppet_master_game(pay: bool, seed: u64) -> Engine<RegistryLookup> {
    let mut e = game(
        seed,
        &[ids::barbary_apes()],
        &[],
        &[ids::puppet_master(), lightning_bolt()],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    float(&mut e, P0, &[(ManaColor::Blue, 3)]);
    cast_at(&mut e, P0, ids::puppet_master(), &[apes], &[]);
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, lightning_bolt());
    aim(&mut e, &[apes], &[]);
    if pay {
        float(&mut e, P0, &[(ManaColor::Blue, 3)]);
    }
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    yes_no(&mut e, pay);
    settle(&mut e);
    e
}

#[test]
fn puppet_master_returns_the_creature_and_stays_put_when_not_paid() {
    let e = puppet_master_game(false, 3051);
    assert!(
        in_hand(&e, P0, ids::barbary_apes()).is_some(),
        "the creature returns"
    );
    assert!(in_hand(&e, P0, ids::puppet_master()).is_none());
    assert!(in_graveyard(&e, P0, ids::puppet_master()).is_some());
}

#[test]
fn puppet_master_returns_itself_when_uuu_is_paid() {
    let e = puppet_master_game(true, 3050);
    assert!(in_hand(&e, P0, ids::barbary_apes()).is_some());
    assert!(
        in_hand(&e, P0, ids::puppet_master()).is_some(),
        "the Aura returns to hand"
    );
}

/// Seeker: "Enchanted creature can't be blocked except by artifact creatures
/// and/or white creatures."
#[test]
fn seeker_lets_only_artifact_and_white_creatures_block() {
    let mut e = game(
        3060,
        &[ids::moss_monster()],
        &[
            ids::barbary_apes(),
            ids::keepers_of_the_faith(),
            clockwork(),
        ],
        &[ids::seeker()],
        &[],
    );
    let moss = obj(&e, P0, ids::moss_monster());
    float(
        &mut e,
        P0,
        &[(ManaColor::White, 2), (ManaColor::Colorless, 2)],
    );
    cast_at(&mut e, P0, ids::seeker(), &[moss], &[]);
    let offer = attack(&mut e, &[moss]);
    assert!(
        !may_block(&offer, obj(&e, P1, ids::barbary_apes()), moss),
        "green"
    );
    assert!(
        may_block(&offer, obj(&e, P1, ids::keepers_of_the_faith()), moss),
        "white"
    );
    assert!(
        may_block(&offer, obj(&e, P1, clockwork()), moss),
        "artifact"
    );
}

/// Greater Realm of Preservation: "{1}{W}: The next time a black or red
/// source of your choice would deal damage to you this turn, prevent that
/// damage."
#[test]
fn greater_realm_of_preservation_prevents_a_red_sources_next_damage() {
    let mut e = game(
        3070,
        &[ids::greater_realm_of_preservation()],
        &[ids::raging_bull()],
        &[],
        &[],
    );
    let bull = obj(&e, P1, ids::raging_bull());
    pass_until(&mut e, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    float(
        &mut e,
        P0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
    );
    activate(&mut e, P0, ids::greater_realm_of_preservation(), 0);
    // The source is chosen as the ability resolves.
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        choice,
        options,
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, P0);
    e.apply(
        P0,
        PlayerAction::ChooseDamageSource {
            choice,
            source: options[0],
        },
    )
    .unwrap();
    settle(&mut e);
    let able = attackers_offered(&mut e);
    assert!(able.contains(&bull));
    declare(&mut e, &[bull]);
    block(&mut e, &[]);
    through_combat(&mut e);
    assert_eq!(life(&e, P0), 20, "the Bull's damage was prevented");
}

/// The walk-cancelling enchantments: Crevasse (mountainwalk), Deadfall
/// (forestwalk), Great Wall (plainswalk), Quagmire (swampwalk), Undertow
/// (islandwalk): "Creatures with <walk> can be blocked as though they didn't
/// have <walk>."
#[test]
fn the_walk_cancelling_enchantments_let_the_walker_be_blocked() {
    let rows = [
        (ids::crevasse(), ids::mountain_yeti(), mountain()),
        (ids::deadfall(), ids::cat_warriors(), forest()),
        (ids::great_wall(), ids::righteous_avengers(), plains()),
        (ids::quagmire(), ids::lost_soul(), swamp()),
        (ids::undertow(), ids::devouring_deep(), island()),
    ];
    for (n, (ench, walker, land)) in rows.into_iter().enumerate() {
        for (with_ench, seed) in [(false, 0u64), (true, 1)] {
            let mut board = vec![ids::barbary_apes(), land];
            let mut mine = vec![walker];
            if with_ench {
                // The enchantment's controller does not matter.
                if n % 2 == 0 {
                    board.push(ench);
                } else {
                    mine.push(ench);
                }
            }
            let mut e = game(3100 + 2 * n as u64 + seed, &mine, &board, &[], &[]);
            let attacker = obj(&e, P0, walker);
            let offer = attack(&mut e, &[attacker]);
            let apes = obj(&e, P1, ids::barbary_apes());
            assert_eq!(may_block(&offer, apes, attacker), with_ench, "row {n}");
        }
    }
    // Gosta Dirk does the same for islandwalk.
    let mut e = game(
        3120,
        &[ids::devouring_deep()],
        &[ids::barbary_apes(), island(), ids::gosta_dirk()],
        &[],
        &[],
    );
    let deep = obj(&e, P0, ids::devouring_deep());
    let offer = attack(&mut e, &[deep]);
    assert!(
        may_block(&offer, obj(&e, P1, ids::barbary_apes()), deep),
        "Gosta Dirk"
    );
}

/// Fortified Area: "Wall creatures you control get +1/+0 and have banding."
#[test]
fn fortified_area_buffs_my_walls() {
    let e = game(
        3130,
        &[
            ids::fortified_area(),
            ids::wall_of_light(),
            ids::barbary_apes(),
        ],
        &[ids::wall_of_opposition()],
        &[],
        &[],
    );
    let wall = obj(&e, P0, ids::wall_of_light());
    assert_eq!(pt(&e, wall), (2, 5));
    assert!(has(&e, wall, KeywordSet::BANDING));
    let apes = obj(&e, P0, ids::barbary_apes());
    assert_eq!(pt(&e, apes), (2, 2));
    assert!(!has(&e, apes, KeywordSet::BANDING));
    let theirs = obj(&e, P1, ids::wall_of_opposition());
    assert_eq!(pt(&e, theirs), (0, 6), "an opponent's Wall is not mine");
    assert!(!has(&e, theirs, KeywordSet::BANDING));
}

/// In the Eye of Chaos: "Whenever a player casts an instant spell, counter it
/// unless that player pays {X}, where X is its mana value."
#[test]
fn in_the_eye_of_chaos_taxes_instants_by_mana_value() {
    for (pay, seed) in [(false, 3140u64), (true, 3141)] {
        let mut e = game(
            seed,
            &[ids::in_the_eye_of_chaos()],
            &[],
            &[lightning_bolt()],
            &[],
        );
        float(&mut e, P0, &[(ManaColor::Red, 1)]);
        cast_with_floating(&mut e, P0, lightning_bolt());
        aim(&mut e, &[], &[P1]);
        if pay {
            float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
        }
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
        });
        if matches!(e.pending(), Pending::YesNo { .. }) {
            yes_no(&mut e, pay);
        }
        settle(&mut e);
        assert_eq!(life(&e, P1), if pay { 17 } else { 20 }, "pay={pay}");
    }
    // A creature spell is not an instant.
    let mut e = game(
        3142,
        &[ids::in_the_eye_of_chaos()],
        &[],
        &[ids::barbary_apes()],
        &[],
    );
    float(
        &mut e,
        P0,
        &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
    );
    cast_at(&mut e, P0, ids::barbary_apes(), &[], &[]);
    assert!(on_battlefield(&e, P0, ids::barbary_apes()).is_some());
}

/// Nether Void: "Whenever a player casts a spell, counter it unless that
/// player pays {3}."
#[test]
fn nether_void_counters_unless_three_is_paid() {
    for (pay, seed) in [(false, 3150u64), (true, 3151)] {
        let mut e = game(
            seed,
            &[ids::nether_void()],
            &[],
            &[ids::barbary_apes()],
            &[],
        );
        float(
            &mut e,
            P0,
            &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        );
        cast_with_floating(&mut e, P0, ids::barbary_apes());
        if pay {
            float(&mut e, P0, &[(ManaColor::Colorless, 3)]);
        }
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
        });
        if matches!(e.pending(), Pending::YesNo { .. }) {
            yes_no(&mut e, pay);
        }
        settle(&mut e);
        assert_eq!(
            on_battlefield(&e, P0, ids::barbary_apes()).is_some(),
            pay,
            "pay={pay}"
        );
        assert_eq!(in_graveyard(&e, P0, ids::barbary_apes()).is_some(), !pay);
    }
}

/// Presence of the Master: "Whenever a player casts an enchantment spell,
/// counter it."
#[test]
fn presence_of_the_master_counters_enchantment_spells_only() {
    let mut e = game(
        3160,
        &[ids::presence_of_the_master()],
        &[],
        &[ids::great_wall(), ids::barbary_apes()],
        &[],
    );
    float(
        &mut e,
        P0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
    );
    cast_with_floating(&mut e, P0, ids::great_wall());
    settle(&mut e);
    assert!(on_battlefield(&e, P0, ids::great_wall()).is_none());
    assert!(
        in_graveyard(&e, P0, ids::great_wall()).is_some(),
        "countered"
    );
    float(
        &mut e,
        P0,
        &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
    );
    cast_at(&mut e, P0, ids::barbary_apes(), &[], &[]);
    assert!(
        on_battlefield(&e, P0, ids::barbary_apes()).is_some(),
        "a creature is not"
    );
}

/// Underworld Dreams: "Whenever an opponent draws a card, this enchantment
/// deals 1 damage to that player."
#[test]
fn underworld_dreams_hurts_the_opponent_for_each_draw() {
    let mut e = game(3170, &[ids::underworld_dreams()], &[], &[], &[]);
    assert_eq!(life(&e, P0), 20, "my own draws cost me nothing");
    reach_their_main_phase(&mut e, P1);
    assert_eq!(life(&e, P1), 19, "their draw step: 1 damage");
    reach_their_main_phase(&mut e, P0);
    assert_eq!(life(&e, P0), 20);
    assert_eq!(life(&e, P1), 19);
}

/// Angelic Voices: "Creatures you control get +1/+1 as long as you control no
/// nonartifact, nonwhite creatures."
#[test]
fn angelic_voices_pumps_while_every_creature_is_white_or_artifact() {
    let e = game(
        3180,
        &[
            ids::angelic_voices(),
            ids::keepers_of_the_faith(),
            clockwork(),
        ],
        &[],
        &[],
        &[],
    );
    assert_eq!(pt(&e, obj(&e, P0, ids::keepers_of_the_faith())), (3, 4));
    let e = game(
        3181,
        &[
            ids::angelic_voices(),
            ids::keepers_of_the_faith(),
            ids::barbary_apes(),
        ],
        &[],
        &[],
        &[],
    );
    assert_eq!(
        pt(&e, obj(&e, P0, ids::keepers_of_the_faith())),
        (2, 3),
        "a green creature turns it off"
    );
    assert_eq!(pt(&e, obj(&e, P0, ids::barbary_apes())), (2, 2));
}

/// Lifeblood: "Whenever a Mountain an opponent controls becomes tapped, you
/// gain 1 life."
#[test]
fn lifeblood_pays_for_the_opponents_tapped_mountains() {
    let mut e = game(
        3190,
        &[ids::lifeblood(), mountain()],
        &[mountain()],
        &[],
        &[],
    );
    tap_all_mana(&mut e, P0);
    settle(&mut e);
    assert_eq!(life(&e, P0), 20, "my own Mountain does nothing");
    reach_their_main_phase(&mut e, P1);
    tap_all_mana(&mut e, P1);
    settle(&mut e);
    assert_eq!(life(&e, P0), 21, "their Mountain tapped");
}

/// Moat: "Creatures without flying can't attack."
#[test]
fn moat_keeps_the_ground_at_home() {
    let mut e = game(
        3200,
        &[ids::barbary_apes(), ids::vampire_bats()],
        &[ids::moat()],
        &[],
        &[],
    );
    let able = attackers_offered(&mut e);
    assert!(
        able.contains(&obj(&e, P0, ids::vampire_bats())),
        "flying may attack"
    );
    assert!(
        !able.contains(&obj(&e, P0, ids::barbary_apes())),
        "no flying, no attack"
    );
}

/// Spiritual Sanctuary: "At the beginning of each player's upkeep, if that
/// player controls a Plains, they gain 1 life."
#[test]
fn spiritual_sanctuary_pays_each_plains_player_at_upkeep() {
    let mut e = game(
        3210,
        &[ids::spiritual_sanctuary(), plains()],
        &[plains()],
        &[],
        &[],
    );
    assert_eq!(life(&e, P0), 21, "my upkeep");
    reach_their_main_phase(&mut e, P1);
    assert_eq!(life(&e, P1), 21, "their upkeep");

    let mut e = game(3211, &[ids::spiritual_sanctuary()], &[forest()], &[], &[]);
    assert_eq!(life(&e, P0), 20, "no Plains, no life");
    reach_their_main_phase(&mut e, P1);
    assert_eq!(life(&e, P1), 20);
}

/// Horror of Horrors: "Sacrifice a Swamp: Regenerate target black creature."
#[test]
fn horror_of_horrors_regenerates_black_creatures_for_a_swamp() {
    let mut e = game(
        3220,
        &[
            ids::horror_of_horrors(),
            swamp(),
            ids::vampire_bats(),
            ids::barbary_apes(),
        ],
        &[],
        &[lightning_bolt()],
        &[],
    );
    let (bats, apes, land) = (
        obj(&e, P0, ids::vampire_bats()),
        obj(&e, P0, ids::barbary_apes()),
        obj(&e, P0, swamp()),
    );
    activate(&mut e, P0, ids::horror_of_horrors(), 0);
    let mut seen_targets = None;
    for _ in 0..4 {
        match e.pending().clone() {
            Pending::ChooseTargets { options, .. } => {
                seen_targets = Some(options.clone());
                aim(&mut e, &[bats], &[]);
            }
            Pending::ChooseCards { .. } => drive(&mut e, &[land], true, 0),
            _ => break,
        }
    }
    let options = seen_targets.expect("a target was asked");
    assert!(options.contains(&bats));
    assert!(!options.contains(&apes), "only black creatures");
    settle(&mut e);
    assert!(
        on_battlefield(&e, P0, swamp()).is_none(),
        "the Swamp was sacrificed"
    );
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_at(&mut e, P0, lightning_bolt(), &[bats], &[]);
    assert!(
        on_battlefield(&e, P0, ids::vampire_bats()).is_some(),
        "regenerated"
    );
    assert!(tapped(&e, bats));
}
