//! The banner over the table while another player's connection is lost:
//! "Mia lost the connection – waiting 2:41", ticking down, then "The house
//! AI plays for Mia", gone when she is back (`docs/protocol.md`
//! §"Leaving, and losing the connection"). What it says is
//! `baylee_client_core::lostbanner`'s; this file only stands it on screen.
//!
//! **Built once, written once a second.** One pill per duel, hidden by
//! `Display`; [`tick_lost_banner`] counts [`Duel::lost_seats`] down every
//! frame past change detection and writes the `Text` only when the
//! [`Banner`] value (the case and the shown second) or the language
//! changes. No tree is rebuilt on a tick.

use super::{UiFonts, palette, tf_bold};
use crate::Duel;
use crate::settings::ClientSettings;
use baylee_client_core::Lang;
use baylee_client_core::lostbanner::Banner;
use bevy::prelude::*;

/// The text's size.
const TEXT_PT: f32 = 15.0;
/// The pill's height.
const PILL_H: f32 = 34.0;

/// The banner's full-width row at the top of the table.
#[derive(Component)]
pub struct LostBannerRoot;

/// The pill inside it, shown while there is something to say.
#[derive(Component)]
pub struct LostBannerPill;

/// The pill's sentence.
#[derive(Component)]
pub struct LostBannerText;

/// Builds the banner once a duel has a view; it lives as long as the duel's
/// stage.
pub fn spawn_lost_banner(
    mut commands: Commands,
    duel: Res<Duel>,
    roots: Query<(), With<LostBannerRoot>>,
    fonts: Res<UiFonts>,
) {
    if duel.view.is_none() || !roots.is_empty() {
        return;
    }
    commands.spawn((
        LostBannerRoot,
        crate::table::DuelStage,
        Node {
            position_type: PositionType::Absolute,
            top: px(14),
            width: percent(100),
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
        // Over the felt and the plates, under everything summoned over the
        // table (the seat bars' `GlobalZIndex(-1)` and its reason).
        GlobalZIndex(-1),
        children![(
            LostBannerPill,
            Node {
                display: Display::None,
                height: px(PILL_H),
                padding: UiRect::horizontal(px(18)),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(PILL_H * 0.5)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            BorderColor::all(palette::ACTIVE.with_alpha(0.55)),
            Pickable::IGNORE,
            children![(
                LostBannerText,
                Text::default(),
                tf_bold(&fonts, TEXT_PT),
                TextColor(palette::INK),
                Pickable::IGNORE,
            )],
        )],
    ));
}

/// Counts the lost seats down and writes the banner when what it says
/// changes: at most once a second while a wait runs, never while it rests.
pub fn tick_lost_banner(
    time: Res<Time>,
    mut duel: ResMut<Duel>,
    settings: Res<ClientSettings>,
    mut pill: Query<&mut Node, With<LostBannerPill>>,
    mut text: Query<&mut Text, With<LostBannerText>>,
    mut shown: Local<Option<(Option<Banner>, Lang)>>,
) {
    // Past change detection: a wait counting down is not the duel changing.
    duel.bypass_change_detection()
        .lost_seats
        .advance(time.delta_secs());
    let me = duel.view.as_ref().map(|v| v.seat);
    let banner = me.and_then(|me| duel.lost_seats.banner(duel.statics.as_ref(), me));
    let lang = Lang::of(&settings.lang);
    let (Ok(mut node), Ok(mut text)) = (pill.single_mut(), text.single_mut()) else {
        *shown = None;
        return;
    };
    if *shown == Some((banner, lang)) {
        return;
    }
    *shown = Some((banner, lang));
    if let Some(banner) = banner {
        text.0 = banner.line(lang, duel.statics.as_ref());
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

    fn count(texts: Query<(), (Changed<Text>, With<LostBannerText>)>, mut writes: ResMut<Writes>) {
        writes.0 += texts.iter().count();
    }

    /// The pill ticks once a second — a frame inside a second writes
    /// nothing — turns to the house once the wait is out, and goes away
    /// when the player is back.
    #[test]
    fn the_banner_is_written_once_a_second_and_goes_when_they_are_back() {
        let mut app = App::new();
        let mut statics = baylee_client_core::test_support::statics(0);
        statics.seats.push(baylee_view::SeatIdentity {
            player: PlayerId::new(1),
            display_name: "Mia".to_owned(),
            is_ai: false,
            away: false,
            team: None,
        });
        let mut duel = Duel {
            statics: Some(statics),
            view: Some(baylee_client_core::test_support::ViewBuilder::new(2).build()),
            ..Duel::default()
        };
        duel.lost_seats.sync(&[baylee_view::LostSeat {
            seat: PlayerId::new(1),
            remaining_ms: Some(3_000),
        }]);
        app.insert_resource(duel)
            .insert_resource(ClientSettings::default())
            .insert_resource(Time::<()>::default())
            .init_resource::<Writes>()
            .add_systems(Update, (tick_lost_banner, count).chain());
        app.world_mut().spawn((
            LostBannerPill,
            Node {
                display: Display::None,
                ..default()
            },
        ));
        app.world_mut().spawn((LostBannerText, Text::default()));
        let tick = |app: &mut App, secs: f32| {
            app.world_mut()
                .resource_mut::<Time<()>>()
                .advance_by(std::time::Duration::from_secs_f32(secs));
            app.update();
        };
        let said = |app: &mut App| {
            let world = app.world_mut();
            let text = world
                .query_filtered::<&Text, With<LostBannerText>>()
                .single(world)
                .expect("one text")
                .0
                .clone();
            let shown = world
                .query_filtered::<&Node, With<LostBannerPill>>()
                .single(world)
                .expect("one pill")
                .display;
            (text, shown)
        };
        tick(&mut app, 0.0);
        let (text, shown) = said(&mut app);
        assert!(text.contains("0:03"), "{text}");
        assert_eq!(shown, Display::Flex);
        let writes = app.world().resource::<Writes>().0;
        tick(&mut app, 0.4);
        tick(&mut app, 0.4);
        assert_eq!(
            app.world().resource::<Writes>().0,
            writes,
            "a second that did not turn was written"
        );
        tick(&mut app, 0.3);
        assert!(said(&mut app).0.contains("0:02"));
        tick(&mut app, 2.5);
        let (text, _) = said(&mut app);
        assert!(text.contains("Mia") && !text.contains("0:0"), "{text}");
        app.world_mut().resource_mut::<Duel>().lost_seats.sync(&[]);
        tick(&mut app, 0.0);
        assert_eq!(said(&mut app).1, Display::None, "back: gone");
    }
}
