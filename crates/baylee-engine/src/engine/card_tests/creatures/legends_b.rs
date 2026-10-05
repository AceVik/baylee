//! Creatures of the Arabian Nights / Antiquities / Legends pass (45c5bbd7),
//! part B: activated and triggered abilities.

#[allow(clippy::wildcard_imports)]
use super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

/// Mana for the upkeep "unless you pay" of the Elder Dragons and Cosmic Horror.
fn upkeep_cost(card: CardIndex) -> Vec<(ManaColor, u32)> {
    use ManaColor::{Black, Blue, Colorless, Green, Red, White};
    if card == ids::cosmic_horror() {
        vec![(Colorless, 3), (Black, 3)]
    } else if card == ids::arcades_sabboth() {
        vec![(Green, 1), (White, 1), (Blue, 1)]
    } else if card == ids::nicol_bolas() {
        vec![(Blue, 1), (Black, 1), (Red, 1)]
    } else if card == ids::palladia_mors() {
        vec![(Red, 1), (Green, 1), (White, 1)]
    } else {
        vec![(Black, 1), (Red, 1), (Green, 1)]
    }
}

/// Plays the first upkeep with the answer `pay`.
fn first_upkeep(
    card: CardIndex,
    pay: bool,
    seed: u64,
    p1: &[CardIndex],
    p1_hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    game_with(seed, &[card], p1, &[], p1_hand, move |e| {
        if pay {
            float(e, P0, &upkeep_cost(card));
        }
        yes_no(e, pay);
    })
}

/// Cosmic Horror, Arcades Sabboth, Nicol Bolas, Palladia-Mors, Vaevictis
/// Asmadi: "At the beginning of your upkeep, <sacrifice/destroy> this unless
/// you pay <cost>" — paying keeps it, not paying loses it.
#[test]
fn the_upkeep_creatures_stay_when_paid_and_go_when_not() {
    for (n, card) in [
        ids::cosmic_horror(),
        ids::arcades_sabboth(),
        ids::nicol_bolas(),
        ids::palladia_mors(),
        ids::vaevictis_asmadi(),
    ]
    .into_iter()
    .enumerate()
    {
        let e = first_upkeep(card, true, 2000 + n as u64, &[], &[]);
        assert!(
            on_battlefield(&e, P0, card).is_some(),
            "row {n}: paid, it stays"
        );
        assert_eq!(
            pool_total(&e, P0),
            0,
            "row {n}: the cost came out of the pool"
        );
        assert_eq!(life(&e, P0), 20, "row {n}");

        let e = first_upkeep(card, false, 2100 + n as u64, &[], &[]);
        assert!(
            on_battlefield(&e, P0, card).is_none(),
            "row {n}: unpaid, it goes"
        );
        assert!(
            in_graveyard(&e, P0, card).is_some(),
            "row {n}: to the graveyard"
        );
        let hurt = if card == ids::cosmic_horror() { 13 } else { 20 };
        assert_eq!(
            life(&e, P0),
            hurt,
            "row {n}: only Cosmic Horror deals damage to you when it dies this way"
        );
    }
}

/// Arcades Sabboth: "Each untapped creature you control gets +0/+2 as long as
/// it's not attacking." and "{W}: gets +0/+1 until end of turn."
#[test]
fn arcades_sabboth_shields_the_untapped_and_pumps_for_white() {
    let mut e = game_with(
        2200,
        &[ids::arcades_sabboth(), ids::barbary_apes()],
        &[],
        &[],
        &[],
        |e| {
            float(e, P0, &upkeep_cost(ids::arcades_sabboth()));
            yes_no(e, true);
        },
    );
    let (arcades, apes) = (
        obj(&e, P0, ids::arcades_sabboth()),
        obj(&e, P0, ids::barbary_apes()),
    );
    assert_eq!(pt(&e, apes), (2, 4), "untapped and not attacking: +0/+2");
    assert_eq!(pt(&e, arcades), (7, 9));
    float(&mut e, P0, &[(ManaColor::White, 1)]);
    use_ability(&mut e, P0, ids::arcades_sabboth(), 2, &[], &[]);
    assert_eq!(pt(&e, arcades), (7, 10), "{{W}}: +0/+1");
    attack(&mut e, &[apes]);
    assert_eq!(pt(&e, apes), (2, 2), "an attacker gets no bonus");
}

/// Vaevictis Asmadi: "{B}: +1/+0 until end of turn. {R}: ... {G}: ..."
#[test]
fn vaevictis_asmadi_gains_a_power_for_each_of_its_three_colours() {
    let mut e = game_with(2210, &[ids::vaevictis_asmadi()], &[], &[], &[], |e| {
        float(e, P0, &upkeep_cost(ids::vaevictis_asmadi()));
        yes_no(e, true);
    });
    let v = obj(&e, P0, ids::vaevictis_asmadi());
    float(
        &mut e,
        P0,
        &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
    );
    for (index, power) in [(1, 8), (2, 9), (3, 10)] {
        use_ability(&mut e, P0, ids::vaevictis_asmadi(), index, &[], &[]);
        assert_eq!(pt(&e, v), (power, 7), "ability {index}");
    }
}

/// Nicol Bolas: "Whenever Nicol Bolas deals damage to an opponent, that
/// player discards their hand."
#[test]
fn nicol_bolas_makes_the_damaged_opponent_discard_their_hand() {
    let mut e = game_with(
        2220,
        &[ids::nicol_bolas()],
        &[],
        &[],
        &[ids::barbary_apes(), ids::raging_bull()],
        |e| {
            float(e, P0, &upkeep_cost(ids::nicol_bolas()));
            yes_no(e, true);
        },
    );
    let bolas = obj(&e, P0, ids::nicol_bolas());
    attack(&mut e, &[bolas]);
    block(&mut e, &[]);
    through_combat(&mut e);
    assert_eq!(life(&e, P1), 13, "7 damage");
    assert!(in_hand(&e, P1, ids::barbary_apes()).is_none());
    assert!(in_hand(&e, P1, ids::raging_bull()).is_none());
    assert!(in_graveyard(&e, P1, ids::barbary_apes()).is_some());
    assert!(in_graveyard(&e, P1, ids::raging_bull()).is_some());
}

/// Primordial Ooze: "At the beginning of your upkeep, put a +1/+1 counter on
/// this creature. Then you may pay {X}, where X is the number of +1/+1
/// counters on it. If you don't, tap this creature and it deals X damage to
/// you."
#[test]
fn primordial_ooze_grows_each_upkeep_and_hurts_you_if_unpaid() {
    let paid = game_with(2230, &[ids::primordial_ooze()], &[], &[], &[], |e| {
        float(e, P0, &[(ManaColor::Colorless, 1)]);
        yes_no(e, true);
    });
    let ooze = obj(&paid, P0, ids::primordial_ooze());
    assert_eq!(pt(&paid, ooze), (2, 2), "one counter");
    assert!(!tapped(&paid, ooze));
    assert_eq!(life(&paid, P0), 20);

    let unpaid = game_with(2231, &[ids::primordial_ooze()], &[], &[], &[], |e| {
        yes_no(e, false);
    });
    let ooze = obj(&unpaid, P0, ids::primordial_ooze());
    assert_eq!(pt(&unpaid, ooze), (2, 2), "the counter stays either way");
    assert!(tapped(&unpaid, ooze), "tapped when X is not paid");
    assert_eq!(life(&unpaid, P0), 19, "and it deals X = 1 to you");
}

/// Sol'kanar the Swamp King: "Whenever a player casts a black spell, you gain
/// 1 life."
#[test]
fn sol_kanar_gains_life_for_every_black_spell_by_anyone() {
    let mut e = game(
        2240,
        &[ids::sol_kanar_the_swamp_king()],
        &[],
        &[ids::vampire_bats(), ids::barbary_apes()],
        &[ids::headless_horseman()],
    );
    float(
        &mut e,
        P0,
        &[
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
            (ManaColor::Green, 1),
        ],
    );
    cast_at(&mut e, P0, ids::vampire_bats(), &[], &[]);
    assert_eq!(life(&e, P0), 21, "my own black spell");
    cast_at(&mut e, P0, ids::barbary_apes(), &[], &[]);
    assert_eq!(life(&e, P0), 21, "a green spell gains nothing");
    // The opponent's black spell, cast in their own turn.
    reach_their_main_phase(&mut e, P1);
    float(
        &mut e,
        P1,
        &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
    );
    cast_at(&mut e, P1, ids::headless_horseman(), &[], &[]);
    assert_eq!(
        life(&e, P0),
        22,
        "an opponent's black spell gains me life too"
    );
}

/// Vampire Bats: "{B}: +1/+0 until end of turn. Activate no more than twice
/// each turn."
#[test]
fn vampire_bats_pump_at_most_twice_a_turn() {
    let mut e = game(2250, &[ids::vampire_bats()], &[], &[], &[]);
    let bats = obj(&e, P0, ids::vampire_bats());
    float(&mut e, P0, &[(ManaColor::Black, 3)]);
    assert_eq!(pt(&e, bats), (0, 1));
    use_ability(&mut e, P0, ids::vampire_bats(), 0, &[], &[]);
    assert_eq!(pt(&e, bats), (1, 1));
    use_ability(&mut e, P0, ids::vampire_bats(), 0, &[], &[]);
    assert_eq!(pt(&e, bats), (2, 1));
    assert!(
        !offered(&e, ids::vampire_bats(), P0, 0),
        "a third activation is not offered, though the mana is there"
    );
}

/// The four creatures that shoot attacking or blocking creatures:
/// D'Avenant Archer ({T}: 1), Crimson Manticore ({R},{T}: 1), Tor Wauki
/// ({T}: 2), Lady Caleria ({T}: 3).
#[test]
#[allow(clippy::type_complexity)]
fn the_combat_pingers_hit_an_attacker_or_blocker_for_their_amount() {
    let rows: [(CardIndex, Vec<(ManaColor, u32)>, u16); 4] = [
        (ids::d_avenant_archer(), vec![], 1),
        (ids::crimson_manticore(), vec![(ManaColor::Red, 1)], 1),
        (ids::tor_wauki(), vec![], 2),
        (ids::lady_caleria(), vec![], 3),
    ];
    for (n, (pinger, mana, damage)) in rows.into_iter().enumerate() {
        let mut e = game(
            2300 + n as u64,
            &[pinger, ids::barbary_apes()],
            &[ids::moss_monster()],
            &[],
            &[],
        );
        let (apes, moss) = (
            obj(&e, P0, ids::barbary_apes()),
            obj(&e, P1, ids::moss_monster()),
        );
        // Nothing is attacking or blocking yet: no legal target.
        float(&mut e, P0, &mana);
        assert!(
            !offered(&e, pinger, P0, 0),
            "row {n}: offered with nobody in combat"
        );
        attack(&mut e, &[apes]);
        block(&mut e, &[(moss, apes)]);
        to_step(&mut e, P0, Step::DeclareBlockers);
        float(&mut e, P0, &mana);
        activate(&mut e, P0, pinger, 0);
        let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
            panic!("row {n}: a target was expected");
        };
        let me = obj(&e, P0, pinger);
        assert!(
            options.contains(&moss) && options.contains(&apes),
            "row {n}"
        );
        assert!(
            !options.contains(&me),
            "row {n}: only attacking or blocking creatures"
        );
        aim(&mut e, &[moss], &[]);
        settle(&mut e);
        assert_eq!(damage_on(&e, moss), damage, "row {n}");
        assert!(tapped(&e, me), "row {n}: {{T}} was paid");
    }
}

/// Spinal Villain: "{T}: Destroy target blue creature."
#[test]
fn spinal_villain_destroys_only_blue_creatures() {
    let mut e = game(
        2400,
        &[ids::spinal_villain()],
        &[ids::devouring_deep(), ids::barbary_apes()],
        &[],
        &[],
    );
    let (deep, apes) = (
        obj(&e, P1, ids::devouring_deep()),
        obj(&e, P1, ids::barbary_apes()),
    );
    activate(&mut e, P0, ids::spinal_villain(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("a target was expected");
    };
    assert!(options.contains(&deep));
    assert!(!options.contains(&apes), "a green creature is not a target");
    aim(&mut e, &[deep], &[]);
    settle(&mut e);
    assert!(in_graveyard(&e, P1, ids::devouring_deep()).is_some());
    assert!(on_battlefield(&e, P1, ids::barbary_apes()).is_some());
}

/// Radjan Spirit: "{T}: Target creature loses flying until end of turn."
#[test]
fn radjan_spirit_takes_flying_away_for_the_turn() {
    let mut e = game(
        2410,
        &[ids::radjan_spirit()],
        &[ids::vampire_bats()],
        &[],
        &[],
    );
    let bats = obj(&e, P1, ids::vampire_bats());
    assert!(keywords(&e, bats).contains(KeywordSet::FLYING));
    use_ability(&mut e, P0, ids::radjan_spirit(), 0, &[bats], &[]);
    assert!(!keywords(&e, bats).contains(KeywordSet::FLYING));
    // Until end of turn only.
    reach_their_main_phase(&mut e, P1);
    assert!(keywords(&e, bats).contains(KeywordSet::FLYING));
}

/// Psionic Entity: "{T}: This creature deals 2 damage to any target and 3
/// damage to itself."
#[test]
fn psionic_entity_shoots_two_and_takes_three() {
    let mut e = game(
        2420,
        &[ids::psionic_entity()],
        &[ids::moss_monster()],
        &[],
        &[],
    );
    use_ability(&mut e, P0, ids::psionic_entity(), 0, &[], &[P1]);
    assert_eq!(life(&e, P1), 18);
    assert!(
        in_graveyard(&e, P0, ids::psionic_entity()).is_some(),
        "3 damage kills a 2/2"
    );

    let mut e = game(
        2421,
        &[ids::psionic_entity()],
        &[ids::moss_monster()],
        &[],
        &[],
    );
    let moss = obj(&e, P1, ids::moss_monster());
    use_ability(&mut e, P0, ids::psionic_entity(), 0, &[moss], &[]);
    assert_eq!(damage_on(&e, moss), 2, "any target includes creatures");
}

/// Gwendlyn Di Corci: "{T}: Target player discards a card at random.
/// Activate only during your turn."
#[test]
fn gwendlyn_makes_a_player_discard_but_only_in_my_turn() {
    let mut e = game(
        2430,
        &[ids::gwendlyn_di_corci()],
        &[],
        &[],
        &[ids::barbary_apes()],
    );
    use_ability(&mut e, P0, ids::gwendlyn_di_corci(), 0, &[], &[P1]);
    assert!(in_hand(&e, P1, ids::barbary_apes()).is_none());
    assert!(in_graveyard(&e, P1, ids::barbary_apes()).is_some());

    // In the opponent's turn the ability is not offered.
    let mut e = game(
        2431,
        &[ids::gwendlyn_di_corci()],
        &[],
        &[],
        &[ids::barbary_apes()],
    );
    pass_until(&mut e, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(!offered(&e, ids::gwendlyn_di_corci(), P0, 0));
}

/// Hell's Caretaker: "{T}, Sacrifice a creature: Return target creature card
/// from your graveyard to the battlefield. Activate only during your
/// upkeep."
#[test]
fn hells_caretaker_reanimates_only_in_my_upkeep() {
    let mut e = game(
        2440,
        &[
            ids::hell_s_caretaker(),
            ids::crimson_kobolds(),
            ids::barbary_apes(),
        ],
        &[],
        &[lightning_bolt()],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_at(&mut e, P0, lightning_bolt(), &[apes], &[]);
    let dead = in_graveyard(&e, P0, ids::barbary_apes()).expect("the Apes died");
    assert!(
        !offered(&e, ids::hell_s_caretaker(), P0, 0),
        "not in main phase"
    );

    pass_until(&mut e, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(
        offered(&e, ids::hell_s_caretaker(), P0, 0),
        "offered in my upkeep"
    );
    let kobolds = obj(&e, P0, ids::crimson_kobolds());
    activate(&mut e, P0, ids::hell_s_caretaker(), 0);
    drive(&mut e, &[kobolds, dead], true, 0);
    settle(&mut e);
    assert!(
        on_battlefield(&e, P0, ids::barbary_apes()).is_some(),
        "returned"
    );
    assert!(
        on_battlefield(&e, P0, ids::crimson_kobolds()).is_none(),
        "sacrificed"
    );
    assert!(tapped(&e, obj(&e, P0, ids::hell_s_caretaker())));
}

/// Adun Oakenshield: "{B}{R}{G}, {T}: Return target creature card from your
/// graveyard to your hand."
#[test]
fn adun_oakenshield_returns_a_creature_card_to_hand() {
    let mut e = game(
        2450,
        &[ids::adun_oakenshield(), ids::barbary_apes()],
        &[],
        &[lightning_bolt()],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_at(&mut e, P0, lightning_bolt(), &[apes], &[]);
    let dead = in_graveyard(&e, P0, ids::barbary_apes()).expect("the Apes died");
    float(
        &mut e,
        P0,
        &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
    );
    use_ability(&mut e, P0, ids::adun_oakenshield(), 0, &[dead], &[]);
    assert!(in_hand(&e, P0, ids::barbary_apes()).is_some());
    assert!(in_graveyard(&e, P0, ids::barbary_apes()).is_none());
}

/// Angus Mackenzie: "{G}{W}{U}, {T}: Prevent all combat damage that would be
/// dealt this turn. Activate only before the combat damage step."
#[test]
fn angus_mackenzie_fogs_the_turn_but_only_before_damage() {
    let fog = [
        (ManaColor::Green, 1),
        (ManaColor::White, 1),
        (ManaColor::Blue, 1),
    ];
    let mut e = game(
        2460,
        &[ids::angus_mackenzie(), ids::barbary_apes()],
        &[],
        &[],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    attack(&mut e, &[apes]);
    block(&mut e, &[]);
    to_step(&mut e, P0, Step::DeclareBlockers);
    float(&mut e, P0, &fog);
    use_ability(&mut e, P0, ids::angus_mackenzie(), 0, &[], &[]);
    through_combat(&mut e);
    assert_eq!(life(&e, P1), 20, "all combat damage was prevented");

    // Before combat damage it is offered; after the combat damage step it is
    // not (an untapped Angus, mana floating, in the second main phase).
    let mut e = game(2461, &[ids::angus_mackenzie()], &[], &[], &[]);
    float(&mut e, P0, &fog);
    assert!(
        offered(&e, ids::angus_mackenzie(), P0, 0),
        "before combat damage it is"
    );
    through_combat(&mut e);
    float(&mut e, P0, &fog);
    assert!(
        !offered(&e, ids::angus_mackenzie(), P0, 0),
        "after it, it is not"
    );
}

/// Lady Evangela: "{W}{B}, {T}: Prevent all combat damage that would be dealt
/// by target creature this turn."
#[test]
fn lady_evangela_stops_one_creatures_combat_damage() {
    let mut e = game(
        2470,
        &[ids::lady_evangela(), ids::barbary_apes()],
        &[ids::barbary_apes()],
        &[],
        &[],
    );
    let mine = on_battlefield(&e, P0, ids::barbary_apes()).unwrap();
    let theirs = on_battlefield(&e, P1, ids::barbary_apes()).unwrap();
    attack(&mut e, &[mine]);
    block(&mut e, &[(theirs, mine)]);
    to_step(&mut e, P0, Step::DeclareBlockers);
    float(&mut e, P0, &[(ManaColor::White, 1), (ManaColor::Black, 1)]);
    use_ability(&mut e, P0, ids::lady_evangela(), 0, &[theirs], &[]);
    through_combat(&mut e);
    assert!(
        in_graveyard(&e, P1, ids::barbary_apes()).is_some(),
        "my Apes hit the blocker"
    );
    assert!(
        on_battlefield(&e, P0, ids::barbary_apes()).is_some(),
        "the blocker's damage was prevented"
    );
}

/// Kei Takahashi: "{T}: Prevent the next 2 damage that would be dealt to
/// target creature this turn."
#[test]
fn kei_takahashi_prevents_the_next_two_damage() {
    let mut e = game(
        2480,
        &[ids::kei_takahashi(), ids::barbary_apes()],
        &[],
        &[lightning_bolt()],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    use_ability(&mut e, P0, ids::kei_takahashi(), 0, &[apes], &[]);
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_at(&mut e, P0, lightning_bolt(), &[apes], &[]);
    assert!(
        on_battlefield(&e, P0, ids::barbary_apes()).is_some(),
        "3 - 2 = 1 damage"
    );
    assert_eq!(damage_on(&e, apes), 1);
}

/// The regenerators: Walking Dead ({B}), Ragnar ({G}{W}{U},{T}, target
/// creature): the next time it would be destroyed, it is tapped, damage
/// removed, and it stays.
#[test]
fn walking_dead_and_ragnar_regenerate_through_lethal_damage() {
    // Walking Dead regenerates itself.
    let mut e = game(2490, &[ids::walking_dead()], &[], &[lightning_bolt()], &[]);
    let wd = obj(&e, P0, ids::walking_dead());
    float(&mut e, P0, &[(ManaColor::Black, 1)]);
    use_ability(&mut e, P0, ids::walking_dead(), 0, &[], &[]);
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_at(&mut e, P0, lightning_bolt(), &[wd], &[]);
    assert!(
        on_battlefield(&e, P0, ids::walking_dead()).is_some(),
        "regenerated"
    );
    assert!(tapped(&e, wd));
    assert_eq!(damage_on(&e, wd), 0);

    // Control: with no shield it dies.
    let mut e = game(2491, &[ids::walking_dead()], &[], &[lightning_bolt()], &[]);
    let wd = obj(&e, P0, ids::walking_dead());
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_at(&mut e, P0, lightning_bolt(), &[wd], &[]);
    assert!(in_graveyard(&e, P0, ids::walking_dead()).is_some());

    // Ragnar regenerates a target creature.
    let mut e = game(
        2492,
        &[ids::ragnar(), ids::barbary_apes()],
        &[],
        &[lightning_bolt()],
        &[],
    );
    let apes = obj(&e, P0, ids::barbary_apes());
    float(
        &mut e,
        P0,
        &[
            (ManaColor::Green, 1),
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
        ],
    );
    use_ability(&mut e, P0, ids::ragnar(), 0, &[apes], &[]);
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_at(&mut e, P0, lightning_bolt(), &[apes], &[]);
    assert!(
        on_battlefield(&e, P0, ids::barbary_apes()).is_some(),
        "regenerated"
    );
    assert!(tapped(&e, apes));
}

/// Cyclopean Mummy: "When this creature dies, exile it."
#[test]
fn cyclopean_mummy_is_exiled_when_it_dies() {
    let mut e = game(
        2500,
        &[ids::cyclopean_mummy()],
        &[],
        &[lightning_bolt()],
        &[],
    );
    let mummy = obj(&e, P0, ids::cyclopean_mummy());
    float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_at(&mut e, P0, lightning_bolt(), &[mummy], &[]);
    assert!(in_graveyard(&e, P0, ids::cyclopean_mummy()).is_none());
    assert!(on_battlefield(&e, P0, ids::cyclopean_mummy()).is_none());
    assert_eq!(zone_of(&e, mummy), Zone::Exile);
}

/// Abomination: "Whenever this creature blocks or becomes blocked by a green
/// or white creature, destroy that creature at end of combat."
#[test]
fn abomination_destroys_green_and_white_creatures_it_fights_at_end_of_combat() {
    // Blocking a green attacker that survives the damage.
    let mut e = game(
        2510,
        &[ids::moss_monster()],
        &[ids::abomination()],
        &[],
        &[],
    );
    let (moss, abom) = (
        obj(&e, P0, ids::moss_monster()),
        obj(&e, P1, ids::abomination()),
    );
    attack(&mut e, &[moss]);
    block(&mut e, &[(abom, moss)]);
    through_combat(&mut e);
    assert!(
        in_graveyard(&e, P0, ids::moss_monster()).is_some(),
        "destroyed at end of combat"
    );
    assert!(on_battlefield(&e, P1, ids::abomination()).is_some());

    // Becoming blocked by a white creature that survives the damage.
    let mut e = game(
        2511,
        &[ids::abomination()],
        &[ids::keepers_of_the_faith()],
        &[],
        &[],
    );
    let (abom, keepers) = (
        obj(&e, P0, ids::abomination()),
        obj(&e, P1, ids::keepers_of_the_faith()),
    );
    attack(&mut e, &[abom]);
    block(&mut e, &[(keepers, abom)]);
    through_combat(&mut e);
    assert!(in_graveyard(&e, P1, ids::keepers_of_the_faith()).is_some());

    // A red creature is not touched.
    let mut e = game(
        2512,
        &[ids::mountain_yeti()],
        &[ids::abomination()],
        &[],
        &[],
    );
    let (yeti, abom) = (
        obj(&e, P0, ids::mountain_yeti()),
        obj(&e, P1, ids::abomination()),
    );
    attack(&mut e, &[yeti]);
    block(&mut e, &[(abom, yeti)]);
    through_combat(&mut e);
    assert!(
        on_battlefield(&e, P0, ids::mountain_yeti()).is_some(),
        "red survives"
    );
}

/// Infernal Medusa: "Whenever this creature blocks a creature, destroy that
/// creature at end of combat. Whenever this creature becomes blocked by a
/// non-Wall creature, destroy that creature at end of combat."
#[test]
fn infernal_medusa_destroys_what_it_blocks_and_non_wall_blockers() {
    let mut e = game(
        2520,
        &[ids::moss_monster()],
        &[ids::infernal_medusa()],
        &[],
        &[],
    );
    let (moss, medusa) = (
        obj(&e, P0, ids::moss_monster()),
        obj(&e, P1, ids::infernal_medusa()),
    );
    attack(&mut e, &[moss]);
    block(&mut e, &[(medusa, moss)]);
    through_combat(&mut e);
    assert!(
        in_graveyard(&e, P0, ids::moss_monster()).is_some(),
        "what it blocked"
    );

    let mut e = game(
        2521,
        &[ids::infernal_medusa()],
        &[ids::keepers_of_the_faith()],
        &[],
        &[],
    );
    let (medusa, keepers) = (
        obj(&e, P0, ids::infernal_medusa()),
        obj(&e, P1, ids::keepers_of_the_faith()),
    );
    attack(&mut e, &[medusa]);
    block(&mut e, &[(keepers, medusa)]);
    through_combat(&mut e);
    assert!(
        in_graveyard(&e, P1, ids::keepers_of_the_faith()).is_some(),
        "a non-Wall blocker"
    );

    let mut e = game(
        2522,
        &[ids::infernal_medusa()],
        &[ids::wall_of_light()],
        &[],
        &[],
    );
    let (medusa, wall) = (
        obj(&e, P0, ids::infernal_medusa()),
        obj(&e, P1, ids::wall_of_light()),
    );
    attack(&mut e, &[medusa]);
    block(&mut e, &[(wall, medusa)]);
    through_combat(&mut e);
    assert!(
        on_battlefield(&e, P1, ids::wall_of_light()).is_some(),
        "a Wall is spared"
    );
}

/// Hyperion Blacksmith: "{T}: You may tap or untap target artifact an
/// opponent controls."
#[test]
fn hyperion_blacksmith_taps_an_opponents_artifact() {
    let mut e = game(
        2530,
        &[ids::hyperion_blacksmith(), ids::black_mana_battery()],
        &[ids::white_mana_battery()],
        &[],
        &[],
    );
    let (mine, theirs) = (
        obj(&e, P0, ids::black_mana_battery()),
        obj(&e, P1, ids::white_mana_battery()),
    );
    activate(&mut e, P0, ids::hyperion_blacksmith(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("a target was expected, got {:?}", e.pending());
    };
    assert!(options.contains(&theirs));
    assert!(
        !options.contains(&mine),
        "only artifacts an opponent controls"
    );
    aim(&mut e, &[theirs], &[]);
    drive(&mut e, &[], true, 0);
    settle(&mut e);
    assert!(tapped(&e, theirs), "the opponent's artifact is tapped");
}
