//! Package 5 behavioral tests for the 1,296 non-triggered `Coverage::Generated`
//! Creature cards (plus Thor Odinson's double Prowess).
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
use baylee_core::ids::SubtypeId;

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
pub(super) struct CreatureSpec {
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
    pub(super) choose_color: bool,
    pub(super) copy_on_enter: bool,
    pub(super) expected_counters: Option<(CounterKind, u16)>,
    pub(super) expected_keywords: KeywordSet,
    pub(super) prowess_bonus: i16,
    pub(super) has_opponent_enter_tapped_replacement: bool,
    pub(super) has_untap_self: bool,
    pub(super) cycling_mana: Option<&'static [(ManaColor, u32)]>,
    pub(super) lord_subtype: Option<SubtypeId>,
    pub(super) lord_pt: Option<(i16, i16)>,
    pub(super) lord_controlled_by_you: bool,
}

#[allow(clippy::too_many_lines)]
fn build_creature_board(spec: &CreatureSpec) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
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

    let my_goblin = p0_bf[0];
    let their_goblin = p1_bf[0];

    if let Some(sub) = spec.lord_subtype {
        let st = engine.dev_state_mut(p0).unwrap();
        std::sync::Arc::make_mut(&mut st.object_mut(my_goblin).unwrap().base)
            .subtypes
            .insert(sub);
        std::sync::Arc::make_mut(&mut st.object_mut(their_goblin).unwrap().base)
            .subtypes
            .insert(sub);
    }

    let p0_to_bury = &p0_bf[p0_bf.len() - 4..];
    let p1_to_bury = &p1_bf[p1_bf.len() - 1..];
    bury(&mut engine, p0_to_bury);
    bury(&mut engine, p1_to_bury);
    engine.refresh_offer();
    (engine, my_goblin, their_goblin)
}

fn drive_creature_to_rest(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> Rest {
    let mut r = drive_to_rest(engine, seat);
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
    }
    r
}

#[track_caller]
#[allow(clippy::too_many_lines)]
pub(super) fn verify_creature(spec: &CreatureSpec) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let (mut engine, my_goblin, their_goblin) = build_creature_board(spec);
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

    // 3. Colorless {{C}} pip check.
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
        let (mut cyc_eng, _, _) = build_creature_board(spec);
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
        assert_eq!(
            library_size(&cyc_eng, p0),
            lib_before - 1,
            "Cycling must draw 1 card"
        );
    }

    // 5. Cast from hand for printed mana cost.
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
        let chosen_types = engine
            .state()
            .object(chosen)
            .unwrap()
            .characteristics()
            .types;
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
        assert!(
            engine
                .state()
                .object(spell_obj)
                .unwrap()
                .characteristics()
                .types
                .contains(chosen_types),
            "CopyOnEnter creature adopts the copied permanent's types"
        );
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
        "Creature resolution must reach rest cleanly, got {rest:?}"
    );

    let perm_obj =
        on_battlefield(&engine, p0, spec.card).expect("Creature resolved onto battlefield");
    assert_eq!(
        is_tapped(&engine, perm_obj),
        spec.enters_tapped,
        "enters_tapped state must match Scryfall Oracle text"
    );
    assert_eq!(
        keywords(&engine, perm_obj).bits() & spec.expected_keywords.bits(),
        spec.expected_keywords.bits(),
        "battlefield keywords must include printed keywords"
    );

    if let Some((kind, count)) = spec.expected_counters {
        assert_eq!(
            engine.state().object(perm_obj).unwrap().counters.get(kind),
            count,
            "entry counters must match Scryfall Oracle text"
        );
    }

    let actual_pt = pt(&engine, perm_obj);
    if let Some(exp_pt) = spec.expected_pt {
        assert_eq!(
            actual_pt, exp_pt,
            "projected battlefield P/T must match printed P/T + entry counters"
        );
    } else {
        assert!(
            actual_pt.1 >= 1,
            "CDA or self-modifying creature must survive on battlefield with positive toughness, got {actual_pt:?}"
        );
    }

    // 6. Prowess trigger verification (CR 702.108, including double prowess on Thor Odinson).
    if spec.prowess_bonus > 0 {
        let before_pt = pt(&engine, perm_obj);
        float(&mut engine, p0, &[(ManaColor::Blue, 1)]);
        let opt_id = in_hand(&engine, p0, opt_spell()).expect("opt in hand");
        engine
            .apply(p0, PlayerAction::CastSpell { card: opt_id })
            .expect("Opt casts to trigger prowess");
        let r_prow = drive_creature_to_rest(&mut engine, p0);
        assert!(matches!(r_prow, Rest::Reached | Rest::Over));
        assert_eq!(
            pt(&engine, perm_obj),
            (
                before_pt.0 + spec.prowess_bonus,
                before_pt.1 + spec.prowess_bonus
            ),
            "Prowess must boost creature P/T when controller casts a noncreature spell"
        );
    }

    // 7. Loxodon Gatekeeper opponent enters-tapped replacement check.
    if spec.has_opponent_enter_tapped_replacement {
        reach_their_main_phase(&mut engine, p1);
        float(&mut engine, p1, &[(ManaColor::Red, 2)]);
        let p1_gob_spell = in_hand(&engine, p1, swab_goblin()).expect("swab_goblin in p1 hand");
        engine
            .apply(p1, PlayerAction::CastSpell { card: p1_gob_spell })
            .expect("p1 casts swab_goblin");
        let r_rep = drive_creature_to_rest(&mut engine, p1);
        assert!(matches!(r_rep, Rest::Reached | Rest::Over));
        assert!(
            is_tapped(&engine, p1_gob_spell),
            "Opponent creature must enter tapped under Loxodon Gatekeeper"
        );
        reach_their_main_phase(&mut engine, p0);
    }

    // 8. Lord static ability P/T boost and controller/opponent check.
    if let Some((dp, dt)) = spec.lord_pt {
        assert_eq!(
            pt(&engine, my_goblin),
            (2 + dp, 2 + dt),
            "Lord static ability must modify controller's matching creature P/T"
        );
        if spec.lord_controlled_by_you {
            assert_eq!(
                pt(&engine, their_goblin),
                (2, 2),
                "ControlledByYou lord must not modify opponent's creature P/T"
            );
        } else {
            assert_eq!(
                pt(&engine, their_goblin),
                (2 + dp, 2 + dt),
                "Symmetrical lord must also modify opponent's matching creature P/T"
            );
        }
    }

    // 9. UntapSelf ({{Q}}) cost verification on the resolved creature.
    if spec.has_untap_self {
        let st = engine.dev_state_mut(p0).unwrap();
        st.object_mut(perm_obj).unwrap().controlled_since = 0;
        st.set_tapped(perm_obj, true);
        engine.refresh_offer();
        float(
            &mut engine,
            p0,
            &[
                (ManaColor::White, 5),
                (ManaColor::Blue, 5),
                (ManaColor::Black, 5),
                (ManaColor::Red, 5),
                (ManaColor::Green, 5),
                (ManaColor::Colorless, 5),
            ],
        );
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected Priority");
        };
        let q_idx = legal
            .abilities
            .iter()
            .find_map(|&(src, idx)| (src == perm_obj).then_some(idx))
            .expect("UntapSelf ability must be offered when creature is tapped without summoning sickness");
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: perm_obj,
                    ability_index: q_idx,
                },
            )
            .expect("UntapSelf ability activates");
        let r_q = drive_creature_to_rest(&mut engine, p0);
        assert!(matches!(r_q, Rest::Reached | Rest::Over));
        assert!(
            !is_tapped(&engine, perm_obj),
            "UntapSelf cost must untap the creature"
        );
    }

    // 10. Hybrid / Twobrid alternative mana payment check.
    if let Some(alt) = spec.alt_mana {
        let (mut alt_engine, _, _) = build_creature_board(spec);
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

#[test]
fn thor_odinson_double_prowess_triggers_twice_per_noncreature_spell() {
    // Thor Odinson — {3}{R}{W} — Legendary Creature — God Warrior Hero (4/4)
    // Flying, vigilance, prowess, prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn twice.)
    let p0 = PlayerId::new(0);
    let thor = card_index("ec9b74d0-e4fe-4764-b628-992a62f2a1cf");

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thor])
        .hand(0, &[swab_goblin(), opt_spell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let thor_id = on_battlefield(&engine, p0, thor).expect("Thor on battlefield");
    assert_eq!(pt(&engine, thor_id), (4, 4));

    // Casting a creature spell (Swab Goblin) must not trigger prowess.
    float(&mut engine, p0, &[(ManaColor::Red, 2)]);
    let gob_id = in_hand(&engine, p0, swab_goblin()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: gob_id })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, thor_id),
        (4, 4),
        "Creature spell must not trigger Prowess"
    );

    // Casting a noncreature spell (Opt) must trigger Prowess twice (+2/+2 -> 6/6).
    float(&mut engine, p0, &[(ManaColor::Blue, 1)]);
    let opt_id = in_hand(&engine, p0, opt_spell()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: opt_id })
        .unwrap();
    let r = drive_creature_to_rest(&mut engine, p0);
    assert!(matches!(r, Rest::Reached | Rest::Over));
    assert_eq!(
        pt(&engine, thor_id),
        (6, 6),
        "Double prowess on Thor Odinson must grant +2/+2 until end of turn"
    );
}

#[test]
fn silhana_starfletcher_taps_for_chosen_entry_color() {
    // Silhana Starfletcher — {2}{G} — Creature — Elf Druid Archer (1/3)
    // Reach; As this creature enters, choose a color; {T}: Add one mana of the chosen color.
    let p0 = PlayerId::new(0);
    let starfletcher = card_index("3b4b6af2-f929-4082-a5ae-61a4e074bf14");

    let mut engine = Duel::new(SEED, forest()).hand(0, &[starfletcher]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
    );
    let spell_id = in_hand(&engine, p0, starfletcher).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell_id })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("choose Blue as Silhana Starfletcher enters");
    pass_until(&mut engine, stack_is_empty);

    let perm_id = on_battlefield(&engine, p0, starfletcher).unwrap();
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(perm_id)
        .unwrap()
        .controlled_since = 0;
    engine.refresh_offer();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: perm_id,
                ability_index: 0,
            },
        )
        .expect("Silhana Starfletcher taps for mana");
    assert_eq!(
        pool_of(&engine, p0, ManaColor::Blue),
        1,
        "Silhana Starfletcher must produce 1 mana of the color chosen as it entered"
    );
}

#[test]
fn order_of_whiteclay_untaps_to_reanimate_small_creature_from_your_graveyard() {
    // Order of Whiteclay — {1}{W}{W} — Creature — Kithkin Cleric (1/4)
    // {1}{W}{W}, {Q}: Return target creature card with mana value 3 or less from your graveyard to the battlefield.
    let p0 = PlayerId::new(0);
    let whiteclay = card_index("16c46eaf-7a31-4957-ab7c-ff07fbbd9ddc");

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[whiteclay, acolyte_of_xathrid(), cloudcrown_oak()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wc_id = on_battlefield(&engine, p0, whiteclay).unwrap();
    let acolyte_id = on_battlefield(&engine, p0, acolyte_of_xathrid()).unwrap();
    let oak_id = on_battlefield(&engine, p0, cloudcrown_oak()).unwrap();
    bury(&mut engine, &[acolyte_id, oak_id]);

    engine.dev_state_mut(p0).unwrap().set_tapped(wc_id, true);
    engine.refresh_offer();

    float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 2)],
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wc_id,
                ability_index: 0,
            },
        )
        .expect("Order of Whiteclay activates when tapped");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        options.contains(&acolyte_id) && !options.contains(&oak_id),
        "MV 1 Acolyte of Xathrid must be offered and MV 4 Cloudcrown Oak must be excluded"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![acolyte_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, wc_id),
        "Order of Whiteclay untaps to pay {{Q}}"
    );
    assert!(
        on_battlefield(&engine, p0, acolyte_of_xathrid()).is_some(),
        "Acolyte of Xathrid returns from graveyard to battlefield"
    );
}
