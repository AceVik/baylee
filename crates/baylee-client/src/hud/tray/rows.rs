//! The rows: every card in the ticked zones as a line, a big line or a
//! tile.

#[allow(clippy::wildcard_imports)] // the tray's own vocabulary
use super::*;

/// The rows in the shape the view asks for: a line each, a big line each,
/// or tiles.
#[allow(clippy::too_many_arguments)] // the rows, the shape, and the stores
pub(super) fn spawn_rows(
    commands: &mut Commands,
    list: Entity,
    lang: Lang,
    rows: &[BrowseRow],
    mode: ViewMode,
    grid: GridCtx<'_>,
    faces: &FaceCtx<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    cards: &mut Option<&mut UiCards<'_>>,
) {
    match mode {
        ViewMode::Detailed | ViewMode::Large => {
            for row in rows {
                let node = if mode == ViewMode::Large {
                    spawn_big_row(
                        commands,
                        lang,
                        row,
                        grid.view,
                        grid.statics,
                        textures,
                        assets,
                        grid.fonts,
                        faces,
                        cards,
                        reach_in(grid.lit, row.id),
                    )
                } else {
                    spawn_row(
                        commands,
                        lang,
                        row,
                        grid.view,
                        grid.statics,
                        textures,
                        assets,
                        grid.fonts,
                        faces,
                        cards,
                        reach_in(grid.lit, row.id),
                    )
                };
                commands.entity(list).add_child(node);
            }
        }
        ViewMode::Grid => spawn_grid(commands, list, lang, rows, grid, textures, assets, cards),
    }
}

/// One row of the list: a checkbox, a thumbnail, the name, the cost in pips,
/// the type line and the pile it is in.
#[allow(clippy::too_many_arguments)] // the row, the view, and the stores
fn spawn_row(
    commands: &mut Commands,
    lang: Lang,
    row: &BrowseRow,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    cards: &mut Option<&mut UiCards<'_>>,
    reach: Option<crate::Reach>,
) -> Entity {
    let slot = row_slot(commands, row, TRAY_ROW_H, TRAY_ROW_PAD);

    let mark = spawn_mark(commands, fonts, row, TRAY_BOX);
    commands.entity(slot).add_child(mark);

    let thumb = spawn_thumb(
        commands,
        lang,
        row,
        statics,
        textures,
        assets,
        fonts,
        cards,
        TRAY_THUMB_W,
    );
    light_thumb(commands, thumb, reach);
    commands.entity(slot).add_child(thumb);

    // ---- the name ----
    //
    // The one thing on the row that grows, with `flex_basis: 0` beside it:
    // grow alone divides only the slack left after every fixed column, which
    // on a narrow sheet is nothing at all.
    let name = dialog_text(
        commands,
        fonts,
        &row.name,
        TRAY_NAME_SIZE,
        palette::DIALOG_INK,
    );
    let name = clipped(
        commands,
        name,
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            ..default()
        },
    );
    commands.entity(slot).add_child(name);

    // ---- the cost, drawn and not spelled ----
    //
    // The face is what carries it: a `BrowseRow` has the projected mana
    // *value*, which is what the sort key reads, and a number is not a price.
    let pips = spawn_cost(commands, fonts, row, view, faces);
    commands.entity(slot).add_child(pips);

    // ---- the type line ----
    let built = view.object(row.id).map(|o| faces.facts(o)).or_else(|| {
        baylee_client_core::decision::known_cards(view)
            .find(|h| h.id == row.id)
            .map(|h| crate::face::of_hand(h, faces.texts))
    });
    let types = built
        .as_ref()
        .map_or_else(String::new, |f| f.type_line.clone());
    let type_line = dialog_text(
        commands,
        fonts,
        &types,
        TRAY_TYPE_SIZE,
        palette::DIALOG_SOFT,
    );
    let type_line = clipped(
        commands,
        type_line,
        Node {
            width: px(TRAY_TYPE_W),
            flex_shrink: 0.0,
            ..default()
        },
    );
    commands.entity(slot).add_child(type_line);

    let badge = spawn_badge(commands, lang, fonts, row);
    commands.entity(slot).add_child(badge);

    slot
}

/// Every card in the ticked zones, as tiles.
///
/// The owner's *"einfach nur alle Karten in der Zone wie auf einer
/// Produktseite"*. The rows are the same rows the lists draw — same filter,
/// same sort, same order — and the only thing that changes is that they are
/// laid across and wrapped instead of down.
///
/// The runs keep the sort's order rather than being regrouped by zone, which
/// is not a choice: [`BrowseZone`]'s `Ord` **is** the tab order and
/// `Browser::rows` already emits zone by zone in it, so walking the rows in
/// order and starting a new run whenever the zone changes gives exactly the
/// grouping the tabs promise. A grid that sorted itself again would be a
/// second opinion about an order that already has one.
#[allow(clippy::too_many_arguments)] // the rows, the measure, and the stores
fn spawn_grid(
    commands: &mut Commands,
    list: Entity,
    lang: Lang,
    rows: &[BrowseRow],
    ctx: GridCtx<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    cards: &mut Option<&mut UiCards<'_>>,
) {
    // Packed as whole tiles and drawn as pictures: what is shared out is the
    // node the wrap sees, and [`TRAY_TILE_CHROME`] is the difference.
    let (_, tile, air) = grid_across(
        ctx.measure,
        TRAY_BIG_THUMB_W + TRAY_TILE_CHROME,
        TRAY_TILE_MAX + TRAY_TILE_CHROME,
        TRAY_TILE_GAP,
    );
    let art = tile - TRAY_TILE_CHROME;
    let mut run: Option<(BrowseZone, Entity)> = None;
    for row in rows {
        let open = match run {
            Some((zone, node)) if zone == row.zone => node,
            _ => {
                if ctx.headed {
                    let head = spawn_run_head(commands, lang, row.zone, &ctx);
                    commands.entity(list).add_child(head);
                }
                let node = commands
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: px(air),
                            row_gap: px(TRAY_TILE_GAP),
                            padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_TILE_GAP)),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(list).add_child(node);
                run = Some((row.zone, node));
                node
            }
        };
        let node = spawn_tile(commands, lang, row, &ctx, textures, assets, cards, art);
        commands.entity(open).add_child(node);
    }
}

/// The line that starts one pile's run of tiles.
///
/// The pile's name and then a rule to the right edge, which is the cheapest
/// thing that reads as a heading in a panel whose only other horizontal line
/// is the rule under a row. It is drawn only when more than one pile is
/// showing: with one tab ticked the tabs above have already said which, and a
/// heading repeating the tab would be chrome on every grid the panel draws.
fn spawn_run_head(
    commands: &mut Commands,
    lang: Lang,
    zone: BrowseZone,
    ctx: &GridCtx<'_>,
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
    let words = dialog_label(
        commands,
        ctx.fonts,
        &zone_label(
            lang,
            zone,
            ctx.view,
            ctx.statics,
            ctx.browser,
            ctx.interaction,
        ),
        10.5,
        palette::DIALOG_SOFT,
    );
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

/// One card in the grid.
///
/// Nothing is written on it and nothing under it. A name under a 73-pixel
/// column clips on most of the pool, and a clipped name is worse than no name
/// — the hover preview already says which card this is, in full, with its
/// text. What the tile has to carry instead is the two things a picture
/// cannot say for itself: whether it is chosen, and where the keyboard is.
///
/// Those two are the same colour at two geometries, which is deliberate.
/// Chosen is the tile's **own** border going candle, tight against the art;
/// focus is a rail standing outside it. So a tile that is both reads as two
/// concentric rings and neither can be mistaken for the other — where a wash
/// over the art, which is what the list uses, would dim the one thing the
/// tile is *for*.
///
/// The ordering number is the list's own candle disc, moved into the tile's
/// top-left corner over the art. Carrying the same mark between the views is
/// what lets a player change view in the middle of an ordering without having
/// to learn it again.
#[allow(clippy::too_many_arguments)] // the row, the context, and the stores
fn spawn_tile(
    commands: &mut Commands,
    lang: Lang,
    row: &BrowseRow,
    ctx: &GridCtx<'_>,
    textures: &mut CardTextures,
    assets: &AssetServer,
    cards: &mut Option<&mut UiCards<'_>>,
    width: f32,
) -> Entity {
    // The focus rail is a border the tile always reserves and only the
    // focused one paints, for the reason a row reserves its own: a ring that
    // appeared would move the tile, and a grid that stepped about as the
    // focus crossed it is worse to read than a grid with no focus at all.
    let tile = commands
        .spawn((
            TrayCard { object: row.id },
            UiButton,
            Node {
                width: px(width + TRAY_TILE_CHROME),
                padding: UiRect::all(px(2.0)),
                border: UiRect::all(px(TRAY_FOCUS)),
                // The card's own corner plus what stands outside it, so the
                // focus ring is concentric with the picture rather than
                // squarer than it — the same arithmetic the panel's head
                // does against the sheet.
                border_radius: BorderRadius::all(px(width * 0.0476 + TRAY_TILE_CHROME / 2.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            // Nothing, chosen or not — and that is the one place this tile
            // parts company with a row. A row says chosen with a wash across
            // its whole line; a tile's whole line *is* the picture, so the
            // wash would dim the one thing it is for. The candle edge below
            // carries the claim instead. Written as the plain rest colour
            // because `Feel` owns this field from its first tick: a wash set
            // here and not named in the `Feel` is a wash nobody ever sees.
            BackgroundColor(Color::NONE),
            BorderColor::all(if row.standing.focused {
                palette::CANDLE_EDGE
            } else {
                Color::NONE
            }),
            // Both ends stated, because `Feel::new` shades towards white and
            // keeps the alpha: a tile resting at nothing would be lifted to a
            // brighter nothing and never answer the pointer.
            //
            // Lifted, where a row is not. A row is a line of writing and
            // growing it grows the sentence; a tile is a picture, and the
            // pointer picking it up a little is what a picture answers with.
            // The gap is what it grows into — a `UiTransform` scale moves
            // nothing else on the row it is in.
            Feel::rising_to(Color::NONE, palette::DIALOG_LIT),
        ))
        .id();

    let art = spawn_thumb(
        commands,
        lang,
        row,
        ctx.statics,
        textures,
        assets,
        ctx.fonts,
        cards,
        width,
    );
    light_thumb(commands, art, reach_in(ctx.lit, row.id));
    // Chosen is the art's own edge, which is why it is inserted here rather
    // than being a property of the tile: a second frame around the picture
    // would read as a card in a holder.
    if row.standing.selected {
        commands
            .entity(art)
            .insert(Outline::new(px(2), px(0), palette::CANDLE));
    }
    commands.entity(tile).add_child(art);

    // The ordering number, over the art's top-left corner. Nothing is drawn
    // for a plain tick: the candle edge above has already said it, and a box
    // in the corner of a picture is a box over a picture.
    if row.place.is_some() {
        let disc = spawn_mark(commands, ctx.fonts, row, 20.0);
        commands.entity(disc).insert(Node {
            position_type: PositionType::Absolute,
            left: px(4),
            top: px(4),
            width: px(20),
            height: px(20),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(10)),
            ..default()
        });
        commands.entity(tile).add_child(disc);
    }
    tile
}

/// The card's price, drawn and not spelled.
///
/// The face is what carries it: a [`BrowseRow`] has the projected mana
/// *value*, which is what the sort key reads, and a number is not a price.
fn spawn_cost(
    commands: &mut Commands,
    fonts: &UiFonts,
    row: &BrowseRow,
    view: &PlayerView,
    faces: &FaceCtx<'_>,
) -> Entity {
    let pips = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(2),
                flex_shrink: 0.0,
                // A reserve rather than a fit, so the type lines beside them
                // start in one column down the whole list. A cost longer than
                // four pips takes the room it needs and pushes the name in,
                // which is the right way round: the name has the slack.
                min_width: px(TRAY_COST_W),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let built = view.object(row.id).map(|o| faces.facts(o)).or_else(|| {
        baylee_client_core::decision::known_cards(view)
            .find(|h| h.id == row.id)
            .map(|h| crate::face::of_hand(h, faces.texts))
    });
    for symbol in built.iter().flat_map(|face| face.cost.iter()) {
        let pip = crate::manaui::spawn_pip(
            commands,
            fonts,
            baylee_client_core::manapip::pip(*symbol),
            TRAY_PIP,
        );
        commands.entity(pips).add_child(pip);
    }
    pips
}

/// Which pile the card is in, as a bordered word.
///
/// The bare zone word, with no seat on it: at a table of four the tabs above
/// already say whose pile is being looked through, and a seat name in a badge
/// this size is a smear. A token says so here instead — a graveyard holds
/// cards and tokens together and they are not the same thing, since a token
/// ceases to exist the next time state-based actions are checked (CR 111.7),
/// so a row that looked like a card would invite a player to plan around
/// something already gone.
///
/// Both lists draw it, and the large one draws it on a line of its own: a
/// merged list is the whole reason the badge exists, and dropping it there
/// would leave two Kommandozonen ticked and nothing on a row saying which one
/// a card came out of.
fn spawn_badge(commands: &mut Commands, lang: Lang, fonts: &UiFonts, row: &BrowseRow) -> Entity {
    let words = if row.token {
        Phrase::IsToken.text(lang).to_string()
    } else {
        row.zone.label().text(lang).to_string()
    };
    let text = dialog_text(
        commands,
        fonts,
        &words,
        TRAY_BADGE_SIZE,
        palette::DIALOG_SOFT,
    );
    let badge = commands
        .spawn((
            Node {
                width: px(TRAY_BADGE_W),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(px(5)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(text);
    badge
}

/// One row of the **large** list: the box, a picture at the one size the art
/// actually has, and beside them the name over the pile it came from.
///
/// It is the detailed row with the type line taken out and the picture
/// doubled, and the type line is the right thing to lose: it was the widest
/// fixed column on the row, and at 73 pixels the frame's colour and a
/// creature's silhouette are legible off the art itself. The name goes up one
/// size and the cost stays beside it, because a name and a price are what a
/// player is scanning for; the badge takes the second line, which is where the
/// room the type line gave up goes.
///
/// The box grows with the row rather than staying at 15: this is the view a
/// player picks when they are *choosing*, and a tick target that shrank
/// relative to everything around it would be the control going the wrong way.
#[allow(clippy::too_many_arguments)] // the row, the view, and the stores
fn spawn_big_row(
    commands: &mut Commands,
    lang: Lang,
    row: &BrowseRow,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    cards: &mut Option<&mut UiCards<'_>>,
    reach: Option<crate::Reach>,
) -> Entity {
    let slot = row_slot(commands, row, TRAY_BIG_ROW_H, TRAY_BIG_ROW_PAD);

    let mark = spawn_mark(commands, fonts, row, TRAY_BIG_BOX);
    commands.entity(slot).add_child(mark);

    let thumb = spawn_thumb(
        commands,
        lang,
        row,
        statics,
        textures,
        assets,
        fonts,
        cards,
        TRAY_BIG_THUMB_W,
    );
    light_thumb(commands, thumb, reach);
    commands.entity(slot).add_child(thumb);

    // The two lines beside the picture. The column grows and its children are
    // the ones allowed to clip, so a long name shortens instead of pushing
    // the cost off the row.
    let words = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: px(6),
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let title = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(TRAY_GAP),
                width: percent(100),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let name = dialog_text(
        commands,
        fonts,
        &row.name,
        TRAY_NAME_SIZE + 1.5,
        palette::DIALOG_INK,
    );
    let name = clipped(
        commands,
        name,
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            ..default()
        },
    );
    let pips = spawn_cost(commands, fonts, row, view, faces);
    commands.entity(title).add_children(&[name, pips]);

    let badge = spawn_badge(commands, lang, fonts, row);
    commands.entity(words).add_children(&[title, badge]);
    commands.entity(slot).add_child(words);

    slot
}

/// What a zone tab reads. A pile belonging to a seat says whose it is,
/// because at a table of four "Graveyard" alone names nothing — and every tab
/// says how many cards are in it, which is the one thing the deleted pile
/// chips carried that nothing else on the sheet does.
///
/// The count goes in brackets rather than after a separator because the
/// dialog's typography already means something by a bracket: [`dialog_text`]
/// hands a bracketed run to [`palette::DIALOG_SOFT`], so "Graveyard (12)" is
/// drawn as a name with a grey aside beside it and reads as one.
pub(super) fn zone_label(
    lang: Lang,
    zone: BrowseZone,
    view: &PlayerView,
    statics: &GameStatic,
    browser: &Browser,
    interaction: Option<&baylee_client_core::Interaction>,
) -> String {
    let name = zone.label().text(lang).to_string();
    let named = match zone.seat() {
        None => name,
        Some(seat) if seat == view.seat => name,
        Some(seat) => {
            let who = seat_name(lang, Some(statics), seat);
            Phrase::BrowseZoneOf.fill(lang, &[&name, &who])
        }
    };
    let count = browser.tab_count(zone, view, interaction);
    Phrase::BrowseTabCount.fill(lang, &[&named, &count.to_string()])
}

/// The row itself: the thing a click lands on, before anything is written in
/// it.
///
/// Both lists build one of these, which is what keeps them one list in two
/// sizes rather than two lists — the picking, the chosen wash, the focus rail
/// and the hover behaviour are stated once here and neither view may differ
/// about them.
///
/// Candle, and a wash of it rather than a fill: a chosen row is still a row
/// being read. The tick and the ink carry the claim.
///
/// The hot end is stated both ways round, because `Feel`'s own hover keeps a
/// colour's alpha (see [`Feel::hot`]): an unchosen row rests at nothing and
/// would be lifted to a brighter nothing, and a chosen one rests at a tenth
/// and would be lifted to a paler tenth. A hundred rows that did not answer
/// the pointer is the whole list not answering it.
fn row_slot(commands: &mut Commands, row: &BrowseRow, height: f32, pad: f32) -> Entity {
    let (fill, hot) = if row.standing.selected {
        (palette::CANDLE_WASH, palette::CANDLE_WASH_LIT)
    } else {
        (Color::NONE, palette::DIALOG_LIT)
    };
    commands
        .spawn((
            TrayCard { object: row.id },
            UiButton,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(TRAY_GAP),
                width: percent(100),
                // Explicit rather than whatever the thumbnail happens to make
                // it: the sheet's whole height is counted in rows, so a row
                // whose height was an accident of its tallest child would put
                // that arithmetic one text metric away from being wrong.
                height: px(height),
                padding: UiRect::axes(px(TRAY_SIDE), px(pad)),
                // Every row carries the focus rail's width, and only the
                // focused one carries its colour: a border that appeared
                // would shift that row's whole content sideways, and a list
                // whose rows step in and out as the focus passes is worse to
                // read than no focus at all.
                border: UiRect {
                    left: px(TRAY_FOCUS),
                    bottom: px(1),
                    ..default()
                },
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor {
                left: if row.standing.focused {
                    palette::CANDLE_EDGE
                } else {
                    Color::NONE
                },
                bottom: palette::DIALOG_LINE,
                ..BorderColor::all(Color::NONE)
            },
            // Lit, never lifted. A row is a *line of writing* — a tick, a
            // picture, a name, the pips, a type line — and `Feel::lift`'s own
            // doc names that as the case that must stay at zero: growing a
            // row grows the sentence on it. A hundred of them under a moving
            // pointer is a list that reflows while it is being read, which is
            // half of what the owner reported as the dialog flickering.
            Feel::tinting_to(fill, hot),
        ))
        .id()
}

/// The box a row is ticked in, or the number saying where it stands in an
/// ordering.
///
/// A number rather than a tick for an ordering, because "third" is not a
/// brighter kind of "chosen" — and it stands in the box's own place, so a
/// list of ordered cards reads down the same gutter a list of ticked ones
/// does. A row the question will not take leaves it empty rather than drawing
/// a box that cannot be ticked.
///
/// Four states decided before the node is spawned rather than patched into it
/// afterwards: a radius is a field of `Node` and not a component of its own,
/// so "insert a rounder corner" would mean writing the whole `Node` back over
/// itself.
///
/// `size` is the one thing the three views differ about, and the glyph inside
/// is scaled from it rather than given: a mark that kept a nine-point tick
/// while its box grew to twenty would be a tick rattling around in a box.
fn spawn_mark(commands: &mut Commands, fonts: &UiFonts, row: &BrowseRow, size: f32) -> Entity {
    let (fill, edge, radius, glyph_in) = if row.place.is_some() {
        (
            palette::CANDLE,
            palette::CANDLE,
            size / 2.0,
            row.place.map(|place| place.to_string()),
        )
    } else if row.standing.selected {
        (
            palette::CANDLE,
            palette::CANDLE,
            3.0,
            Some(glyph::CHECK.to_string()),
        )
    } else if row.standing.selectable {
        (Color::NONE, palette::DIALOG_SOFT, 3.0, None)
    } else {
        (Color::NONE, Color::NONE, 3.0, None)
    };
    let mark = commands
        .spawn((
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(radius)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(inside) = glyph_in {
        // The ordering's number is set in the text face and the tick in the
        // icon one, because a tick is a glyph the text face does not have.
        // Checked again on the change from Inter: Alegreya Sans has no
        // U+2713 either.
        let face = if row.place.is_some() {
            tf(fonts, size * 9.5 / TRAY_BOX)
        } else {
            icon_tf(fonts, size * 9.0 / TRAY_BOX)
        };
        let ink = commands
            .spawn((
                Text::new(inside),
                face,
                TextColor(palette::DIALOG),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(mark).add_child(ink);
    }
    mark
}

/// The card's picture, at whatever size the view draws it.
///
/// `built` is deliberately not passed to it: a built face draws the name, the
/// cost and the type line onto the card, and at forty pixels wide all three
/// would be a grey smear. A list says those three things beside it, in
/// letters a person can read; a grid says them in the hover preview.
///
/// Every view asks for the same [`ImageKey`] — the one the row already
/// carries — so making the picture bigger costs no texture at all. See
/// [`TRAY_BIG_THUMB_W`] for what bounds the size.
///
/// [`ImageKey`]: baylee_client_core::images::ImageKey
#[allow(clippy::too_many_arguments)] // a row, the stores, and one number
fn spawn_thumb(
    commands: &mut Commands,
    lang: Lang,
    row: &BrowseRow,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    cards: &mut Option<&mut UiCards<'_>>,
    width: f32,
) -> Entity {
    let height = width * 88.0 / 63.0;
    let thumb = if let Some(key) = row.art {
        let image = textures.get(key, statics, assets);
        spawn_card_art(
            commands,
            lang,
            image,
            None,
            width,
            height,
            crate::face::Detail::Compact,
            fonts,
            // No keyword sheath: nothing in the browser is on a battlefield,
            // and a card in a graveyard wearing an indestructible border
            // would be claiming something the rules do not say.
            CardLook::art(key, finish_of(statics, Some(key))),
            cards.as_deref_mut(),
            // No face is drawn here, so nothing is fitted.
            &crate::face::Widths::of(None),
        )
    } else {
        // A token in a graveyard, or a card this seat may not identify.
        commands
            .spawn((
                Node {
                    width: px(width),
                    height: px(height),
                    flex_shrink: 0.0,
                    border_radius: card_radius(width),
                    ..default()
                },
                BackgroundColor(palette::DIALOG_LIT),
            ))
            .id()
    };
    commands.entity(thumb).insert(Pickable::IGNORE);
    thumb
}
