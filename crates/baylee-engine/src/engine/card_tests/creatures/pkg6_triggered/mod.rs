//! Package 6 behavioral tests for the 1,309 triggered `Coverage::Generated`
//! Creature cards.
//!
//! Every test is derived from Scryfall Oracle text, mana cost, type line,
//! power/toughness, and colors, exercising regular gameplay actions.

#[allow(clippy::wildcard_imports)]
use super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::testkit::{Deed, Rest, arena, drive_to_rest, presses};
#[allow(unused_imports)]
use baylee_cards_dsl::{CounterKind, counters};
use baylee_core::color::{Color, ColorSet};
#[allow(unused_imports)]
use baylee_core::generated::subtypes;

mod part_01;
mod part_02;
mod part_03;
mod part_04;
mod part_05;
mod part_06;
mod part_07;
mod part_08;
mod part_09;
mod part_10;
mod part_11;
mod part_12;
mod part_13;
mod part_14;
mod part_15;
mod part_16;
mod part_17;
mod part_18;
mod part_19;
mod part_20;
mod part_21;
mod part_22;
mod part_23;
mod part_24;
mod part_25;
mod part_26;
mod part_27;
mod part_28;
mod part_29;
mod part_30;
mod part_31;
mod part_32;
mod part_33;
mod part_34;
mod part_35;
mod part_36;
mod part_37;
mod part_38;
mod part_39;
mod part_40;
mod part_41;
mod part_42;
mod part_43;
mod part_44;

fn bone_saw() -> CardIndex {
    card_index("16e555f2-5aa8-4100-a036-eed48db0e84a")
}
fn circle_of_protection_red() -> CardIndex {
    card_index("df2738fe-9cd1-4347-8808-105fcfde1190")
}
fn snow_covered_forest() -> CardIndex {
    card_index("5f0d3be8-e63e-4ade-ae58-6b0c14f2ce6d")
}
fn acolyte_of_xathrid() -> CardIndex {
    card_index("62dca41b-58b0-45eb-ada7-568e80a2abec")
}
fn dedicated_martyr() -> CardIndex {
    card_index("ca39af65-985d-4ab1-874b-9e8d3455c349")
}
fn esper_battlemage() -> CardIndex {
    card_index("2551f383-7308-4670-9fda-406b20e372a5")
}
fn cloudcrown_oak() -> CardIndex {
    card_index("63c52071-e3a7-4236-9dcb-e5ea68240dc2")
}
fn swab_goblin() -> CardIndex {
    card_index("343ecf2f-0bbc-4840-8fb3-f5f61b26e76f")
}
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn opt_spell() -> CardIndex {
    card_index("713332c1-5bd8-400f-bfff-c1ca0697a043")
}
fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}

#[allow(clippy::struct_excessive_bools, dead_code)]
pub(super) struct TriggeredCreatureSpec {
    pub(super) card: CardIndex,
    pub(super) primary_mana: &'static [(ManaColor, u32)],
    pub(super) alt_mana: Option<&'static [(ManaColor, u32)]>,
    pub(super) total_mana: u32,
    pub(super) colorless_pips: u32,
    pub(super) snow_pips: u32,
    pub(super) has_x: bool,
    pub(super) colors: &'static [Color],
    pub(super) is_artifact: bool,
    pub(super) is_enchantment: bool,
    pub(super) is_legendary: bool,
    pub(super) is_snow: bool,
    pub(super) base_pt: (i16, i16),
    pub(super) expected_pt: Option<(i16, i16)>,
    pub(super) enters_tapped: bool,
    pub(super) etb_changes_tapped: bool,
    pub(super) choose_color: bool,
    pub(super) copy_on_enter: bool,
    pub(super) expected_counters: Option<(CounterKind, u16)>,
    pub(super) expected_keywords: KeywordSet,
    pub(super) has_enters_other: bool,
    pub(super) has_spell_cast: bool,
    pub(super) has_tapped_trigger: bool,
    pub(super) has_dies_trigger: bool,
    pub(super) cycling_mana: Option<&'static [(ManaColor, u32)]>,
}

#[allow(clippy::too_many_lines)]
fn build_triggered_board(spec: &TriggeredCreatureSpec) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bf0 = vec![
        swab_goblin(),
        llanowar_elves(),
        acolyte_of_xathrid(),
        dedicated_martyr(),
        esper_battlemage(),
        cloudcrown_oak(),
        ondu_cleric(),
        bone_saw(),
        circle_of_protection_red(),
        holy_strength(),
        plains(),
        island(),
        swamp(),
        mountain(),
        forest(),
        snow_covered_forest(),
        // Graveyard fodder to bury after reaching main phase (3 creatures + 1 artifact)
        acolyte_of_xathrid(),
        swab_goblin(),
        dedicated_martyr(),
        bone_saw(),
    ];
    let bf1 = vec![
        swab_goblin(),
        acolyte_of_xathrid(),
        bone_saw(),
        circle_of_protection_red(),
        plains(),
        forest(),
        // Graveyard fodder for p1 (1 creature)
        acolyte_of_xathrid(),
    ];
    let mut engine = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                spec.card,
                opt_spell(),
                swab_goblin(),
                forest(),
                plains(),
                bone_saw(),
                circle_of_protection_red(),
            ],
        )
        .hand(1, &[swab_goblin(), forest(), plains()])
        .battlefield(0, &bf0)
        .battlefield(1, &bf1)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Seed library top with basic land, artifact, enchantment, and creature for search abilities.
    for lib_card in [plains(), forest(), bone_saw(), circle_of_protection_red()] {
        hand_to_library_top(&mut engine, p0, lib_card);
    }

    // Attach holy_strength to cloudcrown_oak.
    if let (Some(aura_id), Some(oak_id)) = (
        on_battlefield(&engine, p0, holy_strength()),
        on_battlefield(&engine, p0, cloudcrown_oak()),
    ) {
        engine
            .dev_state_mut(p0)
            .unwrap()
            .object_mut(aura_id)
            .unwrap()
            .attached_to = Some(oak_id);
    }

    let p0_bf: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| engine.state().object(*id).unwrap().controller == p0)
        .collect();
    let p1_bf: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| engine.state().object(*id).unwrap().controller == p1)
        .collect();

    let p0_to_bury = &p0_bf[p0_bf.len() - 4..];
    let p1_to_bury = &p1_bf[p1_bf.len() - 1..];
    bury(&mut engine, p0_to_bury);
    bury(&mut engine, p1_to_bury);
    engine.refresh_offer();
    engine
}

fn drive_creature_to_rest(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> Rest {
    let mut r = drive_to_rest(engine, seat);
    for _ in 0..4 {
        if matches!(r, Rest::Unanswered("ChooseDamageSource")) {
            let Pending::ChooseDamageSource {
                player,
                choice,
                options,
            } = engine.pending().clone()
            else {
                panic!("expected ChooseDamageSource");
            };
            let source = *options.first().expect("at least one damage source offered");
            engine
                .apply(player, PlayerAction::ChooseDamageSource { choice, source })
                .expect("damage source accepted");
            r = drive_to_rest(engine, seat);
        } else {
            break;
        }
    }
    r
}

#[track_caller]
#[allow(clippy::too_many_lines)]
pub(super) fn verify_triggered_creature(spec: &TriggeredCreatureSpec) {
    let p0 = PlayerId::new(0);

    let mut engine = build_triggered_board(spec);
    let spell_obj = in_hand(&engine, p0, spec.card).expect("creature card in hand");

    // 1. Verify card types, supertypes, base P/T, and colors (including Devoid).
    let hand_obj = engine.state().object(spell_obj).unwrap();
    assert_eq!(
        (
            hand_obj.base.power.unwrap_or(0),
            hand_obj.base.toughness.unwrap_or(0)
        ),
        spec.base_pt,
        "base power/toughness must match Scryfall Oracle text"
    );
    let chars = hand_obj.characteristics();
    assert!(
        chars.types.contains(TypeSet::CREATURE),
        "must have Creature type"
    );
    assert_eq!(
        chars.types.contains(TypeSet::ARTIFACT),
        spec.is_artifact,
        "Artifact type mismatch"
    );
    assert_eq!(
        chars.types.contains(TypeSet::ENCHANTMENT),
        spec.is_enchantment,
        "Enchantment type mismatch"
    );
    assert_eq!(
        chars.supertypes.contains(SupertypeSet::LEGENDARY),
        spec.is_legendary,
        "Legendary supertype mismatch"
    );
    assert_eq!(
        chars.supertypes.contains(SupertypeSet::SNOW),
        spec.is_snow,
        "Snow supertype mismatch"
    );
    assert_eq!(
        chars.colors,
        ColorSet::from_slice(spec.colors),
        "Creature colors must match Scryfall Oracle / Devoid rules"
    );

    // 2. Negative castability check with 0 mana in pool.
    if spec.total_mana > 0 && !spec.has_x {
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected Priority");
        };
        assert!(
            !legal.castable.contains(&spell_obj),
            "Creature with positive mana cost must not be castable with 0 mana"
        );
    }

    // 3. Colorless {C} pip check.
    if spec.colorless_pips > 0 {
        float(&mut engine, p0, &[(ManaColor::Red, spec.total_mana)]);
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected Priority");
        };
        assert!(
            !legal.castable.contains(&spell_obj),
            "Colored mana alone must not satisfy {{C}} colorless mana cost"
        );
        engine.dev_state_mut(p0).unwrap().players[0].mana_pool =
            baylee_core::mana::ManaPool::default();
        engine.refresh_offer();
    }

    // 4. Cycling / Hand activation check (CR 702.29a).
    if let Some(cyc_mana) = spec.cycling_mana {
        let mut cyc_eng = build_triggered_board(spec);
        let cyc_obj = in_hand(&cyc_eng, p0, spec.card).unwrap();
        let lib_before = library_size(&cyc_eng, p0);
        float(&mut cyc_eng, p0, cyc_mana);
        let Pending::Priority { legal, .. } = cyc_eng.pending().clone() else {
            panic!("expected Priority");
        };
        let cyc_idx = legal
            .abilities
            .iter()
            .find_map(|&(src, idx)| (src == cyc_obj).then_some(idx))
            .expect("Hand ability must be offered when cost is available");
        cyc_eng
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: cyc_obj,
                    ability_index: cyc_idx,
                },
            )
            .expect("Hand ability activates from hand");
        let r = drive_creature_to_rest(&mut cyc_eng, p0);
        assert!(matches!(r, Rest::Reached | Rest::Over));
        assert!(
            in_graveyard(&cyc_eng, p0, spec.card).is_some(),
            "Cycled creature card must be discarded to graveyard"
        );
        assert!(
            library_size(&cyc_eng, p0) < lib_before,
            "Cycling must draw at least 1 card"
        );
    }

    // 5. Cast from hand for printed mana cost (and resolve any ETB triggers).
    float(&mut engine, p0, spec.primary_mana);
    if spec.snow_pips > 0 {
        engine.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add_snow(ManaColor::Colorless, spec.snow_pips);
        engine.refresh_offer();
    }
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell_obj })
        .expect("Creature casts for its printed mana cost");

    if spec.has_x && matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    }

    if spec.copy_on_enter {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseTargets { .. })
        });
        let Pending::ChooseTargets { options, min, .. } = engine.pending().clone() else {
            panic!("expected CopyOnEnter ChooseTargets");
        };
        assert_eq!(min, 0, "CopyOnEnter is optional (may enter as a copy)");
        assert!(
            !options.is_empty(),
            "CopyOnEnter must offer legal copy choices on the battlefield"
        );
        let chosen = options[0];
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![chosen],
                    players: vec![],
                },
            )
            .expect("CopyOnEnter choice accepted");
        let rest = drive_creature_to_rest(&mut engine, p0);
        assert!(matches!(rest, Rest::Reached | Rest::Over));
        return;
    }

    if spec.choose_color {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseColor { .. })
        });
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
            .expect("ChooseColor accepted on entry");
    }

    let rest = drive_creature_to_rest(&mut engine, p0);
    assert!(
        matches!(rest, Rest::Reached | Rest::Over),
        "Creature resolution and ETB triggers must reach rest cleanly, got {rest:?}"
    );

    if let Some(perm_obj) = on_battlefield(&engine, p0, spec.card) {
        if !spec.etb_changes_tapped {
            assert_eq!(
                is_tapped(&engine, perm_obj),
                spec.enters_tapped,
                "enters_tapped state must match Scryfall Oracle text"
            );
        }
        assert_eq!(
            keywords(&engine, perm_obj).bits() & spec.expected_keywords.bits(),
            spec.expected_keywords.bits(),
            "battlefield keywords must include printed keywords"
        );

        let actual_pt = pt(&engine, perm_obj);
        if let Some(exp_pt) = spec.expected_pt {
            assert_eq!(
                actual_pt, exp_pt,
                "projected battlefield P/T must match printed P/T + entry counters"
            );
        } else {
            assert!(
                actual_pt.1 >= 1,
                "Triggered/self-modifying creature surviving on battlefield must have positive toughness, got {actual_pt:?}"
            );
        }

        // 6. Exercise Trigger::EntersBattlefield (landfall / alliance / creature-ETB).
        if spec.has_enters_other
            && matches!(engine.pending(), Pending::Priority { .. })
            && let Some(gob_hand) = in_hand(&engine, p0, swab_goblin())
        {
            float(&mut engine, p0, &[(ManaColor::Red, 2)]);
            if engine
                .apply(p0, PlayerAction::CastSpell { card: gob_hand })
                .is_ok()
            {
                let r_etb = drive_creature_to_rest(&mut engine, p0);
                assert!(matches!(r_etb, Rest::Reached | Rest::Over));
            }
        }

        // 7. Exercise Trigger::SpellCast.
        if spec.has_spell_cast
            && matches!(engine.pending(), Pending::Priority { .. })
            && let Some(opt_id) = in_hand(&engine, p0, opt_spell())
        {
            float(&mut engine, p0, &[(ManaColor::Blue, 1)]);
            if engine
                .apply(p0, PlayerAction::CastSpell { card: opt_id })
                .is_ok()
            {
                let r_sc = drive_creature_to_rest(&mut engine, p0);
                assert!(matches!(r_sc, Rest::Reached | Rest::Over));
            }
        }

        // 8. Exercise Trigger::BecomesTapped / Trigger::TappedForMana.
        if spec.has_tapped_trigger
            && let Pending::Priority { legal, .. } = engine.pending().clone()
            && let Some(&land_src) = legal.mana_abilities.first()
        {
            let _ = engine.apply(p0, PlayerAction::ActivateManaAbility { source: land_src });
            let r_tap = drive_creature_to_rest(&mut engine, p0);
            assert!(matches!(r_tap, Rest::Reached | Rest::Over));
        }

        // 9. Exercise Trigger::Dies.
        if spec.has_dies_trigger
            && let Some(still_here) = on_battlefield(&engine, p0, spec.card)
            && let Pending::Priority { player, .. } = engine.pending().clone()
        {
            bury(&mut engine, &[still_here]);
            let _ = engine.apply(player, PlayerAction::PassPriority);
            let r_dies = drive_creature_to_rest(&mut engine, p0);
            assert!(matches!(r_dies, Rest::Reached | Rest::Over));
        }
    }

    // 10. Hybrid / Twobrid alternative mana payment check.
    if let Some(alt) = spec.alt_mana {
        let mut alt_engine = build_triggered_board(spec);
        float(&mut alt_engine, p0, alt);
        let c_obj = in_hand(&alt_engine, p0, spec.card).unwrap();
        alt_engine
            .apply(p0, PlayerAction::CastSpell { card: c_obj })
            .expect("Hybrid/twobrid creature casts with alternative mana payment");
    }

    // 11. Exercise all offered abilities on the arena probe board (mana abilities, activated abilities, cycling).
    let seat = PlayerId::new(0);
    if let Some((eng_probe, objects)) = arena(spec.card)
        && let Pending::Priority { legal, .. } = eng_probe.pending().clone()
    {
        let all = presses(&legal, &objects);
        for (slot, press) in &all {
            if matches!(press, Deed::Cast) {
                continue;
            }
            let Some((mut eng, objs)) = arena(spec.card) else {
                continue;
            };
            let Some(object) = objs.get(*slot).copied() else {
                continue;
            };
            eng.apply(seat, press.action(object))
                .expect("offered ability must be legal to activate");
            let r = drive_creature_to_rest(&mut eng, seat);
            assert!(
                matches!(r, Rest::Reached | Rest::Over),
                "ability press must reach rest cleanly, got {r:?}"
            );
        }
    }
}

// ── Dedicated Regression Tests for Package 6 Triggered Creature Archetypes ──

#[test]
fn pitiless_plunderer_creates_treasure_when_another_creature_you_control_dies_not_itself_or_opponents()
 {
    // Pitiless Plunderer — {3}{B} — Creature — Human Pirate (1/4)
    // Whenever another creature you control dies, create a Treasure token.
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let plunderer = card_index("a784481f-eccb-4112-bb38-04a659319660");

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plunderer, acolyte_of_xathrid()])
        .battlefield(1, &[swab_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let count_treasures = |e: &Engine<RegistryLookup>| -> usize {
        e.state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter(|&&id| {
                let o = e.state().object(id).unwrap();
                o.controller == p0 && o.characteristics().types.contains(TypeSet::ARTIFACT)
            })
            .count()
    };
    assert_eq!(count_treasures(&engine), 0);

    // 1. Opponent's creature dying does NOT trigger Pitiless Plunderer
    let opp_goblin = on_battlefield(&engine, p1, swab_goblin()).unwrap();
    kill(&mut engine, opp_goblin);
    assert_eq!(
        count_treasures(&engine),
        0,
        "Opponent's creature dying must not trigger Pitiless Plunderer"
    );

    // 2. Another creature you control dying DOES create a Treasure token
    let my_acolyte = on_battlefield(&engine, p0, acolyte_of_xathrid()).unwrap();
    kill(&mut engine, my_acolyte);
    assert_eq!(
        count_treasures(&engine),
        1,
        "Another creature you control dying must create a Treasure token"
    );

    // 3. Pitiless Plunderer itself dying does NOT trigger its own 'another creature' ability
    let plund_id = on_battlefield(&engine, p0, plunderer).unwrap();
    kill(&mut engine, plund_id);
    assert_eq!(
        count_treasures(&engine),
        1,
        "Pitiless Plunderer dying itself must not trigger 'another creature you control dies'"
    );
}

#[test]
fn ripjaw_raptor_enrage_draws_a_card_when_dealt_nonlethal_damage() {
    // Ripjaw Raptor — {2}{G}{G} — Creature — Dinosaur (4/5)
    // Enrage — Whenever this creature is dealt damage, draw a card.
    let p0 = PlayerId::new(0);
    let raptor = card_index("80bb9ce4-1f3c-42ca-b75f-9a6b2f42073b");
    let shock = card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d");

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[raptor])
        .hand(0, &[shock])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let raptor_id = on_battlefield(&engine, p0, raptor).unwrap();
    let shock_id = in_hand(&engine, p0, shock).unwrap();
    let hand_before = hand_size(&engine, p0);

    float(&mut engine, p0, &[(ManaColor::Red, 1)]);
    engine
        .apply(p0, PlayerAction::CastSpell { card: shock_id })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![raptor_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Hand had 1 card (Shock), cast it (hand_before - 1), Enrage drew 1 card (back to hand_before)
    assert_eq!(
        hand_size(&engine, p0),
        hand_before,
        "Ripjaw Raptor's Enrage trigger must draw a card when dealt 2 damage by Shock"
    );
}

#[test]
fn ultimecia_temporal_threat_taps_only_opponent_creatures_on_etb() {
    // Ultimecia, Temporal Threat — {4}{U}{U} — Legendary Creature — Human Warlock (4/4)
    // When Ultimecia enters, tap all creatures your opponents control.
    // Whenever a creature you control deals combat damage to a player, draw a card.
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let ultimecia = card_index("5a8e7e35-e9ea-4808-8aa7-7580a23b9d9d");

    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[ultimecia])
        .battlefield(0, &[swab_goblin()])
        .battlefield(1, &[cloudcrown_oak()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_goblin = on_battlefield(&engine, p0, swab_goblin()).unwrap();
    let opp_oak = on_battlefield(&engine, p1, cloudcrown_oak()).unwrap();
    assert!(!is_tapped(&engine, my_goblin));
    assert!(!is_tapped(&engine, opp_oak));

    float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 4), (ManaColor::Blue, 2)],
    );
    let ult_hand = in_hand(&engine, p0, ultimecia).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: ult_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, opp_oak),
        "Ultimecia ETB must tap opponent's creatures"
    );
    assert!(
        !is_tapped(&engine, my_goblin),
        "Ultimecia ETB must not tap your own creatures"
    );
}

#[test]
fn covetous_dragon_state_trigger_sacrifices_itself_when_last_artifact_leaves() {
    // Covetous Dragon — {4}{R} — Creature — Dragon (6/5)
    // Flying
    // When you control no artifacts, sacrifice this creature.
    let p0 = PlayerId::new(0);
    let dragon = card_index("bf7710cb-70cb-42fb-b840-cc4f385daa7a");

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dragon, bone_saw()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // While Bone Saw is controlled by p0, Covetous Dragon stays on the battlefield
    assert!(on_battlefield(&engine, p0, dragon).is_some());

    // Destroy Bone Saw -> state trigger fires and sacrifices Covetous Dragon
    let saw_id = on_battlefield(&engine, p0, bone_saw()).unwrap();
    kill(&mut engine, saw_id);
    assert!(
        on_battlefield(&engine, p0, dragon).is_none(),
        "Covetous Dragon must sacrifice itself when you control no artifacts"
    );
}
