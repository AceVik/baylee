//! The former owned reading is the reference for this allocation-only change.

use super::*;
use baylee_cards_dsl::{Amount, Filter, ManaSource, ZoneSel};

fn mixed_table() -> PlayerView {
    let mut forest = card(1, 0, "Forest");
    forest.board_mana = Some(projected(&[ManaColor::Green], 0));
    let mut tower = card(10, 0, "Command Tower");
    // Preserve order and repetitions in Source, but union them for the flames.
    tower.board_mana = Some(projected(
        &[
            ManaColor::Red,
            ManaColor::Red,
            ManaColor::Colorless,
            ManaColor::Blue,
            ManaColor::Green,
        ],
        0,
    ));
    let mut hidden = card(11, 0, "Mountain");
    hidden.status = baylee_view::ObjectStatus::FACE_DOWN;
    hidden.granted_mana = Some(baylee_view::GrantedMana {
        slot: 0,
        colors: vec![ManaColor::White, ManaColor::Black],
        amount: 1,
    });
    let mut pool = card(14, 1, "Reflecting Pool");
    pool.board_mana = Some(projected(
        &[ManaColor::White, ManaColor::Black, ManaColor::Green],
        0,
    ));
    ViewBuilder::new(2)
        .with_battlefield(
            0,
            [
                forest,
                card(2, 0, "Llanowar Elves"),
                card(3, 0, "Mox Opal"),
                card(4, 0, "Harabaz Druid"),
                card(5, 0, "Harabaz Druid"),
                card(6, 0, "Cavern of Souls"),
                card(7, 0, "Ancient Tomb"),
                card(8, 0, "Azorius Chancery"),
                treasure(9),
                tower,
                hidden,
                card(15, 0, "Havenwood Battleground"),
            ],
        )
        .with_battlefield(1, [card(12, 1, "Taiga"), card(13, 1, "Sol Ring"), pool])
        .build()
}

#[test]
fn planner_sources_match_the_owned_reference_in_every_field() {
    let view = mixed_table();
    for object in &view.battlefield {
        for (index, ability) in printed_abilities(object).iter().enumerate() {
            let index = u32::try_from(index).expect("fixture ability index");
            assert_eq!(
                mana_ability(&view, object.id, index, ability),
                owned_mana_ability(&view, object.id, index, ability),
                "object {:?}, ability {index}",
                object.id,
            );
        }
    }
    // Keep the tricky branches live if a fixture's printed abilities change.
    let source = |slot, index| {
        printed_source(&view, ObjectId::new(slot, 0), index).expect("fixture is a source")
    };
    assert_eq!(source(4, 0).amount, 2); // Both Allies belong to this controller.
    assert_eq!(source(3, 0).colors.len(), 5); // ActivatedConditional.
    assert_eq!(source(9, 0).colors.len(), 5); // Token ability, sacrifice cost.
    assert!(source(9, 0).priced);
    assert!(source(7, 0).priced); // A damage rider.
    assert!(source(15, 1).priced); // An additional sacrifice cost.
    assert_eq!(source(15, 1).amount, 2);
    assert!(source(8, 0).bundle);
    assert_eq!(source(8, 0).colors, [ManaColor::White, ManaColor::Blue]);
    assert_eq!(source(10, 0).colors.len(), 5); // No sorting or deduplication.
    assert!(printed_source(&view, ObjectId::new(6, 0), 1).is_none()); // Restricted.
}

#[test]
fn fire_tallies_match_owned_reference_bits_for_mixed_tables() {
    let mut view = mixed_table();
    // Thirds, fifths, duplicate colours, colourless, grants, tokens and a
    // face-down land; each prefix also catches accidental accumulation changes.
    for reverse in [false, true] {
        if reverse {
            view.battlefield.reverse();
        }
        let mut prefix = view.clone();
        for len in (0..=view.battlefield.len()).rev() {
            prefix.battlefield.truncate(len);
            assert_eq!(
                table_mana(&prefix).map(f32::to_bits),
                owned_table_mana(&prefix).map(f32::to_bits),
                "prefix {len}, reverse {reverse}",
            );
        }
    }
    assert!(table_mana(&view).iter().all(|amount| *amount > 0.0));
}

#[test]
fn produced_colors_keeps_projection_presence_index_and_order() {
    let sources = [
        ManaSource::IntrinsicBasicLandTypes,
        ManaSource::CommanderIdentity,
        ManaSource::ColorsAmong {
            filter: &Filter::LAND,
            zone: ZoneSel::Battlefield,
        },
        ManaSource::CircledColors,
        ManaSource::ExiledWithSource,
        ManaSource::LandColor {
            filter: &Filter::LAND,
            mine: true,
            any_type: true,
        },
        ManaSource::Chosen,
        ManaSource::ChosenOr(&[ManaColor::White]),
    ];
    for projection in [
        None,
        Some(projected(&[ManaColor::Green], 1)),
        Some(projected(&[], 0)),
        Some(projected(
            &[ManaColor::Blue, ManaColor::Colorless, ManaColor::Blue],
            0,
        )),
    ] {
        let mut tower = card(1, 0, "Command Tower");
        tower.board_mana = projection;
        let view = ViewBuilder::new(2).with_battlefield(0, [tower]).build();
        let id = ObjectId::new(1, 0);
        for source in sources {
            let expected = owned_produced_colors(&view, id, 0, source);
            assert_eq!(produced_colors(&view, id, 0, source), expected);
            let borrowed = with_produced_colors(&view, id, 0, source, |colors| {
                let projected = &view.battlefield[0].board_mana.as_ref().unwrap().colors;
                assert!(std::ptr::eq(colors.as_ptr(), projected.as_ptr()));
                colors.to_vec()
            });
            assert_eq!(borrowed, expected, "{source:?}");
            assert!(produced_colors(&view, ObjectId::new(99, 0), 0, source).is_none());
        }
        assert_eq!(
            printed_source(&view, id, 0),
            owned_mana_ability(&view, id, 0, ability_at(&view, id, 0).unwrap()),
        );
    }
}

#[test]
fn reader_lends_single_sources_and_moves_the_existing_bundle() {
    static CHOICE: &[ManaColor] = &[ManaColor::Red, ManaColor::Blue];
    static FIXED: AbilityDef =
        baylee_cards_dsl::mana_ability!(&[Effect::mana(ManaColor::Green, 1)]);
    static CHOOSING: AbilityDef = baylee_cards_dsl::mana_ability!(&[Effect::mana_choice(CHOICE)]);
    let view = mixed_table();
    let id = ObjectId::new(1, 0);
    let fixed = read_mana_ability(&view, id, 0, &FIXED, |colors, amount, bundle, priced| {
        assert!(matches!(colors, Cow::Borrowed(_)));
        assert_eq!(colors.as_ref(), &[ManaColor::Green]);
        assert_eq!((amount, bundle, priced), (1, false, false));
    });
    assert_eq!(fixed, Some(()));
    let choice = read_mana_ability(&view, id, 0, &CHOOSING, |colors, _, _, _| {
        assert!(matches!(colors, Cow::Borrowed(_)));
        assert!(std::ptr::eq(colors.as_ptr(), CHOICE.as_ptr()));
    });
    assert_eq!(choice, Some(()));
    let board = read_mana_ability(
        &view,
        id,
        0,
        ability_at(&view, id, 0).unwrap(),
        |colors, _, _, _| {
            assert!(matches!(colors, Cow::Borrowed(_)));
            let projected = &view.battlefield[0].board_mana.as_ref().unwrap().colors;
            assert!(std::ptr::eq(colors.as_ptr(), projected.as_ptr()));
        },
    );
    assert_eq!(board, Some(()));
    let id = ObjectId::new(8, 0);
    let bundle = read_mana_ability(
        &view,
        id,
        0,
        ability_at(&view, id, 0).unwrap(),
        |colors, amount, bundle, priced| {
            assert!(matches!(colors, Cow::Owned(_)));
            assert_eq!((amount, bundle, priced), (2, true, false));
            let original = colors.as_ptr();
            let colors = colors.into_owned();
            assert!(std::ptr::eq(original, colors.as_ptr()));
            colors
        },
    );
    assert_eq!(bundle, Some(vec![ManaColor::White, ManaColor::Blue]));
    for source in [
        ManaSource::Fixed(ManaColor::Green),
        ManaSource::Choice(CHOICE),
    ] {
        assert_eq!(
            produced_colors(&view, id, 0, source),
            owned_produced_colors(&view, id, 0, source)
        );
    }
}

#[test]
fn refused_amounts_and_restrictions_never_call_the_reader() {
    static ZERO: AbilityDef = baylee_cards_dsl::mana_ability!(&[Effect::mana(ManaColor::Green, 0)]);
    static UNKNOWN: AbilityDef = baylee_cards_dsl::mana_ability!(&[Effect::mana_choice_dynamic(
        &[ManaColor::Blue],
        Amount::X,
    )]);
    let mut druid = card(1, 0, "Harabaz Druid");
    druid.subtypes = baylee_core::types::SubtypeSet::from_slice(&[]);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, [druid, card(2, 0, "Cavern of Souls")])
        .build();
    let id = ObjectId::new(1, 0);
    for (object, index, ability) in [
        (id, 0, &ZERO),
        (id, 0, &UNKNOWN),
        (id, 0, ability_at(&view, id, 0).unwrap()),
        (
            ObjectId::new(2, 0),
            1,
            ability_at(&view, ObjectId::new(2, 0), 1).unwrap(),
        ),
    ] {
        assert_eq!(owned_mana_ability(&view, object, index, ability), None);
        assert_eq!(mana_ability(&view, object, index, ability), None);
        assert_eq!(
            read_mana_ability(&view, object, index, ability, |_, _, _, _| panic!(
                "refused source reached consumer"
            )),
            None::<()>,
        );
    }
}

// Frozen pre-refactor reading: intentionally independent of the callback helpers.
fn owned_mana_ability(
    view: &PlayerView,
    id: baylee_core::ids::ObjectId,
    index: u32,
    ability: &'static AbilityDef,
) -> Option<Source> {
    let (AbilityDef::Activated {
        cost,
        effects,
        mana_ability: true,
        ..
    }
    | AbilityDef::ActivatedConditional {
        cost,
        effects,
        mana_ability: true,
        ..
    }) = ability
    else {
        return None;
    };
    if let Some(colors) = baylee_cards_dsl::mana_bundle(cost, effects) {
        let amount = u8::try_from(colors.len()).unwrap_or(u8::MAX);
        return Some(Source {
            id,
            tap: Tap::Ability(index),
            colors,
            amount,
            bundle: true,
            priced: priced(cost, effects),
            preserve: 0,
            dear: None,
        });
    }
    let (source, amount, restricted) = baylee_cards_dsl::mana_with_riders(cost, effects)?;
    if restricted {
        return None;
    }
    let amount = amount
        .or_else(|| baylee_ai::mana_poured(view, id, effects))
        .filter(|amount| *amount > 0)?;
    let colors = owned_produced_colors(view, id, index, source)?;
    Some(Source {
        id,
        tap: Tap::Ability(index),
        colors,
        amount,
        bundle: false,
        priced: priced(cost, effects),
        preserve: 0,
        dear: None,
    })
}

fn owned_produced_colors(
    view: &PlayerView,
    object: baylee_core::ids::ObjectId,
    index: u32,
    source: baylee_cards_dsl::ManaSource,
) -> Option<Vec<baylee_core::mana::ManaColor>> {
    match source {
        baylee_cards_dsl::ManaSource::Fixed(color) => Some(vec![color]),
        baylee_cards_dsl::ManaSource::Choice(colors) => Some(colors.to_vec()),
        baylee_cards_dsl::ManaSource::IntrinsicBasicLandTypes
        | baylee_cards_dsl::ManaSource::CommanderIdentity
        | baylee_cards_dsl::ManaSource::ColorsAmong { .. }
        | baylee_cards_dsl::ManaSource::CircledColors
        | baylee_cards_dsl::ManaSource::ExiledWithSource
        | baylee_cards_dsl::ManaSource::LandColor { .. }
        | baylee_cards_dsl::ManaSource::Chosen
        | baylee_cards_dsl::ManaSource::ChosenOr(_) => {
            let board = view.object(object)?.board_mana.as_ref()?;
            (board.index == index).then(|| board.colors.clone())
        }
    }
}

fn owned_table_mana(view: &PlayerView) -> [f32; 5] {
    let mut tally = [0.0_f32; 5];
    for object in &view.battlefield {
        let mut colors = [false; 5];
        if !object.status.is_face_down() {
            for (subtype, color) in BASIC_MANA {
                if object.subtypes.contains(subtype) {
                    colors[color as usize] = true;
                }
            }
        }
        for granted in object.granted_mana_options() {
            for color in &granted.colors {
                if let Some(slot) = colors.get_mut(*color as usize) {
                    *slot = true;
                }
            }
        }
        for (index, ability) in printed_abilities(object).iter().enumerate() {
            let Ok(index) = u32::try_from(index) else {
                continue;
            };
            let Some(source) = owned_mana_ability(view, object.id, index, ability) else {
                continue;
            };
            for color in source.colors {
                if let Some(slot) = colors.get_mut(color as usize) {
                    *slot = true;
                }
            }
        }

        let made = colors.iter().filter(|on| **on).count();
        if made == 0 {
            continue;
        }
        #[allow(clippy::cast_precision_loss)] // five at the most
        let share = 1.0 / made as f32;
        for (slot, on) in tally.iter_mut().zip(colors) {
            if on {
                *slot += share;
            }
        }
    }
    tally
}
