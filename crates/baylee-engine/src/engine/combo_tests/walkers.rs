//! What a planeswalker does to the list of legal actions, from both ends. Karn, the Great Creator's static takes an opponent's artifact abilities off it — printed, intrinsic (CR 305.6) and granted alike, all three once Mycosynth Lattice has made every permanent an artifact — and a loyalty ability whose "up to one target" has nothing to point at has to stay on it and then resolve without aiming its own `Filter::This` at the walker. Everything is read through `LegalActions` and never through `apply` alone, because a refusal that lives only there is invisible from the seat that is not being refused. A walker's loyalty arithmetic and its abilities on their own are `walker_tests`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Karn, the Great Creator plus an artifact land across the table.
///
/// `karns_lock_spares_a_teammate` proved the lock refuses the right seat;
/// this asks the other half of the same question, which nothing asked:
/// whether the seat it locks is still being *offered* what it cannot do.
/// It was. The refusal lived in `apply` alone, and that is invisible from
/// Karn's own side of the table — the abilities the lock stops are on the
/// opponent's board, and an opponent's board is not what a test driving
/// Karn looks at.
///
/// Two bystanders, because the lock has two edges. My own Llanowar Elves
/// taps for mana exactly as before — the lock is about artifacts, not about
/// me — and Karn's controller keeps their own Vault, which is the sentence
/// `karns_lock_spares_a_teammate` was written for, read here through the
/// offer instead of through the refusal.
#[test]
fn karns_lock_takes_their_artifact_off_the_list_and_leaves_the_rest_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(76, forest())
        .battlefield(0, &[vault_of_whispers(), llanowar_elves()])
        .battlefield(1, &[karn_the_great_creator(), vault_of_whispers()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_vault = on_battlefield(&engine, p0, vault_of_whispers()).expect("my vault");
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves");
    let their_vault = on_battlefield(&engine, p1, vault_of_whispers()).expect("their vault");

    assert!(
        !offers_an_ability(&engine, my_vault),
        "Karn's lock stops my artifact land's mana ability, so it must not be offered"
    );
    assert!(
        offers_an_ability(&engine, my_elves),
        "the lock is about artifacts; an Elf Druid taps for mana as before"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: my_vault,
                    ability_index: 0,
                },
            )
            .is_err(),
        "the two probes have to agree: what is not offered is not applied"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "expected the opponent to hold priority, got {:?}",
        engine.pending()
    );
    assert!(
        offers_an_ability(&engine, their_vault),
        "the lock spares its own controller: {:?}",
        engine.pending()
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: their_vault,
                ability_index: 0,
            },
        )
        .expect("Karn's controller may still tap their own artifact land");
}

/// Karn, the Great Creator plus Mycosynth Lattice — the lock the pair is
/// famous for — with a Chromatic Lantern under it, so that all three doors
/// onto `LegalActions` are shut at once.
///
/// [`karns_lock_takes_their_artifact_off_the_list_and_leaves_the_rest_alone`]
/// reaches the lock through `LegalActions::abilities`, because a Vault of
/// Whispers prints its own `{T}: Add {B}`. A Plains prints nothing: its mana
/// comes off the type line (CR 305.6) and is offered through the separate
/// `mana_abilities` list. The Lantern adds the third — "lands you control
/// have `{T}`: Add one mana of any color" is *granted*, and a granted
/// ability is offered from its own loop under a synthetic index. All three
/// are activated abilities of an artifact once the Lattice has spoken, and
/// each was a separate `push` that had to learn the same word.
///
/// The bystander is across the table and is the same card: Karn's controller
/// keeps their own Plains, because the lock reads "artifacts your opponents
/// control" and the Lattice does not change whose permanent anything is.
#[test]
fn karn_and_the_lattice_lock_their_basic_lands_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(78, forest())
        .battlefield(0, &[plains(), llanowar_elves(), chromatic_lantern()])
        .battlefield(
            1,
            &[karn_the_great_creator(), mycosynth_lattice(), plains()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_plains = on_battlefield(&engine, p0, plains()).expect("my plains");
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves");
    let my_lantern = on_battlefield(&engine, p0, chromatic_lantern()).expect("my lantern");
    let their_plains = on_battlefield(&engine, p1, plains()).expect("their plains");

    assert!(
        !offers_an_ability(&engine, my_plains),
        "under the Lattice my Plains is an artifact, so neither its own mana \
         ability nor the one the Lantern grants it may be offered"
    );
    assert!(
        !offers_an_ability(&engine, my_elves),
        "so is my Elf Druid, and so is its"
    );
    assert!(
        !offers_an_ability(&engine, my_lantern),
        "the Lantern is an artifact whatever the Lattice says"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: my_plains })
            .is_err(),
        "the action validates against the offered list, so taking the land \
         off it is what refuses the tap"
    );

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        offers_an_ability(&engine, their_plains),
        "the Lattice does not change whose permanent anything is: {:?}",
        engine.pending()
    );
    engine
        .apply(
            p1,
            PlayerAction::ActivateManaAbility {
                source: their_plains,
            },
        )
        .expect("Karn's controller taps their own land as before");
}

/// Karn, the Great Creator's `+1` with nothing to point at.
///
/// "Up to **one** target noncreature artifact" was written as a bare
/// `TargetSpec`, which the engine reads as *exactly* one — so on a board
/// with no artifact on it the ability was not offered at all, and a walker
/// that should have ticked to 6 sat at 5. That is a loyalty the printing
/// allows and this engine refused.
///
/// The assertion with teeth is the last one. `Filter::This` inside a
/// targeted ability means *the target*, and the effect that registers it
/// falls back to the ability's own source when there is no target: an
/// unguarded "up to one" would make Karn himself an artifact creature with
/// power and toughness equal to a mana value nobody chose, and the next
/// state-based check would sweep the walker into the graveyard. Offering
/// the ability and resolving it are two different fixes, and only this
/// checks the second.
#[test]
fn karns_plus_one_ticks_up_with_nothing_to_point_at() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(95, forest())
        .battlefield(0, &[karn_the_great_creator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    assert!(
        offers_an_ability(&engine, karn),
        "an ability that may target nothing is offered with nothing on the board"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 1,
            },
        )
        .expect("the +1 may be activated with no target");
    assert!(
        !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "a choice with nothing in it is not a question: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_clear);

    let walker = engine
        .state()
        .object(karn)
        .expect("the walker is still an object");
    assert_eq!(
        walker.counters.get(baylee_cards_dsl::CounterKind::Loyalty),
        6,
        "the +1 is the whole point of activating it with no target"
    );
    assert!(
        !walker
            .characteristics()
            .types
            .intersects(baylee_core::types::TypeSet::CREATURE),
        "the animation had no target and must not have fallen back onto Karn"
    );
    assert!(
        on_battlefield(&engine, p0, karn_the_great_creator()).is_some(),
        "and Karn is still on the battlefield"
    );
}

/// Teferi, Time Raveler's `−3` with nothing to bounce.
///
/// The same sentence, and the half that costs a card: "Return up to one
/// target artifact, creature, or enchantment to its owner's hand. **Draw a
/// card.**" Read as exactly one target, an empty board made the whole
/// ability unactivatable — the draw included — which is a card a player
/// simply never got.
#[test]
fn teferis_minus_three_draws_with_nothing_to_bounce() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(96, forest())
        .battlefield(0, &[teferi_time_raveler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let teferi = on_battlefield(&engine, p0, teferi_time_raveler()).expect("teferi deployed");
    let before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: teferi,
                ability_index: 2,
            },
        )
        .expect("the -3 may be activated with nothing to return");
    pass_until(&mut engine, stack_is_clear);
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        before + 1,
        "the card is drawn whether or not anything was returned"
    );
}

/// Karn, the Great Creator's `+1` on a Lightning Greaves that equips an Elf.
///
/// The Greaves "becomes an artifact creature with power and toughness each
/// equal to its mana value", a 2/2, and a creature attached to anything
/// becomes unattached and remains on the battlefield (CR 704.5p, first
/// sentence). Its host is still a creature, which is all CR 704.5n asks of an
/// Equipment's host, so only that first sentence takes it off; the
/// state-based action read the second alone, and the animated Greaves went
/// on handing the Elf haste and shroud.
///
/// The bystander is the second Greaves on the second Elf, which Karn did not
/// point at: still only an Equipment, still attached, still granting.
#[test]
#[allow(clippy::too_many_lines)] // two equips, the +1 and both Greaves read after it
fn karns_plus_one_takes_an_animated_equipment_off_its_creature() {
    use baylee_cards_dsl::KeywordSet;
    use baylee_core::types::TypeSet;
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(97, forest())
        .battlefield(
            0,
            &[
                karn_the_great_creator(),
                lightning_greaves(),
                lightning_greaves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let all = |engine: &Engine<RegistryLookup>, card| -> Vec<baylee_core::ids::ObjectId> {
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                engine
                    .state()
                    .object(*id)
                    .and_then(|o| o.card)
                    .is_some_and(|c| c.index == card)
            })
            .collect()
    };
    let (greaves, elves) = (
        all(&engine, lightning_greaves()),
        all(&engine, llanowar_elves()),
    );
    let (&[animated, bystander], &[elf, other_elf]) = (greaves.as_slice(), elves.as_slice()) else {
        panic!("two Greaves and two Elves are out: {greaves:?}, {elves:?}")
    };
    // Equip {0} is ability 1; ability 0 is the static that grants.
    for (gear, host) in [(animated, elf), (bystander, other_elf)] {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: gear,
                    ability_index: 1,
                },
            )
            .expect("equip {0} is offered at sorcery speed");
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![host],
                },
            )
            .expect("an Elf nothing equips yet is a creature you control");
        pass_until(&mut engine, |e| {
            e.state()
                .object(gear)
                .is_some_and(|o| o.attached_to == Some(host))
        });
    }
    let keywords = |engine: &Engine<RegistryLookup>, id| {
        engine
            .state()
            .object(id)
            .expect("on the battlefield")
            .characteristics()
            .keywords
    };
    // `KeywordSet::contains` is has-any, so each keyword is asked alone.
    let granted = |engine: &Engine<RegistryLookup>, id| {
        let kw = keywords(engine, id);
        (
            kw.contains(KeywordSet::HASTE),
            kw.contains(KeywordSet::SHROUD),
        )
    };
    assert_eq!(
        granted(&engine, elf),
        (true, true),
        "the Greaves equips the Elf before Karn says anything"
    );

    let karn = on_battlefield(&engine, p0, karn_the_great_creator()).expect("karn deployed");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: karn,
                ability_index: 1,
            },
        )
        .expect("the +1 is offered");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![animated],
            },
        )
        .expect("an attached Greaves is a noncreature artifact");
    pass_until(&mut engine, stack_is_clear);

    let gear = engine
        .state()
        .object(animated)
        .expect("the Greaves is never destroyed by this");
    assert!(
        gear.characteristics().types.contains(TypeSet::CREATURE),
        "Karn made it an artifact creature"
    );
    assert_eq!(
        gear.attached_to, None,
        "and a creature attached to anything becomes unattached (CR 704.5p)"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&animated),
        "and remains on the battlefield"
    );
    assert_eq!(
        granted(&engine, elf),
        (false, false),
        "so the Elf it equipped has neither haste nor shroud any more"
    );

    let kept = engine
        .state()
        .object(bystander)
        .expect("the bystander is on the battlefield");
    assert!(
        !kept.characteristics().types.contains(TypeSet::CREATURE),
        "the Greaves Karn did not point at is still only an Equipment"
    );
    assert_eq!(kept.attached_to, Some(other_elf), "and still attached");
    assert_eq!(
        granted(&engine, other_elf),
        (true, true),
        "and still granting"
    );
}

/// Whether `seat` holds priority now with Demonic Tutor among what it may
/// cast, after tapping every land it has for mana.
fn tutor_castable_now(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> bool {
    tap_all_mana(engine, seat);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, seat);
    in_hand(engine, seat, demonic_tutor()).is_some_and(|card| legal.castable.contains(&card))
}

/// Teferi, Time Raveler's `+1`: "Until your next turn, you may cast sorcery
/// spells as though they had flash."
///
/// Demonic Tutor is offered and cast on the opponent's turn, which is the
/// sentence, and the permission is gone once Teferi's controller's next
/// turn begins. The engine already did this (owner report, 05.10.): what
/// failed was the client, which cannot see the permission in the view.
#[test]
fn teferis_plus_one_lets_a_sorcery_be_cast_on_the_opponents_turn_until_his_next() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(98, forest())
        .battlefield(0, &[teferi_time_raveler(), swamp(), swamp()])
        .hand(0, &[demonic_tutor(), demonic_tutor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let teferi = on_battlefield(&engine, p0, teferi_time_raveler()).expect("Teferi");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: teferi,
                ability_index: 1,
            },
        )
        .expect("the +1 is offered");
    pass_until(&mut engine, stack_is_clear);

    // The opponent's upkeep, with p0 holding priority.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        tutor_castable_now(&mut engine, p0),
        "the +1 gives the Tutor flash on the opponent's turn"
    );
    let tutor = in_hand(&engine, p0, demonic_tutor()).expect("a Tutor in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: tutor })
        .expect("cast at instant speed");
    assert!(on_stack(&engine, demonic_tutor()).is_some());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the Tutor resolves and finds a card");

    // p0's next turn: the permission has ended.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(
        !engine
            .state()
            .effects
            .iter()
            .any(|fx| matches!(fx.modifier, baylee_cards_dsl::Modifier::SorceriesHaveFlash)),
        "until your next turn ends as that turn begins"
    );
}

/// The `+1` on Teferi's controller's own turn, with a spell on the stack:
/// a sorcery is offered over it, which without the `+1` it never is.
#[test]
fn teferis_plus_one_lets_a_sorcery_be_cast_over_a_spell_on_your_own_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(99, forest())
        .battlefield(0, &[teferi_time_raveler(), swamp(), swamp(), forest()])
        .hand(0, &[demonic_tutor(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let teferi = on_battlefield(&engine, p0, teferi_time_raveler()).expect("Teferi");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: teferi,
                ability_index: 1,
            },
        )
        .expect("the +1 is offered");
    pass_until(&mut engine, stack_is_clear);
    cast_from_hand(&mut engine, p0, llanowar_elves());
    assert!(on_stack(&engine, llanowar_elves()).is_some());
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let tutor = in_hand(&engine, p0, demonic_tutor()).expect("the Tutor");
    assert!(
        legal.castable.contains(&tutor),
        "a sorcery over a non-empty stack, as though it had flash"
    );
}
