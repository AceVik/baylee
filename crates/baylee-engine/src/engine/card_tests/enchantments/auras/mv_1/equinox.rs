//! `cards/enchantments/auras/mv_1/equinox.rs`, played.
//!
//! Equinox: "Enchant land / Enchanted land has '{T}: Counter target spell if
//! it would destroy a land you control.'"
//!
//! Player 0 holds Plains, a Mountain that carries the Aura, a Swamp (the land
//! that is shot at) and the Equinox itself; player 1 casts into it on their
//! own turn. The granted ability has no printed index, so it is found by what
//! the enchanted land is offered. "Countered" is read off the journal and off
//! the land that was or was not destroyed. The rulings are the edges: the
//! spell is judged when the ability resolves, it can be aimed at any spell,
//! and what only deals damage, or only destroys someone else's land, is let
//! through.
//!
//! **Not played here:** the ruling of 2008-10-01 ("will not counter a spell
//! which would destroy a land only if a choice is made") is not covered. No
//! spell in the current pool destroys a land only after a choice made on
//! resolution, so there is nothing to cast. The rulings of 2004-10-04 about a
//! spell that destroys a land indirectly, one that sacrifices a land as it
//! enters or as a cost, and one with a random chance of destroying a land are
//! not covered either.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn equinox() -> CardIndex {
    card_index("0fa2cb01-476e-4e82-94e6-9639e53a7743")
}

/// `{3}{G}` sorcery, "Destroy all Islands."
fn tsunami() -> CardIndex {
    card_index("ef2b1565-7b02-4cab-9031-beb1701ee929")
}

fn armageddon() -> CardIndex {
    card_index("c9ed8b01-959a-47d6-891e-0abbdccf6e4f")
}

/// Player 0's first main phase with Plains, Mountain, Swamp and `extra` on the
/// battlefield, Equinox in hand, `theirs` on player 1's battlefield and `hand`
/// in their hand. Equinox is cast on the Mountain, paid with the Plains alone,
/// so the Mountain is untapped for its new ability. Returns the engine and
/// the enchanted Mountain.
#[track_caller]
fn the_mountain_wears_equinox(
    extra: &[CardIndex],
    theirs: &[CardIndex],
    hand: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId) {
    let mut lands = vec![plains(), mountain(), swamp()];
    lands.extend_from_slice(extra);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &lands)
        .hand(0, &[equinox()])
        .battlefield(1, theirs)
        .hand(1, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let white = on_battlefield(&engine, kit::P0, plains()).expect("the Plains");
    let mountain_land = on_battlefield(&engine, kit::P0, mountain()).expect("the Mountain");

    tap_mana_where(&mut engine, kit::P0, |id| id == white);
    cast_with_floating(&mut engine, kit::P0, equinox());
    aim_at(&mut engine, kit::P0, mountain_land);
    pass_until(&mut engine, stack_is_empty);
    (engine, mountain_land)
}

/// Whether a spell was countered, off the journal.
fn countered(engine: &Engine<RegistryLookup>, spell: ObjectId) -> bool {
    engine.journal().entries().iter().any(|entry| {
        matches!(entry.event, crate::event::GameEvent::SpellCountered { object } if object == spell)
    })
}

/// The abilities `land` is offered that were granted to it by an effect (an
/// ability a continuous effect gives is offered under a granted slot and not
/// under a printed index), apart from the mana ability the land's basic type
/// gives it.
#[track_caller]
fn granted_to(engine: &Engine<RegistryLookup>, land: ObjectId) -> Vec<u32> {
    kit::offered(engine, land)
        .into_iter()
        .filter(|index| crate::choice::granted_slot(*index).is_some())
        .collect()
}

/// `seat` activates the ability `land` was granted, aims it at `spell`, and
/// lets everything resolve. Returns the spells the ability offered as targets.
#[track_caller]
fn answer_with_equinox(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    land: ObjectId,
    spell: ObjectId,
) -> Vec<ObjectId> {
    let granted = granted_to(engine, land);
    assert_eq!(
        granted.len(),
        1,
        "the enchanted land is offered the one ability Equinox gives it: {:?}",
        kit::offered(engine, land)
    );
    kit::activate_object(engine, seat, land, granted[0]);
    let options = aim_at(engine, seat, spell);
    pass_until(engine, stack_is_empty);
    options
}

/// Player 1 on their own main phase casts `card` (aimed at `aim` when it
/// asks a target, a land or a player) and passes priority, so that player 0
/// holds it with the spell on the stack. Returns the spell.
#[track_caller]
fn they_cast(
    engine: &mut Engine<RegistryLookup>,
    card: CardIndex,
    aim: Option<ObjectId>,
) -> ObjectId {
    reach_their_main_phase(engine, kit::P1);
    cast_from_hand(engine, kit::P1, card);
    // A spell that asks for a target is not on the stack until it has one.
    if let Some(aim) = aim {
        aim_at(engine, kit::P1, aim);
    }
    let spell = on_stack(engine, card).expect("the spell is on the stack");
    engine
        .apply(kit::P1, PlayerAction::PassPriority)
        .expect("player 1 passes with the spell on the stack");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == kit::P0),
        "player 0 may respond: {:?}",
        engine.pending()
    );
    spell
}

/// "Enchant land" and "Enchanted land has…": the Aura asks for a land and only
/// a land, any land (the opponent's is on the menu too), and afterwards the
/// land it enchants has an ability to activate and its neighbours do not.
#[test]
fn equinox_enchants_a_land_and_gives_that_land_the_ability() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), mountain(), mountain(), swamp(), llanowar_elves()],
        )
        .hand(0, &[equinox(), lightning_bolt()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let white = on_battlefield(&engine, kit::P0, plains()).expect("the Plains");
    let mountain_land = on_battlefield(&engine, kit::P0, mountain()).expect("the Mountain");
    let swamp_land = on_battlefield(&engine, kit::P0, swamp()).expect("the Swamp");
    let elves = on_battlefield(&engine, kit::P0, llanowar_elves()).expect("the Elves");
    let theirs = on_battlefield(&engine, kit::P1, forest()).expect("their Forest");

    tap_mana_where(&mut engine, kit::P0, |id| id == white);
    cast_with_floating(&mut engine, kit::P0, equinox());
    let options = aim_at(&mut engine, kit::P0, mountain_land);
    for land in [white, mountain_land, swamp_land, theirs] {
        assert!(options.contains(&land), "{land:?} is a land: {options:?}");
    }
    assert!(
        !options.contains(&elves),
        "a creature is no land: {options:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    // Its only target is "target spell" (CR 602.2b, 601.2c): with no spell on
    // the stack there is nothing to aim it at, so it is not offered yet.
    assert!(
        granted_to(&engine, mountain_land).is_empty(),
        "no spell to target, no activation"
    );

    // With a spell on the stack the enchanted Mountain, and only it, has the
    // ability. The Bolt is paid by the other Mountain.
    let other_mountain = lands_of(&engine, kit::P0)
        .into_iter()
        .find(|id| {
            *id != mountain_land
                && engine
                    .state()
                    .object(*id)
                    .and_then(|o| o.card)
                    .map(|c| c.index)
                    == Some(mountain())
        })
        .expect("the second Mountain");
    tap_mana_where(&mut engine, kit::P0, |id| id == other_mountain);
    cast_with_floating(&mut engine, kit::P0, lightning_bolt());
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![kit::P1],
            },
        )
        .expect("a player is a legal target for the Bolt");
    assert!(
        on_stack(&engine, lightning_bolt()).is_some(),
        "a spell to aim at"
    );
    assert_eq!(
        granted_to(&engine, mountain_land).len(),
        1,
        "the enchanted Mountain has the ability"
    );
    assert!(
        granted_to(&engine, swamp_land).is_empty(),
        "the Swamp, which Equinox does not enchant, has none"
    );
    assert!(
        granted_to(&engine, white).is_empty(),
        "nor has the Plains that paid for it"
    );
    assert!(
        granted_to(&engine, other_mountain).is_empty(),
        "nor has the other Mountain"
    );
    pass_until(&mut engine, stack_is_empty);
}

/// "Counter target spell if it would destroy a land you control": a Stone
/// Rain aimed at player 0's Swamp is countered by the enchanted Mountain, and
/// the Swamp lives.
#[test]
fn equinox_counters_destroy_target_land_aimed_at_a_land_you_control() {
    let (mut engine, carrier) =
        the_mountain_wears_equinox(&[], &[mountain(), mountain(), mountain()], &[stone_rain()]);
    let swamp_land = on_battlefield(&engine, kit::P0, swamp()).expect("the Swamp");

    let rain = they_cast(&mut engine, stone_rain(), Some(swamp_land));
    let options = answer_with_equinox(&mut engine, kit::P0, carrier, rain);

    assert!(
        options.contains(&rain),
        "Stone Rain is a legal target: {options:?}"
    );
    assert!(countered(&engine, rain), "Stone Rain was countered");
    assert!(
        on_battlefield(&engine, kit::P0, swamp()).is_some(),
        "so the Swamp was not destroyed"
    );
    assert!(
        in_graveyard(&engine, kit::P1, stone_rain()).is_some(),
        "a countered spell goes to its owner's graveyard"
    );
}

/// "…a land **you** control": the same Stone Rain aimed at one of player
/// 1's own lands would not destroy one of player 0's, and is let through. The
/// ability is activated all the same (it can target any spell) and counters
/// nothing: the Mountain player 1 shot at is destroyed.
#[test]
fn equinox_does_not_counter_destroy_target_land_aimed_at_another_players_land() {
    let (mut engine, carrier) = the_mountain_wears_equinox(
        &[],
        &[mountain(), mountain(), mountain(), mountain()],
        &[stone_rain()],
    );
    let theirs = lands_of(&engine, kit::P1)[0];

    let rain = they_cast(&mut engine, stone_rain(), Some(theirs));
    let options = answer_with_equinox(&mut engine, kit::P0, carrier, rain);

    assert!(
        options.contains(&rain),
        "it may target any spell, even one that destroys no land of mine: {options:?}"
    );
    assert!(!countered(&engine, rain), "Stone Rain was not countered");
    assert_eq!(
        engine.state().object(theirs).map(|o| o.zone),
        Some(Zone::Graveyard),
        "player 1's own Mountain was destroyed"
    );
    assert!(
        on_battlefield(&engine, kit::P0, swamp()).is_some(),
        "and mine are untouched"
    );
}

/// "…if it would destroy a land you control": a sweep that destroys every
/// land would destroy player 0's, so Armageddon is countered and every land
/// on both sides survives.
#[test]
fn equinox_counters_destroy_all_lands() {
    let (mut engine, carrier) = the_mountain_wears_equinox(
        &[],
        &[plains(), mountain(), mountain(), mountain()],
        &[armageddon()],
    );
    let before = (
        lands_of(&engine, kit::P0).len(),
        lands_of(&engine, kit::P1).len(),
    );

    let spell = they_cast(&mut engine, armageddon(), None);
    answer_with_equinox(&mut engine, kit::P0, carrier, spell);

    assert!(countered(&engine, spell), "Armageddon was countered");
    assert_eq!(
        (
            lands_of(&engine, kit::P0).len(),
            lands_of(&engine, kit::P1).len()
        ),
        before,
        "every land is still on the battlefield"
    );
}

/// The condition is *would it destroy a land of mine*, not *does it destroy
/// lands*: Tsunami destroys all Islands, player 0 controls none, so the
/// ability lets it resolve, and player 1's own Island goes.
#[test]
fn equinox_does_not_counter_a_sweep_that_misses_every_land_you_control() {
    let (mut engine, carrier) = the_mountain_wears_equinox(
        &[],
        &[forest(), forest(), forest(), forest(), island()],
        &[tsunami()],
    );

    let spell = they_cast(&mut engine, tsunami(), None);
    answer_with_equinox(&mut engine, kit::P0, carrier, spell);

    assert!(!countered(&engine, spell), "Tsunami was not countered");
    assert!(
        on_battlefield(&engine, kit::P1, island()).is_none(),
        "it resolved and destroyed player 1's Island"
    );
    assert_eq!(
        lands_of(&engine, kit::P0).len(),
        3,
        "player 0 lost no land: it controls no Island"
    );
}

/// Ruling (2004-10-04): "The ability can target any spell, even one that
/// would not destroy a land." A Lightning Bolt aimed at player 0 is offered
/// as a target, is targeted, and is not countered: it resolves for 3.
#[test]
fn equinox_may_target_any_spell_and_leaves_a_bolt_alone() {
    let (mut engine, carrier) = the_mountain_wears_equinox(&[], &[mountain()], &[lightning_bolt()]);
    let before = kit::life(&engine, kit::P0);

    reach_their_main_phase(&mut engine, kit::P1);
    cast_from_hand(&mut engine, kit::P1, lightning_bolt());
    engine
        .apply(
            kit::P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![kit::P0],
            },
        )
        .expect("a player is a legal target for the Bolt");
    let bolt = on_stack(&engine, lightning_bolt()).expect("the Bolt");
    engine
        .apply(kit::P1, PlayerAction::PassPriority)
        .expect("player 1 passes");
    let options = answer_with_equinox(&mut engine, kit::P0, carrier, bolt);

    assert!(options.contains(&bolt), "{options:?}");
    assert!(!countered(&engine, bolt), "a Bolt destroys no land");
    assert_eq!(kit::life(&engine, kit::P0), before - 3, "it dealt its 3");
}

/// Ruling (2004-10-04): "Equinox will not counter a spell that deals damage
/// to an animated land, even if it would deal more damage than the land's
/// toughness. This is because the spell itself does not destroy the land
/// directly." In player 1's upkeep player 0 animates a Mutavault; player 1
/// Bolts it; Equinox's ability aimed at the Bolt counters nothing, and the
/// Mutavault dies to the damage.
#[test]
fn equinox_does_not_counter_damage_that_kills_an_animated_land() {
    let (mut engine, carrier) =
        the_mountain_wears_equinox(&[mutavault()], &[mountain()], &[lightning_bolt()]);
    let swamp_land = on_battlefield(&engine, kit::P0, swamp()).expect("the Swamp");
    let vault = on_battlefield(&engine, kit::P0, mutavault()).expect("Mutavault");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && e.state().turn.active == kit::P1
            && matches!(e.state().turn.step, crate::turn::Step::Upkeep)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P0)
    });
    tap_mana_where(&mut engine, kit::P0, |id| id == swamp_land);
    activate(&mut engine, kit::P0, mutavault(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        types(&engine, vault).contains(TypeSet::CREATURE),
        "Mutavault is a creature until end of turn"
    );

    reach_their_main_phase(&mut engine, kit::P1);
    cast_from_hand(&mut engine, kit::P1, lightning_bolt());
    aim_at(&mut engine, kit::P1, vault);
    let bolt = on_stack(&engine, lightning_bolt()).expect("the Bolt");
    engine
        .apply(kit::P1, PlayerAction::PassPriority)
        .expect("player 1 passes");
    answer_with_equinox(&mut engine, kit::P0, carrier, bolt);

    assert!(!countered(&engine, bolt), "damage is not destruction");
    assert!(
        in_graveyard(&engine, kit::P0, mutavault()).is_some(),
        "the Bolt resolved and Mutavault died to the damage"
    );
}

/// "…a land **you** control": whose "you" is the land's controller. Player 0
/// enchants one of player 1's Mountains; on their turn player 1 shoots a
/// Stone Rain at another of their own lands and answers it with the enchanted
/// Mountain, which counters it (it would destroy a land *they* control).
#[test]
fn equinox_serves_the_controller_of_the_land_it_enchants() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[equinox()])
        .battlefield(
            1,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(1, &[stone_rain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let mountains = lands_of(&engine, kit::P1);
    assert_eq!(mountains.len(), 5, "player 1 has five Mountains");
    let (carrier, target) = (mountains[0], mountains[1]);
    cast_from_hand(&mut engine, kit::P0, equinox());
    aim_at(&mut engine, kit::P0, carrier);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(kit::controller_of(&engine, carrier), kit::P1);

    reach_their_main_phase(&mut engine, kit::P1);
    tap_mana_except(&mut engine, kit::P1, carrier);
    cast_with_floating(&mut engine, kit::P1, stone_rain());
    aim_at(&mut engine, kit::P1, target);
    let rain = on_stack(&engine, stone_rain()).expect("Stone Rain");
    answer_with_equinox(&mut engine, kit::P1, carrier, rain);

    assert!(
        countered(&engine, rain),
        "it would destroy a land they control"
    );
    assert_eq!(
        engine.state().object(target).map(|o| o.zone),
        Some(Zone::Battlefield),
        "so their other Mountain stands"
    );
}

/// `{U}` instant: "Choose one — Destroy target red permanent; or return
/// target Mountain to its owner's hand."
fn flash_flood() -> CardIndex {
    card_index("c2b5d847-f52a-43e3-a5d2-d53cae3d8936")
}

/// Player 0 wears Equinox on a Mountain and, in player 1's upkeep, animates a
/// Den of the Bugbear (a red Goblin creature and still a land). Player 1 then
/// casts a Flash Flood in mode `mode` (0 destroys target red permanent, 1
/// returns target Mountain) aimed at the animated Den for mode 0 and at
/// player 0's second Mountain for mode 1, and passes. Returns the engine, the
/// spell, what it is aimed at and the enchanted Mountain, with player 0
/// holding priority and the spell on the stack.
#[track_caller]
fn flash_flood_against_equinox(
    mode: usize,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let (mut engine, carrier) = the_mountain_wears_equinox(
        &[den_of_the_bugbear(), mountain(), forest(), forest()],
        &[island()],
        &[flash_flood()],
    );
    let den = on_battlefield(&engine, kit::P0, den_of_the_bugbear()).expect("the Den");
    let second_mountain = lands_of(&engine, kit::P0)
        .into_iter()
        .find(|id| {
            *id != carrier
                && engine
                    .state()
                    .object(*id)
                    .and_then(|o| o.card)
                    .map(|c| c.index)
                    == Some(mountain())
        })
        .expect("a second Mountain");

    pass_until(&mut engine, |e| {
        e.state().turn.number == 2
            && e.state().turn.active == kit::P1
            && matches!(e.state().turn.step, crate::turn::Step::Upkeep)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P0)
    });
    tap_mana_where(&mut engine, kit::P0, |id| id != carrier && id != den);
    activate(&mut engine, kit::P0, den_of_the_bugbear(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        types(&engine, den).contains(TypeSet::LAND)
            && types(&engine, den).contains(TypeSet::CREATURE),
        "the Den is a land creature until end of turn"
    );

    reach_their_main_phase(&mut engine, kit::P1);
    cast_from_hand(&mut engine, kit::P1, flash_flood());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(n) if n == mode))
        .expect("the mode is offered");
    engine
        .apply(kit::P1, PlayerAction::ChooseMode(slot))
        .expect("a mode of the card");
    let aim = if mode == 0 { den } else { second_mountain };
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&aim),
        "{aim:?} is a legal target: {options:?}"
    );
    engine
        .apply(kit::P1, PlayerAction::ChooseObjects { objects: vec![aim] })
        .expect("the target was on the menu");
    let spell = on_stack(&engine, flash_flood()).expect("the spell is on the stack");
    engine
        .apply(kit::P1, PlayerAction::PassPriority)
        .expect("player 1 passes with the spell on the stack");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == kit::P0),
        "player 0 may respond: {:?}",
        engine.pending()
    );
    (engine, spell, aim, carrier)
}

/// "…if it would destroy a land you control" reads the mode the spell was
/// cast in. A Flash Flood in its destroy mode aimed at player 0's animated
/// Den of the Bugbear (a red permanent that is also a land) would destroy a
/// land of theirs, so Equinox counters it and the Den lives. The same spell in
/// its other mode, returning a Mountain, destroys nothing: Equinox's ability
/// lets it resolve and the Mountain goes back to its owner's hand.
#[test]
fn equinox_reads_the_mode_a_modal_spell_was_cast_in() {
    let (mut engine, spell, den, carrier) = flash_flood_against_equinox(0);
    answer_with_equinox(&mut engine, kit::P0, carrier, spell);
    assert!(
        countered(&engine, spell),
        "the destroy mode would destroy a land player 0 controls"
    );
    assert!(
        on_battlefield(&engine, kit::P0, den_of_the_bugbear()).is_some(),
        "so the Den stands"
    );
    assert_eq!(kit::zone_of(&engine, den), Zone::Battlefield);

    let (mut engine, spell, _, carrier) = flash_flood_against_equinox(1);
    answer_with_equinox(&mut engine, kit::P0, carrier, spell);
    assert!(
        !countered(&engine, spell),
        "the return mode destroys no land, whatever the other mode would do"
    );
    assert!(
        in_hand(&engine, kit::P0, mountain()).is_some(),
        "so it resolved and the Mountain went back to its owner's hand"
    );
    assert!(
        on_battlefield(&engine, kit::P0, den_of_the_bugbear()).is_some(),
        "and the Den is untouched"
    );
}
