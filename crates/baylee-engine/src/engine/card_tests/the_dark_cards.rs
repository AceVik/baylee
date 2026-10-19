//! Behavioral card tests for The Dark (`drk`) and `HarperPrism` Book Promos (`phpr`).

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::ChoicePrompt;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}

fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p),
    );
}

fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e);
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}

fn activate_by_card(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex, ability_index: u32) {
    priority(e, p);
    mana(e);
    let source = on_battlefield(e, p, c).unwrap();
    e.apply(
        p,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .unwrap();
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().unwrap();
    if matches!(e.pending(), Pending::ChoosePlayer { .. }) {
        e.apply(p, PlayerAction::ChoosePlayer(players[0])).unwrap();
    } else {
        e.apply(p, PlayerAction::ChooseTargets { objects, players })
            .unwrap();
    }
}

fn choose_objects(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseObjects { objects }).unwrap();
}

fn yes_no(e: &mut Engine<RegistryLookup>, answer: bool) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::YesNo(answer)).unwrap();
}

#[test]
fn amnesia_reveals_hand_and_discards_all_nonland_cards() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::AMNESIA])
        .hand(
            1,
            &[index::FOREST, index::GRIZZLY_BEARS, index::LIGHTNING_BOLT],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    cast(&mut e, P0, index::AMNESIA);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P1)).len(), 1);
    assert!(in_hand(&e, P1, index::FOREST).is_some());
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(P1)).len(), 2);
}

#[test]
fn inquisition_reveals_hand_and_deals_damage_for_each_white_card() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::INQUISITION])
        .hand(
            1,
            &[
                index::SERRA_ANGEL,
                index::SWORDS_TO_PLOWSHARES,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    cast(&mut e, P0, index::INQUISITION);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().players[1].life, 18);
}

#[test]
fn martyr_s_cry_exiles_all_white_creatures_and_each_controller_draws_that_many() {
    let mut e = prepared(
        &[index::MARTYR_S_CRY],
        &[index::SAVANNAH_LIONS, index::GRIZZLY_BEARS],
        &[index::SERRA_ANGEL],
    );
    let p0_hand_before = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    let p1_hand_before = e.state().zones.list(ZoneLocation::Hand(P1)).len();

    cast(&mut e, P0, index::MARTYR_S_CRY);
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P0, index::SAVANNAH_LIONS).is_none());
    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P1, index::SERRA_ANGEL).is_none());
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand_before
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand_before + 1
    );
}

#[test]
fn cleansing_destroys_each_land_unless_any_player_pays_1_life() {
    let mut e = prepared(&[index::CLEANSING], &[index::FOREST], &[index::PLAINS]);

    cast(&mut e, P0, index::CLEANSING);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));

    yes_no(&mut e, true);
    yes_no(&mut e, false);
    yes_no(&mut e, false);
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P0, index::FOREST).is_some());
    assert_eq!(e.state().players[0].life, 19);
    assert!(on_battlefield(&e, P1, index::PLAINS).is_none());
}

#[test]
fn mind_bomb_allows_each_player_to_discard_up_to_three_cards_to_reduce_damage() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::MIND_BOMB, index::FOREST, index::PLAINS])
        .hand(1, &[index::ISLAND])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    cast(&mut e, P0, index::MIND_BOMB);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    choose_objects(&mut e, vec![options[0], options[1]]);
    choose_objects(&mut e, vec![]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(e.state().players[1].life, 17);
}

#[test]
fn leviathan_untaps_when_sacrificing_two_islands_on_upkeep() {
    let mut e = prepared(&[index::LEVIATHAN], &[index::ISLAND, index::ISLAND], &[]);

    cast(&mut e, P0, index::LEVIATHAN);
    pass_until(&mut e, stack_is_empty);
    let levi = on_battlefield(&e, P0, index::LEVIATHAN).unwrap();
    assert!(
        e.state()
            .object(levi)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    pass_until(&mut e, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 2);
    choose_objects(&mut e, options);
    pass_until(&mut e, stack_is_empty);

    assert!(
        !e.state()
            .object(levi)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

#[test]
fn nameless_race_pays_life_capped_by_white_opponents_cards_and_sets_pt() {
    let mut e = prepared(
        &[index::NAMELESS_RACE],
        &[],
        &[index::SAVANNAH_LIONS, index::SERRA_ANGEL],
    );

    cast(&mut e, P0, index::NAMELESS_RACE);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber { min, max, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!((min, max), (0, 2));
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut e, stack_is_empty);

    let race = on_battlefield(&e, P0, index::NAMELESS_RACE).unwrap();
    let chars = e.state().object(race).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(2), Some(2)));
    assert_eq!(e.state().players[0].life, 18);
}

#[test]
fn necropolis_exiles_creature_from_graveyard_for_mana_value_toughness_counters() {
    let mut e = prepared(&[], &[index::NECROPOLIS, index::SERRA_ANGEL], &[]);
    let angel = on_battlefield(&e, P0, index::SERRA_ANGEL).unwrap();
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(
            angel,
            ZoneLocation::Graveyard(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    let angel_in_gy = e.state().zones.list(ZoneLocation::Graveyard(P0))[0];

    activate_by_card(&mut e, P0, index::NECROPOLIS, 0);
    assert!(matches!(
        e.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::CostExile,
            ..
        }
    ));
    choose_objects(&mut e, vec![angel_in_gy]);
    pass_until(&mut e, stack_is_empty);

    let necro = on_battlefield(&e, P0, index::NECROPOLIS).unwrap();
    let chars = e.state().object(necro).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(0), Some(6)));
}

#[test]
fn sewers_of_estark_makes_attacker_unblockable_and_prevents_blocker_damage() {
    let mut e = prepared(
        &[index::SEWERS_OF_ESTARK],
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P1))],
        },
    )
    .unwrap();

    cast(&mut e, P0, index::SEWERS_OF_ESTARK);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);

    assert!(
        e.state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::UNBLOCKABLE)
    );
}

#[test]
fn the_dark_sorceries_and_instants_work() {
    // Ashes to Ashes: exile two nonartifact creatures and deal 5 damage to you.
    let mut e = prepared(
        &[
            index::ASHES_TO_ASHES,
            index::DUST_TO_DUST,
            index::ETERNAL_FLAME,
            index::HOLY_LIGHT,
            index::MARSH_GAS,
            index::WORD_OF_BINDING,
            index::MANA_CLASH,
            index::BLOOD_OF_THE_MARTYR,
            index::FIRE_AND_BRIMSTONE,
            index::MORALE,
            index::FESTIVAL,
        ],
        &[
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::MOUNTAIN,
            index::GRIZZLY_BEARS,
            index::SAVANNAH_LIONS,
        ],
        &[
            index::HILL_GIANT,
            index::PEARLED_UNICORN,
            index::SOL_RING,
            index::BONE_FLUTE,
        ],
    );
    let giant = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();
    let unicorn = on_battlefield(&e, P1, index::PEARLED_UNICORN).unwrap();
    cast(&mut e, P0, index::ASHES_TO_ASHES);
    targets(&mut e, vec![giant, unicorn], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::HILL_GIANT).is_none());
    assert!(on_battlefield(&e, P1, index::PEARLED_UNICORN).is_none());
    assert_eq!(e.state().players[0].life, 15);

    // Dust to Dust: exile two target artifacts.
    let ring = on_battlefield(&e, P1, index::SOL_RING).unwrap();
    let flute = on_battlefield(&e, P1, index::BONE_FLUTE).unwrap();
    cast(&mut e, P0, index::DUST_TO_DUST);
    targets(&mut e, vec![ring, flute], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::SOL_RING).is_none());
    assert!(on_battlefield(&e, P1, index::BONE_FLUTE).is_none());

    // Eternal Flame: 3 Mountains => 3 damage to P1, ceil(3/2)=2 damage to P0.
    cast(&mut e, P0, index::ETERNAL_FLAME);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 17);
    assert_eq!(e.state().players[0].life, 13);

    // Holy Light: nonwhite creatures get -1/-1 until end of turn.
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let lions = on_battlefield(&e, P0, index::SAVANNAH_LIONS).unwrap();
    cast(&mut e, P0, index::HOLY_LIGHT);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(bears).unwrap().characteristics().power,
        Some(1)
    );
    assert_eq!(
        e.state().object(lions).unwrap().characteristics().power,
        Some(2)
    );

    // Marsh Gas: all creatures get -2/-0 until end of turn.
    cast(&mut e, P0, index::MARSH_GAS);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(lions).unwrap().characteristics().power,
        Some(0)
    );

    // Word of Binding: tap X target creatures (X = 1).
    cast(&mut e, P0, index::WORD_OF_BINDING);
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // Blood of the Martyr: cast and resolve.
    cast(&mut e, P0, index::BLOOD_OF_THE_MARTYR);
    pass_until(&mut e, stack_is_empty);

    // Mana Clash: flip coins and deal damage until both flip tails.
    cast(&mut e, P0, index::MANA_CLASH);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
}

#[test]
fn the_dark_combat_instants_and_lands_work() {
    let mut e = prepared(
        &[index::MORALE, index::FIRE_AND_BRIMSTONE],
        &[
            index::GRIZZLY_BEARS,
            index::PEARLED_UNICORN,
            index::ARENA,
            index::CITY_OF_SHADOWS,
            index::SORROW_S_PATH,
        ],
        &[index::SAVANNAH_LIONS, index::HILL_GIANT],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let unicorn = on_battlefield(&e, P0, index::PEARLED_UNICORN).unwrap();
    let lions = on_battlefield(&e, P1, index::SAVANNAH_LIONS).unwrap();

    // Arena: {3}, {T}: tap target creature you control and target creature of opponent's choice; fight.
    activate_by_card(&mut e, P0, index::ARENA, 0);
    targets(&mut e, vec![bears], vec![]);
    targets(&mut e, vec![lions], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::SAVANNAH_LIONS).is_none());

    // City of Shadows: {T}, exile a creature you control -> put a storage counter on City of Shadows.
    activate_by_card(&mut e, P0, index::CITY_OF_SHADOWS, 0);
    choose_objects(&mut e, vec![unicorn]);
    pass_until(&mut e, stack_is_empty);
    let city = on_battlefield(&e, P0, index::CITY_OF_SHADOWS).unwrap();
    assert_eq!(
        e.state()
            .object(city)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::STORAGE),
        1
    );
}

#[test]
fn the_dark_artifacts_work() {
    let mut e = prepared(
        &[index::LIGHTNING_BOLT],
        &[
            index::BARL_S_CAGE,
            index::BONE_FLUTE,
            index::DARK_SPHERE,
            index::LIVING_ARMOR,
            index::MANA_CRYPT,
            index::REFLECTING_MIRROR,
            index::RUNESWORD,
            index::STONE_CALENDAR,
            index::TORMOD_S_CRYPT,
            index::TOWER_OF_COIREALL,
            index::WAND_OF_ITH,
            index::WAR_BARGE,
            index::SERRA_ANGEL,
        ],
        &[index::HILL_GIANT],
    );
    let angel = on_battlefield(&e, P0, index::SERRA_ANGEL).unwrap();
    let giant = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();

    // Barl's Cage: {3}: target creature doesn't untap during controller's next untap step.
    activate_by_card(&mut e, P0, index::BARL_S_CAGE, 0);
    targets(&mut e, vec![giant], vec![]);
    pass_until(&mut e, stack_is_empty);

    // Bone Flute: {2}, {T}: all creatures get -1/-0 until end of turn.
    activate_by_card(&mut e, P0, index::BONE_FLUTE, 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(angel).unwrap().characteristics().power,
        Some(3)
    );

    // Living Armor: {T}, Sacrifice: put X +0/+1 counters on target creature where X is its MV (5).
    activate_by_card(&mut e, P0, index::LIVING_ARMOR, 0);
    targets(&mut e, vec![angel], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(angel).unwrap().characteristics().toughness,
        Some(9)
    );

    // Tormod's Crypt: {T}, Sacrifice: exile target player's graveyard.
    activate_by_card(&mut e, P0, index::TORMOD_S_CRYPT, 0);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);

    // Tower of Coireall: {T}: target creature can't be blocked by Walls this turn.
    activate_by_card(&mut e, P0, index::TOWER_OF_COIREALL, 0);
    targets(&mut e, vec![angel], vec![]);
    pass_until(&mut e, stack_is_empty);

    // War Barge: {3}: target creature gains islandwalk until end of turn.
    activate_by_card(&mut e, P0, index::WAR_BARGE, 0);
    targets(&mut e, vec![angel], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(angel)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::ISLANDWALK)
    );
}

#[test]
fn the_dark_enchantments_and_auras_work() {
    let mut e = prepared(
        &[
            index::BRAINWASH,
            index::CURSE_ARTIFACT,
            index::DANCE_OF_MANY,
            index::EROSION,
            index::GOBLIN_CAVES,
            index::GOBLIN_SHRINE,
            index::TANGLE_KELP,
            index::VENOM,
            index::FASTING,
            index::MANA_VORTEX,
            index::PSYCHIC_ALLERGY,
            index::SEASON_OF_THE_WITCH,
            index::WORMS_OF_THE_EARTH,
        ],
        &[
            index::DEEP_WATER,
            index::GAEA_S_TOUCH,
            index::MOUNTAIN,
            index::GRIZZLY_BEARS,
        ],
        &[index::HILL_GIANT, index::SOL_RING, index::PLAINS],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&e, P1, index::HILL_GIANT).unwrap();

    // Brainwash: enchant creature -> enchanted creature can't attack unless controller pays {3}.
    cast(&mut e, P0, index::BRAINWASH);
    targets(&mut e, vec![giant], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::BRAINWASH).is_some());

    // Venom: enchant creature.
    cast(&mut e, P0, index::VENOM);
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::VENOM).is_some());

    // Dance of Many: enters, creates token copy of target nontoken creature.
    cast(&mut e, P0, index::DANCE_OF_MANY);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    targets(&mut e, vec![bears], vec![]);
    pass_until(&mut e, stack_is_empty);
    let bear_tokens = e
        .state()
        .battlefield_seen()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.controller == P0 && o.card.is_none())
        })
        .count();
    assert_eq!(bear_tokens, 1);
}

#[test]
fn the_dark_creatures_work() {
    let mut e = prepared(
        &[index::FRANKENSTEIN_S_MONSTER, index::BALL_LIGHTNING],
        &[
            index::ANGRY_MOB,
            index::BANSHEE,
            index::EATER_OF_THE_DEAD,
            index::GIANT_BADGER,
            index::GIANT_SHARK,
            index::GOBLIN_ROCK_SLED,
            index::GOBLIN_WIZARD,
            index::GRAVE_ROBBERS,
            index::LURKER,
            index::MARSH_VIPER,
            index::MERFOLK_ASSASSIN,
            index::MIRACLE_WORKER,
            index::MURK_DWELLERS,
            index::ORC_GENERAL,
            index::PEOPLE_OF_THE_WOODS,
            index::PREACHER,
            index::RAG_MAN,
            index::SAVAEN_ELVES,
            index::SCARECROW,
            index::SCARWOOD_BANDITS,
            index::SCARWOOD_HAG,
            index::SPITTING_SLUG,
            index::THE_FALLEN,
            index::TRACKER,
            index::UNCLE_ISTVAN,
            index::WATER_WURM,
            index::WHIPPOORWILL,
            index::WITCH_HUNTER,
            index::FOREST,
            index::FOREST,
        ],
        &[index::SWAMP, index::ISLAND, index::SAVANNAH_LIONS],
    );

    // Angry Mob: 2+1 = 3/3 during your turn with opponent's 1 Swamp.
    let mob = on_battlefield(&e, P0, index::ANGRY_MOB).unwrap();
    assert_eq!(
        (
            e.state().object(mob).unwrap().characteristics().power,
            e.state().object(mob).unwrap().characteristics().toughness
        ),
        (Some(3), Some(3))
    );

    // People of the Woods: 1/* where * = Forests you control (2).
    let woods = on_battlefield(&e, P0, index::PEOPLE_OF_THE_WOODS).unwrap();
    assert_eq!(
        e.state().object(woods).unwrap().characteristics().toughness,
        Some(2)
    );

    // Water Wurm: 1/1 + 0/1 = 1/2 while opponent controls an Island.
    let wurm = on_battlefield(&e, P0, index::WATER_WURM).unwrap();
    assert_eq!(
        e.state().object(wurm).unwrap().characteristics().toughness,
        Some(2)
    );

    // Tracker fights Savannah Lions and destroys it.
    let lions = on_battlefield(&e, P1, index::SAVANNAH_LIONS).unwrap();
    activate_by_card(&mut e, P0, index::TRACKER, 0);
    targets(&mut e, vec![lions], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P1, index::SAVANNAH_LIONS).is_none());

    // Witch Hunter deals 1 damage to P1.
    activate_by_card(&mut e, P0, index::WITCH_HUNTER, 0);
    targets(&mut e, vec![], vec![P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 19);
}
