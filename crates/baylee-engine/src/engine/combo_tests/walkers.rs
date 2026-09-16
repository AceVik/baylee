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
