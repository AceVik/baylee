//! Two readings of mana: the colour a chosen-colour land adds to what its
//! neighbours can see, and the scry a rider sets off when its mana is spent.

use super::*;
use crate::object::ObjectKind;
use baylee_cards_dsl::effect::ManaSource;
use baylee_cards_dsl::{Filter, ManaColor, SpendRider};

// ------------------------------------------------------ the chosen colour

/// A land on `seat`'s battlefield whose mana is "one mana of the chosen
/// colour", with `chosen` chosen.
///
/// Bare rather than a card: what `colors_of` reads is the land's type, the
/// flag that says its mana is the chosen colour, and the colour itself,
/// and a printed card would put its own mana line in front of the question.
fn haven(state: &mut GameState, seat: PlayerId, chosen: ManaColor) -> ObjectId {
    let name = state.names.intern("Haven");
    let id = state.create_bare(seat, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    let obj = state.object_mut(id).expect("just made it");
    let base = obj.base_mut();
    base.types = TypeSet::LAND;
    base.produced_chosen = true;
    obj.chosen_color = Some(chosen);
    id
}

/// What a Reflecting Pool (`LandColor { mine: true }`) sees beside `havens`
/// and nothing else.
fn seen_beside(havens: &[ManaColor]) -> Vec<ManaColor> {
    let mut engine = bench(7160, vec![], [Seat::default(), Seat::default()]);
    let state = engine
        .dev_state_mut(me())
        .expect("the harness may set boards up");
    let mut pool = None;
    for &chosen in havens {
        pool.get_or_insert(haven(state, me(), chosen));
    }
    let pool = pool.unwrap_or_else(|| haven(state, them(), ManaColor::Red));
    crate::resolve::colors_of(state, me(), ManaSource::LandColor { mine: true }, pool)
}

/// Each colour a land may have had chosen for it is the colour its
/// neighbours see (CR 105.1 names the five), one arm per colour.
///
/// One land at a time, because a union answers for all five at once and
/// would pass with two arms swapped: a Haven that chose red and was read as
/// green still leaves "red" in a five-colour union if the green one was read
/// as red.
#[test]
fn a_land_that_chose_a_colour_shows_that_colour_and_no_other() {
    for chosen in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert_eq!(
            seen_beside(&[chosen]),
            vec![chosen],
            "a Haven that chose {chosen:?}"
        );
    }
}

/// And together, the union in WUBRG order, with a colourless "choice" adding
/// nothing to it: colourless is not a colour (CR 105.1), so a land whose
/// record says it chose colourless has chosen no colour a neighbour could
/// make.
#[test]
fn chosen_colours_join_the_union_and_colourless_joins_nothing() {
    assert_eq!(
        seen_beside(&[
            ManaColor::Green,
            ManaColor::Colorless,
            ManaColor::White,
            ManaColor::Red,
            ManaColor::Black,
            ManaColor::Blue,
        ]),
        vec![
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ]
    );
    assert!(
        seen_beside(&[ManaColor::Colorless]).is_empty(),
        "a colourless choice is no colour, and the land prints no colourless \
         mana either"
    );
    assert!(
        seen_beside(&[]).is_empty(),
        "and the other seat's Haven is not on the side that was asked about"
    );
}

// ------------------------------------------------------------ the rider

const SCRY_GROVE: u32 = 7161;
const DEEP_GROVE: u32 = 7162;
const SAPLING: u32 = 7163;
const TWIN_SAPLING: u32 = 7164;
const GROWTH: u32 = 7165;
const TWIN_GROVE: u32 = 7166;

static CREATURE_SPELL: Filter = Filter::CREATURE;

/// "{T}: Add {G}. When that mana is spent to cast a creature spell, scry 1."
/// — Path of Ancestry's rider, on a land with nothing else to say.
static SCRY_GROVE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::mana_ability!(&[Effect::mana(
    ManaColor::Green,
    1
)
.when_spent(&CREATURE_SPELL, SpendRider::Scry(1))])];

/// The same, scrying 2.
static DEEP_GROVE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::mana_ability!(&[Effect::mana(
    ManaColor::Green,
    1
)
.when_spent(&CREATURE_SPELL, SpendRider::Scry(2))])];

/// The scry-1 rider on two mana made by one activation.
static TWIN_GROVE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::mana_ability!(&[Effect::mana(
    ManaColor::Green,
    2
)
.when_spent(&CREATURE_SPELL, SpendRider::Scry(1))])];

static GAIN_ONE: &[Effect] = &[Effect::gain_life(1)];
static GROWTH_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
    effects: GAIN_ONE,
    targets: None,
    second_targets: None,
}];

fn grove_cards() -> Vec<&'static CardDef> {
    vec![
        super::super::synthetic::land(SCRY_GROVE, "Scry Grove", SCRY_GROVE_ABILITIES),
        super::super::synthetic::land(DEEP_GROVE, "Deep Grove", DEEP_GROVE_ABILITIES),
        super::super::synthetic::land(TWIN_GROVE, "Twin Grove", TWIN_GROVE_ABILITIES),
        card(
            SAPLING,
            creature_face("Sapling", "{G}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TWIN_SAPLING,
            creature_face("Twin Sapling", "{G}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            GROWTH,
            face("Growth", "{G}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            GROWTH_ABILITIES,
        ),
    ]
}

/// Seat 0 at its main phase with `groves` on the battlefield, `spell` in
/// hand, and every grove's mana floating.
fn floated(seed: u64, groves: &[u32], spell: u32) -> (Bench, ObjectId) {
    let mut engine = bench(
        seed,
        grove_cards(),
        [Seat::with(groves).holding(&[spell]), Seat::default()],
    );
    to_main(&mut engine, me());
    for &grove in groves {
        for id in objects(&engine, ZoneLocation::Battlefield, grove) {
            if !engine
                .state()
                .object(id)
                .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
            {
                activate(&mut engine, me(), id, 0);
            }
        }
    }
    let card = the(&engine, ZoneLocation::Hand(me()), spell);
    (engine, card)
}

/// Casts `card` and lets everything resolve, answering each scry by keeping
/// its cards on top; returns how many cards each scry looked at.
fn cast_and_count_scries(engine: &mut Bench, card: ObjectId) -> Vec<usize> {
    engine
        .apply(me(), PlayerAction::CastSpell { card })
        .expect("the offer's spell is cast with the floating mana");
    let mut scries = Vec::new();
    for _ in 0..200 {
        if engine.state().zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me())
        {
            return scries;
        }
        match engine.pending().clone() {
            Pending::Arrange {
                player,
                cards,
                prompt: crate::choice::ArrangePrompt::Scry,
                ..
            } => {
                assert_eq!(player, me(), "the caster scries");
                scries.push(cards.len());
                engine
                    .apply(player, super::super::testkit::look_answer(&cards, &[]))
                    .expect("keeping every card on top is an answer");
            }
            pending => assert!(
                walk_past(engine, &pending),
                "unexpected question: {pending:?}"
            ),
        }
    }
    panic!("the stack never settled");
}

/// The rider's mana spent on a creature spell: one scry of one card
/// (CR 106.6: an additional effect of the mana, CR 701.22a: scry 1).
#[test]
fn mana_spent_on_the_spell_it_names_scries() {
    let (mut engine, sapling) = floated(7161, &[SCRY_GROVE], SAPLING);
    assert_eq!(cast_and_count_scries(&mut engine, sapling), vec![1]);
    assert_eq!(
        objects(&engine, ZoneLocation::Battlefield, SAPLING),
        vec![sapling],
        "and the spell it paid for resolved as well"
    );
}

/// The rider's number is the number it scries: two cards for `Scry(2)`.
#[test]
fn a_rider_scries_as_many_cards_as_it_says() {
    let (mut engine, sapling) = floated(7162, &[DEEP_GROVE], SAPLING);
    assert_eq!(cast_and_count_scries(&mut engine, sapling), vec![2]);
}

/// Spent on a spell the rider does not name, the mana is ordinary mana: it
/// pays, and nothing is scried.
#[test]
fn mana_spent_on_another_spell_scries_nothing() {
    let (mut engine, growth) = floated(7163, &[SCRY_GROVE], GROWTH);
    let before = life(&engine, me());
    assert!(cast_and_count_scries(&mut engine, growth).is_empty());
    assert_eq!(
        life(&engine, me()),
        before + 1,
        "the instant still resolved"
    );
}

/// One scry for each mana of the rider's spent on the spell, whether the two
/// mana came from two groves or from one activation that made both: the
/// reading `apply_spend_riders` takes from CR 106.6a, where "a separate
/// delayed triggered ability is created for each mana produced".
#[test]
fn each_unit_of_ridden_mana_scries_once() {
    let (mut engine, twin) = floated(7164, &[SCRY_GROVE, SCRY_GROVE], TWIN_SAPLING);
    assert_eq!(
        cast_and_count_scries(&mut engine, twin),
        vec![1, 1],
        "two groves, one mana each"
    );
    let (mut engine, twin) = floated(7165, &[TWIN_GROVE], TWIN_SAPLING);
    assert_eq!(
        cast_and_count_scries(&mut engine, twin),
        vec![1, 1],
        "one grove, two mana"
    );
}
