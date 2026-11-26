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
/// `url` as a QR code, its picture and its side in modules (the kit's).
pub(super) use crate::shellkit::qr::image as code_image;

/// The community's Discord invitation (owner, 09.10.2026), drawn on the
/// front door's top-right tile as a code and as text, opened on a click.
pub(crate) const DISCORD_URL: &str = "https://discord.gg/4pP3uAwgY";

/// The community's `WhatsApp` invitation (`baylee-central` `web/src/config.ts`),
/// drawn on the front door as a community button and QR tile, opened on a click.
pub(crate) const WHATSAPP_URL: &str = "https://chat.whatsapp.com/JsWHl71rnPPGEG74w48vk6";

/// Baylee's website, linked from Central's sign-in form.
pub(crate) const CENTRAL_WEBSITE_URL: &str = "https://baylee.app";

/// Where a Baylee account is made: the website's registration page, which
/// Central's sign-in form links to (baylee-central `docs/web-accounts.md`).
pub(crate) const CENTRAL_REGISTER_URL: &str = "https://baylee.app/register";

/// Resolves the Central website origin for the current environment: strictly
/// [`CENTRAL_WEBSITE_URL`] in shipped builds and unit tests, or a validated
/// loopback `http://127.0.0.1:<port>` origin in local debug/`dev-control` runs
/// when `BAYLEE_CENTRAL_WEB_ORIGIN` (or `BAYLEE_CENTRAL_DEV_ENDPOINT` on the
/// standard 28790->28794 pair) is set.
pub(crate) fn central_website_url() -> String {
    #[cfg(all(
        not(target_arch = "wasm32"),
        any(debug_assertions, feature = "dev-control")
    ))]
    if !cfg!(test)
        && let Some(origin) = central_web_origin_from_env(
            std::env::var("BAYLEE_CENTRAL_WEB_ORIGIN").ok().as_deref(),
            std::env::var("BAYLEE_CENTRAL_DEV_ENDPOINT").ok().as_deref(),
        )
    {
        return origin;
    }
    CENTRAL_WEBSITE_URL.to_owned()
}

/// Resolves the Central registration URL for the current environment.
pub(crate) fn central_register_url() -> String {
    let base = central_website_url();
    if base == CENTRAL_WEBSITE_URL {
        CENTRAL_REGISTER_URL.to_owned()
    } else {
        format!("{base}/register")
    }
}

#[cfg(any(
    test,
    all(
        not(target_arch = "wasm32"),
        any(debug_assertions, feature = "dev-control")
    )
))]
fn central_web_origin_from_env(
    web_origin: Option<&str>,
    dev_endpoint: Option<&str>,
) -> Option<String> {
    if let Some(raw) = web_origin {
        return baylee_account::Endpoint::development(raw)
            .ok()
            .map(|ep| ep.origin().to_owned());
    }
    let ep = baylee_account::Endpoint::development(dev_endpoint?).ok()?;
    if ep.origin() == "http://127.0.0.1:28790" {
        return Some("http://127.0.0.1:28794".to_owned());
    }
    None
}

/// Releases and changelog on GitHub, opened when clicking the bottom-left
/// version badge.
pub(crate) const CHANGELOG_URL: &str = "https://github.com/AceVik/baylee/releases";

/// Official Wizards of the Coast Fan Content Policy page (`NOTICE`, `docs/legal.md`).
pub(crate) const FAN_CONTENT_POLICY_URL: &str =
    "https://company.wizards.com/en/legal/fancontentpolicy";

/// Scryfall's home page (`NOTICE`, `docs/legal.md` §3).
pub(crate) const SCRYFALL_URL: &str = baylee_client_core::images::SCRYFALL_SITE;

/// Closed set of verified external HTTPS reference links opened from the front
/// door's colophon and the About sheet.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(crate) enum ExternalLink {
    FanContentPolicy,
    Scryfall,
    AgplLicence,
    AlegreyaSans,
    Faustina,
    FontAwesome,
    ManaFont,
    MtgJson,
    Bevy,
    Wgpu,
    Rav1d,
    Tokio,
    Rustls,
    SeaOrm,
    OpaqueKe,
    RustCrypto,
    Libsecret,
    Vsco2,
    FreePats,
    Mutopia,
    Fluent,
    Qrcodegen,
}

impl ExternalLink {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 22] = [
        Self::FanContentPolicy,
        Self::Scryfall,
        Self::AgplLicence,
        Self::AlegreyaSans,
        Self::Faustina,
        Self::FontAwesome,
        Self::ManaFont,
        Self::MtgJson,
        Self::Bevy,
        Self::Wgpu,
        Self::Rav1d,
        Self::Tokio,
        Self::Rustls,
        Self::SeaOrm,
        Self::OpaqueKe,
        Self::RustCrypto,
        Self::Libsecret,
        Self::Vsco2,
        Self::FreePats,
        Self::Mutopia,
        Self::Fluent,
        Self::Qrcodegen,
    ];

    /// Compile-time HTTPS destination for this reference link.
    #[must_use]
    pub(crate) const fn url(self) -> &'static str {
        match self {
            Self::FanContentPolicy => FAN_CONTENT_POLICY_URL,
            Self::Scryfall => SCRYFALL_URL,
            Self::AgplLicence => "https://www.gnu.org/licenses/agpl-3.0.html",
            Self::AlegreyaSans => "https://github.com/huertatipografica/Alegreya-Sans",
            Self::Faustina => "https://github.com/Omnibus-Type/Faustina",
            Self::FontAwesome => "https://fontawesome.com",
            Self::ManaFont => "https://mana.andrewgioia.com",
            Self::MtgJson => "https://mtgjson.com",
            Self::Bevy => "https://bevy.org",
            Self::Wgpu => "https://wgpu.rs",
            Self::Rav1d => "https://github.com/memorysafety/rav1d",
            Self::Tokio => "https://tokio.rs",
            Self::Rustls => "https://github.com/rustls/rustls",
            Self::SeaOrm => "https://www.sea-ql.org/SeaORM/",
            Self::OpaqueKe => "https://github.com/facebook/opaque-ke",
            Self::RustCrypto => "https://github.com/RustCrypto",
            Self::Libsecret => "https://gitlab.gnome.org/GNOME/libsecret",
            Self::Vsco2 => "https://versilian-studios.com/vsco-community/",
            Self::FreePats => "https://freepats.zenvoid.org",
            Self::Mutopia => "https://www.mutopiaproject.org",
            Self::Fluent => "https://projectfluent.org",
            Self::Qrcodegen => "https://www.nayuki.io/page/qr-code-generator-library",
        }
    }
}

/// How many logical pixels one module is drawn at.
///
/// A whole number, so every module is the same size on screen. An address
/// this long is a code of about 41 modules with its quiet zone, so about 80
/// pixels, which a phone reads off a laptop at arm's length.
pub(super) const MODULE_PX: f32 = 2.0;

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
    if state.whatsapp_code.is_none() {
        state.whatsapp_code = code(&mut images, WHATSAPP_URL);
    }
    if state.website_code.is_none() {
        let site = central_website_url();
        state.website_code = code(&mut images, &site);
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

/// Opens the address a tile's press names in the player's browser, on
/// their click.
///
/// Checked again right before it goes to the system: the address is the
/// one the tile drew, and nothing else is ever opened.
pub(super) fn open(press: super::front::FrontPress, state: &LobbyState) {
    if let Some(url) = address_of(press, state) {
        #[cfg(test)]
        if let Ok(mut urls) = state.opened_urls.lock() {
            urls.push(url);
        }
        #[cfg(not(test))]
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
        FrontPress::OpenWhatsApp => web_address(WHATSAPP_URL),
        FrontPress::OpenWebsite => web_address(&central_website_url()),
        FrontPress::OpenCentralRegister => web_address(&central_register_url()),
        FrontPress::OpenChangelog => web_address(CHANGELOG_URL),
        FrontPress::OpenExternal(link) => web_address(link.url()),
        FrontPress::OpenGiving(platform) => web_address(platform.address()),
        FrontPress::OpenImpressum => super::front::impressum_link(state).map(|(url, _)| url),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn take_opened_urls_for_test(app: &App) -> Vec<String> {
    app.world()
        .resource::<LobbyState>()
        .opened_urls
        .lock()
        .map_or_else(
            |e| std::mem::take(&mut *e.into_inner()),
            |mut g| std::mem::take(&mut *g),
        )
}

/// Hands `url` to the system's browser.
#[cfg(not(test))]
fn browse(url: &str) {
    if let Err(err) = webbrowser::open(url) {
        warn!("could not open {url}: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellkit::qr::{DARK, LIGHT, QUIET};
    use qrcodegen::{QrCode, QrCodeEcc};

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

    #[test]
    fn the_whatsapp_code_says_the_invitation() {
        assert_eq!(
            WHATSAPP_URL,
            "https://chat.whatsapp.com/JsWHl71rnPPGEG74w48vk6"
        );
        assert_eq!(web_address(WHATSAPP_URL).as_deref(), Some(WHATSAPP_URL));
        let state = LobbyState::from_settings(crate::settings::ClientSettings::default());
        assert_eq!(
            address_of(super::super::front::FrontPress::OpenWhatsApp, &state).as_deref(),
            Some(WHATSAPP_URL)
        );
    }

    #[test]
    fn central_web_origin_maps_local_loopback_env_and_rejects_remote_overrides() {
        assert_eq!(central_web_origin_from_env(None, None), None);
        assert_eq!(
            central_web_origin_from_env(None, Some("http://127.0.0.1:28790")).as_deref(),
            Some("http://127.0.0.1:28794")
        );
        assert_eq!(
            central_web_origin_from_env(
                Some("http://127.0.0.1:29000"),
                Some("http://127.0.0.1:28790")
            )
            .as_deref(),
            Some("http://127.0.0.1:29000")
        );
        for bad in [
            "https://evil.example",
            "https://127.0.0.1:28794",
            "http://192.168.0.53:28794",
            "http://localhost:28794",
        ] {
            assert_eq!(central_web_origin_from_env(Some(bad), None), None, "{bad}");
        }
    }

    #[test]
    fn every_external_reference_link_is_a_valid_https_address() {
        let state = LobbyState::from_settings(crate::settings::ClientSettings::default());
        for link in ExternalLink::ALL {
            let resolved = address_of(super::super::front::FrontPress::OpenExternal(link), &state);
            assert_eq!(
                resolved.as_deref(),
                Some(link.url()),
                "{link:?} failed web_address check"
            );
            assert!(
                link.url().starts_with("https://"),
                "{link:?} must use HTTPS: {}",
                link.url()
            );
        }
    }
}
