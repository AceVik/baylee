//! Cards whose front face is an enchantment, the door
//! `cards/enchantments/` puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use baylee_cards_dsl::counters;
use baylee_core::color::{Color, ColorSet};

mod animate_artifact;
mod aura_bindings;
mod auras;
mod classes;
mod consecrate_land;
mod creature_bond;
mod earthbind;
mod earthbind_independent;
mod gloom;
mod island_sanctuary;
mod legends;
mod lich;
mod mv_1;
mod mv_2;
mod mv_3;
mod mv_4;
mod mv_5;
mod mv_6;
mod rooms;
mod sagas;

fn circle_of_protection_red() -> CardIndex {
    card_index("df2738fe-9cd1-4347-8808-105fcfde1190")
}

fn orcish_artillery() -> CardIndex {
    card_index("c000811e-bde7-4840-a3bb-9714d5c977eb")
}

// oracle_id = "119d719d-e965-45b4-9bc9-ac03211b10c2"
fn survival_of_the_fittest() -> baylee_core::ids::CardIndex {
    card_index("119d719d-e965-45b4-9bc9-ac03211b10c2")
}

/// Survival of the Fittest ({1}{G}): "{G}, Discard a creature card: Search
/// your library for a creature card, reveal that card, put it into your
/// hand, then shuffle." A cost that says "a creature card" names none, so
/// the engine has to ask before it may be paid (CR 601.2h) — and this is
/// the board that drives `ChoicePrompt::CostDiscard` and the hand-only
/// option list behind it.
///
/// Reading the card cannot replace playing it, because the card is the half
/// that does not say who may be asked for what. Its filter is a bare
/// `Filter::CREATURE` with no "you control" in it at all, and a hand is a
/// hidden zone the printed line says nothing about; the rule that a player
/// discards from their own hand is CR 701.9a and lives in `cost_wizard`.
/// So three bystanders stand beside the Elves to say what the list is not:
/// a Forest and a Counterspell in the same hand, which are cards but not
/// creature cards, and a Llanowar Elves in the **opponent's** hand, which is
/// a creature card but not one this player may discard. Reading the filter
/// alone over every hand at the table would have offered that last one.
///
/// The prompt is asserted beside the options, because the variant is what
/// tells a client this is a cost and not a search: the same ability publishes
/// a second `Pending::ChooseCards` two steps later carrying
/// `ChoicePrompt::SearchLibrary`, and a test that ignored the field would
/// pass with the two swapped. `ChooseCards` and not `ChooseTargets` is the
/// other half of that: choosing what to discard is not targeting (CR 115.1).
///
/// What the old test pinned — an ability the engine offered to nobody — is
/// gone. The ability is on the list, the question comes before anything is
/// paid, and the green mana and the chosen card are both spent when it is
/// answered.
#[allow(clippy::too_many_lines)] // one activation, from the offer to what the search found
#[test]
fn survival_of_the_fittest_asks_which_creature_card_to_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The library is creature cards, so the search the cost pays for has
    // something to find — and a different creature from the one in hand, so
    // the card that paid and the card that was found cannot be confused.
    let mut engine = Duel::new(41, ondu_cleric())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(
            0,
            &[
                survival_of_the_fittest(),
                llanowar_elves(),
                forest(),
                counterspell(),
            ],
        )
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Two Forests pay the {1}{G}. The third is kept back on purpose: the
    // ability's own mana has to be *in the pool* when the offer is read,
    // because `can_afford` asks the pool and never what the board could still
    // tap.
    let forests = mine(&engine, p0, forest(), crate::zone::Zone::Battlefield);
    assert_eq!(forests.len(), 3, "three Forests were dealt to seat 0");
    let spare = forests[2];
    tap_mana_except(&mut engine, p0, spare);
    let spell =
        in_hand(&engine, p0, survival_of_the_fittest()).expect("the enchantment is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("a two-mana enchantment off two Forests");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let survival = on_battlefield(&engine, p0, survival_of_the_fittest())
        .expect("Survival of the Fittest resolved onto the battlefield");
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: spare })
        .expect("the Forest that was kept back still taps");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the green mana the ability asks for is in the pool"
    );

    // The one creature card in hand, and the three cards that are not it.
    let elves = in_hand(&engine, p0, llanowar_elves()).expect("a creature card is in hand");
    let land_in_hand = in_hand(&engine, p0, forest()).expect("a land card is in hand");
    let instant_in_hand = in_hand(&engine, p0, counterspell()).expect("an instant is in hand");
    let theirs = in_hand(&engine, p1, llanowar_elves()).expect("seat 1 holds a creature card too");

    // The offer the old gap withheld: a cost that has to ask is now asked of
    // the board instead of refused outright.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(survival, 0)),
        "the enchantment's own ability is on no list: {:?}",
        legal.abilities
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: survival,
                ability_index: 0,
            },
        )
        .expect("the activation the offer promised");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "a cost that names a card to choose has to ask, and got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "a player discards from their own hand");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature card, no more and no fewer"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "the prompt is what tells a client this is a cost and not a search"
    );
    assert!(
        options.contains(&elves),
        "the creature card in hand pays this cost, and was not offered: {options:?}"
    );
    assert!(
        !options.contains(&land_in_hand),
        "a Forest in hand is a card and not a creature card"
    );
    assert!(
        !options.contains(&instant_in_hand),
        "and neither is a Counterspell"
    );
    assert!(
        !options.contains(&theirs),
        "the opponent's creature card is in the opponent's hand"
    );
    // Seat 0 takes the first turn and skips its draw (CR 103.8), so the hand
    // is exactly what the duel dealt, less the enchantment it cast.
    assert_eq!(options.len(), 1, "and nothing else at all: {options:?}");

    // The question comes before the payment (CR 601.2h): the mana is still in
    // the pool and the card is still in hand, so nobody is made to choose
    // what to give up for an activation that cannot happen.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the mana half of the cost is unspent while the question stands"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&elves),
        "and so is the card the question is about"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the answer comes off the list the engine published");
    assert_eq!(
        in_graveyard(&engine, p0, llanowar_elves()),
        Some(elves),
        "the discarded card is in its owner's graveyard"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&elves),
        "and it left the hand to get there"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "the mana half of the cost was paid with it"
    );

    // What the cost bought. The same ability asks again, and the second
    // question carries the prompt a search carries.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the search")
    };
    let found = *options.first().expect("the library holds creature cards");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .expect("a creature card off the list the search published");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&found),
        "the card the search found is in hand"
    );
    assert!(
        engine
            .state()
            .object(found)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE)),
        "and it is a creature card, which is all the ability may take"
    );
}

// oracle_id = "3a3e8c9b-e458-4661-980d-0a84a4c2452b"

/// Glasswing Grace // Age-Graced Chapel ({3}{W/B}{W/B}, MH3): a modal
/// double-faced card whose front is an Aura and whose back, Age-Graced
/// Chapel, is a land that enters tapped and taps for {W} or {B}.
fn glasswing_grace() -> CardIndex {
    card_index("3a3e8c9b-e458-4661-980d-0a84a4c2452b")
}

/// Casts the Aura on seat 0's Llanowar Elves and hands back the board it
/// left: the engine, seat 0's enchanted Elves, seat 1's untouched Elves and
/// the Aura permanent. A Swords to Plowshares is left in hand for the caller
/// that wants the host gone.
///
/// The second Elves is the control the whole thing rests on. "Enchanted
/// creature gets +2/+2" is one `Filter::AttachedToBySource`, and a filter that
/// had come out as "every creature" would read exactly the same on a board
/// with only one creature on it.
#[track_caller]
fn a_glasswing_on_the_elves() -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(517, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[glasswing_grace(), swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elves deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elves deployed");
    assert_eq!(pt(&engine, mine), (1, 1), "Llanowar Elves prints 1/1");

    reach_main_phase(&mut engine, p0);
    // Five Plains, and the Elves is not among the mana `cast_from_hand` taps:
    // `legal.mana_abilities` is the CR 305.6 shortcut, which only a land with
    // a basic land type is on. So the pool is five white against
    // {3}{W/B}{W/B}, and `mana_pay::pay` spends the hybrids before the
    // generic — white twice, then three more for the {3}.
    cast_from_hand(&mut engine, p0, glasswing_grace());

    // "Enchant creature" (CR 303.4a): an Aura spell requires a target, which
    // its enchant ability defines, so the question is asked as it is cast and
    // not on the way onto the battlefield. The Chapel is never offered
    // beside it: a land back face is played and never cast
    // (`casting::castable_back_faces` drops it), so the wizard has one option
    // and asks no mode at all.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an Aura spell targets as it is cast, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "any creature may be enchanted, and the board has two: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| at_rest(e, p0));
    let aura =
        on_battlefield(&engine, p0, glasswing_grace()).expect("the Aura resolved onto the table");
    (engine, mine, theirs, aura)
}

/// "Enchanted creature gets +2/+2 and has flying and lifelink." One printed
/// sentence and two layers — CR 613.1 applies layer 6 (the keywords) before
/// layer 7c (the P/T change) — so the only reading that can see both at once
/// is the creature's projected characteristics, taken after the layers ran.
///
/// It also pins where the effect lands. The Aura arrives attached to the
/// creature its spell targeted, the grant follows that attachment, and
/// neither half reaches the identical Elves the opponent controls.
#[test]
fn glasswing_grace_gives_the_creature_it_enchants_plus_two_two_flying_and_lifelink() {
    let (engine, mine, theirs, aura) = a_glasswing_on_the_elves();

    assert_eq!(
        engine
            .state()
            .object(aura)
            .and_then(|o| o.attached_to)
            .expect("the Aura is attached to something"),
        mine,
        "an Aura enters attached to the creature its spell targeted"
    );
    assert_eq!(pt(&engine, mine), (3, 3), "+2/+2 on a 1/1");
    let kw = keywords(&engine, mine);
    assert!(
        kw.contains(KeywordSet::FLYING),
        "the enchanted creature has flying"
    );
    assert!(
        kw.contains(KeywordSet::LIFELINK),
        "the enchanted creature has lifelink"
    );

    // The Aura grants; it does not keep.
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::FLYING),
        "the Aura itself is no flier"
    );
    // And the creature it is not attached to is the card it always was.
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the opponent's Elves is enchanted by nothing"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "and picked up no keyword either"
    );
}

// oracle_id = "1a8c996d-ca93-4c17-ace5-66ecd6b99317"

/// Strength of the Harvest // Haven of the Harvest ({2}{G/W}) — a modal
/// double-faced card whose front is an Aura and whose back is a land.
fn strength_of_the_harvest() -> CardIndex {
    card_index("1a8c996d-ca93-4c17-ace5-66ecd6b99317")
}

// oracle_id = "5d47e820-913f-441a-a6cc-37ab3181d79a"
fn swift_reconfiguration() -> CardIndex {
    card_index("5d47e820-913f-441a-a6cc-37ab3181d79a")
}

/// `Artifact — Vehicle`, 3/3, and — uncrewed — no creature at all.
///
/// It is on this board for the half of "enchant creature or Vehicle" that a
/// creature cannot stand for: a Vehicle is a legal host precisely because it
/// is *not* a creature, so a filter that read the printed line as "enchant
/// creature" would still pass every other assertion here.
fn smugglers_copter() -> CardIndex {
    card_index("49136bdc-bc50-49a2-999a-1ef9c16ea130")
}

// oracle_id = "50aa7aff-1f01-4224-9a83-01f74d703ec2"
fn earthcraft() -> baylee_core::ids::CardIndex {
    card_index("50aa7aff-1f01-4224-9a83-01f74d703ec2")
}

/// Earthcraft ({1}{G}): "Tap an untapped creature you control: Untap target
/// basic land."
///
/// The pool's first `CostPart::TapOther`, and the one asking cost whose
/// answer is still on the battlefield afterwards. That is the half a reader
/// of the card cannot settle and the reason the cost has its own prompt: a
/// player shown `ChoicePrompt::CostSacrifice` over their own creatures would
/// decline a cost that only taps one.
///
/// Both enumerations are struck, and the menu is asserted by exact equality
/// rather than by what it contains. The tapped Elf is mine and is refused by
/// CR 118.3 — a permanent already tapped cannot be tapped to pay a cost,
/// whether or not the card thought to say "untapped". The opponent's Elf is
/// untapped and is refused because no rule makes a cost reach across the
/// table, so `cost_wizard::options` draws that line itself. The Earthcraft
/// is neither, and is on nobody's menu.
///
/// The creature that *does* pay arrived this turn, which is the claim the
/// card is silent about and the rules are not: CR 302.6 restricts a
/// creature's own `{T}` ability and says nothing about it being tapped to
/// pay for somebody else's, so a Bird cast this turn can already work the
/// land. Reading `cost_wizard` alone would not settle it — `can_afford`'s
/// `TapSelf` arm *does* ask about summoning sickness, one arm away.
///
/// The target list is the other half. "Target basic land" carries no "you
/// control", so the opponent's Forest is on it and the Badlands beside it is
/// not — a dual land is no basic land however many basic types it has.
#[allow(clippy::too_many_lines)] // one activation, two enumerations, and a tap that is not a sacrifice
#[test]
fn earthcraft_taps_a_summoning_sick_creature_to_untap_a_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[earthcraft(), forest(), llanowar_elves()])
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), forest(), badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let craft = on_battlefield(&engine, p0, earthcraft()).expect("the Earthcraft is out");
    let veteran = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf that was here");
    let land = on_battlefield(&engine, p0, forest()).expect("my Forest");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest");
    let their_dual = on_battlefield(&engine, p1, badlands()).expect("their Badlands");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");

    // The older Elf is tapped by hand and the Forest by the cast, which is
    // what puts a tapped creature of my own on the board to be refused and
    // leaves the newcomer as the only untapped one. By hand because a
    // printed `mana_ability!` is an ordinary `(source, index)` in
    // `legal.abilities` — `legal.mana_abilities` is the CR 305.6 shortcut,
    // lands and granted abilities, and `cast_from_hand` walks only that.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: veteran,
                ability_index: 0,
            },
        )
        .expect("the Elf taps for {G}");
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let rookie = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            *id != veteran
                && engine.state().object(*id).is_some_and(|o| {
                    o.controller == p0 && o.card.is_some_and(|c| c.index == llanowar_elves())
                })
        })
        .expect("the Elf cast this turn is on the battlefield");
    assert!(
        is_tapped(&engine, veteran),
        "the older Elf paid for the cast"
    );
    assert!(is_tapped(&engine, land), "and so did my Forest");
    assert!(!is_tapped(&engine, rookie), "the newcomer arrived untapped");

    activate(&mut engine, p0, earthcraft(), 0);

    // CR 601.2c: targets first, and "basic land" says nothing about whose.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the untap asks which land: {:?}", engine.pending())
    };
    assert!(
        options.contains(&land) && options.contains(&their_land),
        "either side of the table prints a basic land: {options:?}"
    );
    assert!(
        !options.contains(&their_dual),
        "a dual land is no basic land, whatever types it has: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: Vec::new(),
            },
        )
        .expect("my own tapped Forest is one of the legal targets");

    // CR 601.2h, and the one step of it the player takes.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the cost asks which creature to tap: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostTap,
        "not `CostSacrifice`: the creature named here is still standing \
         afterwards, and the two questions cannot share a word"
    );
    assert_eq!((min, max), (1, 1), "one creature, and exactly one");
    assert_eq!(
        options,
        vec![rookie],
        "the only untapped creature this seat controls. The older Elf \
         {veteran:?} is tapped (CR 118.3), the Earthcraft {craft:?} is no \
         creature, and the Elf {their_elf:?} across the table is not mine \
         to tap"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rookie],
            },
        )
        .expect("a creature that arrived this turn may still be tapped for a cost");

    assert!(
        is_tapped(&engine, rookie),
        "the cost is paid, so the creature named is tapped"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some()
            && engine.state().object(rookie).is_some(),
        "and it is still on the battlefield: a tap is not a sacrifice"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability is not a mana ability (CR 605.1), so it uses the stack"
    );
    assert!(
        is_tapped(&engine, land),
        "and nothing has untapped yet — the effect happens on resolution"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, land),
        "the targeted basic land is untapped"
    );
    assert!(
        is_tapped(&engine, rookie),
        "and the creature that paid stays tapped: the cost is not refunded"
    );
}

// oracle_id = "8a52f3c0-2552-4425-b2e3-5496eb2232a7"
fn mystic_remora() -> CardIndex {
    card_index("8a52f3c0-2552-4425-b2e3-5496eb2232a7")
}

/// Walks to the next cumulative-upkeep question put to `seat` and returns
/// the mana it asks for.
fn remora_upkeep_question(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> u16 {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, seat,
        "the upkeep cost is asked of the Remora's controller"
    );
    assert_eq!(
        engine.state().turn.step,
        crate::turn::Step::Upkeep,
        "at the beginning of the upkeep"
    );
    mana
}

fn age_counters(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .map_or(0, |o| o.counters.get(baylee_cards_dsl::counters::AGE))
}

/// Casts Walk-In Closet's right half, Forgotten Cellar, off floating mana.
#[track_caller]
fn cast_forgotten_cellar(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    cast_with_floating(engine, seat, walk_in_closet());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "with both halves affordable a Room asks which is cast, got {:?}",
            engine.pending()
        )
    };
    let cellar = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Face(1)))
        .expect("Forgotten Cellar is a half to cast");
    engine
        .apply(seat, PlayerAction::ChooseMode(cellar))
        .expect("casting the right half");
}

/// Casts Giant Growth from `seat`'s hand on `seat`'s Llanowar Elves, off
/// floating mana, and lets it resolve.
#[track_caller]
fn grow_own_elf(engine: &mut Engine<RegistryLookup>, seat: PlayerId, rest: PlayerId) {
    let elf = on_battlefield(engine, seat, llanowar_elves()).expect("the Elf is out");
    cast_with_floating(engine, seat, giant_growth());
    engine
        .apply(seat, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    pass_until(engine, |e| at_rest(e, rest));
}

/// Whether `card` is in `seat`'s exile.
fn exiled_card(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
) -> Option<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// Takes Sylvan Library's offer at `p0`'s draw step and answers "which two
/// drawn this turn" with the first two, returning them and the hand size
/// from before the draw step.
fn sylvan_library_offer(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
) -> (Vec<ObjectId>, usize) {
    let offered = |e: &Engine<RegistryLookup>| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    };
    // The player on the play has no draw step on turn 1 (CR 103.8a), so the
    // first offer is turn 3's; the step's own draw came before the trigger
    // (CR 504.1) and is already in the hand.
    pass_until(engine, offered);
    assert_eq!(engine.state().turn.number, 3, "no offer on the first turn");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the drawn cards, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!(
        options.len(),
        3,
        "the draw step's card and the two additional ones"
    );
    assert_eq!((min, max), (2, 2), "choose two of them");
    let chosen = options[..2].to_vec();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: chosen.clone(),
            },
        )
        .unwrap();
    (chosen, hand_before)
}

/// Erode: the removal, and the land the *other* player is offered for it.
///
/// `Effect::OptionalBasicLandSearchFor { player: ControllerOfTarget }` is the
/// half that is easy to write pointing at the wrong seat, so the assertion is
/// on whose battlefield the basic arrives — and it is not the caster's.
#[test]
fn erode_destroys_a_creature_and_offers_its_controller_a_basic() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(409, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[erode()])
        .battlefield(1, &[rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("the Wurm is seated");
    let their_library = library_size(&engine, p1);
    cast_from_hand(&mut engine, p0, erode());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .expect("the Wurm is a legal target");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the search the other seat is offered is answered on the way"
    );

    assert!(
        on_battlefield(&engine, p1, rootbreaker_wurm()).is_none(),
        "the Wurm is destroyed"
    );
    assert!(
        library_size(&engine, p1) < their_library,
        "and it is the Wurm's controller whose library the basic came out of"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_size(&engine, p0),
        "the caster searched nothing"
    );
}

/// Whether the journal holds a move of `object` from a graveyard to exile.
fn went_from_graveyard_to_exile(engine: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    engine.journal().entries().iter().any(|e| {
        matches!(
            e.event,
            crate::event::GameEvent::ZoneChanged {
                object: moved,
                from: crate::zone::Zone::Graveyard,
                to: crate::zone::Zone::Exile,
                ..
            } if moved == object
        )
    })
}

/// The Goblin Shaman tokens `seat` controls.
fn goblin_shamans(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    tokens_of(engine, seat)
        .into_iter()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.token)
                .is_some_and(|t| t.name == "Goblin Shaman")
        })
        .collect()
}

/// The Treasure tokens `seat` controls.
fn treasures(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    tokens_of(engine, seat)
        .into_iter()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.token)
                .is_some_and(|t| t.name == "Treasure")
        })
        .collect()
}

/// Casts Fable of the Mirror-Breaker off three Mountains and resolves it and
/// its chapter I.
fn cast_fable(hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut cards = vec![fable_of_the_mirror_breaker()];
    cards.extend_from_slice(hand);
    let mut engine = Duel::new(101, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &cards)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, fable_of_the_mirror_breaker());
    pass_until(&mut engine, stack_is_empty);
    engine
}

/// Walks from the turn Fable was cast to its controller's next main phase,
/// where chapter II triggers, and stops on the question it asks.
fn to_fable_chapter_two(engine: &mut Engine<RegistryLookup>) -> (Vec<ObjectId>, u8, u8) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    reach_their_main_phase(engine, p1);
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Discard,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the Saga's controller discards");
    assert_eq!(
        options,
        engine.state().zones.list(ZoneLocation::Hand(p0)).clone(),
        "any card in hand"
    );
    (options, min, max)
}

/// Fable of the Mirror-Breaker, chapter I: "Create a 2/2 red Goblin Shaman
/// creature token with 'Whenever this token attacks, create a Treasure
/// token.'" The token arrives as the Saga's first chapter resolves, and when
/// it attacks two turns later its own trigger makes a Treasure. Chapter II
/// is declined on the way, and declining moves nothing: no card is
/// discarded, none drawn.
#[test]
fn fable_chapter_one_makes_a_goblin_shaman_whose_attack_makes_a_treasure() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cast_fable(&[]);
    let goblins = goblin_shamans(&engine, p0);
    assert_eq!(goblins.len(), 1, "chapter I made one Goblin Shaman");
    let goblin = goblins[0];
    assert_eq!(pt(&engine, goblin), (2, 2));
    let face = engine.state().object(goblin).unwrap().characteristics();
    assert!(face.types.contains(TypeSet::CREATURE));
    assert_eq!(
        face.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Red])
    );
    assert!(
        face.subtypes
            .contains(baylee_core::generated::subtypes::creature::GOBLIN)
            && face
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::SHAMAN),
        "a Goblin Shaman"
    );
    assert!(treasures(&engine, p0).is_empty());

    let (_, min, max) = to_fable_chapter_two(&mut engine);
    assert_eq!((min, max), (0, 1), "up to two, of the one card in hand");
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    let library = library_size(&engine, p0);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)),
        &hand,
        "nothing discarded"
    );
    assert_eq!(library_size(&engine, p0), library, "nothing drawn");
    pass_until(&mut engine, stack_is_empty);

    attack_and_collect_blocks(&mut engine, goblin, p1);
    assert_eq!(
        treasures(&engine, p0).len(),
        1,
        "the token's own attack trigger made a Treasure"
    );
}

/// Chapter II: "You may discard up to two cards. If you do, draw that many
/// cards." Two named, two discarded, two drawn: the hand keeps its size, the
/// library is two shorter, and the journal says both were discarded.
#[test]
fn fable_chapter_two_discards_up_to_two_and_draws_that_many() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_fable(&[quiet_creature(), lightning_elemental()]);
    let (options, min, max) = to_fable_chapter_two(&mut engine);
    assert_eq!((min, max), (0, 2), "up to two");
    let creature = in_hand(&engine, p0, quiet_creature()).unwrap();
    let elemental = in_hand(&engine, p0, lightning_elemental()).unwrap();
    assert!(options.contains(&creature) && options.contains(&elemental));
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library = library_size(&engine, p0);
    let journal_from = engine.state().journal.len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature, elemental],
            },
        )
        .unwrap();
    for card in [quiet_creature(), lightning_elemental()] {
        assert!(in_graveyard(&engine, p0, card).is_some(), "discarded");
    }
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand,
        "two out, two in"
    );
    assert_eq!(library_size(&engine, p0), library - 2, "that many drawn");
    let discarded = engine.state().journal.entries()[journal_from..]
        .iter()
        .filter(|e| matches!(e.event, GameEvent::Discarded { player, .. } if player == p0))
        .count();
    assert_eq!(discarded, 2, "each one a discard, for what cares about one");
}

/// Chapter II with one card named: one discarded, one drawn — "that many"
/// is what was discarded, not the two the chapter allows.
#[test]
fn fable_chapter_two_draws_one_for_one_discard() {
    let p0 = PlayerId::new(0);
    let mut engine = cast_fable(&[quiet_creature(), lightning_elemental()]);
    to_fable_chapter_two(&mut engine);
    let creature = in_hand(&engine, p0, quiet_creature()).unwrap();
    let library = library_size(&engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, p0, quiet_creature()).is_some());
    assert!(in_hand(&engine, p0, lightning_elemental()).is_some());
    assert_eq!(library_size(&engine, p0), library - 1, "one drawn");
}

fn alexi_s_cloak() -> CardIndex {
    card_index("e52eb1a6-fff1-4a47-b434-31a74d76231c")
}

fn buoyancy() -> CardIndex {
    card_index("f4e4060d-bfff-4991-9ae5-8f848304cd1e")
}

fn capashen_standard() -> CardIndex {
    card_index("75510429-41bb-409e-b6fe-04a8bb174c6b")
}

fn crackling_club() -> CardIndex {
    card_index("876affb5-155d-4268-9e46-4437a9fbccc7")
}

fn diplomatic_immunity() -> CardIndex {
    card_index("f1a3153c-0200-4ff2-b7d5-48d23920bb3c")
}

fn enfeeblement() -> CardIndex {
    card_index("42b2db4c-4a1d-436f-9eeb-53a04db46c58")
}

fn eternal_warrior() -> CardIndex {
    card_index("dab28bc6-3b2a-444f-b596-0a8d95d6d28c")
}

fn flaming_sword() -> CardIndex {
    card_index("05c62f91-2a5b-4cba-8e66-78fb370ea409")
}

fn flight() -> CardIndex {
    card_index("6a4068b0-fb4f-429c-a94e-47849f3eb7ef")
}

// oracle_id = "861c5374-a8e7-4684-9a1b-e6ce3b98e4a2"
fn frog_tongue() -> CardIndex {
    card_index("861c5374-a8e7-4684-9a1b-e6ce3b98e4a2")
}

fn giant_strength() -> CardIndex {
    card_index("db85ba13-f00d-4cdd-99e1-22a4d39c8837")
}

fn greels_caress() -> CardIndex {
    card_index("e9e0b78e-07d5-4603-8e3b-27274148d1a1")
}

/// Phyrexian Fleshgorger: a printed 7/5 whose keywords are stubs, so on a
/// battlefield it is a body and nothing else — which is what makes "-3/-0"
/// readable as a number rather than as a fight with a ward trigger.
fn a_body_to_shrink() -> CardIndex {
    card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
}

fn hero_s_resolve() -> CardIndex {
    card_index("1854a99d-f8c7-45b3-83a2-98ae1c5b5b09")
}

fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}

fn illuminated_wings() -> CardIndex {
    card_index("7a22389a-34e7-4726-a551-f6fbc225cefe")
}

/// Immolation — {R} Aura: "Enchant creature. Enchanted creature gets +2/-2."
///
/// Both halves of the modifier are read off one body: Rootbreaker Wurm is a
/// printed 6/6 and has to become an 8/4, so a `+2/+2` or a `-2/+2` in the card
/// def shows up as a different pair instead of passing. The two bystanders and
/// the Mountain draw the two lines the card prints — the static reaches only
/// the permanent it holds (the Elf beside it and the Elf across the table stay
/// 1/1), and the target choice offers every creature on the table and no land,
/// which is "enchant creature" and not "enchant permanent".
fn immolation() -> CardIndex {
    card_index("9f40ad89-3767-4837-a078-f2dcfaf368df")
}

fn imposing_visage() -> CardIndex {
    card_index("6f67058b-ed14-4e3c-9af3-d61570870e36")
}

// oracle_id = "cca60afe-b044-4401-8322-170aa015873c"
fn indomitable_will() -> CardIndex {
    card_index("cca60afe-b044-4401-8322-170aa015873c")
}

// oracle_id = "864c7575-4589-416c-b143-310d5ef238c5"
fn inertia_bubble() -> CardIndex {
    card_index("864c7575-4589-416c-b143-310d5ef238c5")
}

fn lance() -> CardIndex {
    card_index("5960dd01-6797-4c73-b48a-f637b9c288cc")
}

fn magetas_boon() -> CardIndex {
    card_index("e3e5b12c-2103-4b40-83d8-6d5449179b6f")
}

fn primal_frenzy() -> CardIndex {
    card_index("705b4da3-d463-4808-b79c-dc0c1830945a")
}

fn reflexes() -> CardIndex {
    card_index("dc87b0a5-3d9d-44eb-b415-b022acd63cf1")
}

fn robe_of_mirrors() -> CardIndex {
    card_index("093a20e5-ff14-41c7-b16c-1f745ddf6942")
}

fn sicken() -> CardIndex {
    card_index("5208a5f2-eebe-4adc-8f29-543b60116817")
}

fn sinister_strength() -> CardIndex {
    card_index("deb499bd-71d4-4430-8a58-a31f5ab1b239")
}

fn unholy_strength() -> CardIndex {
    card_index("090d88a9-7f2d-4bd1-a30a-7c48d05068be")
}

fn vigilance() -> CardIndex {
    card_index("70570170-be76-4c56-9151-c4b6e253f462")
}

fn weakness() -> CardIndex {
    card_index("f07a24c0-bf3c-4733-9473-c6be3b16950e")
}

fn a_seven_five_wurm() -> CardIndex {
    card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
}

fn web() -> CardIndex {
    card_index("5aa12aff-db3c-4be5-822b-3afdf536b33e")
}

fn angelic_shield() -> CardIndex {
    card_index("05b020fd-21be-495d-ae45-7de3b1224e6d")
}

fn carnival_of_souls() -> CardIndex {
    card_index("95b10ca7-7360-4da5-bd93-686ae3051833")
}

fn clutch_of_undeath() -> CardIndex {
    card_index("5a68c925-db79-44f2-a5c1-5a607298f6cf")
}

fn compulsion() -> CardIndex {
    card_index("3fafb6b2-5cae-45b6-8550-3ff8daa02802")
}

fn concordant_crossroads() -> CardIndex {
    card_index("ff01b408-6d17-40a3-9efd-a1b341ec1307")
}

fn dark_heart_of_the_wood() -> CardIndex {
    card_index("c44f40da-867e-4237-b4b1-ed6feb1f37b7")
}

fn darkest_hour() -> CardIndex {
    card_index("5667376e-e59c-4b17-b096-5d92cdfe3db1")
}

fn dehydration() -> CardIndex {
    card_index("db27c686-e202-4b60-9a10-0a0fef25576c")
}

fn divine_transformation() -> CardIndex {
    card_index("292e7135-8804-43f2-a486-51ef97b83f77")
}

fn elven_palisade() -> CardIndex {
    card_index("0fb94fa4-2aff-4636-ac1b-ed39dc9451a6")
}

fn feast_of_the_unicorn() -> CardIndex {
    card_index("274d89b8-1e59-4992-9299-dc793b7f6752")
}

fn flight_of_fancy() -> CardIndex {
    card_index("cd20a2a4-5e5e-420d-9420-651bce511f76")
}

fn improvised_armor() -> CardIndex {
    card_index("aca7c0a7-b365-421e-aeb0-49d3a9873e4f")
}

fn maggot_therapy() -> CardIndex {
    card_index("ab4887e6-f71b-462d-9243-9ebe54da98f4")
}

fn mass_hysteria() -> CardIndex {
    card_index("4500131b-7417-4f30-a1b0-97d51b2e6458")
}

fn mythic_proportions() -> CardIndex {
    card_index("e03322a0-e477-4223-969e-27f6772e3d6d")
}

fn need_for_speed() -> CardIndex {
    card_index("8894ab96-17e1-41d8-a4fb-28b510807394")
}

fn onslaught() -> CardIndex {
    card_index("ae9ca82c-e07e-4a41-a387-0ef7d6df14b6")
}

fn scavenged_weaponry() -> CardIndex {
    card_index("2d3010d5-5c21-4342-ad79-a737b6731230")
}

fn seal_of_fire() -> CardIndex {
    card_index("348a345e-4639-41ca-b015-a5d43459eb64")
}

fn seal_of_removal() -> CardIndex {
    card_index("f0801029-bcf7-4bdb-84bf-e88dcaa9dc03")
}

fn seal_of_strength() -> CardIndex {
    card_index("e41a68b3-e1cb-4f51-bd54-68882d2cc015")
}

fn serra_s_embrace() -> CardIndex {
    card_index("6d6ba936-4a15-4c40-aaa6-71605fb732d1")
}

fn spectral_cloak() -> CardIndex {
    card_index("fadfa9f9-d096-4083-8630-1c18928133ff")
}

fn tiger_claws() -> CardIndex {
    card_index("2ef8ebc9-4f95-42f8-86e6-85eff0b8f021")
}

fn torment() -> CardIndex {
    card_index("b53ec6e6-fcc9-4471-88c5-7ad0fbd7bbea")
}

fn twisted_experiment() -> CardIndex {
    card_index("4066d4df-d98f-44cd-bf25-ebb5e9d9ddeb")
}

fn wings_of_aesthir() -> CardIndex {
    card_index("06413d87-d119-4c04-93d5-5ced7ad4a858")
}

fn wings_of_hope() -> CardIndex {
    card_index("c1df6359-edf4-48cf-b8d1-6240ac291cf7")
}

fn zephid_s_embrace() -> CardIndex {
    card_index("2f8b07ef-9d00-4eb6-a395-b5033aa3f80e")
}

fn arenson_s_aura() -> CardIndex {
    card_index("465843dc-57d0-46fd-ac47-238723034563")
}

fn armistice() -> CardIndex {
    card_index("da103316-2a85-4de9-8531-ac2cd2859d6f")
}

fn aura_fracture() -> CardIndex {
    card_index("3495d83a-b103-42be-8708-9ce971b352bd")
}

fn aura_shards() -> CardIndex {
    card_index("8d03d050-391c-4311-8c42-4ee632d40fdc")
}

fn back_to_basics() -> CardIndex {
    card_index("05c2dec2-d2f7-4036-b91f-4fccba10a8bb")
}

// oracle_id = "10e95489-a94d-4523-964c-ec9753103a62"
fn blanket_of_night() -> CardIndex {
    card_index("10e95489-a94d-4523-964c-ec9753103a62")
}

fn captive_flame() -> CardIndex {
    card_index("fada1102-bbb7-4c90-a72e-6c595c08a55d")
}

fn choke() -> CardIndex {
    card_index("057fa60b-10b0-4612-be0d-157076c82241")
}

fn contemplation() -> CardIndex {
    card_index("fa7efcef-a688-4e25-a823-4d53b2e96508")
}

fn deadapult() -> CardIndex {
    card_index("5e0fea29-0fd5-4535-b1df-cd66e50662cc")
}

fn dralnu_s_crusade() -> CardIndex {
    card_index("ff48cf80-4950-4ae4-9f7c-8d826b2f26f7")
}

// oracle_id = "795b096a-2bce-4588-a2c9-abc5ea40dc0c"
fn enchantress_s_presence() -> CardIndex {
    card_index("795b096a-2bce-4588-a2c9-abc5ea40dc0c")
}

fn fervor() -> CardIndex {
    card_index("8e0cea9c-3110-4728-9378-76849e33bb90")
}

fn fires_of_yavimaya() -> CardIndex {
    card_index("e23d6f3b-0e18-423b-943b-15db7837255b")
}

fn flowstone_surge() -> CardIndex {
    card_index("fb1755a0-3334-419b-8cb5-5a3ac7fa5b13")
}

fn ghitu_war_cry() -> CardIndex {
    card_index("0439df39-0324-4110-b3f4-4a32393021de")
}

fn glorious_anthem() -> CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

fn goblin_bombardment() -> CardIndex {
    card_index("edad60c6-80de-4033-af1b-a703ac332983")
}

fn goblin_trenches() -> CardIndex {
    card_index("b43f40e6-c0ad-4a12-b75d-f2ba12629bfe")
}

fn goblin_war_drums() -> CardIndex {
    card_index("29c21edd-781b-448e-824a-17bc8b8f4077")
}

fn gravity_sphere() -> CardIndex {
    card_index("8ddf93fe-980b-4dc4-b56f-6a2ee50100a6")
}

fn hannas_custody() -> CardIndex {
    card_index("57b0205d-ad9d-45b7-8556-0733aa7a4987")
}

fn peace_of_mind() -> CardIndex {
    card_index("4f8c5fd7-f280-4b0c-bb84-6ff9b258c50f")
}

fn primal_rage() -> CardIndex {
    card_index("d6464ee4-23fc-4d68-bbda-3b53772015d1")
}

fn seal_of_cleansing() -> CardIndex {
    card_index("a75dbe70-7e3e-446f-9a76-9fbb414f2e7c")
}

fn serras_blessing() -> CardIndex {
    card_index("49cdd05c-feeb-4c24-9d88-069f5d2e08c3")
}

fn shivan_harvest() -> CardIndex {
    card_index("27c3d7cd-f92e-4203-9fb5-f6b4776f6ffd")
}

// oracle_id = "d32a32d2-203d-4be1-8a33-e037747053c7"
fn sustenance() -> CardIndex {
    card_index("d32a32d2-203d-4be1-8a33-e037747053c7")
}

fn trade_routes() -> CardIndex {
    card_index("120fa67e-e5e0-4c23-9a78-d6171d357aee")
}

// ---------------------------------------------------------------------------
// Alpha cards played by their Oracle text (31 cards, `docs/mechanics-roadmap.md`
// §E8). Each card gets its own `fn <name>() -> CardIndex` naming its oracle
// id, immediately beside the test(s) that play it.
// ---------------------------------------------------------------------------

// oracle_id = "6452b6a6-6235-46a3-a712-a26592450438"
fn thoughtlace() -> CardIndex {
    card_index("6452b6a6-6235-46a3-a712-a26592450438")
}

// oracle_id = "fb80aaba-352a-4b58-8db2-1e02d542819c"
fn deathlace() -> CardIndex {
    card_index("fb80aaba-352a-4b58-8db2-1e02d542819c")
}

// oracle_id = "08842aa3-f923-46e9-a106-f542331e9cc1"
fn chaoslace() -> CardIndex {
    card_index("08842aa3-f923-46e9-a106-f542331e9cc1")
}

/// A vanilla {U} 1/1: the ground creature a blue ward is tested against.
// oracle_id = "218d9277-c179-4de3-9c7f-79b5a6d4fa38"
fn merfolk_of_the_pearl_trident() -> CardIndex {
    card_index("218d9277-c179-4de3-9c7f-79b5a6d4fa38")
}

/// A {B} 1/1 whose only text is an activated ability nobody presses.
// oracle_id = "7d406aa7-636a-45c1-903f-11c2ce2ef3e3"
fn bile_urchin() -> CardIndex {
    card_index("7d406aa7-636a-45c1-903f-11c2ce2ef3e3")
}

// oracle_id = "0c07d09e-e127-4573-b827-6c50246f7a31"
fn skyhunter_skirmisher() -> CardIndex {
    card_index("0c07d09e-e127-4573-b827-6c50246f7a31")
}

fn life_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

/// Casts `card` off the floating pool at `target` and lets it resolve.
#[track_caller]
fn cast_on(engine: &mut Engine<RegistryLookup>, card: CardIndex, target: ObjectId) {
    cast_with_floating(engine, PlayerId::new(0), card);
    aim_at(engine, PlayerId::new(0), target);
    pass_until(engine, stack_is_empty);
}

/// What the target prompt in front of the engine offers.
#[track_caller]
fn target_menu(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("expected a target choice, got {other:?}"),
    }
}

/// The "ward" Auras print "This effect doesn't remove this Aura": an Aura
/// that is itself the color it protects from stays on. All three Wards are
/// white, so the sentence only matters once a lace has turned the Ward into
/// the color it guards against; then a second Aura of that color on the same
/// creature must still fall off, which is the control: it is the Ward's own
/// exception that kept the Ward there, not a protection that does nothing.
#[track_caller]
fn a_ward_outlasts_its_own_color(ward: CardIndex, lace: CardIndex, land: CardIndex, color: Color) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), land, land, quiet_creature()])
        .hand(0, &[ward, holy_armor(), lace, lace])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, quiet_creature()).expect("the creature");
    tap_mana_where(&mut engine, p0, |id| id != creature);

    cast_on(&mut engine, ward, creature);
    cast_on(&mut engine, holy_armor(), creature);
    let ward_id = on_battlefield(&engine, p0, ward).expect("the Ward resolved");
    let armor = on_battlefield(&engine, p0, holy_armor()).expect("the Armor resolved");
    assert_eq!(pt(&engine, creature), (1, 3), "Holy Armor's +0/+2 is on");
    assert_eq!(
        engine
            .state()
            .object(ward_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(Color::White),
        "a Ward is white as printed"
    );

    cast_on(&mut engine, lace, ward_id);
    assert_eq!(
        engine
            .state()
            .object(ward_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::of(color),
        "the lace made the Ward the color it guards against"
    );
    assert_eq!(
        engine.state().object(ward_id).unwrap().attached_to,
        Some(creature),
        "\"This effect doesn't remove this Aura\": the Ward stays on"
    );
    assert!(
        on_battlefield(&engine, p0, ward).is_some(),
        "and on the battlefield"
    );
    assert_eq!(pt(&engine, creature), (1, 3), "the other Aura is untouched");

    cast_on(&mut engine, lace, armor);
    assert!(
        in_graveyard(&engine, p0, holy_armor()).is_some(),
        "an Aura that is not the Ward is removed by the same protection"
    );
    assert_eq!(pt(&engine, creature), (1, 1), "its +0/+2 went with it");
    assert_eq!(
        engine.state().object(ward_id).unwrap().attached_to,
        Some(creature),
        "while the Ward is still there"
    );
}

/// Protection from a color also means "can't be enchanted by Auras of that
/// color": with the Ward on the creature, an Aura of that color is offered
/// the bystander and not the creature, while a white Aura (the control) is
/// still offered both.
#[track_caller]
fn a_ward_keeps_auras_of_its_color_off(ward: CardIndex, aura: CardIndex, land: CardIndex) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                land,
                quiet_creature(),
                festering_goblin(),
            ],
        )
        .hand(0, &[ward, holy_armor(), aura])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, quiet_creature()).expect("the creature");
    let bystander = on_battlefield(&engine, p0, festering_goblin()).expect("the bystander");
    tap_mana_where(&mut engine, p0, |id| id != creature && id != bystander);

    cast_on(&mut engine, ward, creature);
    cast_with_floating(&mut engine, p0, holy_armor());
    let menu = target_menu(&engine);
    assert!(
        menu.contains(&creature) && menu.contains(&bystander),
        "a white Aura may enchant either: {menu:?}"
    );
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, p0, aura);
    let menu = target_menu(&engine);
    assert!(menu.contains(&bystander), "{menu:?}");
    assert!(
        !menu.contains(&creature),
        "the warded creature can't be enchanted by an Aura of that color: {menu:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![creature],
                    players: vec![],
                },
            )
            .is_err(),
        "and naming it anyway is refused"
    );
    aim_at(&mut engine, p0, bystander);
    pass_until(&mut engine, stack_is_empty);
    let aura_id = on_battlefield(&engine, p0, aura).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura_id).unwrap().attached_to,
        Some(bystander)
    );
}

/// Protection from a color also prevents the damage a source of that color
/// would deal to the creature. The same block, without the Ward, kills the
/// 1/1 Elf; with it, the Elf walks away.
#[track_caller]
fn a_ward_prevents_damage_from_its_color(ward: CardIndex, attacker: CardIndex, warded: bool) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), quiet_creature()])
        .hand(0, &[ward])
        .battlefield(1, &[attacker])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf");
    if warded {
        tap_mana_where(&mut engine, p0, |id| id != elf);
        cast_on(&mut engine, ward, elf);
    }
    reach_their_main_phase(&mut engine, p1);
    let foe = on_battlefield(&engine, p1, attacker).expect("their attacker");
    let blocks = attack_and_collect_blocks(&mut engine, foe, p0);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == elf && b.attackers.contains(&foe)),
        "the Elf may block: {blocks:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, foe)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && e.state().turn.active == p1
    });
    if warded {
        assert!(
            on_battlefield(&engine, p0, quiet_creature()).is_some(),
            "the damage was prevented: the Elf lives"
        );
    } else {
        assert!(
            in_graveyard(&engine, p0, quiet_creature()).is_some(),
            "control: with no Ward the same block kills the Elf"
        );
    }
}

/// Protection from a color also means it can't be blocked by creatures of
/// that color: the Elf attacking offers the other creature as a blocker and
/// not the one of the Ward's color; without the Ward both are offered.
#[track_caller]
fn a_warded_creature_slips_past_its_color(ward: CardIndex, blocker: CardIndex, warded: bool) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), quiet_creature()])
        .hand(0, &[ward])
        .battlefield(1, &[blocker, quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf");
    let colored = on_battlefield(&engine, p1, blocker).expect("their colored creature");
    let bystander = on_battlefield(&engine, p1, quiet_creature()).expect("their bystander");
    if warded {
        tap_mana_where(&mut engine, p0, |id| id != elf);
        cast_on(&mut engine, ward, elf);
    }
    let blocks = attack_and_collect_blocks(&mut engine, elf, p1);
    assert!(
        blocks.iter().any(|b| b.blocker == bystander),
        "a creature of another color may block: {blocks:?}"
    );
    assert_eq!(
        blocks.iter().any(|b| b.blocker == colored),
        !warded,
        "the creature of the Ward's color may block only the unwarded Elf: {blocks:?}"
    );
}

/// Declares every one of `attackers` against `defender` and stops at the
/// declare-blockers question, which the caller answers.
#[track_caller]
fn attack_with_all(
    engine: &mut Engine<RegistryLookup>,
    attackers: &[ObjectId],
    defender: PlayerId,
) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { player, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(defender)))
                    .collect(),
            },
        )
        .expect("every attacker came out of the offer");
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { .. } => return,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to the blockers: {other:?}"),
        }
    }
    panic!("never reached the declare-blockers question");
}

/// p0 pays for the Circle's `{1}` ability, activates it, and names `pick`
/// from the sources it is offered. Returns the objects of the whole offer.
#[track_caller]
fn raise_a_circle(
    engine: &mut Engine<RegistryLookup>,
    circle: CardIndex,
    pick: ObjectId,
) -> Vec<ObjectId> {
    let p0 = PlayerId::new(0);
    tap_all_mana(engine, p0);
    activate(engine, p0, circle, 0);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        options,
        choice,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "its controller chooses");
    let source = baylee_core::ids::DamageSourceRef {
        object: pick,
        version: engine.state().object(pick).expect("source exists").version,
    };
    assert!(options.contains(&source), "the pick is on the offer");
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .expect("off the list");
    options.iter().map(|o| o.object).collect()
}

/// Both Circles' "the next time a source of your choice would deal damage
/// to you this turn" read through a combat in which the opponent attacks
/// with `chosen`, `other_of_color` (a second source of the Circle's color)
/// and `off_color` (a source of another color). The offer names the two
/// sources of the color and not the third; the one picked is prevented once;
/// everything else is dealt. `expected_life` is what is left of 20.
/// would deal in all; `expected_life` is what is left of 20.
#[track_caller]
#[allow(clippy::too_many_arguments)] // one board, the way it was declared
fn a_circle_prevents_the_next_damage_of_one_chosen_source(
    circle: CardIndex,
    chosen: CardIndex,
    other_of_color: CardIndex,
    off_color: CardIndex,
    expected_life: i32,
) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[circle, forest()])
        .battlefield(1, &[chosen, other_of_color, off_color])
        .start();
    keep_mulligans(&mut engine);
    let picked = on_battlefield(&engine, p1, chosen).expect("the chosen source");
    let other = on_battlefield(&engine, p1, other_of_color).expect("the second source");
    let off = on_battlefield(&engine, p1, off_color).expect("the off-color source");
    reach_their_main_phase(&mut engine, p1);

    attack_with_all(&mut engine, &[picked, other, off], p0);
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let offered = raise_a_circle(&mut engine, circle, picked);
    assert!(
        offered.contains(&picked) && offered.contains(&other),
        "both sources of the color are on the offer: {offered:?}"
    );
    assert!(
        !offered.contains(&off),
        "and a source of another color is not: {offered:?}"
    );
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(life_of(&engine, p0), expected_life);
    assert!(
        engine.state().shields.is_empty(),
        "\"the next time\": the shield was used up"
    );
}

/// "… this turn": a shield raised in its controller's own main phase, with
/// nothing yet dealt, does not last into the opponent's turn, when the same
/// source attacks and deals its damage in full.
#[track_caller]
fn a_circles_shield_ends_with_the_turn(circle: CardIndex, source: CardIndex, expected_life: i32) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[circle, forest()])
        .battlefield(1, &[source])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let foe = on_battlefield(&engine, p1, source).expect("their source");
    raise_a_circle(&mut engine, circle, foe);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !engine.state().shields.is_empty(),
        "the shield is standing in the turn that made it"
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(
        engine.state().shields.is_empty(),
        "and gone when that turn ended"
    );
    let blocks = attack_and_collect_blocks(&mut engine, foe, p0);
    assert!(blocks.is_empty(), "p0 has nothing to block with");
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        life_of(&engine, p0),
        expected_life,
        "the source dealt its damage in full"
    );
}

fn black_ward() -> CardIndex {
    card_index("7861ac9b-3024-4935-804c-2ca4c5a46bf4")
}

fn blue_ward() -> CardIndex {
    card_index("fc0bf1d0-46a2-4305-ab2e-466d79d60ab2")
}

fn green_ward() -> CardIndex {
    card_index("727ab7f2-741e-4442-b5cb-e3032549fa87")
}

fn red_ward() -> CardIndex {
    card_index("73f84440-425f-4cf5-b01a-3ae89f1f6e37")
}

fn blessing() -> CardIndex {
    card_index("5c84d8da-2bfb-4618-89a0-7d9ed604e854")
}

fn holy_armor() -> CardIndex {
    card_index("912164c2-b4d4-42e3-a10e-903b8c7b2e6d")
}

fn firebreathing() -> CardIndex {
    card_index("8603bf74-faab-4910-8e45-0f2e3b318efb")
}

fn circle_of_protection_blue() -> CardIndex {
    card_index("d7572f17-f85d-45c0-ac64-43aac760eafe")
}

fn circle_of_protection_green() -> CardIndex {
    card_index("41b0f347-1398-4778-bf3f-4007d8a77162")
}

fn circle_of_protection_white() -> CardIndex {
    card_index("5f46f86a-9779-4ed3-99cb-76a03d380598")
}

fn crusade() -> CardIndex {
    card_index("4692740f-be90-459f-8d90-c4ae71771595")
}

fn bad_moon() -> CardIndex {
    card_index("fc5d3341-cbce-49e5-93cc-8add92479dca")
}

fn karma() -> CardIndex {
    card_index("fac4da47-f0b2-4b57-9703-e0ed100d3499")
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

fn steal_artifact() -> CardIndex {
    card_index("cd8ae9f2-edac-473a-8846-c08219e617c3")
}

fn copy_artifact() -> CardIndex {
    card_index("80bc56a9-40e0-48da-ae86-190e39c8a4a3")
}

fn feedback() -> CardIndex {
    card_index("2ff92886-5c17-47a4-a02b-a97432d9203e")
}

fn invisibility() -> CardIndex {
    card_index("de26b0c6-dfb7-45a8-9d7f-f8d45522d675")
}

fn wall_of_roots() -> CardIndex {
    card_index("3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f")
}

fn lifetap() -> CardIndex {
    card_index("52ac09af-2aa7-4d80-be26-3e6a6efd5c23")
}

fn psychic_venom() -> CardIndex {
    card_index("a60422e8-f2f4-4c37-a0f4-eedad27eb08c")
}

fn cursed_land() -> CardIndex {
    card_index("0d61239f-28e4-4adb-8f6e-b56e9c8699af")
}

fn deathgrip() -> CardIndex {
    card_index("20ae75a7-14ca-4366-af0a-3f3f02159f3f")
}

fn fear() -> CardIndex {
    card_index("355bbe9b-59bf-470f-8600-410af4c7fe18")
}

fn paralyze() -> CardIndex {
    card_index("e9b4e857-39e5-4a06-89ad-3dd94b3f252a")
}

fn pestilence() -> CardIndex {
    card_index("dafe63ef-f3d6-45e7-877a-573da92ba85e")
}

fn warp_artifact() -> CardIndex {
    card_index("f4b18451-1f40-48bd-8ca9-eec784ad5dd7")
}

fn burrowing() -> CardIndex {
    card_index("d6b9b88b-e31b-4b88-9d53-3df5687804ba")
}

fn lifeforce() -> CardIndex {
    card_index("07ae1fe5-5c3e-4d94-b809-8defd2ef44e3")
}

fn regeneration() -> CardIndex {
    card_index("89390a33-b289-4edd-a114-6616e49a49c2")
}

fn wanderlust() -> CardIndex {
    card_index("73bddfdb-d1fb-4038-b676-201f6b82beb0")
}

fn wild_growth() -> CardIndex {
    card_index("706ae742-1807-44b7-a4fa-f2e26f61519a")
}

fn stasis() -> CardIndex {
    card_index("a8cf1379-0195-4e11-b994-481ef1284245")
}

fn smoke() -> CardIndex {
    card_index("8aa97d25-cd51-4ceb-b7eb-af64f0914a8c")
}

fn wall_of_spears() -> CardIndex {
    card_index("ed836d84-ff1e-4af8-b4b8-314569b3faec")
}

fn animate_wall() -> CardIndex {
    card_index("c7a6a165-b709-46e0-ae42-6f69a17c0621")
}

fn instill_energy() -> CardIndex {
    card_index("8695c3c1-fb4b-4429-ae33-ec186f68796b")
}

fn evil_presence() -> CardIndex {
    card_index("3d8ac41c-0566-48b2-a744-39db2f72272c")
}

fn darksteel_citadel() -> CardIndex {
    card_index("8dc067bf-f78f-4ac4-b6e7-b305c42cf0bc")
}

fn conversion() -> CardIndex {
    card_index("a24e05fb-dffb-4400-b4ca-22fdde45e7a7")
}

fn phantasmal_terrain() -> CardIndex {
    card_index("7dcbce46-2973-4a9f-93df-95ac41ce668a")
}

/// Casts `aura` off an open board, asserting that `wrong_kind` — a
/// permanent the Enchant line's kind excludes — is never among the
/// offered targets while `target` is, then resolves it onto `target` and
/// returns the Aura's own object once it is attached.
///
/// Shared by every Aura this batch reports as attaching and doing nothing
/// else: all of them share exactly this shape.
#[track_caller]
fn attaches_only_to(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    aura: CardIndex,
    target: ObjectId,
    wrong_kind: ObjectId,
) -> ObjectId {
    cast_from_hand(engine, seat, aura);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&target),
        "the Enchant line's own kind is offered: {options:?}"
    );
    assert!(
        !options.contains(&wrong_kind),
        "the wrong kind is never offered: {options:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
    let object = on_battlefield(engine, seat, aura).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(object).and_then(|o| o.attached_to),
        Some(target),
        "ends attached to the chosen permanent"
    );
    object
}

fn consecrate_land() -> CardIndex {
    card_index("4627691c-4ed4-4add-9cc3-2e019be2f9fd")
}

fn animate_artifact() -> CardIndex {
    card_index("2dd7a4dc-902a-4e85-8a3b-c96a898fba86")
}

fn creature_bond() -> CardIndex {
    card_index("70492e32-ba4d-4314-b016-892fb15f7a23")
}

fn earthbind() -> CardIndex {
    card_index("e8e35b49-8cfb-4fb5-89aa-8050f15b11bf")
}

fn aspect_of_wolf() -> CardIndex {
    card_index("77b7277d-90a1-4774-a998-8c35c3f94e4a")
}

fn living_artifact() -> CardIndex {
    card_index("4ff9af56-ac18-4966-9e48-183e1ca1c2d0")
}

fn lure() -> CardIndex {
    card_index("7a7425ba-4478-4bc4-855f-abf947ea4fa2")
}

// ---------------------------------------------------------------- Lure
//
// "All creatures able to block enchanted creature do so" (CR 509.1c): the
// engine's own block-requirement machinery, played on real cards rather
// than the synthetic fixtures `block_requirement_tests.rs` uses.

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hurloon_minotaur() -> CardIndex {
    card_index("8f1dae40-b307-446e-bbd2-86aa35813871")
}

fn pearled_unicorn() -> CardIndex {
    card_index("c071be90-0531-40cc-af46-0cbe80c4ddd4")
}

fn fire_sprites() -> CardIndex {
    card_index("fc5e42b5-4da2-4777-828b-138c0a5d234f")
}

fn angelic_wall() -> CardIndex {
    card_index("4502b24f-604b-4e36-9168-31c1a1ab4dab")
}

/// What `actions.rs`' block-requirement check refuses a declaration with
/// (`"a creature that must block if able does not"`).
const MUST_BLOCK: &str = "a creature that must block if able does not";

/// Asserts `result` is the engine's own refusal `why`, naming the message
/// and not merely that something failed — the distinction a wrong-mana
/// refusal and a wrong-restriction refusal would otherwise erase.
#[track_caller]
fn refused(result: Result<(), EngineError>, why: &str) {
    match result {
        Err(EngineError::IllegalAction(message)) => assert_eq!(message, why),
        other => panic!("expected the refusal {why:?}, got {other:?}"),
    }
}

/// With no Lure anywhere on this board, a creature is under no requirement
/// to block: the question names nothing to obey, and declaring no blocks
/// is legal.
#[test]
fn not_blocking_is_legal_with_no_lure_anywhere() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grizzly_bears()])
        .battlefield(1, &[hurloon_minotaur()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
            },
        )
        .expect("the Bear attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { obeying, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on exactly this")
    };
    assert!(
        obeying.is_empty(),
        "with no Lure anywhere, nothing is required to block"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("not blocking is legal with no Lure in play");
}

// ---------------------------------------------------------- Living Artifact
//
// "Enchant artifact" itself — offered only artifacts, and attaching to the
// one chosen — is already read by `living_artifact_attaches_only_to_an_artifact`
// above; what follows plays the two triggered abilities its own printing
// adds beyond that.

/// A vanilla {W} 1/1 with first strike and nothing else.
fn serra_zealot() -> CardIndex {
    card_index("989a7353-8d3d-4ea2-ab5e-8535d95dddae")
}

/// How many times the vitality counters on `id` changed over the whole game.
///
/// [`counters_on`] answers "how many now"; a trigger firing once for a
/// step's whole damage total and a trigger firing twice for the same total
/// split across two events look identical read that way, so the sentence
/// "one trigger" is only checked by counting the [`GameEvent::CounterChanged`]
/// entries themselves.
fn vitality_counter_changes(engine: &Engine<RegistryLookup>, id: ObjectId) -> usize {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|e| {
            matches!(e.event, crate::event::GameEvent::CounterChanged { object, .. } if object == id)
        })
        .count()
}

fn healing_salve() -> CardIndex {
    card_index("8da8644c-75a1-4fe9-8e94-900d948d631c")
}

// ---------------------------------------------------------------------------
// Circle of Protection: Black.
// ---------------------------------------------------------------------------

fn circle_of_protection_black() -> CardIndex {
    card_index("7a5a8414-4da4-4dd0-93ae-210d50f4d6f6")
}

// oracle_id = "75457fe5-4ab6-42c4-98e5-8ed6e8bf122c"
fn fishliver_oil() -> CardIndex {
    card_index("75457fe5-4ab6-42c4-98e5-8ed6e8bf122c")
}

// oracle_id = "278b237e-9699-43eb-a03e-0b68eccc08b3"
fn unstable_mutation() -> CardIndex {
    card_index("278b237e-9699-43eb-a03e-0b68eccc08b3")
}

fn energy_flux() -> CardIndex {
    card_index("7a756cd1-29a8-4edf-bb74-fbb5b4020022")
}

fn gate_to_phyrexia() -> CardIndex {
    card_index("bbb005de-bbba-458e-87c1-912a004e80da")
}
