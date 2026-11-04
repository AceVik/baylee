//! The banner over the table while one player makes another's decisions
//! (CR 723.5): "You decide for Mia" at the table of the player in control,
//! "Ben controls your turn" at the controlled player's, "Ben decides for
//! Mia" at everyone else's. Control of a player is public, so every seat is
//! told; what it says is `baylee_client_core::perspective::ControlLine`'s,
//! read off the question in hand, and this file only stands it on screen.
//!
//! **Built once, written on change.** One pill per duel under the lost
//! connection's banner, hidden by `Display`; [`show_control_banner`] writes
//! its words and shows it only when the line or the language changes. The
//! brain mark and the words carry it, the rose edge only repeats them.
//!
//! The source of the control (Mindslaver, Word of Command) is not named:
//! the view does not carry it (`docs/client.md` §"Playing another player's
//! turn").

use super::{UiFonts, glyph, icon_tf, palette, tf_bold};
use crate::Duel;
use crate::settings::ClientSettings;
use baylee_client_core::Lang;
use baylee_client_core::perspective::ControlLine;
use bevy::prelude::*;

/// The text's size.
const TEXT_PT: f32 = 15.0;
/// The pill's height.
const PILL_H: f32 = 34.0;
/// Below the lost banner's row (its `top` and pill) and a gap.
const TOP: f32 = 14.0 + 34.0 + 8.0;

/// The banner's full-width row near the top of the table.
#[derive(Component)]
pub struct ControlBannerRoot;

/// The pill inside it, shown while somebody decides for somebody else.
#[derive(Component)]
pub struct ControlBannerPill;

/// The pill's sentence.
#[derive(Component)]
pub struct ControlBannerText;

/// Builds the banner once a duel has a view; it lives as long as the
/// duel's stage.
pub fn spawn_control_banner(
    mut commands: Commands,
    duel: Res<Duel>,
    roots: Query<(), With<ControlBannerRoot>>,
    fonts: Res<UiFonts>,
) {
    if duel.view.is_none() || !roots.is_empty() {
        return;
    }
    commands.spawn((
        ControlBannerRoot,
        crate::table::DuelStage,
        Node {
            position_type: PositionType::Absolute,
            top: px(TOP),
            width: percent(100),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
        // Where the lost banner stands, and for its reason.
        GlobalZIndex(-1),
        children![(
            ControlBannerPill,
            Node {
                display: Display::None,
                height: px(PILL_H),
                padding: UiRect::horizontal(px(18)),
                column_gap: px(10),
                align_items: AlignItems::Center,
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(PILL_H * 0.5)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(palette::CONTROL),
            Pickable::IGNORE,
            children![
                (
                    Text::new(glyph::CONTROL.to_string()),
                    icon_tf(&fonts, TEXT_PT),
                    TextColor(palette::CONTROL),
                    Pickable::IGNORE,
                ),
                (
                    ControlBannerText,
                    Text::default(),
                    tf_bold(&fonts, TEXT_PT),
                    TextColor(palette::INK),
                    Pickable::IGNORE,
                )
            ],
        )],
    ));
}

/// Shows the banner while the view says one player decides for another,
/// and writes it only when what it says changes.
pub fn show_control_banner(
    duel: Res<Duel>,
    settings: Res<ClientSettings>,
    mut pill: Query<&mut Node, With<ControlBannerPill>>,
    mut text: Query<&mut Text, With<ControlBannerText>>,
    mut shown: Local<Option<(Option<ControlLine>, Lang)>>,
) {
    let line = duel.view.as_ref().and_then(ControlLine::of);
    let lang = Lang::of(&settings.lang);
    let (Ok(mut node), Ok(mut text)) = (pill.single_mut(), text.single_mut()) else {
        *shown = None;
        return;
    };
    if *shown == Some((line, lang)) {
        return;
    }
    *shown = Some((line, lang));
    if let Some(line) = line {
        text.0 = line.text(lang, duel.statics.as_ref());
        node.display = Display::Flex;
    } else {
        text.0.clear();
        node.display = Display::None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;

    /// Counts frames on which the banner's `Text` was written.
    #[derive(Resource, Default)]
    struct Writes(usize);

    fn count(
        texts: Query<(), (Changed<Text>, With<ControlBannerText>)>,
        mut writes: ResMut<Writes>,
    ) {
        writes.0 += texts.iter().count();
    }

    fn said(app: &mut App) -> (String, Display) {
        let world = app.world_mut();
        let text = world
            .query_filtered::<&Text, With<ControlBannerText>>()
            .single(world)
            .expect("one text")
            .0
            .clone();
        let shown = world
            .query_filtered::<&Node, With<ControlBannerPill>>()
            .single(world)
            .expect("one pill")
            .display;
        (text, shown)
    }

    /// The controller's table says whom it decides for, the controlled
    /// player's says who controls its turn, and the banner goes once each
    /// decides for itself again; a frame with nothing new writes nothing.
    #[test]
    fn the_banner_says_who_decides_for_whom_and_goes_after() {
        let mut app = App::new();
        let controlling = baylee_client_core::test_support::ViewBuilder::new(2)
            .controlling(1, vec![])
            .build();
        app.insert_resource(Duel {
            view: Some(controlling.clone()),
            ..Duel::default()
        })
        .insert_resource(ClientSettings::default())
        .init_resource::<Writes>()
        .add_systems(Update, (show_control_banner, count).chain());
        app.world_mut().spawn((
            ControlBannerPill,
            Node {
                display: Display::None,
                ..default()
            },
        ));
        app.world_mut().spawn((ControlBannerText, Text::default()));
        app.update();
        let (text, display) = said(&mut app);
        assert_eq!(display, Display::Flex);
        assert!(text.starts_with("You decide for"), "{text}");
        let writes = app.world().resource::<Writes>().0;
        app.update();
        assert_eq!(
            app.world().resource::<Writes>().0,
            writes,
            "a quiet frame wrote"
        );

        // The controlled player's own table, on its own turn.
        let mut theirs = controlling.clone();
        theirs.seat = PlayerId::new(1);
        theirs.controlled_hands.clear();
        app.world_mut().resource_mut::<Duel>().view = Some(theirs);
        app.update();
        let (text, _) = said(&mut app);
        assert!(text.ends_with("controls your turn"), "{text}");

        // My own decision in that turn (CR 723.8): no banner.
        let mut mine = controlling;
        mine.decision_player = Some(mine.seat);
        app.world_mut().resource_mut::<Duel>().view = Some(mine);
        app.update();
        assert_eq!(said(&mut app).1, Display::None);
    }
}
