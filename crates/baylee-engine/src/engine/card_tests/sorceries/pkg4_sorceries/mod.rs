//! Package 4 Sorcery behavioral tests (363 cards).
//! Every test is derived from Scryfall Oracle text, mana cost, type line,
//! and colors, exercising regular gameplay actions.

#[allow(clippy::wildcard_imports)]
use super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::combat::{AttackerInfo, BlockerInfo};
use crate::engine::testkit::{Rest, drive_to_rest};
use crate::event::{Cause, GameEvent};
use crate::zone::ZonePosition;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::Defender;

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

fn bone_saw() -> CardIndex {
    card_index("16e555f2-5aa8-4100-a036-eed48db0e84a")
}
fn circle_of_protection_red() -> CardIndex {
    card_index("df2738fe-9cd1-4347-8808-105fcfde1190")
}
fn snow_covered_forest() -> CardIndex {
    card_index("5f0d3be8-e63e-4ade-ae58-6b0c14f2ce6d")
}
fn azorius_guildgate() -> CardIndex {
    card_index("ad1712d8-809f-410c-8b91-ffe6fb8a69a1")
}
fn adagia_windswept_bastion() -> CardIndex {
    card_index("70d35dbd-1d91-4a2a-a643-6870d168f4f5")
}
fn smuggler_s_copter() -> CardIndex {
    card_index("49136bdc-bc50-49a2-999a-1ef9c16ea130")
}
fn aminatou_the_fateshifter() -> CardIndex {
    card_index("3a30089d-cd2d-49be-9b06-7a2454117692")
}
fn chardalyn_dragon() -> CardIndex {
    card_index("0bf1f2c1-2148-4302-a6e7-d509a005a85f")
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
fn vampire_of_the_dire_moon() -> CardIndex {
    card_index("5a938371-8428-48e3-85ed-6a84a39b29c6")
}
fn slippery_bogle() -> CardIndex {
    card_index("f64878c1-1fba-44f7-a24f-d24bef2e03ae")
}
fn child_of_thorns() -> CardIndex {
    card_index("f94002a1-582f-4b9d-a4f7-dae816f21c1d")
}
fn excavated_wall() -> CardIndex {
    card_index("16e48b95-f030-400c-a2b7-c3f3111cc165")
}
fn gaea_s_skyfolk() -> CardIndex {
    card_index("dd0da475-55d0-462e-bfac-6eeb5d39a099")
}
fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}
fn opt_spell() -> CardIndex {
    card_index("713332c1-5bd8-400f-bfff-c1ca0697a043")
}
fn borne_upon_a_wind() -> CardIndex {
    card_index("ce19962d-94f9-4b2b-b668-963c0acce308")
}
fn aftershock() -> CardIndex {
    card_index("e6bef0c4-3dfc-49a8-8fec-22b2570b704b")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub(super) enum CounterTarget {
    Any,
    BlueInstantCmc1,
    InstantCmc2,
    RedSorceryCmc4,
    CreatureCmc2,
    ArtifactCmc1,
}

#[allow(clippy::struct_excessive_bools, dead_code)]
pub(super) struct SpellSpec {
    pub(super) card: CardIndex,
    pub(super) is_instant: bool,
    pub(super) is_kindred: bool,
    pub(super) primary_mana: &'static [(ManaColor, u32)],
    pub(super) alt_mana: Option<&'static [(ManaColor, u32)]>,
    pub(super) total_mana: u32,
    pub(super) colorless_pips: u32,
    pub(super) has_x: bool,
    pub(super) phyrexian_pips: u32,
    pub(super) colors: &'static [Color],
    pub(super) cycling_mana: Option<&'static [(ManaColor, u32)]>,
    pub(super) mode_count: usize,
    pub(super) choose_two: bool,
    pub(super) combat_target_only: bool,
    pub(super) targets_opponent_only: bool,
    pub(super) self_exile: bool,
    pub(super) is_trap_search: bool,
    pub(super) counter_target: Option<CounterTarget>,
    pub(super) expected_damage: Option<u32>,
    pub(super) expected_draw: Option<u16>,
    pub(super) expected_life_gain: Option<i32>,
    pub(super) expected_life_loss: Option<i32>,
    pub(super) expected_token: bool,
    pub(super) expected_destroy: bool,
    pub(super) expected_exile: bool,
    pub(super) expected_extra_turn: bool,
}

#[allow(clippy::too_many_lines)]
fn build_fixture_board(spec: &SpellSpec) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bf0 = vec![
        swab_goblin(),
        swab_goblin(),
        acolyte_of_xathrid(),
        dedicated_martyr(),
        esper_battlemage(),
        cloudcrown_oak(),
        vampire_of_the_dire_moon(),
        slippery_bogle(),
        child_of_thorns(),
        excavated_wall(),
        gaea_s_skyfolk(),
        chardalyn_dragon(),
        rootbreaker_wurm(),
        bone_saw(),
        bone_saw(),
        sol_ring(),
        smuggler_s_copter(),
        circle_of_protection_red(),
        holy_strength(),
        aminatou_the_fateshifter(),
        plains(),
        island(),
        swamp(),
        mountain(),
        forest(),
        forest(),
        azorius_guildgate(),
        adagia_windswept_bastion(),
        // Graveyard fodder to bury before main phase
        acolyte_of_xathrid(),
        swab_goblin(),
        child_of_thorns(),
        bone_saw(),
        circle_of_protection_red(),
        forest(),
    ];
    let bf1 = vec![
        swab_goblin(),
        swab_goblin(),
        acolyte_of_xathrid(),
        dedicated_martyr(),
        esper_battlemage(),
        cloudcrown_oak(),
        vampire_of_the_dire_moon(),
        child_of_thorns(),
        excavated_wall(),
        gaea_s_skyfolk(),
        chardalyn_dragon(),
        rootbreaker_wurm(),
        bone_saw(),
        sol_ring(),
        smuggler_s_copter(),
        circle_of_protection_red(),
        aminatou_the_fateshifter(),
        plains(),
        forest(),
        azorius_guildgate(),
        // Graveyard fodder for p1
        acolyte_of_xathrid(),
        forest(),
    ];
    let mut engine = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                spec.card,
                forest(),
                plains(),
                opt_spell(),
                aftershock(),
                snow_covered_forest(),
                azorius_guildgate(),
                circle_of_protection_red(),
                holy_strength(),
                bone_saw(),
                chardalyn_dragon(),
                aminatou_the_fateshifter(),
            ],
        )
        .hand(
            1,
            &[
                forest(),
                plains(),
                opt_spell(),
                borne_upon_a_wind(),
                aftershock(),
                swab_goblin(),
                sol_ring(),
            ],
        )
        .battlefield(0, &bf0)
        .battlefield(1, &bf1)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Put seeded library targets onto top of p0's library for SearchLibrary effects.
    for lib_card in [
        plains(),
        forest(),
        snow_covered_forest(),
        azorius_guildgate(),
        circle_of_protection_red(),
        holy_strength(),
        bone_saw(),
        chardalyn_dragon(),
        aminatou_the_fateshifter(),
    ] {
        hand_to_library_top(&mut engine, p0, lib_card);
    }
    if spec.is_trap_search {
        hand_to_library_top(&mut engine, p0, opt_spell());
        let top_id = *engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .first()
            .expect("top of library");
        std::sync::Arc::make_mut(
            &mut engine
                .dev_state_mut(p0)
                .unwrap()
                .object_mut(top_id)
                .unwrap()
                .base,
        )
        .subtypes
        .insert(subtypes::spell::TRAP);
    }

    // Attach holy_strength on p0's battlefield to cloudcrown_oak so it is a legal attached Aura.
    if let (Some(aura_id), Some(oak_id)) = (
        on_battlefield(&engine, p0, holy_strength()),
        on_battlefield(&engine, p0, cloudcrown_oak()),
    ) {
        let st = engine.dev_state_mut(p0).unwrap();
        st.object_mut(aura_id).unwrap().attached_to = Some(oak_id);
    }

    // Bury graveyard fodder for p0 (last 6 battlefield objects of p0) and p1 (last 2 of p1),
    // and move opt_spell & aftershock from p0's hand into p0's graveyard so Instant & Sorcery are in p0's graveyard.
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
    let p0_to_bury = &p0_bf[p0_bf.len() - 6..];
    let p1_to_bury = &p1_bf[p1_bf.len() - 2..];
    bury(&mut engine, p0_to_bury);
    bury(&mut engine, p1_to_bury);
    for spell_gy in [opt_spell(), aftershock()] {
        if let Some(id) = in_hand(&engine, p0, spell_gy) {
            let st = engine.dev_state_mut(p0).unwrap();
            st.move_object(
                id,
                ZoneLocation::Graveyard(p0),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
        }
    }

    // Tap one swab_goblin, one bone_saw, and one forest on p0's battlefield, and one swab_goblin on p1's battlefield.
    let st = engine.dev_state_mut(p0).unwrap();
    if let Some(gob2) = p0_bf.get(1).copied() {
        st.set_tapped(gob2, true);
    }
    if let Some(saw2) = p0_bf.get(14).copied() {
        st.set_tapped(saw2, true);
    }
    if let Some(for2) = p0_bf.get(25).copied() {
        st.set_tapped(for2, true);
    }
    if let Some(p1_gob2) = p1_bf.get(1).copied() {
        st.set_tapped(p1_gob2, true);
    }
    engine.refresh_offer();
    engine
}

fn enable_combat_statuses(engine: &mut Engine<RegistryLookup>) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let g0 = all_on_battlefield(engine, p0, swab_goblin())[0];
    let g1 = all_on_battlefield(engine, p1, swab_goblin())[0];
    let p0_cleric = on_battlefield(engine, p0, acolyte_of_xathrid()).unwrap();
    let p1_cleric = on_battlefield(engine, p1, acolyte_of_xathrid()).unwrap();
    let st = engine.dev_state_mut(p0).unwrap();
    st.combat.declare_attackers([
        AttackerInfo {
            creature: g0,
            defending: Defender::Player(p1),
            blocked: true,
            band: None,
        },
        AttackerInfo {
            creature: g1,
            defending: Defender::Player(p0),
            blocked: true,
            band: None,
        },
    ]);
    st.combat.blockers.push(BlockerInfo {
        blocker: p0_cleric,
        attacker: g1,
    });
    st.combat.blockers.push(BlockerInfo {
        blocker: p1_cleric,
        attacker: g0,
    });
    engine.refresh_offer();
}

fn drive_spell_to_rest(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> Rest {
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
pub(super) fn verify_spell(spec: &SpellSpec) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = build_fixture_board(spec);
    let spell_obj = in_hand(&engine, p0, spec.card).expect("spell card in hand");

    // 1. Verify card types and colors (including Devoid / colorless rules).
    let chars = engine.state().object(spell_obj).unwrap().characteristics();
    if spec.is_instant {
        assert!(
            chars.types.contains(TypeSet::INSTANT),
            "expected Instant type"
        );
    } else {
        assert!(
            chars.types.contains(TypeSet::SORCERY),
            "expected Sorcery type"
        );
    }
    if spec.is_kindred {
        assert!(
            chars.types.contains(TypeSet::KINDRED),
            "expected Kindred type"
        );
    }
    assert_eq!(
        chars.colors,
        ColorSet::from_slice(spec.colors),
        "spell colors must match Scryfall Oracle / Devoid rules"
    );

    // 2. Negative castability check with 0 mana in pool.
    if spec.total_mana > 0 && !spec.has_x && spec.phyrexian_pips < spec.total_mana {
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected Priority");
        };
        assert!(
            !legal.castable.contains(&spell_obj),
            "Spell with positive mana cost must not be castable with 0 mana"
        );
    }

    // 3. Colorless {C} pip check (e.g. Spatial Contortion {1}{C}).
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

    // 4. Cycling hand activation check (CR 702.29a).
    if let Some(cyc_mana) = spec.cycling_mana {
        let mut cyc_eng = build_fixture_board(spec);
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
            .expect("Cycling ability must be offered from hand when cycling cost is floated");
        cyc_eng
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: cyc_obj,
                    ability_index: cyc_idx,
                },
            )
            .expect("Cycling activates from hand");
        let r = drive_spell_to_rest(&mut cyc_eng, p0);
        assert!(matches!(r, Rest::Reached | Rest::Over));
        assert!(
            in_graveyard(&cyc_eng, p0, spec.card).is_some(),
            "Cycled card must be discarded to graveyard"
        );
        assert_eq!(
            library_size(&cyc_eng, p0),
            lib_before - 1,
            "Cycling must draw 1 card"
        );
    }

    // 5. Counterspell stack targeting and resolution check.
    if let Some(ct) = spec.counter_target {
        float(&mut engine, p0, spec.primary_mana);
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected Priority");
        };
        assert!(
            !legal.castable.contains(&spell_obj),
            "Counterspell must not be castable when stack is empty"
        );

        let (bait_card, bait_mana): (CardIndex, &[(ManaColor, u32)]) = match ct {
            CounterTarget::Any | CounterTarget::BlueInstantCmc1 => {
                (opt_spell(), &[(ManaColor::Blue, 1)])
            }
            CounterTarget::InstantCmc2 => (borne_upon_a_wind(), &[(ManaColor::Blue, 2)]),
            CounterTarget::RedSorceryCmc4 => (aftershock(), &[(ManaColor::Red, 4)]),
            CounterTarget::CreatureCmc2 => (swab_goblin(), &[(ManaColor::Red, 2)]),
            CounterTarget::ArtifactCmc1 => (sol_ring(), &[(ManaColor::Colorless, 1)]),
        };

        if matches!(
            ct,
            CounterTarget::RedSorceryCmc4
                | CounterTarget::CreatureCmc2
                | CounterTarget::ArtifactCmc1
        ) {
            reach_their_main_phase(&mut engine, p1);
        } else {
            engine.apply(p0, PlayerAction::PassPriority).unwrap();
        }
        float(&mut engine, p1, bait_mana);
        let bait_obj = in_hand(&engine, p1, bait_card).expect("bait in p1 hand");
        engine
            .apply(p1, PlayerAction::CastSpell { card: bait_obj })
            .expect("p1 casts bait spell");
        while let Pending::ChooseTargets {
            player, options, ..
        } = engine.pending().clone()
        {
            engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![options[0]],
                        players: vec![],
                    },
                )
                .unwrap();
        }
        if matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1) {
            engine.apply(p1, PlayerAction::PassPriority).unwrap();
        }

        float(&mut engine, p0, spec.primary_mana);
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell_obj })
            .expect("Counterspell casts in response to matching spell on stack");
        if spec.has_x && matches!(engine.pending(), Pending::ChooseNumber { .. }) {
            engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
        }
        if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
            engine
                .apply(p0, PlayerAction::ChooseMode(options[0].index as usize))
                .unwrap();
        }
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!(
                "expected counterspell ChooseTargets, got {:?}",
                engine.pending()
            );
        };
        assert!(
            options.contains(&bait_obj),
            "Matching spell on stack must be a legal target for counterspell"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bait_obj],
                    players: vec![],
                },
            )
            .expect("Counterspell targets bait spell");
        let r = drive_spell_to_rest(&mut engine, p0);
        assert!(
            matches!(r, Rest::Reached | Rest::Over),
            "Counterspell resolution must reach rest, got {r:?}"
        );
        assert!(
            on_stack(&engine, bait_card).is_none(),
            "Countered spell must leave the stack"
        );
        assert_ne!(
            engine.state().object(bait_obj).map(|o| o.zone),
            Some(Zone::Battlefield),
            "Countered permanent spell must not enter the battlefield"
        );
        return;
    }

    // 6. Combat-target-only negative & positive check.
    if spec.combat_target_only {
        float(&mut engine, p0, spec.primary_mana);
        let Pending::Priority { legal, .. } = engine.pending() else {
            panic!("expected Priority");
        };
        assert!(
            !legal.castable.contains(&spell_obj),
            "Combat-target spell must not be castable when no creature is attacking or blocking"
        );
        enable_combat_statuses(&mut engine);
    }

    // 7. Modal spell: exercise every castable mode branch.
    if spec.mode_count > 0 {
        let mut cast_modes_tested = 0;
        for mode_idx in 0..spec.mode_count {
            let mut m_eng = build_fixture_board(spec);
            enable_combat_statuses(&mut m_eng);
            float(&mut m_eng, p0, spec.primary_mana);
            let m_spell = in_hand(&m_eng, p0, spec.card).unwrap();
            m_eng
                .apply(p0, PlayerAction::CastSpell { card: m_spell })
                .expect("Modal spell casts");
            if spec.has_x && matches!(m_eng.pending(), Pending::ChooseNumber { .. }) {
                m_eng.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
            }
            if let Pending::ChooseCastMode { options, .. } = m_eng.pending().clone() {
                let Some(opt) = options.iter().find(|o| o.index as usize == mode_idx) else {
                    continue;
                };
                m_eng
                    .apply(p0, PlayerAction::ChooseMode(opt.index as usize))
                    .expect("mode choice accepted");
                if spec.choose_two
                    && let Pending::ChooseCastMode { options: opts2, .. } = m_eng.pending().clone()
                {
                    let Some(second) = opts2.first() else {
                        continue;
                    };
                    m_eng
                        .apply(p0, PlayerAction::ChooseMode(second.index as usize))
                        .expect("second mode choice accepted");
                }
            } else if mode_idx == 0 && spec.is_instant {
                // Only one mode was legal on an empty stack because mode 0 is a counterspell.
                // Put an artifact spell on the stack so both mode 0 and mode 1 are offered.
                let mut c_eng = build_fixture_board(spec);
                reach_their_main_phase(&mut c_eng, p1);
                float(&mut c_eng, p1, &[(ManaColor::Colorless, 1)]);
                let bait_obj = in_hand(&c_eng, p1, sol_ring()).expect("sol_ring in p1 hand");
                c_eng
                    .apply(p1, PlayerAction::CastSpell { card: bait_obj })
                    .expect("p1 casts sol_ring");
                if matches!(c_eng.pending(), Pending::Priority { player, .. } if *player == p1) {
                    c_eng.apply(p1, PlayerAction::PassPriority).unwrap();
                }
                float(&mut c_eng, p0, spec.primary_mana);
                let c_spell = in_hand(&c_eng, p0, spec.card).unwrap();
                c_eng
                    .apply(p0, PlayerAction::CastSpell { card: c_spell })
                    .expect("Modal spell casts in response to sol_ring");
                let Pending::ChooseCastMode { options, .. } = c_eng.pending().clone() else {
                    panic!("expected ChooseCastMode when spell is on stack");
                };
                let opt = options
                    .iter()
                    .find(|o| o.index == 0)
                    .expect("counter mode 0 offered when spell is on stack");
                c_eng
                    .apply(p0, PlayerAction::ChooseMode(opt.index as usize))
                    .expect("counter mode choice accepted");
                m_eng = c_eng;
            }
            let r = drive_spell_to_rest(&mut m_eng, p0);
            assert!(
                matches!(r, Rest::Reached | Rest::Over),
                "Modal spell mode {mode_idx} must resolve to rest, got {r:?}"
            );
            assert!(
                in_graveyard(&m_eng, p0, spec.card).is_some(),
                "Resolved modal spell moves to graveyard"
            );
            cast_modes_tested += 1;
        }
        assert!(
            cast_modes_tested >= 1,
            "At least one mode of ModalSpell must be castable and tested"
        );
        return;
    }

    // 8. Standard spell cast, target verification, and resolution.
    enable_combat_statuses(&mut engine);
    float(&mut engine, p0, spec.primary_mana);
    let mark = engine.journal().entries().len();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell_obj })
        .expect("Spell casts for its printed mana cost");

    if spec.has_x && matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    }
    if spec.phyrexian_pips > 0 && matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    }

    if spec.targets_opponent_only {
        let player_opts = match engine.pending().clone() {
            Pending::ChooseTargets { player_options, .. } => player_options,
            Pending::ChoosePlayer { options, .. } => options,
            other => panic!(
                "expected ChooseTargets or ChoosePlayer for opponent-targeting spell, got {other:?}"
            ),
        };
        assert_eq!(
            player_opts,
            vec![p1],
            "Target opponent spell must offer p1 and exclude caster p0"
        );
    }

    let r = drive_spell_to_rest(&mut engine, p0);
    assert!(
        matches!(r, Rest::Reached | Rest::Over),
        "Spell resolution must reach rest cleanly, got {r:?}"
    );

    if spec.self_exile {
        assert_eq!(
            engine.state().object(spell_obj).map(|o| o.zone),
            Some(Zone::Exile),
            "Self-exiling spell must move to Exile on resolution"
        );
    } else {
        assert!(
            in_graveyard(&engine, p0, spec.card).is_some(),
            "Resolved spell must move to caster's graveyard"
        );
    }

    let delta = &engine.journal().entries()[mark..];
    assert!(
        delta
            .iter()
            .any(|e| matches!(e.event, GameEvent::StackObjectResolved { .. })),
        "Journal must record StackObjectResolved for cast spell"
    );

    if let Some(dmg) = spec.expected_damage {
        assert!(
            delta
                .iter()
                .any(|e| matches!(e.event, GameEvent::DamageDealt { amount, .. } if amount == dmg)),
            "Expected DamageDealt({dmg}) from Scryfall Oracle text"
        );
    }
    if let Some(drw) = spec.expected_draw {
        assert!(
            delta
                .iter()
                .any(|e| matches!(e.event, GameEvent::CardsDrawn { count, .. } if count == drw)),
            "Expected CardsDrawn({drw}) from Scryfall Oracle text"
        );
    }
    if let Some(gain) = spec.expected_life_gain {
        assert!(
            delta.iter().any(
                |e| matches!(e.event, GameEvent::LifeChanged { old, new, .. } if new - old == gain)
            ),
            "Expected LifeChanged(+{gain}) from Scryfall Oracle text"
        );
    }
    if let Some(loss) = spec.expected_life_loss {
        assert!(
            delta.iter().any(
                |e| matches!(e.event, GameEvent::LifeChanged { old, new, .. } if old - new == loss)
            ),
            "Expected LifeChanged(-{loss}) from Scryfall Oracle text"
        );
    }
    if spec.expected_token {
        assert!(
            delta.iter().any(|e| matches!(
                e.event,
                GameEvent::ZoneChanged {
                    from: Zone::OutsideGame,
                    to: Zone::Battlefield,
                    ..
                }
            )),
            "Expected token creation from Scryfall Oracle text"
        );
    }
    if spec.expected_destroy {
        assert!(
            delta.iter().any(|e| matches!(
                e.event,
                GameEvent::ZoneChanged {
                    from: Zone::Battlefield,
                    to: Zone::Graveyard,
                    ..
                }
            )),
            "Expected permanent destruction from Scryfall Oracle text"
        );
    }
    if spec.expected_exile {
        assert!(
            delta.iter().any(|e| matches!(
                e.event,
                GameEvent::ZoneChanged {
                    to: Zone::Exile,
                    ..
                }
            )),
            "Expected exile event from Scryfall Oracle text"
        );
    }
    if spec.expected_extra_turn {
        assert!(
            !engine.state().extra_turns.is_empty(),
            "Expected extra turn queued from Scryfall Oracle text"
        );
    }

    // 9. Alternative hybrid / twobrid mana payment check.
    if let Some(alt) = spec.alt_mana {
        let mut alt_eng = build_fixture_board(spec);
        enable_combat_statuses(&mut alt_eng);
        float(&mut alt_eng, p0, alt);
        let c_obj = in_hand(&alt_eng, p0, spec.card).unwrap();
        alt_eng
            .apply(p0, PlayerAction::CastSpell { card: c_obj })
            .expect("Spell casts with alternative hybrid/twobrid mana");
    }

    // 10. Phyrexian life payment check.
    if spec.phyrexian_pips > 0 {
        let mut phy_eng = build_fixture_board(spec);
        enable_combat_statuses(&mut phy_eng);
        float(&mut phy_eng, p0, spec.primary_mana);
        let life_before = phy_eng.state().players[0].life;
        let c_obj = in_hand(&phy_eng, p0, spec.card).unwrap();
        phy_eng
            .apply(p0, PlayerAction::CastSpell { card: c_obj })
            .expect("Phyrexian spell casts");
        if spec.has_x && matches!(phy_eng.pending(), Pending::ChooseNumber { .. }) {
            phy_eng.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
        }
        if let Pending::ChooseNumber { max, .. } = phy_eng.pending().clone() {
            let pay_pips = max.min(spec.phyrexian_pips);
            phy_eng
                .apply(p0, PlayerAction::ChooseNumber(pay_pips))
                .expect("Phyrexian life payment accepted");
            let r2 = drive_spell_to_rest(&mut phy_eng, p0);
            assert!(matches!(r2, Rest::Reached | Rest::Over));
            assert!(
                phy_eng.state().players[0].life <= life_before - (2 * pay_pips as i32),
                "Paying Phyrexian mana with life must deduct 2 life per pip"
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn nature_s_spiral_call_to_mind_and_wargate_enforce_permanent_and_graveyard_owner_filters() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // 1. Nature's Spiral — Return target permanent card from your graveyard to your hand.
    let spiral = card_index("565922d6-8152-411f-8d4d-dc7c8523a49b");
    let mut eng = Duel::new(SEED, forest())
        .hand(0, &[spiral, opt_spell(), aftershock()])
        .battlefield(0, &[bone_saw()])
        .battlefield(1, &[bone_saw()])
        .start();
    keep_mulligans(&mut eng);
    reach_main_phase(&mut eng, p0);
    let p0_saw = on_battlefield(&eng, p0, bone_saw()).unwrap();
    let p1_saw = on_battlefield(&eng, p1, bone_saw()).unwrap();
    let p0_opt = in_hand(&eng, p0, opt_spell()).unwrap();
    let p0_aft = in_hand(&eng, p0, aftershock()).unwrap();
    bury(&mut eng, &[p0_saw, p1_saw]);
    for s in [p0_opt, p0_aft] {
        eng.dev_state_mut(p0)
            .unwrap()
            .move_object(
                s,
                ZoneLocation::Graveyard(p0),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
    }
    eng.refresh_offer();
    float(&mut eng, p0, &[(ManaColor::Green, 2)]);
    let sp_obj = in_hand(&eng, p0, spiral).unwrap();
    eng.apply(p0, PlayerAction::CastSpell { card: sp_obj })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = eng.pending().clone() else {
        panic!("expected ChooseTargets for Nature's Spiral");
    };
    assert!(
        options.contains(&p0_saw),
        "Nature's Spiral must offer permanent card in your graveyard"
    );
    assert!(
        !options.contains(&p0_opt),
        "Nature's Spiral must NOT offer Instant card"
    );
    assert!(
        !options.contains(&p0_aft),
        "Nature's Spiral must NOT offer Sorcery card"
    );
    assert!(
        !options.contains(&p1_saw),
        "Nature's Spiral must NOT offer opponent's graveyard permanent"
    );
    eng.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![p0_saw],
            players: vec![],
        },
    )
    .unwrap();
    assert!(matches!(drive_spell_to_rest(&mut eng, p0), Rest::Reached));
    assert!(in_hand(&eng, p0, bone_saw()).is_some());

    // 2. Call to Mind — Return target instant or sorcery card from your graveyard to your hand.
    let call_to_mind = card_index("72e2b883-a284-4ab6-8174-1ca289141a49");
    let mut eng2 = Duel::new(SEED, forest())
        .hand(0, &[call_to_mind, opt_spell(), aftershock()])
        .battlefield(0, &[bone_saw()])
        .start();
    keep_mulligans(&mut eng2);
    reach_main_phase(&mut eng2, p0);
    let saw2 = on_battlefield(&eng2, p0, bone_saw()).unwrap();
    let opt2 = in_hand(&eng2, p0, opt_spell()).unwrap();
    let aft2 = in_hand(&eng2, p0, aftershock()).unwrap();
    bury(&mut eng2, &[saw2]);
    for s in [opt2, aft2] {
        eng2.dev_state_mut(p0)
            .unwrap()
            .move_object(
                s,
                ZoneLocation::Graveyard(p0),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
    }
    eng2.refresh_offer();
    float(&mut eng2, p0, &[(ManaColor::Blue, 3)]);
    let ctm_obj = in_hand(&eng2, p0, call_to_mind).unwrap();
    eng2.apply(p0, PlayerAction::CastSpell { card: ctm_obj })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = eng2.pending().clone() else {
        panic!("expected ChooseTargets for Call to Mind");
    };
    assert!(
        options.contains(&opt2),
        "Call to Mind must offer Instant in your graveyard"
    );
    assert!(
        options.contains(&aft2),
        "Call to Mind must offer Sorcery in your graveyard"
    );
    assert!(
        !options.contains(&saw2),
        "Call to Mind must NOT offer Artifact in your graveyard"
    );

    // 3. Wargate — {X}{G}{W}{U}: Search your library for a permanent card with mana value X or less, put it onto the battlefield.
    let wargate = card_index("26f39abe-ea0b-400d-b11c-03ee4cf50915");
    let mut eng3 = Duel::new(SEED, forest())
        .hand(0, &[wargate, opt_spell(), bone_saw()])
        .start();
    keep_mulligans(&mut eng3);
    reach_main_phase(&mut eng3, p0);
    let lib_opt = in_hand(&eng3, p0, opt_spell()).unwrap();
    let lib_saw = in_hand(&eng3, p0, bone_saw()).unwrap();
    hand_to_library_top(&mut eng3, p0, opt_spell());
    hand_to_library_top(&mut eng3, p0, bone_saw());
    float(
        &mut eng3,
        p0,
        &[
            (ManaColor::Green, 1),
            (ManaColor::White, 1),
            (ManaColor::Blue, 2),
        ],
    );
    let wg_obj = in_hand(&eng3, p0, wargate).unwrap();
    eng3.apply(p0, PlayerAction::CastSpell { card: wg_obj })
        .unwrap();
    eng3.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    eng3.apply(p0, PlayerAction::PassPriority).unwrap();
    eng3.apply(p1, PlayerAction::PassPriority).unwrap();
    let Pending::ChooseCards { options, .. } = eng3.pending().clone() else {
        panic!("expected ChooseCards for Wargate, got {:?}", eng3.pending());
    };
    assert!(
        options.contains(&lib_saw),
        "Wargate must offer permanent card with MV <= X"
    );
    assert!(
        !options.contains(&lib_opt),
        "Wargate must NOT offer Instant card Opt (Filter::Permanent)"
    );
    eng3.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![lib_saw],
        },
    )
    .unwrap();
    assert!(on_battlefield(&eng3, p0, bone_saw()).is_some());
}
