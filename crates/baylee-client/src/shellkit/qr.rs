//! A QR code a phone reads off the screen: an address as a picture, one
//! texel a module, and the thin dark frame it stands in. The front door's
//! codes (#299: the source, the community, the donations) and a friend
//! invite's link are drawn by it.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use qrcodegen::{QrCode, QrCodeEcc};

use super::metrics::px_fixed;

/// The light round a code, in modules: the standard's quiet zone.
pub(crate) const QUIET: i32 = 4;

/// The code's dark modules and its light ground: near-black on warm paper.
///
/// The paper is darker than white (#295): a white square was the brightest
/// thing on the front door, brighter than the scene's own light. The pair
/// still differs by more than ten to one, far more than a reader needs.
pub(crate) const DARK: [u8; 4] = [0x1a, 0x16, 0x12, 0xff];
pub(crate) const LIGHT: [u8; 4] = [0xd2, 0xc6, 0xae, 0xff];

/// `url` as a QR code, one texel a module, with its quiet zone, and its
/// side in modules.
///
/// Medium error correction: a screen photographed at an angle loses a few
/// modules, and an address this short leaves the code small either way.
pub fn image(url: &str) -> Option<(Image, u32)> {
    let code = QrCode::encode_text(url, QrCodeEcc::Medium).ok()?;
    let side = code.size() + 2 * QUIET;
    let mut data = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let dark = code.get_module(x - QUIET, y - QUIET);
            data.extend_from_slice(if dark { &DARK } else { &LIGHT });
        }
    }
    let side = u32::try_from(side).ok()?;
    let mut image = Image::new(
        Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    // Nearest, so a module's edge stays an edge when it is drawn larger.
    image.sampler = ImageSampler::nearest();
    Some((image, side))
}

/// A code's picture drawn `side` wide whatever its module count, in the
/// thin dark frame a code stands in.
pub fn framed(commands: &mut Commands, image: Handle<Image>, side: Val) -> Entity {
    let picture = commands
        .spawn((
            ImageNode::new(image),
            Node {
                width: side,
                height: side,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands
        .spawn((
            Node {
                padding: UiRect::all(px_fixed(5.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(8.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.04, 0.07, 0.96)),
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.14)),
            Pickable::IGNORE,
        ))
        .add_child(picture)
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A code is the address's modules inside the standard's quiet zone:
    /// light all round, and the top-left finder pattern where a reader
    /// looks for it (a dark ring, a light ring, a dark core).
    #[test]
    fn a_code_is_its_modules_inside_a_quiet_zone() {
        for url in [
            "https://github.com/AceVik/baylee",
            "https://baylee.app/de/invite/Zm9vYmFyLWJheWxlZS0xMA",
        ] {
            a_code_says(url);
        }
    }

    fn a_code_says(url: &str) {
        let (image, side) = image(url).expect("an address this short encodes");
        let modules = QrCode::encode_text(url, QrCodeEcc::Medium)
            .expect("encodes")
            .size();
        assert_eq!(i64::from(side), i64::from(modules + 2 * QUIET));
        let data = image.data.as_ref().expect("the texels are kept");
        let dark = |x: i32, y: i32| {
            let at = ((y * modules + y * 2 * QUIET + x) * 4) as usize;
            data[at] == DARK[0]
        };
        let q = QUIET;
        for edge in 0..side as i32 {
            assert!(!dark(edge, 0) && !dark(0, edge), "the quiet zone at {edge}");
        }
        assert!(dark(q, q), "the finder's outer ring");
        assert!(!dark(q + 1, q + 1), "the finder's light ring");
        assert!(dark(q + 3, q + 3), "the finder's core");
    }
}
