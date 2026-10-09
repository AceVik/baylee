//! The front door's source line (#299): the AGPL's §13 offer as a link the
//! player can open, and as a QR code a phone can read off the screen.
//!
//! The address is whatever the chosen gateway said in `/info`, and that
//! gateway may be hostile. It was checked on the way in
//! ([`web_address`], through `GatewayInfo::read`), and it is checked again
//! here, at the door: only a plain `http://` or `https://` address of at
//! most 200 characters, with no whitespace, control or bidi character, is
//! opened or drawn as a code, and `webbrowser`'s `hardened` feature refuses
//! any other scheme a third time. Anything else would stay plain text. It
//! opens only on the player's own click, and the whole address stays in
//! sight beside the code, so what a phone reads is what the screen says.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::lobby::gateway_info::web_address;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use qrcodegen::{QrCode, QrCodeEcc};

/// The community's Discord invitation (owner, 09.10.2026), drawn on the
/// front door's top-right tile as a code and as text, opened on a click.
pub(crate) const DISCORD_URL: &str = "https://discord.gg/4pP3uAwgY";

/// The light round a code, in modules: the standard's quiet zone.
const QUIET: i32 = 4;

/// How many logical pixels one module is drawn at.
///
/// A whole number, so every module is the same size on screen. An address
/// this long is a code of about 41 modules with its quiet zone, so about 80
/// pixels, which a phone reads off a laptop at arm's length.
pub(super) const MODULE_PX: f32 = 2.0;

/// The code's dark modules and its light ground: near-black on warm paper.
///
/// The paper is darker than white (#295): a white square was the brightest
/// thing on the front door, brighter than the scene's own light. The pair
/// still differs by more than ten to one, far more than a reader needs.
const DARK: [u8; 4] = [0x1a, 0x16, 0x12, 0xff];
const LIGHT: [u8; 4] = [0xd2, 0xc6, 0xae, 0xff];

/// The code drawn for the address on show.
#[derive(Clone, Debug)]
pub(crate) struct Code {
    /// The address it says.
    pub url: String,
    /// Its picture, one texel a module.
    pub image: Handle<Image>,
    /// Its side, in modules, quiet zone included.
    pub side: u32,
}

/// Keeps [`LobbyState::source_code`] the code of the address the front door
/// shows, making a new one only when that address changes.
///
/// Before `ui` in the same frame, so the colophon that names a new address
/// is drawn with its code.
///
/// A headless app has no image assets, and then there is no code and the
/// line stays text.
pub(super) fn keep_the_code(mut state: ResMut<LobbyState>, images: Option<ResMut<Assets<Image>>>) {
    let Some(mut images) = images else {
        return;
    };
    if state.discord_code.is_none() {
        state.discord_code = code(&mut images, DISCORD_URL);
    }
    let url = super::front::source_address(&state);
    let drawn = state.source_code.as_ref().map(|code| code.url.as_str());
    let wanted = web_address(url);
    if drawn == wanted.as_deref() {
        return;
    }
    state.source_code = wanted.and_then(|url| code(&mut images, &url));
}

/// `url` as a [`Code`], its picture added to `images`.
fn code(images: &mut Assets<Image>, url: &str) -> Option<Code> {
    let (image, side) = code_image(url)?;
    Some(Code {
        url: url.to_string(),
        image: images.add(image),
        side,
    })
}

/// `url` as a QR code, one texel a module, with its quiet zone.
///
/// Medium error correction: a screen photographed at an angle loses a few
/// modules, and an address this short leaves the code small either way.
pub(super) fn code_image(url: &str) -> Option<(Image, u32)> {
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

/// Opens the address a tile's press names in the player's browser, on
/// their click.
///
/// Checked again right before it goes to the system: the address is the
/// one the tile drew, and nothing else is ever opened.
pub(super) fn open(press: super::front::FrontPress, state: &LobbyState) {
    if let Some(url) = address_of(press, state) {
        browse(&url);
    }
}

/// The address a front-door press opens, if it opens one: the one hook
/// both tiles' clicks go through, so a test reads what a click would open.
pub(crate) fn address_of(press: super::front::FrontPress, state: &LobbyState) -> Option<String> {
    use super::front::FrontPress;
    match press {
        FrontPress::OpenSource => web_address(super::front::source_address(state)),
        FrontPress::OpenDiscord => web_address(DISCORD_URL),
        _ => None,
    }
}

/// Hands `url` to the system's browser.
fn browse(url: &str) {
    if let Err(err) = webbrowser::open(url) {
        warn!("could not open {url}: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A code is the address's modules inside the standard's quiet zone:
    /// light all round, and the top-left finder pattern where a reader
    /// looks for it (a dark ring, a light ring, a dark core).
    #[test]
    fn a_code_is_its_modules_inside_a_quiet_zone() {
        for url in ["https://github.com/AceVik/baylee", DISCORD_URL] {
            a_code_says(url);
        }
    }

    /// The Discord tile's code is the owner's invitation, module for module:
    /// every texel inside the quiet zone is the standard's encoding of
    /// exactly that address (no decoder here; the encoder is the reader's
    /// other half), and the address passes the check at the door.
    #[test]
    fn the_discord_code_says_the_invitation() {
        assert_eq!(DISCORD_URL, "https://discord.gg/4pP3uAwgY");
        assert_eq!(web_address(DISCORD_URL).as_deref(), Some(DISCORD_URL));
        let (image, side) = code_image(DISCORD_URL).expect("encodes");
        let code = QrCode::encode_text(DISCORD_URL, QrCodeEcc::Medium).expect("encodes");
        let data = image.data.as_ref().expect("the texels are kept");
        let side = i32::try_from(side).expect("small");
        for y in 0..side {
            for x in 0..side {
                let at = ((y * side + x) * 4) as usize;
                let dark = code.get_module(x - QUIET, y - QUIET);
                assert_eq!(
                    data[at..at + 4],
                    if dark { DARK } else { LIGHT },
                    "module ({x}, {y})"
                );
            }
        }
    }

    fn a_code_says(url: &str) {
        let (image, side) = code_image(url).expect("an address this short encodes");
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
