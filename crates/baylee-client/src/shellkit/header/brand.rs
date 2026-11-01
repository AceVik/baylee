//! The shell header's brand art.

#[allow(clippy::wildcard_imports)] // the header module's shared vocabulary
use super::*;

/// The lockup's width over its height (the trimmed master, 1928 × 809).
pub const LOGO_ASPECT: f32 = 1942.0 / 809.0;

/// How much of the header's height the logo stands in.
pub const LOGO_SHARE: f32 = 0.80;

/// The header's brand: the logo, as an image. Its picture is set by
/// [`sharpen_brand`] after layout (no texture until then: a frame of the
/// default white square would flash on every rebuild otherwise).
#[derive(Component, Clone, Copy, Debug)]
pub struct BrandMark;

/// The lockup's cuts, by height in pixels, smallest first.
pub const LOGO_CUTS: [(f32, &str); 3] = [
    (48.0, "brand/baylee-logo-48.png"),
    (96.0, "brand/baylee-logo-96.png"),
    (192.0, "brand/baylee-logo-192.png"),
];

/// The cuts, loaded once.
#[derive(Resource, Clone)]
pub struct BrandArt(pub [Handle<Image>; 3]);

/// Which cut draws a logo `physical` pixels high: the smallest at least as
/// tall (a downsample by under two, never an upsample), else the largest.
#[must_use]
pub fn cut_for(physical: f32) -> usize {
    LOGO_CUTS
        .iter()
        .position(|(h, _)| *h >= physical - 0.5)
        .unwrap_or(LOGO_CUTS.len() - 1)
}

/// The brand mark for a header `header` pixels tall, named `name` for
/// assistive technology. The builder's own header wears it too.
pub fn brand_mark(commands: &mut Commands, header: f32, name: &str) -> Entity {
    let height = (header * LOGO_SHARE).round();
    let mut node = accesskit::Node::new(accesskit::Role::Image);
    node.set_label(name);
    commands
        .spawn((
            BrandMark,
            Name::new(name.to_string()),
            bevy::a11y::AccessibilityNode(node),
            ImageNode {
                color: Color::NONE,
                ..default()
            },
            Node {
                width: px_fixed((height * LOGO_ASPECT).round()),
                height: px_fixed(height),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            UiTransform::default(),
            crate::ambience::Feel::rising_to(Color::NONE, tokens::HOVER),
        ))
        .id()
}

/// Gives each brand mark the cut its drawn height asks for, after layout and
/// before the frame is drawn; writes only when the cut changes.
pub fn sharpen_brand(
    art: Option<Res<BrandArt>>,
    mut marks: Query<(&ComputedNode, &mut ImageNode), With<BrandMark>>,
) {
    let Some(art) = art else {
        return;
    };
    for (computed, mut image) in &mut marks {
        let handle = &art.0[cut_for(computed.size().y)];
        if image.image != *handle || image.color != Color::WHITE {
            image.image = handle.clone();
            image.color = Color::WHITE;
        }
    }
}
