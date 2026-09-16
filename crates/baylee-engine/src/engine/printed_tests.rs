//! What a permanent *is* while nothing is happening to it, swept over every
//! permanent card in the pool.
//!
//! The third of the CR property sweeps, and the one that pins the **layer
//! system's resting state**: a permanent alone on a battlefield, with no
//! other object anywhere that could reach it, has to project exactly what its
//! own face prints. CR 613.1 is the whole claim — the layers exist to apply
//! continuous *effects*, and with no effect in existence there is nothing for
//! any of the eleven of them to do.
//!
//! It works the way the plan splits the job in two. `xtask validate` pins a
//! card's **data** to what Scryfall prints, and a sweep here pins the
//! **engine's reading** of that data. Neither does both jobs, so neither is
//! circular.
//!
//! # Why every expectation is written out rather than called
//!
//! [`Characteristics::from_face`] is the engine's own reading of a face, and
//! `CachedChar` stores a projection *only when it differs from the base* — so
//! a sweep that asked `from_face` what to expect would, on a quiet board, be
//! comparing one value with itself. It would pass every card in the pool
//! while checking none of them, which is the exact failure this tier is
//! written to avoid. Every expectation below is therefore read off the
//! [`FaceDef`] directly, and the two *derived* ones — a face's colors and a
//! front face's keywords — are stated a second time here rather than
//! borrowed. That duplication is the point: two statements of one rule can
//! disagree, and a rule stated once and called twice cannot.
//!
//! # What is skipped, and why it is counted
//!
//! A card carrying its own `AbilityDef::Static` may legitimately differ from
//! its printing — Mycosynth Lattice is an artifact that makes all permanents
//! artifacts, itself included. So a static shields the *fields its modifier
//! can move* and nothing else: the Lattice's types and colors are skipped and
//! its name, cost and supertypes are still swept. Every skip is counted and
//! every count has a floor, because a sweep that stopped reaching the pool
//! would otherwise look exactly like a sweep that found nothing wrong.
//!
//! The two `…IfCountersAtLeast` modifiers are deliberately **not** shielded.
//! A permanent seeded onto the battlefield has no counters, so the condition
//! is false and the modifier must not apply, which is a claim worth making
//! rather than a case worth skipping.
//!
//! # What a board of one cannot say
//!
//! The shield and the empty table cut the same way, so the sweep's claim is
//! narrower than "the layers are right": it is that **nothing applies to a
//! permanent that prints no continuous effect**. A card whose anthem names
//! far too much has no second permanent to reach here, and its own fields are
//! shielded, so a filter that is too wide passes this sweep untouched.
//! Filters are a two-card question, which is what
//! `an_effect_on_the_board_is_exactly_what_this_sweep_catches` asks — one
//! Lattice beside one Elf, with the fields that must move and the fields that
//! must not both named. Widening that from one pair to the pool is a later
//! tier, not this one.
//!
//! What this one does hold against the engine was measured rather than
//! argued: swapping `power` and `toughness` in [`Characteristics::from_face`]
//! fails it with 76 named offenders across 38 cards.

use super::testkit::{Duel, RegistryLookup, basic_forest, card_index, on_battlefield};
use super::*;
use crate::object::Characteristics;
use baylee_cards_dsl::static_ability::{Layer, Modifier, StaticAbility};
use baylee_cards_dsl::{AbilityDef, CardDef, EnterModifier, FaceDef};
use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, ObjectId, SubtypeId};
use baylee_core::types::SubtypeSet;

/// Llanowar Elves: a 1/1 green Elf Druid with one mana ability and no static
/// of its own, so every field of it is swept and none is shielded.
fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// Mycosynth Lattice: "all permanents are artifacts" and "…are colorless",
/// both under `Filter::Any`. The one card in the pool that changes what
/// everything else *is* while doing nothing else to it.
fn mycosynth_lattice() -> CardIndex {
    card_index("ae1f2ab5-c6a5-4d49-a746-3cb4668bf805")
}

/// One characteristic a printed face states and a resting projection has to
/// give back unchanged.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Field {
    Name,
    ManaCost,
    Colors,
    Types,
    Supertypes,
    Subtypes,
    Keywords,
    Power,
    Toughness,
    Loyalty,
}

/// Every field, which is also the order a failure lists them in.
const FIELDS: [Field; 10] = [
    Field::Name,
    Field::ManaCost,
    Field::Colors,
    Field::Types,
    Field::Supertypes,
    Field::Subtypes,
    Field::Keywords,
    Field::Power,
    Field::Toughness,
    Field::Loyalty,
];

/// Which fields one static ability could move.
///
/// Read off the modifier, with the layer as a backstop: every layer 7 effect
/// is about power and toughness whatever its modifier says, so a modifier
/// this table has not learned about yet still shields the right two fields
/// when it declares one. A modifier that changes nothing a permanent *is* —
/// granting an activated ability, preventing damage, a rule about its
/// controller — shields nothing, which is what keeps most of the pool swept.
fn touched(ability: &StaticAbility) -> &'static [Field] {
    const PT: &[Field] = &[Field::Power, Field::Toughness];
    const EVERYTHING: &[Field] = &FIELDS;
    let by_modifier: &'static [Field] = match ability.modifier {
        // A copy effect is layer 1 and replaces the lot (CR 613.2).
        Modifier::BecomeCopyOf(_) => EVERYTHING,
        Modifier::AddType(_) | Modifier::RemoveType(_) => &[Field::Types],
        Modifier::AddSubtype(_) | Modifier::AllCreatureTypes | Modifier::AllBasicLandTypes => {
            &[Field::Subtypes]
        }
        Modifier::AddColor(_) | Modifier::SetColor(_) => &[Field::Colors],
        Modifier::AddKeyword(_) | Modifier::RemoveKeyword(_) | Modifier::LoseKeywords => {
            &[Field::Keywords]
        }
        Modifier::ModifyPT(..)
        | Modifier::SetPT(..)
        | Modifier::SwitchPT
        | Modifier::ModifyPTPerCount { .. } => PT,
        _ => &[],
    };
    if by_modifier.is_empty()
        && matches!(
            ability.layer,
            Layer::PtCda | Layer::PtSet | Layer::PtModify | Layer::PtCounters | Layer::PtSwitch
        )
    {
        return PT;
    }
    by_modifier
}

/// Every field shielded by the card's own printed text.
///
/// The static's filter is not consulted, so this is the conservative reading:
/// a card whose anthem pumps only *other* creatures still has its own power
/// and toughness skipped. Narrowing that would mean evaluating a filter
/// against a board, which is the thing the sweep exists to check.
///
/// `EnterModifier::ChooseSubtype` shields subtypes for the same reason from
/// the other direction — Roaming Throne is "the chosen type in addition to
/// its other types", so the permanent on the table legitimately carries a
/// subtype its face does not print, and which one depends on the answer
/// [`settle`] gave.
fn shielded(def: &CardDef) -> Vec<Field> {
    let mut out = Vec::new();
    for ability in def.abilities_for_face(0) {
        if let AbilityDef::Static(stat) = ability {
            for field in touched(stat) {
                if !out.contains(field) {
                    out.push(*field);
                }
            }
        }
    }
    if def.faces[0]
        .enter_modifiers
        .contains(&EnterModifier::ChooseSubtype)
        && !out.contains(&Field::Subtypes)
    {
        out.push(Field::Subtypes);
    }
    out
}

/// What one field of a face prints, against what the game says it is.
///
/// Separate from the sweep so the counter-test can hand it a pair that does
/// not belong together, and separate from the board so it can be handed one
/// at all.
///
/// Each arm answers with the **comparison** and then with two strings for the
/// report, never with the strings alone. That order is the whole of it:
/// `SubtypeSet` prints as `SubtypeSet(..)`, so an arm that had compared what
/// it renders would have passed an Elf against a Forest — which is exactly
/// what `the_comparison_notices_…` caught before this module ever ran over
/// the pool.
fn mismatch(
    field: Field,
    def: &CardDef,
    face: &FaceDef,
    c: &Characteristics,
    name: &str,
) -> Option<String> {
    let subtypes = |set: SubtypeSet| {
        let ids: Vec<u16> = set.iter().map(SubtypeId::get).collect();
        format!("{ids:?}")
    };
    let (same, printed, projected) = match field {
        Field::Name => (
            name == face.name,
            format!("{:?}", face.name),
            format!("{name:?}"),
        ),
        Field::ManaCost => (
            c.mana_cost == face.mana_cost,
            format!("{:?}", face.mana_cost),
            format!("{:?}", c.mana_cost),
        ),
        // CR 105.2, stated here a second time on purpose: a face's colors
        // are the colors of its cost, plus the indicator a face with no cost
        // prints instead of one.
        Field::Colors => {
            let want: ColorSet = face.mana_cost.colors().union(face.color_indicator);
            (
                c.colors == want,
                format!("{want:?}"),
                format!("{:?}", c.colors),
            )
        }
        Field::Types => (
            c.types == face.types,
            format!("{:?}", face.types),
            format!("{:?}", c.types),
        ),
        Field::Supertypes => (
            c.supertypes == face.supertypes,
            format!("{:?}", face.supertypes),
            format!("{:?}", c.supertypes),
        ),
        Field::Subtypes => {
            let want = SubtypeSet::from_slice(face.subtypes);
            (c.subtypes == want, subtypes(want), subtypes(c.subtypes))
        }
        // The other rule stated twice: a front face keeps the keywords it
        // prints and falls back to the card's list when it prints none.
        Field::Keywords => {
            let want = if face.keywords.is_empty() {
                def.keywords
            } else {
                face.keywords
            };
            (
                c.keywords == want,
                format!("{want:?}"),
                format!("{:?}", c.keywords),
            )
        }
        Field::Power => (
            c.power == face.power,
            format!("{:?}", face.power),
            format!("{:?}", c.power),
        ),
        Field::Toughness => (
            c.toughness == face.toughness,
            format!("{:?}", face.toughness),
            format!("{:?}", c.toughness),
        ),
        Field::Loyalty => (
            c.loyalty == face.loyalty,
            format!("{:?}", face.loyalty),
            format!("{:?}", c.loyalty),
        ),
    };
    (!same).then(|| {
        format!(
            "{}: its {field:?} prints {printed} and projects {projected}",
            face.name
        )
    })
}

/// The board: this card and nothing else, on either side.
///
/// A placement rather than a play (`Cause::Setup`), which is right here and
/// was wrong for the entry sweep: nothing about a resting characteristic is a
/// replacement effect, and a placement is the only way to get a card costing
/// eight mana onto a first-turn battlefield at all.
fn alone(cards: &[CardIndex]) -> Result<Engine<RegistryLookup>, String> {
    let mut engine = Duel::new(11, basic_forest()).battlefield(0, cards).start();
    settle(&mut engine)?;
    Ok(engine)
}

/// One pass of the machine, and the questions a permanent's own arrival asks.
///
/// A seeded battlefield is not yet a projected one: `sync_static_effects`
/// runs inside `progress`, so a board read straight out of `Engine::new` has
/// **no continuous effects registered at all** — including the ones its own
/// permanents print. Every card would have projected its printing there and
/// the sweep would have been a tautology that could not see a Mycosynth
/// Lattice on the table beside it.
///
/// That first pass is also where a setup-placed permanent meets its own
/// enter modifiers, which is worth knowing and is *not* what the entry
/// sweep's "`Cause::Setup` is a placement rather than an entry" refers to: no
/// replacement effect looks at the move, and `apply_enter_modifiers` still
/// reaches the permanent on the pass that follows it. Measured on a seeded
/// Bojuka Bog — tapped the moment the first mulligan is kept, and untapped
/// again by the untap step before anyone gets priority, which is why the
/// entry sweep sees an untapped board and why that is not evidence the
/// modifier was skipped. So a shockland asks for
/// its two life and a Roaming Throne asks for a creature type, either side of
/// the mulligans depending on the card. Both are answered the cheap way — no,
/// and the first option — because neither answer is a characteristic: the
/// sweep reads what a permanent *is*, and a tapped Watery Grave is a Watery
/// Grave.
///
/// Only those three questions are answered, and the loop stops at anything
/// else rather than answering it blind — a sweep that said "no" to every
/// yes-or-no would be making decisions about the permanent it is measuring,
/// and a wrong one would look like a projection bug.
fn settle(engine: &mut Engine<RegistryLookup>) -> Result<(), String> {
    for _ in 0..64 {
        let (player, action) = match engine.pending().clone() {
            Pending::Mulligan { player, .. } => (player, PlayerAction::MulliganKeep),
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { .. },
                ..
            } => (player, PlayerAction::YesNo(false)),
            Pending::ChooseSubtype { player, options } => (
                player,
                PlayerAction::ChooseSubtype(*options.first().ok_or("was offered no subtype")?),
            ),
            _ => return Ok(()),
        };
        engine
            .apply(player, action)
            .map_err(|err| format!("refused a setup answer: {err:?}"))?;
    }
    Err("never settled after sixty-four answers".into())
}

/// The permanent, its face, and the name the game interned for it.
fn read(engine: &Engine<RegistryLookup>, card: CardIndex) -> Result<(ObjectId, String), String> {
    let seat = PlayerId::new(0);
    let object = on_battlefield(engine, seat, card).ok_or("never reached the battlefield")?;
    let state = engine.state();
    let permanent = state.object(object).ok_or("was placed and then vanished")?;
    let name = state
        .names
        .get(permanent.characteristics().name)
        .to_string();
    Ok((object, name))
}

/// What one chunk of the pool managed.
#[derive(Default)]
struct Tally {
    /// Permanents that reached a battlefield and were read.
    cards: usize,
    /// Field comparisons actually made.
    checked: usize,
    /// Comparisons skipped because the card's own printed text could move
    /// that field.
    shielded: usize,
    /// Cards that were placed and were gone again before anyone held
    /// priority. See [`LEAVING_CEILING`].
    left: Vec<&'static str>,
}

impl Tally {
    fn absorb(&mut self, other: &mut Self) {
        self.cards += other.cards;
        self.checked += other.checked;
        self.shielded += other.shielded;
        self.left.append(&mut other.left);
    }
}

/// How small the sweep is allowed to get before it stops measuring anything.
///
/// The guard against this tier's own failure mode: a sweep that finds nothing
/// is indistinguishable from a sweep that checks nothing. The numbers sit
/// under what a full run reports — measured 2026-09-11: 1268 permanents read,
/// 12663 characteristics compared, 17 shielded by the card's own text — with
/// room for cards to be adopted or a static to be added to one.
const CARD_FLOOR: usize = 1100;
const CHECK_FLOOR: usize = 11000;

/// And the one bound that is a **ceiling** rather than a floor.
///
/// A few cards cannot be measured this way and are right not to be. A clone
/// placed with nothing to copy is a 0/0 and dies to CR 704.5f; an Aura placed
/// attached to nothing goes to the graveyard under CR 704.5m; and Karmic
/// Guide is sacrificed in the first upkeep because echo (CR 702.30a) came due
/// on a board with no mana on it — the engine does not even ask, there being
/// nothing to pay with. All three are the rules working.
///
/// But "the permanent is not there" is also what a broken battlefield looks
/// like, so the bucket is capped rather than ignored: a regression that lost
/// a tenth of the pool fails here instead of quietly shrinking the sweep.
///
/// Measured 2026-09-11: eleven cards — seven clones, three Auras, and the
/// Guide.
const LEAVING_CEILING: usize = 20;

fn walk(slice: &[&'static CardDef]) -> (Vec<String>, Tally) {
    let mut offenders = Vec::new();
    let mut tally = Tally::default();
    for def in slice {
        let face = &def.faces[0];
        let engine = match alone(&[def.index]) {
            Ok(engine) => engine,
            Err(why) => {
                offenders.push(format!("{} {why}", face.name));
                continue;
            }
        };
        // A card that is not there is not an offence and not silence either:
        // a clone with nothing to copy and an Aura with nothing to enchant
        // are both gone by the time anyone holds priority, and both are the
        // rules working. Counted, and capped by [`LEAVING_CEILING`].
        let Ok((object, name)) = read(&engine, def.index) else {
            tally.left.push(face.name);
            continue;
        };
        let c = engine
            .state()
            .object(object)
            .expect("the permanent was just read")
            .characteristics();
        let skip = shielded(def);
        tally.cards += 1;
        for field in FIELDS {
            if skip.contains(&field) {
                tally.shielded += 1;
                continue;
            }
            tally.checked += 1;
            if let Some(found) = mismatch(field, def, face, c, &name) {
                offenders.push(found);
            }
        }
    }
    (offenders, tally)
}

/// The sweep, cut into one chunk per core the way the entry sweep is: a board
/// is built once per card and there are more than a thousand of them.
fn sweep() -> (Vec<String>, Tally) {
    let cards: Vec<&'static CardDef> = baylee_cards::all()
        .filter(|d| d.faces[0].types.is_permanent())
        .collect();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let chunk = cards.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = cards
            .chunks(chunk)
            .map(|slice| scope.spawn(move || walk(slice)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("printed chunk"))
            .fold(
                (Vec::new(), Tally::default()),
                |(mut all, mut total), (found, mut one)| {
                    all.extend(found);
                    total.absorb(&mut one);
                    (all, total)
                },
            )
    })
}

/// CR 613.1: the layers apply continuous effects, and a permanent alone on a
/// board is under none of them.
#[test]
fn every_permanent_in_the_pool_projects_what_its_own_face_prints() {
    let (offenders, tally) = sweep();
    println!(
        "{} permanents read, {} characteristics compared, {} shielded by the card's own text, \
         {} gone before priority: {:?}",
        tally.cards,
        tally.checked,
        tally.shielded,
        tally.left.len(),
        tally.left
    );
    assert!(
        offenders.is_empty(),
        "{} characteristics differ from the face that prints them: {offenders:#?}",
        offenders.len()
    );
    assert!(
        tally.cards >= CARD_FLOOR,
        "only {} permanents reached a battlefield, under the floor of {CARD_FLOOR} — either \
         the pool lost most of itself or this sweep stopped reaching it",
        tally.cards
    );
    assert!(
        tally.checked >= CHECK_FLOOR,
        "only {} characteristics were compared, under the floor of {CHECK_FLOOR}",
        tally.checked
    );
    assert!(
        tally.left.len() <= LEAVING_CEILING,
        "{} permanents were gone before anyone held priority, over the ceiling of \
         {LEAVING_CEILING}: {:?}",
        tally.left.len(),
        tally.left
    );
}

/// The counter-test the sweep is worth nothing without: proof the comparison
/// reads *two* things.
///
/// Every expectation is read off the card and every projection off the game,
/// so the failure to rule out is the two being one value under two names. The
/// same comparison is handed a pair that does not belong together — a
/// Forest's face against a Llanowar Elves' projection — and has to complain
/// about every field the two cards differ in.
#[test]
fn the_comparison_notices_when_the_face_and_the_permanent_are_not_the_same_card() {
    let elves = baylee_cards::by_index(llanowar_elves()).expect("Llanowar Elves is in the pool");
    let wood = baylee_cards::by_index(basic_forest()).expect("Forest is in the pool");
    assert!(
        shielded(elves).is_empty() && shielded(wood).is_empty(),
        "both fixtures have to be unshielded or this proves nothing"
    );

    let engine = alone(&[llanowar_elves()]).expect("the board is built");
    let (object, name) = read(&engine, llanowar_elves()).expect("the Elves are placed");
    let c = engine
        .state()
        .object(object)
        .expect("the Elves exist")
        .characteristics();

    for field in FIELDS {
        assert!(
            mismatch(field, elves, &elves.faces[0], c, &name).is_none(),
            "{field:?} of a Llanowar Elves disagreed with a Llanowar Elves"
        );
    }
    for want in [
        Field::Name,
        Field::ManaCost,
        Field::Colors,
        Field::Types,
        Field::Subtypes,
        Field::Power,
        Field::Toughness,
    ] {
        assert!(
            mismatch(want, wood, &wood.faces[0], c, &name).is_some(),
            "a Forest's face against a Llanowar Elves' projection passed on {want:?}"
        );
    }
}

/// And the mutant, because the sweep is a claim about the **projection** and
/// not about the card data: a continuous effect on the board has to break it.
///
/// Mycosynth Lattice rather than an invented effect, because the sweep's own
/// board builder is what puts it there — a mutant reached through the same
/// door as the thing it is testing. The Lattice makes every permanent an
/// artifact and colorless, so the Elves' types and colors must stop matching
/// their face while everything else about them still does. That last half is
/// what says a shield belongs to a *field* and not to a card.
#[test]
fn an_effect_on_the_board_is_exactly_what_this_sweep_catches() {
    let elves = baylee_cards::by_index(llanowar_elves()).expect("Llanowar Elves is in the pool");
    let engine = alone(&[llanowar_elves(), mycosynth_lattice()]).expect("the board is built");
    let (object, name) = read(&engine, llanowar_elves()).expect("the Elves are placed");
    let c = engine
        .state()
        .object(object)
        .expect("the Elves exist")
        .characteristics();

    assert!(
        c.types.contains(TypeSet::ARTIFACT) && c.colors.is_empty(),
        "the Lattice this counter-test rests on stopped reaching the board: {:?} {:?}",
        c.types,
        c.colors
    );
    for field in [Field::Types, Field::Colors] {
        assert!(
            mismatch(field, elves, &elves.faces[0], c, &name).is_some(),
            "{field:?} passed with a Mycosynth Lattice on the table, so the sweep is \
             reading the card and not the game"
        );
    }
    for field in [
        Field::Name,
        Field::ManaCost,
        Field::Subtypes,
        Field::Power,
        Field::Toughness,
    ] {
        assert!(
            mismatch(field, elves, &elves.faces[0], c, &name).is_none(),
            "{field:?} was reported changed by an effect that does not touch it"
        );
    }
    // The Lattice's own two fields are the shield working: it is an artifact
    // that makes itself an artifact, and colorless already, so a sweep
    // without the shield would have reported the card that is doing the
    // thing rather than the card it was done to.
    let lattice =
        baylee_cards::by_index(mycosynth_lattice()).expect("Mycosynth Lattice is in the pool");
    let shield = shielded(lattice);
    assert!(
        shield.contains(&Field::Types) && shield.contains(&Field::Colors),
        "the Lattice shields {shield:?}"
    );
    assert!(
        !shield.contains(&Field::Name) && !shield.contains(&Field::Power),
        "the Lattice shields fields its statics cannot move: {shield:?}"
    );
}
