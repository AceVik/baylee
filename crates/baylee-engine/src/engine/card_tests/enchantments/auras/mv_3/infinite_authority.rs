//! `cards/enchantments/auras/mv_3/infinite_authority.rs`, played.
//!
//! Oracle: "Enchant creature. Whenever enchanted creature blocks or becomes
//! blocked by a creature with toughness 3 or less, destroy the other creature
//! at end of combat. At the beginning of the next end step, if that creature
//! was destroyed this way, put a +1/+1 counter on the first creature."
//!
//! Written from that text, the two Scryfall rulings (one counter per creature
//! that triggers it; moving the Aura after the trigger changes nothing) and
//! the Comprehensive Rules: CR 603.7 (delayed triggered abilities),
//! CR 701.8a (destroy), CR 701.19a (regenerate), CR 704.5g (lethal damage
//! destroys a creature, which is not "destroyed this way"), CR 122.1a
//! (+1/+1 counters).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::aura_kit as kit;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_cards_dsl::CounterKind;
use baylee_core::generated::index;

// oracle_id = "b4faba1a-23db-4678-9ce6-a7816105f22a"
fn infinite_authority() -> CardIndex {
    card_index("b4faba1a-23db-4678-9ce6-a7816105f22a")
}

// oracle_id = "c80935bf-c17f-4940-ad9f-3f4e3e8f71bc"
fn enchantment_alteration() -> CardIndex {
    card_index("c80935bf-c17f-4940-ad9f-3f4e3e8f71bc")
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const WWW: [(ManaColor, u32); 1] = [(ManaColor::White, 3)];

/// The +1/+1 counters on `id`.
#[track_caller]
fn plus_counters(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    counters_on(engine, id, CounterKind::P1P1)
}

/// Plays on, passing priority, dividing the attacker's combat damage as
/// fully as it allows and answering "use the regeneration shield?" with
/// `regenerate`, until `done` holds.
#[track_caller]
fn play_until(
    engine: &mut Engine<RegistryLookup>,
    regenerate: bool,
    done: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..150 {
        if done(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseNumber { player, max, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseNumber(max))
                    .unwrap();
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::UseRegenerationShield { .. },
                ..
            } => {
                engine
                    .apply(player, PlayerAction::YesNo(regenerate))
                    .unwrap();
            }
            other => panic!("unexpected while playing on: {other:?}"),
        }
    }
    panic!("the game never got where this scenario was going");
}

/// Plays on until `step` of `turn_of`'s turn has begun (its beginning
/// triggers are on the stack, and the holder has priority), then lets the
/// stack empty.
#[track_caller]
fn through_step(
    engine: &mut Engine<RegistryLookup>,
    regenerate: bool,
    turn_of: PlayerId,
    step: Step,
) {
    play_until(engine, regenerate, |e| {
        e.state().turn.active == turn_of
            && e.state().turn.step == step
            && matches!(e.pending(), Pending::Priority { .. })
    });
    lk::settle(engine);
}

/// Seat 0 wears the Aura on a `host` and attacks with it, seat 1 blocks it
/// with `blocker`, and the stack is left to empty: the declare-blockers step
/// with the Aura's trigger, if there is one, resolved.
#[track_caller]
fn host_attacks_into(
    host: CardIndex,
    blocker: CardIndex,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = lk::game(SEED, &[host], &[blocker], &[infinite_authority()], &[]);
    let attacker = lk::obj(&engine, P0, host);
    let wall = lk::obj(&engine, P1, blocker);
    kit::cast_aura(&mut engine, P0, infinite_authority(), attacker, &WWW);
    lk::attack(&mut engine, &[attacker]);
    lk::block(&mut engine, &[(wall, attacker)]);
    pass_until(&mut engine, stack_is_empty);
    (engine, attacker, wall)
}

/// "Whenever enchanted creature … becomes blocked by a creature with toughness
/// 3 or less, destroy the other creature at end of combat." A Grizzly Bears
/// wearing the Aura is blocked by a Wall of Wood (0/3, the largest toughness
/// the card names). The Wall is *not* destroyed when the trigger resolves or
/// when combat damage is dealt (two damage on a 0/3): it is destroyed as the
/// end-of-combat step begins. And "at the beginning of the next end step" the
/// Bears get their counter, and not before.
#[test]
fn a_creature_blocked_by_toughness_three_is_destroyed_at_end_of_combat_and_the_host_grows_at_the_end_step()
 {
    let (mut engine, bears, wood) = host_attacks_into(index::GRIZZLY_BEARS, index::WALL_OF_WOOD);
    assert!(
        kit::on_the_table(&engine, wood),
        "the trigger only promises the destruction: the Wall is still here"
    );

    // After combat damage, before the end of combat step.
    play_until(&mut engine, false, |e| {
        e.state().turn.step == Step::CombatDamage && matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(
        kit::on_the_table(&engine, wood),
        "combat damage (two on a 0/3) did not kill it, and nothing else has yet"
    );
    assert_eq!(lk::damage_on(&engine, wood), 2, "the Bears did hit it");

    through_step(&mut engine, false, P0, Step::CombatEnd);
    assert!(
        !kit::on_the_table(&engine, wood),
        "\"destroy the other creature at end of combat\""
    );
    assert_eq!(
        kit::zone_now(&engine, wood),
        Zone::Graveyard,
        "destroyed means its owner's graveyard (CR 701.8a)"
    );
    assert_eq!(
        plus_counters(&engine, bears),
        0,
        "the counter waits for the next end step"
    );

    through_step(&mut engine, false, P0, Step::End);
    assert_eq!(
        plus_counters(&engine, bears),
        1,
        "\"if that creature was destroyed this way, put a +1/+1 counter on the first creature\""
    );
    assert_eq!(pt(&engine, bears), (3, 3), "a 2/2 with one +1/+1 counter");
}

/// "… a creature with toughness 3 or less". The edge is between three and
/// four: the same Bears blocked by a Wall of Wood (0/3) destroy it and grow;
/// blocked by a Steel Wall (0/4) they destroy nothing and get nothing.
#[test]
fn a_blocker_with_toughness_four_is_left_alone() {
    for (blocker, destroyed) in [(index::STEEL_WALL, false), (index::WALL_OF_WOOD, true)] {
        let (mut engine, bears, wall) = host_attacks_into(index::GRIZZLY_BEARS, blocker);
        through_step(&mut engine, false, P0, Step::End);
        assert_eq!(
            kit::on_the_table(&engine, wall),
            !destroyed,
            "the blocker survives exactly when its toughness is above three"
        );
        assert_eq!(
            plus_counters(&engine, bears),
            u16::from(destroyed),
            "and the counter comes exactly when something was destroyed"
        );
    }
}

/// "Whenever enchanted creature *blocks* …": the other direction. Seat 0's
/// Wall of Wood wears the Aura and blocks seat 1's Grizzly Bears (toughness
/// 2). The Bears are destroyed at end of combat, and at the end step of that
/// turn -- seat 1's -- the Wall of Wood has its counter.
#[test]
fn an_enchanted_blocker_destroys_the_attacker_and_grows() {
    let mut engine = lk::game(
        SEED,
        &[index::WALL_OF_WOOD],
        &[index::GRIZZLY_BEARS],
        &[infinite_authority()],
        &[],
    );
    let wood = lk::obj(&engine, P0, index::WALL_OF_WOOD);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    kit::cast_aura(&mut engine, P0, infinite_authority(), wood, &WWW);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    let offer = lk::declare(&mut engine, &[bears]);
    assert!(lk::may_block(&offer, wood, bears), "the Wall may block it");
    lk::block(&mut engine, &[(wood, bears)]);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        kit::on_the_table(&engine, bears),
        "not destroyed before the end of combat"
    );

    through_step(&mut engine, false, P1, Step::CombatEnd);
    assert_eq!(
        kit::zone_now(&engine, bears),
        Zone::Graveyard,
        "the attacker, toughness two, was destroyed at end of combat"
    );
    through_step(&mut engine, false, P1, Step::End);
    assert_eq!(
        plus_counters(&engine, wood),
        1,
        "the enchanted creature, the first creature, got the counter at seat 1's end step"
    );
    assert_eq!(pt(&engine, wood), (1, 4), "a 0/3 with one counter");
}

/// Ruling (2004): "The enchanted creature gets a +1/+1 counter for each
/// creature that triggers the ability." Two Walls of Wood both block the
/// Bears: each one triggers it, both are destroyed at end of combat, and at
/// the end step there are two counters.
#[test]
fn two_blockers_make_two_triggers_and_two_counters() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::WALL_OF_WOOD, index::WALL_OF_WOOD],
        &[infinite_authority()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let walls = kit::battlefield(&engine)
        .into_iter()
        .filter(|id| kit::controller_of(&engine, *id) == P1)
        .collect::<Vec<_>>();
    assert_eq!(walls.len(), 2, "two Walls of Wood");
    kit::cast_aura(&mut engine, P0, infinite_authority(), bears, &WWW);
    lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[(walls[0], bears), (walls[1], bears)]);
    pass_until(&mut engine, stack_is_empty);

    through_step(&mut engine, false, P0, Step::CombatEnd);
    for wall in &walls {
        assert_eq!(
            kit::zone_now(&engine, *wall),
            Zone::Graveyard,
            "each blocker was destroyed at end of combat"
        );
    }
    through_step(&mut engine, false, P0, Step::End);
    assert_eq!(
        plus_counters(&engine, bears),
        2,
        "one +1/+1 counter for each creature that triggered it"
    );
    assert_eq!(pt(&engine, bears), (4, 4));
}

/// "… if that creature was destroyed *this way*": a blocker that died to
/// combat damage (CR 704.5g destroys it, but not by this ability) gives no
/// counter. A Hill Giant wearing the Aura kills a blocking Grizzly Bears in
/// the damage step; at end of combat there is nothing left to destroy and the
/// end step brings no counter. The control, in the same turn structure, is the
/// Bears wearing it against a Wall of Wood that the ability itself destroys.
#[test]
fn a_blocker_that_died_to_damage_was_not_destroyed_this_way() {
    for (host, blocker, counters) in [
        (index::HILL_GIANT, index::GRIZZLY_BEARS, 0),
        (index::GRIZZLY_BEARS, index::WALL_OF_WOOD, 1),
    ] {
        let (mut engine, attacker, other) = host_attacks_into(host, blocker);
        through_step(&mut engine, false, P0, Step::CombatEnd);
        assert_eq!(
            kit::zone_now(&engine, other),
            Zone::Graveyard,
            "the blocker is gone by the end of combat either way"
        );
        through_step(&mut engine, false, P0, Step::End);
        assert_eq!(
            plus_counters(&engine, attacker),
            counters,
            "counters after a blocker that {}",
            if counters == 0 {
                "died to damage"
            } else {
                "the ability destroyed"
            }
        );
    }
}

/// "… if that creature was destroyed this way": a creature that regenerates
/// is not destroyed (CR 701.19a), so there is no counter. Seat 0's enchanted
/// Wall of Wood blocks seat 1's Drudge Skeletons (1/1; the Wood deals it
/// nothing, so the shield is still there at end of combat). With `{B}` spent
/// to regenerate it survives the end of combat and the Wall gets nothing;
/// without it, the Skeletons are destroyed and the Wall grows.
#[test]
fn a_creature_that_regenerates_is_not_destroyed_this_way_and_gives_no_counter() {
    for regenerate in [true, false] {
        let mut engine = lk::game(
            SEED,
            &[index::WALL_OF_WOOD],
            &[index::DRUDGE_SKELETONS],
            &[infinite_authority()],
            &[],
        );
        let wood = lk::obj(&engine, P0, index::WALL_OF_WOOD);
        let skeletons = lk::obj(&engine, P1, index::DRUDGE_SKELETONS);
        kit::cast_aura(&mut engine, P0, infinite_authority(), wood, &WWW);
        pass_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
        );
        lk::declare(&mut engine, &[skeletons]);
        lk::block(&mut engine, &[(wood, skeletons)]);
        if regenerate {
            lk::float(&mut engine, P1, &[(ManaColor::Black, 1)]);
            lk::use_ability(&mut engine, P1, index::DRUDGE_SKELETONS, 0, &[], &[]);
            assert_eq!(
                engine
                    .state()
                    .object(skeletons)
                    .unwrap()
                    .regeneration_shields,
                1,
                "the {{B}} bought a shield"
            );
        }
        pass_until(&mut engine, stack_is_empty);

        through_step(&mut engine, regenerate, P1, Step::End);
        assert_eq!(
            kit::on_the_table(&engine, skeletons),
            regenerate,
            "the Skeletons survive the destruction exactly when they regenerated"
        );
        assert_eq!(
            plus_counters(&engine, wood),
            u16::from(!regenerate),
            "and the Wall gets its counter exactly when they did not"
        );
    }
}

/// Ruling (2006): "Moving the enchantment after the ability triggers will not
/// affect which creatures are affected." The Bears wear the Aura and are
/// blocked by a Wall of Wood; with the trigger on the stack, seat 0 casts
/// Enchantment Alteration and moves the Aura to a Hill Giant that is not in
/// combat. The Wall is still destroyed at end of combat and the *Bears*, the
/// creature the Aura was on when it triggered, get the counter, not the Giant.
#[test]
fn moving_the_aura_after_it_triggers_changes_nothing_about_who_is_affected() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[index::WALL_OF_WOOD],
        &[infinite_authority(), enchantment_alteration()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let wood = lk::obj(&engine, P1, index::WALL_OF_WOOD);
    let aura = kit::cast_aura(&mut engine, P0, infinite_authority(), bears, &WWW);
    lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[(wood, bears)]);
    assert!(
        !stack_is_empty(&engine),
        "the Aura's trigger is on the stack, seat 0 has priority"
    );

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, enchantment_alteration());
    lk::aim(&mut engine, &[aura], &[]);
    pass_until(&mut engine, kit::asks_for_a_permanent);
    kit::choose_among(&mut engine, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        kit::attached_to(&engine, aura),
        Some(giant),
        "the Aura now enchants the Giant"
    );

    through_step(&mut engine, false, P0, Step::CombatEnd);
    assert_eq!(
        kit::zone_now(&engine, wood),
        Zone::Graveyard,
        "the Wall was still destroyed at end of combat"
    );
    through_step(&mut engine, false, P0, Step::End);
    assert_eq!(
        plus_counters(&engine, bears),
        1,
        "the counter went to the creature that was enchanted when the ability triggered"
    );
    assert_eq!(
        plus_counters(&engine, giant),
        0,
        "and not to the creature the Aura moved to"
    );
}
