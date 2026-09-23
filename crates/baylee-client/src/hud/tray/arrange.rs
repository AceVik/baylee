//! The zone dialog's body while a `Pending::Arrange` is being answered.
//!
//! One run per row of the arrangement, top to bottom in the order the
//! question lists its piles: a heading saying where those cards are going
//! and how many are there, the cards in the view's own shape, and then the
//! pile's **end** — a strip that takes the held card with one tap. The end is
//! what the tap-then-place gesture had no way to say without it: a tap on a
//! card puts the held one *in front of* it, so the last place in a pile, and
//! any place at all in an empty one, was reachable only from the keyboard.
//!
//! The model is `baylee_client_core::arrange::Arrangement` and every decision
//! is made there — which rows exist, what each is called, whether its end
//! would take the card. This file draws what it says.

use super::{
    BrowseRow, CardTextures, FaceCtx, GridCtx, TRAY_GAP, TRAY_ROW_PAD, TRAY_SIDE, TRAY_TILE_GAP,
    UiCards, UiFonts, ViewMode, dialog_label, dialog_text, spawn_rows,
};
use crate::ambience::Feel;
use crate::hud::{ArrangeSlot, btn_radius, palette};
use baylee_client_core::arrange::{Arrangement, Row};
use baylee_client_core::i18n::{Lang, Phrase};
use bevy::prelude::*;

/// How tall a pile's end stands: a finger's width, and the same whatever the
/// view, because it is a place to tap and not a card.
const END_H: f32 = 30.0;

/// Every row of the arrangement, each under its own heading and over its own
/// end. Rows the arrangement does not hold — a card revealed beside the
/// question — follow unheaded, as the sheet draws them anywhere else.
#[allow(clippy::too_many_arguments)] // the rows, the shape, and the stores
pub(super) fn spawn_piles(
    commands: &mut Commands,
    list: Entity,
    lang: Lang,
    arrangement: &Arrangement,
    rows: &[BrowseRow],
    mode: ViewMode,
    grid: GridCtx<'_>,
    faces: &FaceCtx<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    cards: &mut Option<&mut UiCards<'_>>,
) {
    // `Browser::rows` lists an arrangement row by row, so each pile is one
    // unbroken run of the list and the loose rows are its tail.
    let mut from = 0;
    for row in arrangement.rows() {
        let run = rows[from..]
            .iter()
            .take_while(|r| r.pile == Some(row))
            .count();
        let head = spawn_pile_head(commands, lang, grid.fonts, arrangement, row);
        commands.entity(list).add_child(head);
        spawn_rows(
            commands,
            list,
            lang,
            &rows[from..from + run],
            mode,
            grid,
            faces,
            textures,
            assets,
            cards,
        );
        if let Some(end) = spawn_pile_end(commands, lang, grid.fonts, arrangement, row) {
            commands.entity(list).add_child(end);
        }
        from += run;
    }
    spawn_rows(
        commands,
        list,
        lang,
        &rows[from..],
        mode,
        grid,
        faces,
        textures,
        assets,
        cards,
    );
}

/// Where a pile's cards are going, how many are in it, and a rule to the
/// edge — the grid's run heading, carrying a count because a pile's size is
/// part of the answer.
fn spawn_pile_head(
    commands: &mut Commands,
    lang: Lang,
    fonts: &UiFonts,
    arrangement: &Arrangement,
    row: Row,
) -> Entity {
    let head = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(TRAY_GAP),
                padding: UiRect::new(px(TRAY_SIDE), px(TRAY_SIDE), px(TRAY_TILE_GAP), px(0)),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let count = arrangement.cards(row).len().to_string();
    let words = Phrase::BrowseTabCount.fill(lang, &[arrangement.label(row).text(lang), &count]);
    let words = dialog_label(commands, fonts, &words, 10.5, palette::DIALOG_SOFT);
    let rule = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                height: px(1),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head).add_children(&[words, rule]);
    head
}

/// The end of a pile: a button while a card is held that could go there, a
/// quiet "empty" while nothing is held and nothing is in it, and nothing at
/// all otherwise — a pile with cards in it already shows where it ends.
fn spawn_pile_end(
    commands: &mut Commands,
    lang: Lang,
    fonts: &UiFonts,
    arrangement: &Arrangement,
    row: Row,
) -> Option<Entity> {
    let live = arrangement.can_place(row);
    if !live && !arrangement.cards(row).is_empty() {
        return None;
    }
    let node = Node {
        height: px(END_H),
        margin: UiRect::axes(px(TRAY_SIDE), px(TRAY_ROW_PAD)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        flex_shrink: 0.0,
        border: UiRect::all(px(1)),
        border_radius: btn_radius(),
        ..default()
    };
    let end = if live {
        // Lit the way the sheet's other controls are: a wash that rises
        // under the pointer, and the candle for the line round it, which is
        // the colour this dialog gives whatever the held card can go to.
        commands
            .spawn((
                ArrangeSlot(row),
                Button,
                node,
                BackgroundColor(palette::CANDLE_WASH),
                BorderColor::all(palette::CANDLE),
                Feel::tinting_to(palette::CANDLE_WASH, palette::CANDLE_WASH_LIT),
            ))
            .id()
    } else {
        commands
            .spawn((
                node,
                BorderColor::all(palette::DIALOG_LINE),
                Pickable::IGNORE,
            ))
            .id()
    };
    let (words, ink) = if live {
        (Phrase::ArrangePutHere, palette::CANDLE)
    } else {
        (Phrase::ArrangeEmptyPile, palette::DIALOG_SOFT)
    };
    let words = dialog_text(commands, fonts, words.text(lang), 10.5, ink);
    commands.entity(end).add_child(words);
    Some(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::ObjectId;
    use baylee_engine::choice::{ArrangePile, ArrangePlace};

    fn obj(n: u32) -> ObjectId {
        ObjectId::new(n, 0)
    }

    /// A scry of two, every card still on top.
    fn scry() -> Arrangement {
        Arrangement::new(
            &[obj(1), obj(2)],
            &[
                ArrangePile::up_to(ArrangePlace::LibraryTop, 2),
                ArrangePile::up_to(ArrangePlace::LibraryBottom, 2),
            ],
        )
    }

    /// Builds one pile's end and reports `(drawn, a tap on it places)`.
    fn end_of(arrangement: &Arrangement, row: Row) -> (bool, bool) {
        let mut app = App::new();
        let fonts = UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            bold: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        };
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let end = {
            let mut commands = Commands::new(&mut queue, app.world());
            spawn_pile_end(&mut commands, Lang::En, &fonts, arrangement, row)
        };
        queue.apply(app.world_mut());
        let places = end.is_some_and(|e| {
            app.world()
                .entity(e)
                .get::<ArrangeSlot>()
                .is_some_and(|slot| slot.0 == row)
        });
        (end.is_some(), places)
    }

    /// The end is a control exactly while the model would take the held
    /// card there, a quiet "empty" on a pile with nothing in it, and not
    /// drawn at all under a pile whose cards already show where it ends.
    #[test]
    fn a_piles_end_is_a_control_only_while_it_would_take_the_held_card() {
        let mut a = scry();
        assert_eq!(end_of(&a, Row::Pile(0)), (false, false), "full and idle");
        assert_eq!(
            end_of(&a, Row::Pile(1)),
            (true, false),
            "an empty pile is still shown, and is no control while nothing is held"
        );
        a.toggle(obj(1));
        assert_eq!(end_of(&a, Row::Pile(0)), (true, true), "its own pile's end");
        assert_eq!(end_of(&a, Row::Pile(1)), (true, true), "the empty pile");
    }
}
