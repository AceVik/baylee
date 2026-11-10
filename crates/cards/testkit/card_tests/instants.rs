//! Instants, the door `cards/instants/` puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
pub use super::*;
pub use baylee_core::color::{Color, ColorSet};

// oracle_id = "1c747fe2-289e-492a-a846-aa77707e2dc3"
pub fn abrupt_decay() -> baylee_core::ids::CardIndex {
    card_index("1c747fe2-289e-492a-a846-aa77707e2dc3")
}

// oracle_id = "464c0150-3dbc-403b-9ada-fef25ab1f29d"
pub fn brain_freeze() -> baylee_core::ids::CardIndex {
    card_index("464c0150-3dbc-403b-9ada-fef25ab1f29d")
}

// oracle_id = "0456ec64-2c81-4763-a352-8ff64a4c3d6b"
pub fn deadly_rollick() -> baylee_core::ids::CardIndex {
    card_index("0456ec64-2c81-4763-a352-8ff64a4c3d6b")
}

// oracle_id = "86bf58f2-7f25-4e10-b797-25e0e8e67769"
pub fn flusterstorm() -> baylee_core::ids::CardIndex {
    card_index("86bf58f2-7f25-4e10-b797-25e0e8e67769")
}

// oracle_id = "16e015b2-f8a3-4b1a-80be-58a8f5fb5e8c"
pub fn frantic_search() -> baylee_core::ids::CardIndex {
    card_index("16e015b2-f8a3-4b1a-80be-58a8f5fb5e8c")
}

/// The pool's one land that is also a creature, and therefore the only
/// permanent an Equipment can hand a keyword to that still answers to
/// `Filter::LAND`.
pub fn dryad_arbor() -> baylee_core::ids::CardIndex {
    card_index("e996cd67-739c-40f4-b276-0042acf26c71")
}

/// A cast Search over four tapped lands, one with shroud from Greaves.
pub fn a_frantic_search_choosing_its_lands() -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(77, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                dryad_arbor(),
                lightning_greaves(),
            ],
        )
        .hand(0, &[frantic_search()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Equip {0}, which is how a land comes to have shroud at all.
    let arbor = on_battlefield(&engine, p0, dryad_arbor()).expect("the Arbor is on the table");
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves too");
    activate(&mut engine, p0, lightning_greaves(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![arbor], "the only creature p0 controls");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![arbor],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(arbor))
    });
    assert!(
        keywords(&engine, arbor).contains(KeywordSet::SHROUD),
        "the Greaves grant shroud to what they are attached to"
    );
    assert!(
        types(&engine, arbor).contains(TypeSet::LAND),
        "and the thing they are attached to is still a land"
    );

    let islands = all_on_battlefield(&engine, p0, island());
    assert_eq!(islands.len(), 3, "the rest of the board is three Islands");
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, frantic_search()).expect("the Search is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("three Islands pay {2}{U}");
    (engine, arbor, islands)
}

// oracle_id = "1a0770e6-b093-4439-baff-6889a50ba12e"
pub fn mental_misstep() -> baylee_core::ids::CardIndex {
    card_index("1a0770e6-b093-4439-baff-6889a50ba12e")
}

// oracle_id = "ededbdae-d9dc-4206-9335-d7158f2d7700"
pub fn vampiric_tutor() -> baylee_core::ids::CardIndex {
    card_index("ededbdae-d9dc-4206-9335-d7158f2d7700")
}

// oracle_id = "e8863518-0bfa-49c3-8c6e-6c9116a81051"
pub fn worldly_tutor() -> baylee_core::ids::CardIndex {
    card_index("e8863518-0bfa-49c3-8c6e-6c9116a81051")
}

// oracle_id = "5b5bf1fa-6502-4790-b66b-f0f8504ebc7c"
pub fn cabal_ritual() -> CardIndex {
    card_index("5b5bf1fa-6502-4790-b66b-f0f8504ebc7c")
}

// oracle_id = "133c99c0-3652-410f-8100-68015a47af9f"
pub fn dispatch() -> baylee_core::ids::CardIndex {
    card_index("133c99c0-3652-410f-8100-68015a47af9f")
}

// oracle_id = "ce19962d-94f9-4b2b-b668-963c0acce308"
pub fn borne_upon_a_wind() -> CardIndex {
    card_index("ce19962d-94f9-4b2b-b668-963c0acce308")
}

pub fn deflecting_swat() -> CardIndex {
    card_index("ae120613-97d6-4393-b39d-c3e6c076f5d6")
}

/// `seat` casts Deflecting Swat for its printed `{2}{R}` at `target`, a spell
/// or ability on the stack, and passes until it resolves. Returns its
/// new-target question, or `None` when it asks none.
#[track_caller]
pub fn swatted(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    target: ObjectId,
) -> Option<Pending> {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    cast_from_hand(engine, seat, deflecting_swat());
    let mut aimed = false;
    for _ in 0..24 {
        match engine.pending().clone() {
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                let slot = options
                    .iter()
                    .position(|o| !matches!(o.kind, CastModeKind::Alternative(_)))
                    .expect("the printed cost is offered");
                engine
                    .apply(player, PlayerAction::ChooseMode(slot))
                    .unwrap();
            }
            Pending::ChooseTargets { player, .. } if !aimed => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![target],
                        },
                    )
                    .expect("\"target spell or ability\": the one on the stack");
                aimed = true;
            }
            question @ Pending::ChooseTargets { .. } => return Some(question),
            Pending::Priority { player, .. } => {
                if aimed && on_stack(engine, deflecting_swat()).is_none() {
                    return None;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Swat is cast: {other:?}"),
        }
    }
    panic!("the Swat never resolved")
}

/// Casts Archdruid's Charm off floating mana and answers its mode question
/// with mode `mode`, the way a player presses one of the offered buttons.
/// Returns the modes that were offered; none when only one mode could be
/// chosen, which the engine takes without asking.
#[track_caller]
pub fn cast_archdruids_charm(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    mode: usize,
) -> Vec<CastModeKind> {
    cast_with_floating(engine, seat, archdruid_s_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        return Vec::new();
    };
    let offered: Vec<CastModeKind> = options.iter().map(|o| o.kind).collect();
    let slot = offered
        .iter()
        .position(|kind| *kind == CastModeKind::Mode(mode))
        .unwrap_or_else(|| panic!("mode {mode} was not offered: {offered:?}"));
    engine
        .apply(seat, PlayerAction::ChooseMode(slot))
        .expect("an offered mode");
    offered
}

/// Casts Vantress Visions, the Virtue's adventure, off mana already floating
/// and aims it at `ability`, answering as a player would: the face, if the
/// engine asks which (it does not when only the adventure is affordable),
/// then the target, out of a list that must hold it.
#[track_caller]
pub fn cast_vantress_visions(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    ability: ObjectId,
) {
    cast_with_floating(engine, seat, virtue_of_knowledge());
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let adventure = options
            .iter()
            .position(|o| matches!(o.kind, baylee_engine::choice::CastModeKind::Face(1)))
            .expect("Vantress Visions is a way to cast the card");
        engine
            .apply(seat, PlayerAction::ChooseMode(adventure))
            .expect("the face came out of the list");
    }
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Visions asks for its target, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&ability),
        "an ability you control on the stack: {options:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![ability],
                players: vec![],
            },
        )
        .expect("the ability");
}

/// The object on top of the stack.
pub fn top_of_stack(engine: &Engine<RegistryLookup>) -> ObjectId {
    *engine
        .state()
        .zones
        .list(baylee_engine::zone::ZoneLocation::Stack)
        .last()
        .expect("something is on the stack")
}

/// How many times the journal says an ability of `source` was put on the
/// stack by being activated or triggered.
pub fn put_on_stack_from(engine: &Engine<RegistryLookup>, source: ObjectId) -> usize {
    engine
        .journal()
        .entries()
        .iter()
        .filter(|e| {
            matches!(e.event, baylee_engine::event::GameEvent::AbilityTriggered { source: s, .. } if s == source)
        })
        .count()
}

/// Casts Lose Focus off `seat`'s floating mana and answers its replicate
/// question with `times` and its target with `target`, checking the question
/// says what it counts.
#[track_caller]
pub fn cast_lose_focus_replicated(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    most: u32,
    times: u32,
    target: ObjectId,
) {
    cast_from_hand(engine, seat, lose_focus());
    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason,
    } = engine.pending().clone()
    else {
        panic!(
            "replicate is announced with the additional costs, before the \
             target (CR 601.2b), got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat);
    assert_eq!(
        (min, max),
        (0, most),
        "any number of times, as many as the floating mana pays for"
    );
    assert_eq!(
        reason,
        baylee_engine::choice::NumberPrompt::Replicate {
            cost: baylee_core::mana::ManaCost::parse("{U}")
        },
        "the question says it counts replicate payments of {{U}}, not X"
    );
    assert!(
        engine
            .apply(seat, PlayerAction::ChooseNumber(most + 1))
            .is_err(),
        "a count the pool cannot pay is refused"
    );
    engine
        .apply(seat, PlayerAction::ChooseNumber(times))
        .expect("a count inside the offer");
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the spell is a legal target");
}

/// The replicate trigger on the stack, if there is one: a synthetic ability
/// whose source is `spell`.
pub fn replicate_trigger(engine: &Engine<RegistryLookup>, spell: ObjectId) -> Option<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .find(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.ability.is_some_and(|loc| {
                    loc.source == spell && loc.index == baylee_core::ids::AbilityRef::SYNTHETIC
                })
            })
        })
}

pub fn ancestral_recall() -> CardIndex {
    card_index("550c74d4-1fcb-406a-b02a-639a760a4380")
}

pub fn annul() -> CardIndex {
    card_index("d08e9784-75f7-4164-ac48-d06160f8c56b")
}

pub fn artifact_blast() -> CardIndex {
    card_index("2d4aedc5-31c5-4281-98e1-b0c2233c3c8a")
}

pub fn battlegrowth() -> CardIndex {
    card_index("650bf82e-7f83-470c-bf4a-34281bfe9341")
}

pub fn brightstone_ritual() -> CardIndex {
    card_index("08e90e85-4103-4acb-a8a7-e1329b460aa7")
}

pub fn burst_of_energy() -> CardIndex {
    card_index("b795ecfd-31ac-4a2a-9679-97b088932d93")
}

pub fn demystify() -> CardIndex {
    card_index("fd591199-9f7a-4147-a150-13279dbb4498")
}

pub fn enrage() -> CardIndex {
    card_index("0f6e66d5-4f27-485b-999d-ee55c1e218b9")
}

pub fn envelop() -> CardIndex {
    card_index("30062dd0-c049-4872-8011-8b4810a3fa26")
}

pub fn erase() -> CardIndex {
    card_index("c2ceb15c-5d02-4cf4-a7c9-c1a40b7ca667")
}

pub fn heat_ray() -> CardIndex {
    card_index("76ec76b9-da0f-4b9d-ad0a-d734052a5f2b")
}

pub fn howl_from_beyond() -> CardIndex {
    card_index("403cf6ae-48a9-4ea9-894e-7135cfca4e1b")
}

pub fn iron_will() -> CardIndex {
    card_index("dc0decb9-32da-47aa-a51a-0d9c623c534a")
}

pub fn jump() -> CardIndex {
    card_index("f7518456-45ed-41d4-bd3c-5aacea28eb35")
}

pub fn leap() -> CardIndex {
    card_index("e89a3ce0-6b38-4326-a7af-8575af371baa")
}

pub fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

pub fn lose_hope() -> CardIndex {
    card_index("4f6dbd90-f7fe-4adf-a00f-27573c0bd5c4")
}

pub fn mental_note() -> CardIndex {
    card_index("e8d5f31c-7abf-4fbb-977e-8353a97daf7a")
}

pub fn opt() -> CardIndex {
    card_index("713332c1-5bd8-400f-bfff-c1ca0697a043")
}

// oracle_id = "0ff78353-a26e-4b5a-948d-1c3b41d2dd1f"
pub fn oxidize() -> CardIndex {
    card_index("0ff78353-a26e-4b5a-948d-1c3b41d2dd1f")
}

pub fn quiet_purity() -> CardIndex {
    card_index("3f16dc14-3d3f-4ffa-90bc-9abc67db75cf")
}

pub fn reach_through_mists() -> CardIndex {
    card_index("c81ca8ff-92f3-481e-82f7-0673b6c74ea0")
}

pub fn rescue() -> CardIndex {
    card_index("f0ae687a-7222-4521-a514-ba10373fed7e")
}

pub fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

/// Storm Crow, whose printed 1/2 is the whole reason it is here: two
/// damage is exactly lethal on it where one would not be, so the number
/// Shock deals is readable off the graveyard. A 1/1 would have died to
/// either.
pub fn a_one_two_bird() -> CardIndex {
    card_index("000d5588-5a4c-434e-988d-396632ade42c")
}

pub fn shrink() -> CardIndex {
    card_index("252330b1-67cf-4d9e-a413-917ba61e731f")
}

pub fn silk_net() -> CardIndex {
    card_index("58511cb8-348d-409e-8fd6-772be20e57cd")
}

pub fn spark_spray() -> CardIndex {
    card_index("c37f1921-7ac3-4185-8579-1dfdfe647ea2")
}

pub fn sprout() -> CardIndex {
    card_index("424c6f5e-b386-47e9-b3fe-25b263097d40")
}

pub fn stand_firm() -> CardIndex {
    card_index("c0406e70-8131-4e13-b1d5-4e943ad296b8")
}

pub fn tunnel() -> CardIndex {
    card_index("80559618-9dd9-4987-b3bc-1a1b5537bbc5")
}

/// Wall of Roots — `{1}{G}` 0/5 Plant Wall, the pool's plainest Wall and the
/// only one a test can name by oracle id.
pub fn wall_of_roots() -> CardIndex {
    card_index("3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f")
}

pub fn accelerate() -> CardIndex {
    card_index("79189464-9645-4411-8886-68fd40a588ed")
}

pub fn aggressive_urge() -> CardIndex {
    card_index("43f5a93b-0f8d-48d2-ab9d-275d44cf88b5")
}

pub fn aura_blast() -> CardIndex {
    card_index("4e3c3bdc-667d-42ec-b960-159a39c53cb3")
}

pub fn clear() -> CardIndex {
    card_index("d0783be9-e518-431a-8e71-121610da50e1")
}

pub fn disenchant() -> CardIndex {
    card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a")
}

pub fn eladamri_s_call() -> CardIndex {
    card_index("4acb6612-54e8-428d-acb6-c7259a5ad6a8")
}

// oracle_id = "b2c9f074-57ca-4709-976d-f432f632483f"
pub fn extinguish() -> CardIndex {
    card_index("b2c9f074-57ca-4709-976d-f432f632483f")
}

pub fn false_summoning() -> CardIndex {
    card_index("4f891c68-c959-4210-94e5-94a8e487d5ef")
}

// oracle_id = "fc05e582-e760-4fe2-ba43-e9d8e63f3f85"
pub fn fists_of_the_anvil() -> CardIndex {
    card_index("fc05e582-e760-4fe2-ba43-e9d8e63f3f85")
}

pub fn flowstone_strike() -> CardIndex {
    card_index("d67048eb-eaf1-4b5b-a8e3-a004bab438f3")
}

pub fn gerrards_command() -> CardIndex {
    card_index("de8ebe4b-d86d-478b-946e-f83e1409b63f")
}

pub fn guided_strike() -> CardIndex {
    card_index("a215e813-df7e-4f21-9718-eb264eb62813")
}

pub fn hero_s_demise() -> CardIndex {
    card_index("69ec0b59-a0aa-4878-a3b3-130cea18fee0")
}

pub fn heroes_reunion() -> CardIndex {
    card_index("beab66e0-2b6c-478f-ba42-8c0c216a527e")
}

pub fn hisoka_s_defiance() -> CardIndex {
    card_index("2ac6f1c7-f4c5-4d45-9644-da49d8fe4758")
}

pub fn hoodwink() -> CardIndex {
    card_index("a2fe10b8-857e-4881-85ef-24a0f96a3d79")
}

pub fn mage_s_guile() -> CardIndex {
    card_index("f371cdaf-7ab8-4555-8c96-61dfbf8c0179")
}

/// Magma Jet prints two effects and both are played by one cast: "deals 2
/// damage to any target", then "Scry 2". The damage has to *kill*, so the
/// target is a printed 1/1 — a creature still standing would mean the amount
/// was never read — and the 20 life its controller keeps says the two went to
/// the permanent that was named and not to the seat whose board it stood on
/// (CR 115.4 offers both in one choice). The scry is read as a *move*: the two
/// cards the question offers, the one that is chosen landing on the bottom
/// while the other becomes the new top, with the library exactly as long as it
/// was because scry draws nothing.
pub fn magma_jet() -> CardIndex {
    card_index("2f292253-64d3-4cf2-881f-a3eea4fda388")
}

pub fn naturalize() -> CardIndex {
    card_index("bdb3ca68-ec1f-4e16-81cc-d23f8f52c728")
}

pub fn nourish() -> CardIndex {
    card_index("9d5e82d3-79ac-4dbe-80e8-1db6dd6c8767")
}

pub fn predators_strike() -> CardIndex {
    card_index("f62e9a9f-9989-4a62-b063-cca32e21fe8b")
}

pub fn preemptive_strike() -> CardIndex {
    card_index("250f8642-9754-48fd-8f09-70ed13d7a42c")
}

pub fn raise_the_alarm() -> CardIndex {
    card_index("5b2364d7-a811-4595-a1b4-224c70555ffa")
}

// oracle_id = "b13c0f76-fbda-4911-9442-c3d7e97f1aac"
pub fn remove_soul() -> CardIndex {
    card_index("b13c0f76-fbda-4911-9442-c3d7e97f1aac")
}

pub fn turn_to_dust() -> CardIndex {
    card_index("56828166-eaa3-4711-91b6-401a3e3b733f")
}

pub fn unnatural_speed() -> CardIndex {
    card_index("fa46ced5-b701-4aa3-b4a1-36fb15b80696")
}

pub fn terminate() -> CardIndex {
    card_index("6257c2fd-005f-41e3-8a72-af76df1eb134")
}

pub fn fissure() -> CardIndex {
    card_index("c8b1e9f3-b014-4e57-b278-6d84a7e88b23")
}

/// `seat` casts `card` off its whole board, aimed at `objects` and `players`,
/// and the spell on the stack is returned.
#[track_caller]
pub fn aimed(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    objects: &[ObjectId],
    players: &[PlayerId],
) -> ObjectId {
    cast_from_hand(engine, seat, card);
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: objects.to_vec(),
                players: players.to_vec(),
            },
        )
        .expect("the spell's own target question offered these");
    on_stack(engine, card).expect("the spell is on the stack")
}

/// `seat` answers `spell` with Misdirection, pitching the Counterspell in hand
/// for it, and passes until it resolves (#247). Returns the new-target
/// question it asks, or `None` when it asks none.
///
/// Misdirection's own target question comes first and names a spell; the
/// next target question is the one the card is for.
#[track_caller]
pub fn misdirected(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    spell: ObjectId,
) -> Option<Pending> {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    let pitch = in_hand(engine, seat, counterspell()).expect("a blue card to pitch");
    cast_with_floating(engine, seat, misdirection());
    let mut aimed = false;
    for _ in 0..24 {
        match engine.pending().clone() {
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Alternative(_)))
                    .expect("the pitch cost is offered");
                engine
                    .apply(player, PlayerAction::ChooseMode(slot))
                    .unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![pitch],
                        },
                    )
                    .expect("the Counterspell pays the pitch cost");
            }
            Pending::ChooseTargets { player, .. } if !aimed => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![spell],
                        },
                    )
                    .expect("\"target spell\": the one on the stack");
                aimed = true;
            }
            question @ Pending::ChooseTargets { .. } => return Some(question),
            Pending::Priority { player, .. } => {
                if aimed && on_stack(engine, misdirection()).is_none() {
                    return None;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while Misdirection is cast: {other:?}"),
        }
    }
    panic!("Misdirection never resolved")
}

/// Casts Pact of Negation at p0's own Llanowar Elves, the one spell a duel
/// with nothing else in hand can offer it, and lets it resolve. What is left
/// is the debt, with `board` to pay it from and `also_in_hand` still held.
pub fn a_pact_owed_by_p0(
    board: &[CardIndex],
    also_in_hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut hand = vec![llanowar_elves(), pact_of_negation()];
    hand.extend_from_slice(also_in_hand);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, board)
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_with_floating(&mut engine, p0, pact_of_negation());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let elves = on_stack(&engine, llanowar_elves()).expect("the Elves are on the stack");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Pact targets the Elves");
    pass_until(&mut engine, stack_is_empty);
    engine
}

/// Clever Concealment on `board`, main phase, every land tapped for mana.
pub fn concealment_table(board: &[CardIndex]) -> Engine<RegistryLookup> {
    let seat = PlayerId::new(0);
    let mut engine = Duel::new(229, plains())
        .hand(0, &[clever_concealment()])
        .battlefield(0, board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, seat);
    tap_all_mana(&mut engine, seat);
    engine
}

pub fn concealment_is_offered(engine: &Engine<RegistryLookup>) -> bool {
    let seat = PlayerId::new(0);
    let card = in_hand(engine, seat, clever_concealment()).expect("in hand");
    matches!(engine.pending(), Pending::Priority { legal, .. } if legal.castable.contains(&card))
}

/// Casts Clever Concealment phasing nothing out, and returns the tap question
/// its convoke asks, if it asks one.
pub fn concealment_convoke_question(
    engine: &mut Engine<RegistryLookup>,
) -> Option<(Vec<ObjectId>, u32)> {
    let seat = PlayerId::new(0);
    let card = in_hand(engine, seat, clever_concealment()).expect("in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the spell is offered");
    if let Pending::ChooseTargets { reason, .. } = engine.pending()
        && *reason != baylee_engine::choice::TargetPrompt::Convoke
    {
        engine
            .apply(
                seat,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![],
                },
            )
            .expect("\"any number of target\" permanents may be none");
    }
    tap_to_pay_question(engine)
}

/// `{W}` Aura: "Enchanted creature gets +1/+2."
pub fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}

pub fn memory_deluge() -> CardIndex {
    card_index("e6fd55f2-7e26-469c-a44a-ea2eb90e19a9")
}

pub fn consult_the_star_charts() -> CardIndex {
    card_index("e921839f-9d91-41a9-bc89-016af3c757aa")
}

/// Answers a look-and-keep with the first `min` cards offered, after
/// checking how many were looked at and how many are kept, and returns the
/// cards that were not kept.
#[track_caller]
pub fn keep_first(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    looked: usize,
    kept: u8,
) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = pass_to_card_choice(engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, seat);
    assert_eq!(prompt, ChoicePrompt::PutIntoHand);
    assert_eq!(options.len(), looked, "how many were looked at");
    assert_eq!((min, max), (kept, kept), "how many are kept");
    let keep = options[..usize::from(kept)].to_vec();
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: keep.clone(),
            },
        )
        .unwrap();
    // "In a random order": nobody is asked to arrange the rest.
    assert!(
        !matches!(engine.pending(), Pending::Arrange { .. }),
        "the rest are not the player's to order"
    );
    pass_until(engine, stack_is_empty);
    for card in &keep {
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(seat))
                .contains(card)
        );
    }
    options[usize::from(kept)..].to_vec()
}

pub fn realms_uncharted() -> CardIndex {
    card_index("e21c8fc6-d4ef-42b6-b11e-d9c931da1387")
}

/// Moves the named cards from the seat's hand into its library, the harness
/// way, so a search has lands of several names to find among the Forests.
#[track_caller]
pub fn hide_in_library(engine: &mut Engine<RegistryLookup>, seat: PlayerId, cards: &[CardIndex]) {
    for &card in cards {
        let id = in_hand(engine, seat, card).expect("the card starts in hand");
        engine
            .dev_state_mut(seat)
            .expect("the harness may set boards up")
            .move_object(
                id,
                ZoneLocation::Library(seat),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .expect("into the library");
    }
}

/// Casts Realms Uncharted and finds the first `n` lands it offers, after
/// checking the offer is one card of each name, at most four of them.
#[track_caller]
pub fn realms_search(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    n: usize,
) -> Vec<ObjectId> {
    cast_from_hand(engine, seat, realms_uncharted());
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = pass_to_card_choice(engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, seat);
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    // "Up to four", and a counted choice is fitted to what it offers
    // (CR 609.3): four of five names, two of two.
    assert_eq!(
        (min, usize::from(max)),
        (0, options.len().min(4)),
        "up to four"
    );
    let mut names: Vec<_> = options
        .iter()
        .map(|id| engine.state().object(*id).unwrap().characteristics().name)
        .collect();
    let offered = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), offered, "one card of each name is offered");
    let found = options[..n].to_vec();
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: found.clone(),
            },
        )
        .unwrap();
    found
}

// oracle_id = "437b2dab-15e0-4b9a-a204-58622d37a3b3"
pub fn fact_or_fiction() -> CardIndex {
    card_index("437b2dab-15e0-4b9a-a204-58622d37a3b3")
}

/// Fact or Fiction cast with three known cards on top of the library, up to
/// the opponent's question. Returns the engine and the five revealed cards,
/// top first: Ondu Cleric, Counterspell, Llanowar Elves, then two Forests.
pub fn fact_or_fiction_revealed(seed: u64) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(
            0,
            &[
                fact_or_fiction(),
                llanowar_elves(),
                counterspell(),
                ondu_cleric(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let mut on_top = Vec::new();
    for card in [llanowar_elves(), counterspell(), ondu_cleric()] {
        on_top.insert(0, hand_to_library_top(&mut engine, p0, card));
    }
    let journal_from = engine.state().journal.entries().len();
    cast_from_hand(&mut engine, p0, fact_or_fiction());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert_eq!(player, p1, "\"an opponent separates those cards\"");
    assert_eq!(options.len(), 5, "\"the top five cards\"");
    assert_eq!(
        options[..3],
        on_top[..],
        "the top of the library, top first"
    );
    assert_eq!((min, max), (0, 5), "either pile may hold any of them");
    assert_eq!(prompt, ChoicePrompt::FirstPile);
    assert!(
        engine.state().journal.entries()[journal_from..]
            .iter()
            .any(|e| matches!(&e.event, GameEvent::Revealed { cards, .. } if cards == &options)),
        "\"reveal the top five\", before anyone separates them"
    );
    (engine, options)
}

/// Seat 0 casts Dark Ritual and, holding priority, Three Steps Ahead with
/// its counter and copy modes, {4}{U}{U}: the Ritual is the spell's first
/// instance of "target", its own Llanowar Elves the second (CR 700.2c).
/// Seat 1 holds `theirs` and the lands `lands`. Returns the engine with the
/// Three Steps on the stack and seat 0 holding priority, the Ritual and the
/// Elves.
pub fn three_steps_ahead_of_a_ritual(
    lands: &[CardIndex],
    theirs: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                swamp(),
                island(),
                island(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, lands)
        .hand(0, &[dark_ritual(), three_steps_ahead()])
        .hand(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, dark_ritual());
    let ritual = on_stack(&engine, dark_ritual()).unwrap();
    cast_with_floating(&mut engine, p0, three_steps_ahead());
    let offered = choose_modes(&mut engine, p0, 0b011);
    assert!(offered.iter().any(|o| o.kind == CastModeKind::Modes(0b011)
        && o.cost == baylee_core::mana::ManaCost::parse("{4}{U}{U}")));
    // Each instance is explained as its own mode's sentence: the counter's,
    // then the copy's. Neither is "the second target" of one sentence, and
    // what the counter chose is nothing for the copy to weigh.
    let context = engine.decision_context();
    assert_eq!((context.mode, context.second_instance), (Some(0), false));
    let _ = aim_at(&mut engine, p0, ritual);
    let context = engine.decision_context();
    assert_eq!(
        (context.mode, context.second_instance, context.first_targets),
        (Some(1), false, &[][..])
    );
    let _ = aim_at(&mut engine, p0, elves);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    (engine, ritual, elves)
}

// ---------------------------------------------------------------------------
// Alpha cards, played by their Oracle text.
// ---------------------------------------------------------------------------

/// A seat's current life total, read the way most of this batch reads it.
pub fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

/// Passes priority, declaring nothing, until `seat` is the one asked — the
/// shape of every response window in this batch where the other seat has
/// nothing left to add and only needs to get out of the way.
#[track_caller]
pub fn pass_until_priority(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    loop {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if player == seat => return,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("expected a priority round on the way to {seat:?}, got {other:?}"),
        }
    }
}

/// A vanilla {6} 4/6 artifact creature — Obsianus Golem, no printed
/// keywords or abilities. Used here only as an ordinary body to attack or
/// block with; its own Oracle text is checked in `artifacts.rs`.
pub fn obsianus_golem() -> CardIndex {
    card_index("ac41171e-c454-49e9-9004-c082ae099630")
}

pub fn death_ward() -> CardIndex {
    card_index("0544b707-ec67-43e3-a25d-fd7005ab673d")
}

pub fn healing_salve() -> CardIndex {
    card_index("8da8644c-75a1-4fe9-8e94-900d948d631c")
}

pub fn reverse_damage() -> CardIndex {
    card_index("eaaf7c30-f463-4115-a40e-7dc717063413")
}

pub fn righteousness() -> CardIndex {
    card_index("3f6b2f76-0364-415e-a6a2-e9e5bf31b745")
}

pub fn blue_elemental_blast() -> CardIndex {
    card_index("65e1558c-6b09-4ddc-b520-f19f4fb972af")
}

pub fn mana_short() -> CardIndex {
    card_index("48207d1c-448a-4e1b-974a-642dfea75933")
}

pub fn psionic_blast() -> CardIndex {
    card_index("7f221ad6-7ec4-483d-a6b5-1456c95c1cad")
}

pub fn thoughtlace() -> CardIndex {
    card_index("6452b6a6-6235-46a3-a712-a26592450438")
}

pub fn twiddle() -> CardIndex {
    card_index("773ad2ef-5acc-49ea-8d85-056330e87039")
}

pub fn deathlace() -> CardIndex {
    card_index("fb80aaba-352a-4b58-8db2-1e02d542819c")
}

pub fn terror() -> CardIndex {
    card_index("b81f041d-98db-4408-9472-c483e4a502bc")
}

pub fn chaoslace() -> CardIndex {
    card_index("08842aa3-f923-46e9-a106-f542331e9cc1")
}

/// The opponent answers `spell` (cast by seat 0 off `spell_land`) with `lace`
/// (cast by seat 1 off `lace_land`): "Target spell or permanent becomes
/// <color>. (Its mana symbols remain unchanged.)" aimed at the *spell*.
/// Asserts the spell is its printed color on the stack first, the lace's
/// color once the lace has resolved, the mana cost untouched, and the
/// permanent it becomes still that color afterwards: the change has no end,
/// so it is not the stack's alone.
#[track_caller]
pub fn a_lace_recolours_a_spell(
    lace: CardIndex,
    lace_land: CardIndex,
    spell: CardIndex,
    spell_land: CardIndex,
    printed: Color,
    becomes: Color,
) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1002, forest())
        .battlefield(0, &[spell_land])
        .battlefield(1, &[lace_land])
        .hand(0, &[spell])
        .hand(1, &[lace])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    cast_from_hand(&mut engine, p0, spell);
    let on_the_stack = on_stack(&engine, spell).expect("the creature spell is on the stack");
    let before = engine
        .state()
        .object(on_the_stack)
        .unwrap()
        .characteristics()
        .clone();
    assert_eq!(
        before.colors,
        ColorSet::of(printed),
        "its printed color before the lace"
    );
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, lace);
    let offered = aim_at(&mut engine, p1, on_the_stack);
    assert!(
        offered.contains(&on_the_stack),
        "a spell on the stack is a legal target: {offered:?}"
    );
    pass_until(&mut engine, |e| on_stack(e, lace).is_none());
    let after = engine
        .state()
        .object(on_the_stack)
        .expect("the creature spell is still on the stack")
        .characteristics()
        .clone();
    assert_eq!(after.colors, ColorSet::of(becomes), "\"becomes\"");
    assert_eq!(
        after.mana_cost, before.mana_cost,
        "\"its mana symbols remain unchanged\""
    );

    pass_until(&mut engine, stack_is_empty);
    let permanent = on_battlefield(&engine, p0, spell).expect("the spell resolved");
    assert_eq!(
        engine
            .state()
            .object(permanent)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(becomes),
        "the permanent it became keeps the color"
    );
}

pub fn red_elemental_blast() -> CardIndex {
    card_index("bb329a5c-b9f9-4973-a53f-090024146325")
}

pub fn fog() -> CardIndex {
    card_index("27e9db49-7af7-4bef-ad4c-bf5dfb92030d")
}

pub fn purelace() -> CardIndex {
    card_index("3773001a-8868-49ec-a406-298cf72359c2")
}

pub fn lifelace() -> CardIndex {
    card_index("eec1de80-4b3d-481d-a235-c299e0381830")
}

pub fn natural_selection() -> CardIndex {
    card_index("57f90b30-bcb0-447e-8788-5c5ded187207")
}

pub fn power_sink() -> CardIndex {
    card_index("39412e6d-2837-4729-abf9-e64a5ba87e40")
}

/// Casts Power Sink for `x` at `target` (an object on the stack), answering
/// the announce-time questions (CR 601.2b, 601.2c) in whichever order the
/// engine asks them, and returns the target menu it published.
#[track_caller]
pub fn cast_power_sink_at(
    engine: &mut Engine<RegistryLookup>,
    caster: PlayerId,
    x: u32,
    target: ObjectId,
) -> Vec<ObjectId> {
    let sink = in_hand(engine, caster, power_sink()).expect("Power Sink in hand");
    engine
        .apply(caster, PlayerAction::CastSpell { card: sink })
        .expect("Power Sink is castable");
    let mut asked_x = false;
    let mut targeted = false;
    let mut menu = Vec::new();
    for _ in 0..8 {
        if asked_x && targeted {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert!(
                    min <= x && x <= max,
                    "X = {x} must be one of the values on offer: {min}..={max}"
                );
                engine
                    .apply(player, PlayerAction::ChooseNumber(x))
                    .expect("the answer came out of the question");
                asked_x = true;
            }
            Pending::ChooseTargets {
                player, options, ..
            } => {
                menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![target],
                        },
                    )
                    .expect("the named spell was on the menu");
                targeted = true;
            }
            other => panic!("unexpected while casting Power Sink: {other:?}"),
        }
    }
    assert!(
        asked_x && targeted,
        "both questions were asked and answered"
    );
    menu
}

pub fn fork() -> CardIndex {
    card_index("50c53ae0-51ba-4046-ac74-87c65e688032")
}

pub fn berserk() -> CardIndex {
    card_index("8b67d192-9a05-4a47-82ae-5fc4b7834d88")
}

pub fn siren_s_call() -> CardIndex {
    card_index("269fc857-a052-4f0a-9759-467ccf42bebb")
}

pub fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Taps whichever untapped basic land `seat` still has, floating one mana
/// of its color. `legal.castable` reads the pool that is already floating
/// (CR 106.4 empties it at the end of every step), never a land that merely
/// *could* be tapped, so a "not castable" reading is only about timing when
/// this was called first — otherwise it is indistinguishable from "no mana".
///
/// Filtered for an actual land rather than taking `mana_abilities.first()`
/// blind: that list also carries a granted mana ability with no printed
/// source (`docs/protocol.md` §"Granted mana"), which is not a land this
/// helper could have meant to tap.
#[track_caller]
pub fn float_one_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let source = *priority_offer(engine)
        .mana_abilities
        .iter()
        .find(|&&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.characteristics().types.contains(TypeSet::LAND))
        })
        .expect("an untapped basic land is still available");
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .expect("the land taps for mana");
}

pub fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

// ---------------------------------------------------------- Blaze of Glory

pub fn blaze_of_glory() -> CardIndex {
    card_index("b330ac89-790e-4cc9-96a5-532c48252088")
}

pub fn scathe_zombies() -> CardIndex {
    card_index("e0fefaf0-da20-4d58-8db7-019dba16c780")
}

pub fn fire_sprites() -> CardIndex {
    card_index("fc5e42b5-4da2-4777-828b-138c0a5d234f")
}

pub fn pearled_unicorn() -> CardIndex {
    card_index("c071be90-0531-40cc-af46-0cbe80c4ddd4")
}

pub fn hurloon_minotaur() -> CardIndex {
    card_index("8f1dae40-b307-446e-bbd2-86aa35813871")
}

/// What `actions.rs`' block-requirement check refuses a declaration with.
pub const MUST_BLOCK: &str = "a creature that must block if able does not";

/// Asserts `result` is the engine's own refusal `why`, naming the message
/// rather than merely that something failed.
#[track_caller]
pub fn refused(result: Result<(), EngineError>, why: &str) {
    match result {
        Err(EngineError::IllegalAction(message)) => assert_eq!(message, why),
        other => panic!("expected the refusal {why:?}, got {other:?}"),
    }
}

/// Taps one of `seat`'s untapped Plains for `{W}`, and asserts that exactly
/// one white mana is now floating — so a "not castable" refusal checked
/// right after this can never be read as "not enough mana" instead.
#[track_caller]
pub fn float_one_white(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let source = all_on_battlefield(engine, seat, plains())
        .into_iter()
        .find(|id| !is_tapped(engine, *id))
        .expect("an untapped Plains remains");
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .expect("a Plains taps for white");
    assert_eq!(
        engine.state().players[seat.get() as usize]
            .mana_pool
            .available(ManaColor::White),
        1,
        "exactly the one white this float put there"
    );
}

pub fn wall_of_stone() -> CardIndex {
    card_index("cd4cadb4-3156-49bd-b36e-12ba5c85938b")
}

// -------------------------------------------------------------- Simulacrum

pub fn simulacrum() -> CardIndex {
    card_index("20d69989-7250-40c7-a064-8ed78ccbe556")
}

pub fn white_knight() -> CardIndex {
    card_index("ddb021df-ae4a-4ac1-8353-d0b375761714")
}

pub fn flawless_maneuver() -> CardIndex {
    card_index("4e183439-17d2-47ff-9d99-5e22821d91e3")
}

// oracle_id = "7bb41690-f8ec-462a-ba29-be453eb86fca"
pub fn sandstorm() -> CardIndex {
    card_index("7bb41690-f8ec-462a-ba29-be453eb86fca")
}

// oracle_id = "3483946d-8645-4c22-b0ba-a65a44456324"
pub fn army_of_allah() -> CardIndex {
    card_index("3483946d-8645-4c22-b0ba-a65a44456324")
}

// oracle_id = "0c017406-7fc3-4701-93ec-ddb02044c12a"
pub fn piety() -> CardIndex {
    card_index("0c017406-7fc3-4701-93ec-ddb02044c12a")
}

pub fn crumble() -> CardIndex {
    card_index("8d6e39b0-a190-40a0-a8e1-ee82f477376f")
}

/// Living Wall: an artifact creature of mana value 4 whose own `{1}` ability
/// can raise the shield every "it can't be regenerated" test has to ignore.
pub fn living_wall() -> CardIndex {
    card_index("4844312c-3c9d-4ca1-986d-4ad35e68454e")
}

/// p0 at `life`, with Llanowar Elves on the stack (paid by a Forest) and
/// Mental Misstep in hand; `islands` Islands are tapped into the pool too.
/// Returns the engine at p0's priority with the Elves on the stack.
pub fn misstep_board(life: i32, islands: usize) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut lands = vec![forest()];
    lands.extend(std::iter::repeat_n(island(), islands));
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &lands)
        .hand(0, &[llanowar_elves(), mental_misstep()])
        .life(0, life)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, llanowar_elves());
    assert!(on_stack(&engine, llanowar_elves()).is_some());
    engine
}

pub fn misstep_is_offered(engine: &Engine<RegistryLookup>) -> bool {
    let p0 = PlayerId::new(0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    in_hand(engine, p0, mental_misstep()).is_some_and(|card| legal.castable.contains(&card))
}

pub fn aim_misstep_at_the_elves(engine: &mut Engine<RegistryLookup>) {
    let p0 = PlayerId::new(0);
    let elves = on_stack(engine, llanowar_elves()).expect("the Elves are on the stack");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the target question, got {:?}", engine.pending())
    };
    assert!(options.contains(&elves));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
}

pub fn assert_plural_color_change(card: CardIndex, color: Color, mana: ManaColor) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The fourth creature is not a target. It must not already be the colour
    // on offer, or "it stayed" would hold for a spell that recoloured it too.
    let lions = baylee_core::generated::index::SAVANNAH_LIONS;
    let (third, third_was, fourth, fourth_was) = if color == Color::Green {
        (quiet_creature(), Color::Green, lions, Color::White)
    } else {
        (lions, Color::White, quiet_creature(), Color::Green)
    };
    assert_ne!(
        fourth_was, color,
        "the untargeted creature has another colour"
    );
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[card])
        .battlefield(0, &[quiet_creature(), quiet_creature()])
        .battlefield(1, &[third, fourth])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let creatures = e.state().zones.list(ZoneLocation::Battlefield).clone();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(mana, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, card);
    let hash = e.state().snapshot_hash();
    assert!(
        e.apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.state().snapshot_hash(), hash);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: creatures[..3].to_vec(),
            players: vec![],
        },
    )
    .unwrap();
    // A target that leaves does not stop the other two from changing color.
    bury(&mut e, &[creatures[1]]);
    pass_until(&mut e, stack_is_empty);
    for &id in &[creatures[0], creatures[2]] {
        assert_eq!(
            e.state().object(id).unwrap().characteristics().colors,
            ColorSet::of(color)
        );
    }
    assert_eq!(
        e.state()
            .object(creatures[3])
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(fourth_was),
        "the creature that was not a target keeps its own colour"
    );
    reach_their_main_phase(&mut e, p1);
    for (id, original) in [(creatures[0], Color::Green), (creatures[2], third_was)] {
        assert_eq!(
            e.state().object(id).unwrap().characteristics().colors,
            ColorSet::of(original)
        );
    }
    assert_eq!(
        e.state()
            .object(creatures[3])
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(fourth_was)
    );
}
