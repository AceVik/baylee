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
//! [`Portraits`] holds the pictures this client has, by account tag. The
//! desktop client fills it from Baylee Central (`crate::portraits`): the
//! keys of the account's own picture, its contacts' and its groups'
//! members', each picture fetched, decoded bounded (256 pixels square,
//! Central's bounds) and kept at half that, off the render thread. A
//! picture laid in [`Portraits`] is drawn in every disc of that account
//! ([`wear_portraits`]), and a disc without one keeps its monogram. A
//! picture that moves keeps one image, whose pixels `crate::portraits`
//! changes when the frame shown changes, so a playing picture writes no
//! disc, no node and nothing here.

use std::collections::HashMap;

use baylee_client_core::avatar::{self, RIM_ALPHA, Tint};
use bevy::prelude::*;

use crate::hud::{UiFonts, tf_bold};

/// The pictures this client holds, by account tag as a handle spells it
/// (`af03`), each for the whole handle Baylee Central knows the account by.
///
/// A disc wears a picture only when its handle is that one, name and tag:
/// a game gateway gives its own accounts tags of its own, which may equal a
/// Central account's, and such a stranger keeps the monogram its handle
/// already shares with that account.
#[derive(Resource, Default)]
pub struct Portraits {
    worn: HashMap<String, Portrait>,
}

/// One account's picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Portrait {
    /// The handle Central knows the account by (`Alice#af03`).
    pub handle: String,
    /// The image every disc of the account draws.
    pub image: Handle<Image>,
    /// Whether the picture moves (its frames play in the same image).
    pub moving: bool,
}

impl Portraits {
    /// The picture of the account a handle (`Alice#af03`) names, if this
    /// client has one.
    #[must_use]
    pub fn of(&self, handle: &str) -> Option<Handle<Image>> {
        self.portrait(handle).map(|portrait| portrait.image.clone())
    }

    /// The account a handle names, as it wears its picture: only for the
    /// handle Central knows it by.
    #[must_use]
    pub fn portrait(&self, handle: &str) -> Option<&Portrait> {
        let (_, tag) = handle.rsplit_once('#')?;
        self.worn
            .get(tag)
            .filter(|portrait| portrait.handle == handle)
    }

    /// The account with this tag, as it wears its picture.
    #[must_use]
    pub fn by_tag(&self, tag: &str) -> Option<&Portrait> {
        self.worn.get(tag)
    }

    /// Lays in the still picture of the account `handle` names.
    pub fn set(&mut self, handle: &str, picture: Handle<Image>) {
        self.wear(Portrait {
            handle: handle.to_owned(),
            image: picture,
            moving: false,
        });
    }

    /// Lays in the picture of the account its handle names; a handle
    /// without a tag names none.
    pub fn wear(&mut self, portrait: Portrait) {
        if let Some((_, tag)) = portrait.handle.rsplit_once('#') {
            self.worn.insert(tag.to_owned(), portrait);
        }
    }

    /// Forgets the picture of the account with this tag: it shows its
    /// monogram again.
    pub fn forget(&mut self, tag: &str) {
        self.worn.remove(tag);
    }

    /// Every account's picture, by tag.
    pub fn all(&self) -> impl Iterator<Item = (&str, &Portrait)> {
        self.worn
            .iter()
            .map(|(tag, portrait)| (tag.as_str(), portrait))
    }

    /// How many accounts wear a picture.
    #[must_use]
    pub fn len(&self) -> usize {
        self.worn.len()
    }

    /// Whether no account wears a picture.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.worn.is_empty()
    }
}

/// An avatar's disc, and whose it is.
#[derive(Component, Clone, PartialEq, Debug)]
pub struct Avatar {
    /// The clipped picture and its ground, scaled together inside a ranked frame.
    pub(crate) plate: Entity,
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
    let plate = commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
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
    let disc = commands
        .spawn((
            Avatar {
                plate,
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
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_child(plate)
        .id();
    let letters = face(commands, fonts, handle, size, quiet, None);
    commands.entity(plate).add_child(letters);
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
    discs: Query<Ref<Avatar>>,
    plates: Query<&Children>,
    faces: Query<&AvatarFace>,
) {
    // No faces to set letters in yet: the discs keep what they were drawn
    // with, and are looked at again when the pictures change.
    if fonts.is_none() {
        return;
    }
    let all = portraits.is_changed();
    for avatar in &discs {
        if !all && !avatar.is_added() {
            continue;
        }
        let want = portraits
            .of(&avatar.handle)
            .map_or(AvatarFace::Letters, AvatarFace::Picture);
        let disc = avatar.plate;
        let worn: Vec<Entity> = plates
            .get(disc)
            .ok()
            .into_iter()
            .flatten()
            .copied()
            .filter(|child| faces.contains(*child))
            .collect();
        if worn.len() == 1 && faces.get(worn[0]) == Ok(&want) {
            continue;
        }
        for old in worn {
            commands.entity(old).try_despawn();
        }
        let picture = match want {
            AvatarFace::Picture(image) => Some(image),
            AvatarFace::Letters => None,
        };
        let handle = avatar.handle.clone();
        let size = avatar.size;
        let quiet = avatar.quiet;
        commands.queue(move |world: &mut World| {
            if world.get_entity(disc).is_err() {
                return;
            }
            let Some(fonts) = world.get_resource::<UiFonts>().cloned() else {
                return;
            };
            let mut cmds = world.commands();
            let new = face(&mut cmds, &fonts, &handle, size, quiet, picture);
            cmds.entity(disc).add_child(new);
        });
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
            world
                .get::<BackgroundColor>(world.get::<Avatar>(plain).unwrap().plate)
                .expect("a ground")
                .0,
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
            .set("Mia Wallace#00ff", Handle::default());
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
            .set("Alice#0001", Handle::default());
        app.update();
        assert_eq!(pictures(&mut app), 2, "a picture that arrives is laid in");
        assert!(texts(&mut app).is_empty());
        assert_eq!(
            app.world()
                .get::<Children>(app.world().get::<Avatar>(other).unwrap().plate)
                .expect("a face")
                .len(),
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
        let disc = app.world().get::<Avatar>(disc).unwrap().plate;
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
            .set("Mia#00ff", Handle::default());
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

    /// A picture is kept by the tag a handle ends in and worn only by the
    /// whole handle Central knows: a gateway's own account that happens to
    /// hold the same tag under another name keeps its monogram.
    #[test]
    fn a_picture_is_worn_only_by_the_handle_central_knows() {
        let mut portraits = Portraits::default();
        assert!(portraits.of("Alice#0001").is_none());
        portraits.set("Alice#0001", Handle::default());
        assert!(portraits.of("Alice#0001").is_some());
        assert!(portraits.by_tag("0001").is_some());
        assert!(
            portraits.of("Mallory#0001").is_none(),
            "the same tag under another name is someone else"
        );
        assert!(portraits.of("Alice#0002").is_none());
        assert!(portraits.of("Alice").is_none(), "no tag, no account");
        portraits.forget("0001");
        assert!(portraits.of("Alice#0001").is_none());
    }
}
