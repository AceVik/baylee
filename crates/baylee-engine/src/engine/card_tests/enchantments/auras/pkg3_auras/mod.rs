//! Package 3 behavioral tests for the 277 remaining `Coverage::Generated`
//! Aura cards under `cards/enchantments/auras/`.
//!
//! Every test is derived from Scryfall Oracle text, mana cost, colors, and
//! `Enchant <quality>` targeting rules, exercising regular gameplay actions.

#[allow(clippy::wildcard_imports)]
use super::super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::testkit::{Deed, Rest, arena, drive_to_rest, presses};
use baylee_core::color::{Color, ColorSet};

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

fn jasmine_boreal() -> CardIndex {
    card_index("942407bf-862a-4a0b-b19d-b9f8ee833026")
}

fn goblin_piker() -> CardIndex {
    card_index("50608184-90d3-43d2-a221-deb186c78323")
}

fn steel_wall() -> CardIndex {
    card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9")
}

fn bone_saw() -> CardIndex {
    card_index("16e555f2-5aa8-4100-a036-eed48db0e84a")
}

fn unlicensed_hearse() -> CardIndex {
    card_index("c640654c-487e-4a2c-aced-126ed835b78f")
}

fn inspirit_flagship_vessel() -> CardIndex {
    card_index("554df866-3dbb-4811-8573-6033481591aa")
}

fn dread_rider() -> CardIndex {
    card_index("e5c15ef5-6795-40c2-a131-a73f74142314")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn circle_of_protection_red() -> CardIndex {
    card_index("df2738fe-9cd1-4347-8808-105fcfde1190")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AuraHostKind {
    Creature,
    Land,
    YourCreature,
    ArtifactOrCreature,
    Artifact,
    CreatureOrVehicle,
    Permanent,
    RedOrGreenCreature,
    YourLand,
    Enchantment,
    TappedCreature,
    OpponentCreature,
    ForestOrPlains,
    NonblackCreature,
    CreaturePower3OrLess,
    CreatureMv2OrLess,
    CreatureWithoutFlying,
    CreatureOrSpacecraft,
    YourArtifactOrCreature,
    GreenCreature,
}

#[allow(clippy::struct_excessive_bools, dead_code)]
pub(super) struct AuraSpec<'a> {
    pub(super) card: CardIndex,
    pub(super) primary_mana: &'static [(ManaColor, u32)],
    pub(super) alt_mana: Option<&'static [(ManaColor, u32)]>,
    pub(super) total_mana: u32,
    pub(super) colors: &'static [Color],
    pub(super) host_kind: AuraHostKind,
    pub(super) expected_pt_delta: Option<(i16, i16)>,
    pub(super) expected_set_pt: Option<(i16, i16)>,
    pub(super) expected_kws: &'a [KeywordSet],
    pub(super) gain_control: bool,
    pub(super) does_not_untap: bool,
    pub(super) etb_draws: usize,
    pub(super) etb_taps_attached: bool,
}

#[track_caller]
#[allow(clippy::too_many_lines)]
pub(super) fn verify_aura(spec: &AuraSpec<'_>) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[spec.card, forest()])
        .hand(1, &[forest(), forest(), forest()])
        .battlefield(
            0,
            &[
                jasmine_boreal(),
                goblin_piker(),
                steel_wall(),
                forest(),
                mountain(),
                bone_saw(),
                unlicensed_hearse(),
                circle_of_protection_red(),
            ],
        )
        .battlefield(
            1,
            &[
                goblin_piker(),
                dread_rider(),
                serra_angel(),
                forest(),
                bone_saw(),
                inspirit_flagship_vessel(),
                circle_of_protection_red(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_jasmine = on_battlefield(&engine, p0, jasmine_boreal()).expect("my jasmine");
    let my_piker = on_battlefield(&engine, p0, goblin_piker()).expect("my piker");
    let my_wall = on_battlefield(&engine, p0, steel_wall()).expect("my wall");
    let my_forest = on_battlefield(&engine, p0, forest()).expect("my forest");
    let my_mountain = on_battlefield(&engine, p0, mountain()).expect("my mountain");
    let my_artifact = on_battlefield(&engine, p0, bone_saw()).expect("my artifact");
    let _my_vehicle = on_battlefield(&engine, p0, unlicensed_hearse()).expect("my vehicle");
    let their_piker = on_battlefield(&engine, p1, goblin_piker()).expect("their piker");
    let their_black = on_battlefield(&engine, p1, dread_rider()).expect("their black creature");
    let their_flyer = on_battlefield(&engine, p1, serra_angel()).expect("their flyer");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their forest");
    let their_spacecraft =
        on_battlefield(&engine, p1, inspirit_flagship_vessel()).expect("their spacecraft");
    let their_ench =
        on_battlefield(&engine, p1, circle_of_protection_red()).expect("their enchantment");

    // If testing TappedCreature, tap my_piker by attacking or tapping a mana elf;
    // even simpler: put llanowar_elves on board and tap it for mana!
    let (legal_host, illegal_target, base_pt) = match spec.host_kind {
        AuraHostKind::Creature => {
            if spec.gain_control {
                (their_piker, my_forest, Some((2, 1)))
            } else {
                (my_jasmine, my_forest, Some((4, 5)))
            }
        }
        AuraHostKind::Land => {
            if spec.gain_control {
                (their_forest, my_jasmine, None)
            } else {
                (my_forest, my_jasmine, None)
            }
        }
        AuraHostKind::YourCreature | AuraHostKind::YourArtifactOrCreature => {
            (my_jasmine, their_piker, Some((4, 5)))
        }
        AuraHostKind::ArtifactOrCreature | AuraHostKind::CreatureOrVehicle => {
            (my_jasmine, my_forest, Some((4, 5)))
        }
        AuraHostKind::Artifact => (my_artifact, my_forest, None),
        AuraHostKind::Permanent => {
            if spec.gain_control {
                (their_piker, ObjectId::new(999_999, 0), Some((2, 1)))
            } else {
                (my_jasmine, ObjectId::new(999_999, 0), Some((4, 5)))
            }
        }
        AuraHostKind::RedOrGreenCreature => (my_jasmine, my_wall, Some((4, 5))),
        AuraHostKind::YourLand => (my_forest, their_forest, None),
        AuraHostKind::Enchantment => (their_ench, my_forest, None),
        AuraHostKind::TappedCreature => {
            // Tap my_forest doesn't tap a creature; let's tap a llanowar_elves if needed,
            // handled in a dedicated setup below.
            (my_piker, my_jasmine, Some((2, 1)))
        }
        AuraHostKind::OpponentCreature => (their_piker, my_jasmine, Some((2, 1))),
        AuraHostKind::ForestOrPlains => (my_forest, my_mountain, None),
        AuraHostKind::NonblackCreature => (my_jasmine, their_black, Some((4, 5))),
        AuraHostKind::CreaturePower3OrLess | AuraHostKind::CreatureMv2OrLess => {
            (my_piker, my_jasmine, Some((2, 1)))
        }
        AuraHostKind::CreatureWithoutFlying => (my_jasmine, their_flyer, Some((4, 5))),
        AuraHostKind::CreatureOrSpacecraft => (their_spacecraft, my_forest, None),
        AuraHostKind::GreenCreature => (my_jasmine, my_piker, Some((4, 5))),
    };

    if spec.host_kind == AuraHostKind::TappedCreature {
        verify_tapped_creature_aura(spec);
        return;
    }

    // 1. Negative cast check when mana pool is empty (all Auras have total_mana >= 1).
    let spell_obj = in_hand(&engine, p0, spec.card).expect("aura in hand");
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected Priority");
    };
    assert!(
        !legal.castable.contains(&spell_obj),
        "Aura must not be castable with 0 mana"
    );

    // 2. Float exact printed mana cost and cast.
    float(&mut engine, p0, spec.primary_mana);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell_obj })
        .expect("Aura casts for its printed mana cost");

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseTargets for Aura cast, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert!(
        options.contains(&legal_host),
        "legal host {legal_host:?} must be offered for {:?}: {options:?}",
        spec.host_kind
    );
    if illegal_target != ObjectId::new(999_999, 0) {
        assert!(
            !options.contains(&illegal_target),
            "illegal permanent {illegal_target:?} must NOT be offered for {:?}: {options:?}",
            spec.host_kind
        );
    }

    let lib_before = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![legal_host],
                players: vec![],
            },
        )
        .expect("legal host accepted");

    let rest = drive_to_rest(&mut engine, p0);
    assert!(
        matches!(rest, Rest::Reached | Rest::Over),
        "Aura resolution must reach rest cleanly, got {rest:?}"
    );

    let aura_obj = on_battlefield(&engine, p0, spec.card).expect("Aura resolved onto battlefield");
    let chars = engine.state().object(aura_obj).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::ENCHANTMENT));
    assert_eq!(chars.colors, ColorSet::from_slice(spec.colors));
    assert_eq!(
        engine.state().object(aura_obj).and_then(|o| o.attached_to),
        Some(legal_host),
        "Aura is attached to chosen host"
    );

    // 3. Verify static P/T & keyword changes on host vs vanilla bystander.
    let bystander = if legal_host == their_piker {
        my_piker
    } else {
        their_piker
    };
    let host_counters = &engine.state().object(legal_host).unwrap().counters;
    let counter_delta = i16::try_from(host_counters.get(baylee_cards_dsl::CounterKind::P1P1))
        .unwrap()
        - i16::try_from(host_counters.get(baylee_cards_dsl::CounterKind::M1M1)).unwrap();
    if let Some((bp, bt)) = base_pt {
        if let Some((sp, st)) = spec.expected_set_pt {
            let (dp, dt) = spec.expected_pt_delta.unwrap_or((0, 0));
            assert_eq!(
                pt(&engine, legal_host),
                (sp + dp + counter_delta, st + dt + counter_delta)
            );
        } else if let Some((dp, dt)) = spec.expected_pt_delta {
            assert_eq!(
                pt(&engine, legal_host),
                (bp + dp + counter_delta, bt + dt + counter_delta)
            );
            assert_eq!(
                pt(&engine, bystander),
                (2, 1),
                "unenchanted bystander PT unchanged"
            );
        }
    }
    for kw in spec.expected_kws {
        assert!(
            keywords(&engine, legal_host).contains(*kw),
            "host must have granted keyword {kw:?}"
        );
        assert!(
            !keywords(&engine, bystander).contains(*kw),
            "unenchanted bystander must not gain {kw:?}"
        );
    }
    if spec.gain_control {
        assert_eq!(
            engine.state().object(legal_host).unwrap().controller,
            p0,
            "GainControl Aura transfers control of host to caster"
        );
    }
    if spec.etb_draws > 0 {
        assert_eq!(
            library_size(&engine, p0),
            lib_before - spec.etb_draws,
            "ETB draw count matches Scryfall Oracle text"
        );
    }
    if spec.etb_taps_attached {
        assert!(
            is_tapped(&engine, legal_host),
            "ETB tap trigger taps enchanted permanent"
        );
    }

    // 4. If hybrid mana exists, verify casting with alternative hybrid mana.
    if let Some(alt) = spec.alt_mana {
        let mut alt_engine = Duel::new(SEED, forest())
            .hand(0, &[spec.card])
            .battlefield(0, &[jasmine_boreal(), goblin_piker(), forest(), bone_saw()])
            .battlefield(1, &[goblin_piker(), forest()])
            .start();
        keep_mulligans(&mut alt_engine);
        reach_main_phase(&mut alt_engine, p0);
        float(&mut alt_engine, p0, alt);
        let c_obj = in_hand(&alt_engine, p0, spec.card).unwrap();
        alt_engine
            .apply(p0, PlayerAction::CastSpell { card: c_obj })
            .expect("hybrid Aura casts with alternative mana");
    }

    // 5. Also exercise arena presses (activated/granted/cycling abilities).
    exercise_arena_presses(spec.card);
}

#[track_caller]
fn verify_tapped_creature_aura(spec: &AuraSpec<'_>) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[spec.card])
        .battlefield(0, &[llanowar_elves(), jasmine_boreal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let untapped_jasmine = on_battlefield(&engine, p0, jasmine_boreal()).unwrap();

    // Tap the Elf using its mana ability so it is a tapped creature.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: elf,
                ability_index: 0,
            },
        )
        .unwrap();
    assert!(is_tapped(&engine, elf));

    float(&mut engine, p0, spec.primary_mana);
    let spell_obj = in_hand(&engine, p0, spec.card).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell_obj })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        options.contains(&elf),
        "tapped creature is a legal target for Enchant tapped creature"
    );
    assert!(
        !options.contains(&untapped_jasmine),
        "untapped creature is NOT a legal target for Enchant tapped creature"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    let rest = drive_to_rest(&mut engine, p0);
    assert!(matches!(rest, Rest::Reached | Rest::Over));
    let aura_obj = on_battlefield(&engine, p0, spec.card).unwrap();
    assert_eq!(
        engine.state().object(aura_obj).and_then(|o| o.attached_to),
        Some(elf)
    );
}

#[track_caller]
pub(super) fn exercise_arena_presses(card: CardIndex) {
    let seat = PlayerId::new(0);
    let Some((engine, objects)) = arena(card) else {
        return;
    };
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        return;
    };
    let all = presses(&legal, &objects);
    for (slot, press) in &all {
        let Some((mut eng, objs)) = arena(card) else {
            continue;
        };
        let Some(object) = objs.get(*slot).copied() else {
            continue;
        };
        if matches!(press, Deed::Cast) {
            continue;
        }
        eng.apply(seat, press.action(object))
            .expect("offered ability must be legal to activate");
        let mut rest = drive_to_rest(&mut eng, seat);
        if matches!(rest, Rest::Unanswered("ChooseDamageSource")) {
            let Pending::ChooseDamageSource {
                player,
                choice,
                options,
            } = eng.pending().clone()
            else {
                panic!("expected ChooseDamageSource");
            };
            let source = *options.first().expect("at least one damage source offered");
            eng.apply(player, PlayerAction::ChooseDamageSource { choice, source })
                .expect("damage source accepted");
            rest = drive_to_rest(&mut eng, seat);
        }
        assert!(
            matches!(rest, Rest::Reached | Rest::Over),
            "activated ability press must reach rest cleanly, got {rest:?}"
        );
    }
}

#[test]
fn tractor_beam_taps_controls_and_keeps_uncrewed_spacecraft_tapped() {
    // Tractor Beam — {2}{U}{U} — Enchantment — Aura
    // Enchant creature or Spacecraft
    // When this Aura enters, tap enchanted permanent.
    // You control enchanted permanent.
    // Enchanted permanent doesn't untap during its controller's untap step.
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let tractor_beam = card_index("ed99a0b8-d82d-4c80-af97-cabd333620df");

    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[tractor_beam])
        .battlefield(0, &[forest()])
        .battlefield(1, &[inspirit_flagship_vessel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spacecraft =
        on_battlefield(&engine, p1, inspirit_flagship_vessel()).expect("opponent spacecraft");
    let my_forest = on_battlefield(&engine, p0, forest()).expect("my forest");

    // Tap our Forest so we can compare it untapping in p0's next untap step.
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: my_forest })
        .unwrap();
    assert!(is_tapped(&engine, my_forest));
    assert!(!is_tapped(&engine, spacecraft));

    float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 2), (ManaColor::Blue, 2)],
    );
    let spell_obj = in_hand(&engine, p0, tractor_beam).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell_obj })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        options.contains(&spacecraft),
        "uncrewed Spacecraft is a legal target for Tractor Beam"
    );
    assert!(
        !options.contains(&my_forest),
        "Forest is not a legal target for Tractor Beam"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![spacecraft],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(spacecraft).unwrap().controller,
        p0,
        "Tractor Beam gives p0 control of enchanted Spacecraft"
    );
    assert!(
        is_tapped(&engine, spacecraft),
        "Tractor Beam ETB trigger taps enchanted Spacecraft"
    );
    assert!(
        !engine
            .state()
            .object(spacecraft)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "Inspirit, Flagship Vessel is an uncrewed Spacecraft (not a creature)"
    );

    // Advance a full turn cycle to p0's next main phase.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0));

    assert!(
        !is_tapped(&engine, my_forest),
        "p0's untap step untapped p0's Forest"
    );
    assert!(
        is_tapped(&engine, spacecraft),
        "Tractor Beam keeps enchanted uncrewed Spacecraft tapped during its controller's untap step"
    );
}
