//! A person's avatar: their picture, or the monogram that stands in for it.
//!
//! One builder for every place a person is drawn (the header's account
//! pill, the contacts list, a chat's head, the card that opens over a name),
//! so the same person wears the same disc everywhere. What the disc says
//! where there is no picture is `baylee_client_core::avatar`'s decision and
//! Baylee Central's rule: the same letters and the same tint as on the
//! website.
//!
//! # Where a picture comes from
//!
//! [`Portraits`] holds the pictures this client has, by account tag. Nothing
//! fills it from the network yet: Central's native listener does not answer
//! picture requests before its M6, so a client cannot learn which picture an
//! account wears. The seam is here so that what is drawn does not change
//! when it can: a picture laid in [`Portraits`] is drawn in every disc of
//! that account ([`wear_portraits`]), and a disc without one keeps its
//! monogram. A picture is outside input
//! (another player chose it); whatever fills this resource decodes it
//! bounded (`baylee_avif::Bounds::PICTURE`, 256 pixels square) before it
//! gets here.

use std::collections::HashMap;

use baylee_client_core::avatar::{self, RIM_ALPHA, Tint};
use bevy::prelude::*;

use crate::hud::{UiFonts, tf_bold};

/// The pictures this client holds, by account tag as a handle spells it
/// (`af03`).
#[derive(Resource, Default)]
pub struct Portraits {
    stills: HashMap<String, Handle<Image>>,
}

impl Portraits {
    /// The picture of the account a handle (`Alice#af03`) names, if this
    /// client has one.
    #[must_use]
    pub fn of(&self, handle: &str) -> Option<Handle<Image>> {
        let (_, tag) = handle.rsplit_once('#')?;
        self.stills.get(tag).cloned()
    }

    /// Lays in the picture of the account with this tag.
    pub fn set(&mut self, tag: impl Into<String>, picture: Handle<Image>) {
        self.stills.insert(tag.into(), picture);
    }

    /// Forgets the picture of the account with this tag: it shows its
    /// monogram again.
    pub fn forget(&mut self, tag: &str) {
        self.stills.remove(tag);
    }
}

/// An avatar's disc, and whose it is.
#[derive(Component, Clone, PartialEq, Debug)]
pub struct Avatar {
    /// The handle the disc was drawn for.
    pub handle: String,
    /// How wide the disc is, in logical pixels.
    pub size: f32,
    /// Drawn held back, for someone who is not there: a friend who is
    /// offline keeps their letters and their tint at half strength.
    pub quiet: bool,
}

/// A place a person is named: a row of a list, a pill, a line of a roster.
///
/// The pointer resting on it opens that person's card (`social::card`). The
/// kit only carries the mark; what a card says is the social machine's.
#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub struct PersonLink {
    /// Who: `Alice#af03`.
    pub handle: String,
    /// A guest, who has no account.
    pub guest: bool,
    /// A row of a list: the card stands beside it, never over the next row.
    pub row: bool,
}

impl PersonLink {
    /// A name where it stands in a sentence or a pill.
    #[must_use]
    pub fn named(handle: impl Into<String>, guest: bool) -> Self {
        Self {
            handle: handle.into(),
            guest,
            row: false,
        }
    }

    /// A person's row in a list.
    #[must_use]
    pub fn row(handle: impl Into<String>, guest: bool) -> Self {
        Self {
            handle: handle.into(),
            guest,
            row: true,
        }
    }
}

/// How much of a quiet disc is drawn.
const QUIET: f32 = 0.5;

/// What a disc shows: the person's picture, or their letters.
#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub enum AvatarFace {
    /// The monogram.
    Letters,
    /// This picture.
    Picture(Handle<Image>),
}

/// A colour of `baylee_client_core::avatar`'s, which are sRGB components.
fn colour(srgb: [f32; 3]) -> Color {
    Color::srgb(srgb[0], srgb[1], srgb[2])
}

/// The disc's ground, its letters' ink and its rim for a tint.
#[must_use]
pub fn palette(tint: Tint) -> (Color, Color, Color) {
    (
        colour(tint.ground()),
        colour(tint.ink()),
        colour(tint.rim()).with_alpha(RIM_ALPHA),
    )
}

/// The letters' nominal size in a disc `size` across: one letter stands
/// larger than two.
fn letter_size(letters: &str, size: f32) -> f32 {
    if letters.chars().count() > 1 {
        size * 0.31
    } else {
        size * 0.38
    }
}

/// What stands in a disc `size` across for `handle`: `picture` cut to the
/// disc, or the monogram.
fn face(
    commands: &mut Commands,
    fonts: &UiFonts,
    handle: &str,
    size: f32,
    quiet: bool,
    picture: Option<Handle<Image>>,
) -> Entity {
    let strength = if quiet { QUIET } else { 1.0 };
    if let Some(image) = picture {
        commands
            .spawn((
                AvatarFace::Picture(image.clone()),
                ImageNode::new(image).with_color(Color::WHITE.with_alpha(strength)),
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    border_radius: BorderRadius::all(px(size / 2.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id()
    } else {
        let mark = avatar::monogram(handle);
        let (_, ink, _) = palette(mark.tint);
        commands
            .spawn((
                AvatarFace::Letters,
                tf_bold(fonts, letter_size(&mark.letters, size)),
                Text::new(mark.letters),
                TextColor(ink.with_alpha(strength)),
                TextLayout::no_wrap(),
                Pickable::IGNORE,
            ))
            .id()
    }
}

/// An avatar `size` logical pixels across for the person `handle` names:
/// their monogram on their tint, until [`wear_portraits`] lays their picture
/// in, which it does before the frame is drawn. It takes no pointer: what it
/// stands in decides what a press on it does.
pub fn disc(commands: &mut Commands, fonts: &UiFonts, handle: &str, size: f32) -> Entity {
    disc_as(commands, fonts, handle, size, false)
}

/// [`disc`], held back (`quiet`) for someone who is not there.
pub fn disc_as(
    commands: &mut Commands,
    fonts: &UiFonts,
    handle: &str,
    size: f32,
    quiet: bool,
) -> Entity {
    let (ground, _, rim) = palette(avatar::monogram(handle).tint);
    let strength = if quiet { QUIET } else { 1.0 };
    let (ground, rim) = (
        ground.with_alpha(strength),
        rim.with_alpha(RIM_ALPHA * strength),
    );
    let disc = commands
        .spawn((
            Avatar {
                handle: handle.to_owned(),
                size,
                quiet,
            },
            Node {
                width: px(size),
                height: px(size),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(super::metrics::px_fixed(1.0)),
                border_radius: BorderRadius::all(px(size / 2.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(rim),
            Pickable::IGNORE,
        ))
        .id();
    let letters = face(commands, fonts, handle, size, quiet, None);
    commands.entity(disc).add_child(letters);
    disc
}

/// Gives every disc the face [`Portraits`] has for it: a disc drawn this
/// frame, and every disc when the pictures change. One system for every
/// place a person is drawn, so no builder has to know where pictures are
/// kept and no retained tree is rebuilt for one arriving. It writes nothing
/// while neither a disc nor a picture is new.
pub fn wear_portraits(
    mut commands: Commands,
    portraits: Res<Portraits>,
    fonts: Option<Res<UiFonts>>,
    discs: Query<(Entity, Ref<Avatar>, Option<&Children>)>,
    faces: Query<&AvatarFace>,
) {
    // No faces to set letters in yet: the discs keep what they were drawn
    // with, and are looked at again when the pictures change.
    let Some(fonts) = fonts else {
        return;
    };
    let all = portraits.is_changed();
    for (disc, avatar, children) in &discs {
        if !all && !avatar.is_added() {
            continue;
        }
        let want = portraits
            .of(&avatar.handle)
            .map_or(AvatarFace::Letters, AvatarFace::Picture);
        let worn: Vec<Entity> = children
            .into_iter()
            .flatten()
            .copied()
            .filter(|child| faces.contains(*child))
            .collect();
        if worn.len() == 1 && faces.get(worn[0]) == Ok(&want) {
            continue;
        }
        for old in worn {
            commands.entity(old).despawn();
        }
        let picture = match want {
            AvatarFace::Picture(image) => Some(image),
            AvatarFace::Letters => None,
        };
        let new = face(
            &mut commands,
            &fonts,
            &avatar.handle,
            avatar.size,
            avatar.quiet,
            picture,
        );
        commands.entity(disc).add_child(new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Portraits>()
            .init_resource::<UiFonts>()
            .add_systems(PostUpdate, wear_portraits);
        app
    }

    fn draw(app: &mut App, handle: &str) -> Entity {
        let fonts = app.world().resource::<UiFonts>().clone();
        let mut commands = app.world_mut().commands();
        let disc = disc(&mut commands, &fonts, handle, 28.0);
        app.world_mut().flush();
        disc
    }

    fn texts(app: &mut App) -> Vec<String> {
        let world = app.world_mut();
        let mut query = world.query::<&Text>();
        query.iter(world).map(|text| text.0.clone()).collect()
    }

    fn pictures(app: &mut App) -> usize {
        let world = app.world_mut();
        let mut query = world.query::<&ImageNode>();
        query.iter(world).count()
    }

    /// Without a picture the disc says the person's letters on their tint.
    #[test]
    fn a_disc_without_a_picture_wears_the_monogram() {
        let mut app = app();
        let plain = draw(&mut app, "Mia Wallace#00ff");
        app.update();
        assert_eq!(texts(&mut app), vec!["MW".to_string()]);
        let (ground, _, _) = palette(avatar::monogram("Mia Wallace#00ff").tint);
        let world = app.world();
        assert_eq!(
            world.get::<BackgroundColor>(plain).expect("a ground").0,
            ground
        );
        assert_eq!(
            world.get::<Avatar>(plain).expect("whose").handle,
            "Mia Wallace#00ff"
        );
    }

    /// A picture that is there when the disc is drawn is worn before the
    /// frame ends; one that arrives later is laid into the disc that stands;
    /// and one taken away gives the letters back. Exactly one face each time.
    #[test]
    fn a_disc_wears_the_picture_when_there_is_one() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<Portraits>()
            .set("00ff", Handle::default());
        draw(&mut app, "Mia Wallace#00ff");
        let other = draw(&mut app, "Alice#0001");
        app.update();
        assert_eq!(
            pictures(&mut app),
            1,
            "Mia's picture, on the frame it was drawn"
        );
        assert_eq!(texts(&mut app), vec!["A".to_string()], "and Alice's letter");

        app.world_mut()
            .resource_mut::<Portraits>()
            .set("0001", Handle::default());
        app.update();
        assert_eq!(pictures(&mut app), 2, "a picture that arrives is laid in");
        assert!(texts(&mut app).is_empty());
        assert_eq!(
            app.world().get::<Children>(other).expect("a face").len(),
            1,
            "one face, never the letters under the picture"
        );

        app.world_mut().resource_mut::<Portraits>().forget("0001");
        app.update();
        assert_eq!(pictures(&mut app), 1);
        assert_eq!(texts(&mut app), vec!["A".to_string()]);
    }

    /// With nothing new, nothing is written: the discs of a list at rest
    /// keep their faces untouched.
    #[test]
    fn discs_at_rest_are_not_rebuilt() {
        let mut app = app();
        let disc = draw(&mut app, "Alice#0001");
        app.update();
        let face = app.world().get::<Children>(disc).expect("a face")[0];
        app.update();
        app.update();
        assert_eq!(
            app.world().get::<Children>(disc).expect("a face")[0],
            face,
            "the same entity: nothing despawned it"
        );
    }

    /// The disc is a description of a person and never takes the pointer
    /// from the row or the pill it stands in.
    #[test]
    fn no_part_of_a_disc_takes_the_pointer() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<Portraits>()
            .set("00ff", Handle::default());
        draw(&mut app, "Alice#0001");
        draw(&mut app, "Mia#00ff");
        app.update();
        let world = app.world_mut();
        let mut nodes = world.query_filtered::<(Entity, Option<&Pickable>), With<Node>>();
        for (entity, pickable) in nodes.iter(world) {
            assert_eq!(
                pickable,
                Some(&Pickable::IGNORE),
                "{entity:?} would take the pointer"
            );
        }
    }

    /// A picture is found by the tag a handle ends in, and only by that.
    #[test]
    fn a_picture_is_kept_by_the_tag_a_handle_ends_in() {
        let mut portraits = Portraits::default();
        assert!(portraits.of("Alice#0001").is_none());
        portraits.set("0001", Handle::default());
        assert!(portraits.of("Alice#0001").is_some());
        assert!(portraits.of("Renamed#0001").is_some(), "a name may change");
        assert!(portraits.of("Alice#0002").is_none());
        assert!(portraits.of("Alice").is_none(), "no tag, no account");
        portraits.forget("0001");
        assert!(portraits.of("Alice#0001").is_none());
    }
}
