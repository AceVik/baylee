//! Deck cosmetics: the sleeve a deck's cards show face-down, and the playmat
//! its seat plays on.
//!
//! Two rules shape the whole module.
//!
//! **Cosmetics never travel through the engine.** `GameStatic` is rules data
//! and a view is what a seat is entitled to know about the game; a sleeve is
//! neither. The gateway already builds every table's `GamePreset` and so knows
//! which deck sits in which chair, which makes `GET /games/{id}/cosmetics` the
//! natural home: one fetch at join, no `VIEW_VERSION` bump, and an engine that
//! stays as ignorant of decoration as it is of card text.
//!
//! **What is stored is normalised, not what was uploaded.** Every upload is
//! decoded, resized to the one size that kind is drawn at and re-encoded, so a
//! 40-megapixel photograph and a 12-pixel thumbnail become the same object and
//! a client cannot be handed anything but a valid image of a known size. The
//! decode is also the validation: a file that does not decode is not an image
//! whatever its bytes claim, and `image` is already in the tree, so this costs
//! no new dependency to audit.
//!
//! Cropping happens in the client, which has the picture on screen and the
//! player's hands on it. What arrives here is the rectangle they chose.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path as UrlPath, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};

use crate::{Shared, err};

/// How the browser is told to cache one of these.
///
/// Immutable, like the card art, and for the same reason: the name *is* the
/// content hash, so a changed sleeve is a different URL and can never be a
/// stale one.
const MAX_AGE: &str = "public, max-age=31536000, immutable";

/// Largest upload accepted, before decoding.
///
/// Generous enough for a photograph off a phone and far below anything that
/// would strain the decoder. A body over this is refused without being read
/// into an image at all.
pub const MAX_UPLOAD_BYTES: usize = 8 * 1024 * 1024;

/// What a cosmetic is for, which is the same thing as what size it is stored
/// at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The back of a card in this deck. Card-shaped, and stored at the size
    /// the client's `ArtSize::Normal` art uses so the two are interchangeable
    /// in one material.
    Sleeve,
    /// The mat this deck's seat plays on. Two to one, which is what
    /// `tabletop::seat_mat` generates and therefore what the geometry under it
    /// already expects.
    Playmat,
}

impl Kind {
    /// The name this kind goes by in a URL.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sleeve => "sleeve",
            Self::Playmat => "playmat",
        }
    }

    /// The kind a URL segment names.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "sleeve" => Some(Self::Sleeve),
            "playmat" => Some(Self::Playmat),
            _ => None,
        }
    }

    /// The one size this kind is stored at.
    #[must_use]
    pub const fn size(self) -> (u32, u32) {
        match self {
            // 63:88, and the same pixels as a `normal` card face.
            Self::Sleeve => (488, 680),
            // Twice the generated mat's resolution, because this one is a
            // photograph rather than arithmetic and will be looked at closely.
            Self::Playmat => (1024, 512),
        }
    }
}

/// Where uploaded cosmetics live, and the ids of the ones that are there.
pub struct Store {
    /// The directory, or `None` when uploads are switched off.
    dir: Option<PathBuf>,
}

impl Store {
    /// Reads `BAYLEE_DECK_IMAGE_PATH`.
    ///
    /// `off` or an empty value disables uploads entirely, which is what the
    /// end-to-end tests set: a suite that writes image files into its working
    /// directory is a suite that leaves a mess and can fail on a full disk.
    /// Every other value is a directory, created on first use.
    #[must_use]
    pub fn from_env() -> Self {
        let raw = std::env::var("BAYLEE_DECK_IMAGE_PATH").unwrap_or_else(|_| "deck-images".into());
        let dir = (!raw.is_empty() && raw != "off").then(|| PathBuf::from(raw));
        Self { dir }
    }

    /// Whether uploads are accepted at all.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.dir.is_some()
    }

    /// The path one id is stored at, or `None` when uploads are off.
    fn path(&self, id: &str) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join(format!("{id}.jpg")))
    }

    /// Normalises `bytes` as `kind` and stores it, answering with its id.
    ///
    /// The id is the hash of the *stored* bytes rather than of the upload, so
    /// two players who crop the same picture the same way share one file — and
    /// re-uploading an unchanged image writes nothing new.
    ///
    /// # Errors
    /// [`Rejected`] when uploads are off, the body is too large, it does not
    /// decode, or the directory cannot be written.
    pub fn put(&self, kind: Kind, bytes: &[u8]) -> Result<String, Rejected> {
        let dir = self.dir.as_ref().ok_or(Rejected::Disabled)?;
        if bytes.len() > MAX_UPLOAD_BYTES {
            return Err(Rejected::TooLarge);
        }
        let encoded = normalise(kind, bytes)?;
        let id = hex(&Sha256::digest(&encoded));
        std::fs::create_dir_all(dir).map_err(|_| Rejected::Storage)?;
        let path = dir.join(format!("{id}.jpg"));
        if !path.exists() {
            std::fs::write(&path, &encoded).map_err(|_| Rejected::Storage)?;
        }
        Ok(id)
    }

    /// Reads one back, or `None` when it is not there.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<Vec<u8>> {
        if !well_formed(id) {
            return None;
        }
        std::fs::read(self.path(id)?).ok()
    }
}

/// Why an upload was refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rejected {
    /// The gateway is running without an image directory.
    Disabled,
    /// Over [`MAX_UPLOAD_BYTES`].
    TooLarge,
    /// The bytes are not an image this build can read.
    NotAnImage,
    /// The directory could not be written.
    Storage,
}

impl Rejected {
    /// The status and message this becomes on the wire.
    #[must_use]
    pub const fn parts(self) -> (StatusCode, &'static str) {
        match self {
            Self::Disabled => (StatusCode::SERVICE_UNAVAILABLE, "image uploads are off"),
            Self::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "image too large"),
            // Named formats, because the common failure is a picture that is
            // perfectly good and simply not one of them — a phone photograph
            // is often HEIC and a browser export often WebP. "Not an image"
            // would send that player looking for a corrupt file.
            Self::NotAnImage => (
                StatusCode::BAD_REQUEST,
                "the picture must be a PNG or a JPEG",
            ),
            Self::Storage => (StatusCode::INTERNAL_SERVER_ERROR, "could not store image"),
        }
    }
}

/// Decodes, cuts to `kind`'s shape, scales to its one size, and re-encodes as
/// JPEG.
///
/// A picture of the wrong shape has to lose something, and of the three ways
/// to make it fit, two lose more than they need to. Letterboxing puts grey
/// bars on a card back, which every card at the table then wears; stretching
/// keeps every pixel and ruins all of them. A centred crop loses the edges of
/// one photograph and leaves everything inside them exactly as it was.
///
/// JPEG on the way out whatever came in, because the store is addressed by the
/// hash of what it holds: one encoding is what makes the same picture uploaded
/// twice the same file.
fn normalise(kind: Kind, bytes: &[u8]) -> Result<Vec<u8>, Rejected> {
    let decoded = image::load_from_memory(bytes).map_err(|_| Rejected::NotAnImage)?;
    let (w, h) = kind.size();
    // Crop to the target's shape *before* scaling to its size. `resize_exact`
    // on its own stretches, and a sleeve is the one place that is impossible
    // to miss: every card at the table wears it, so a portrait photograph
    // squeezed into 488×680 is wrong forty times over on one screen.
    //
    // The client's own tool sends an image already cut to shape, so this
    // agrees with it rather than overriding it — a crop of the right shape is
    // its own centred crop, and the arithmetic below is a no-op. It is here
    // for everything that did not come through that tool.
    let (sw, sh) = (decoded.width(), decoded.height());
    let (cw, ch) = centred_crop(sw, sh, w, h);
    let decoded = decoded.crop_imm((sw - cw) / 2, (sh - ch) / 2, cw, ch);
    let resized = decoded.resize_exact(w, h, image::imageops::FilterType::Lanczos3);
    let mut out = Vec::new();
    resized
        .to_rgb8()
        .write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut out, 88,
        ))
        // Not `NotAnImage`: the picture decoded a moment ago, so a failure
        // here is ours and telling the player their file is bad would send
        // them off to convert a picture that was fine.
        .map_err(|_| Rejected::Storage)?;
    Ok(out)
}

/// The largest rectangle of the target's shape that fits inside a source of
/// `sw × sh`, which is then taken from its middle.
///
/// The comparison is the cross-multiplication in `u64` rather than a ratio in
/// `f32`, for the ordinary reason: two 20 000-pixel sides multiply past `u32`
/// long before an 8 MB upload runs out of room, and a float ratio would
/// decide a square image's shape by rounding.
fn centred_crop(sw: u32, sh: u32, tw: u32, th: u32) -> (u32, u32) {
    if sw == 0 || sh == 0 || tw == 0 || th == 0 {
        return (sw, sh);
    }
    if u64::from(sw) * u64::from(th) > u64::from(sh) * u64::from(tw) {
        // Wider than the target: keep every row, trim the sides.
        let width = u64::from(sh) * u64::from(tw) / u64::from(th);
        (u32::try_from(width).unwrap_or(sw).clamp(1, sw), sh)
    } else {
        let height = u64::from(sw) * u64::from(th) / u64::from(tw);
        (sw, u32::try_from(height).unwrap_or(sh).clamp(1, sh))
    }
}

/// Whether `id` is one this module could have written.
///
/// Sixty-four lowercase hex characters and nothing else — which is what makes
/// [`Store::path`] safe: no separator, no `.`, so no id can name a file
/// outside the directory however it was typed into a URL.
#[must_use]
pub fn well_formed(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Lowercase hex of a digest.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// The sleeve and playmat one seat is playing with.
#[derive(Clone, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct SeatCosmetics {
    /// Image id of the deck's sleeve, if it set one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sleeve: Option<String>,
    /// Image id of the deck's playmat, if it set one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playmat: Option<String>,
}

impl SeatCosmetics {
    /// Whether this seat set anything at all.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.sleeve.is_none() && self.playmat.is_none()
    }
}

/// What every seat at one table is playing with, by seat number.
///
/// Keyed the way `lobby::Seat` numbers itself, so the map is built by moving
/// seats into it rather than by converting each one — a narrower key would
/// make every caller prove a bound the lobby has already enforced.
pub type TableCosmetics = HashMap<usize, SeatCosmetics>;

// ------------------------------------------------------------------- routes

/// `POST /images?kind=sleeve|playmat` — one raw image body, normalised and
/// stored.
///
/// Raw rather than multipart: there is exactly one field, and a form encoding
/// would be a parser and a dependency for no gain. The account is required so
/// that filling the disk takes an account first.
pub async fn upload(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    if let Err(e) = crate::authed(&state, &headers) {
        return e.into_response();
    }
    // A missing `kind` and an unknown one are the same answer on purpose:
    // both mean the caller named a kind of picture this gateway does not
    // draw, and neither is worth a second error code.
    let Some(kind) = params.get("kind").and_then(|k| Kind::parse(k)) else {
        return err(StatusCode::NOT_FOUND, "no such image kind").into_response();
    };
    let store = Arc::clone(&state.deck_images);
    let bytes = body.to_vec();
    // Decoding and rescaling a photograph is not something to do on the async
    // runtime's thread: one upload would stall every socket the gateway is
    // holding, and it holds all of them.
    let stored = tokio::task::spawn_blocking(move || store.put(kind, &bytes)).await;
    match stored {
        // The kind comes back with the id because the client uploads both
        // through one path and files the answer under what it asked for; a
        // reply that only said `id` would make the caller remember which
        // request this was.
        Ok(Ok(id)) => {
            axum::Json(serde_json::json!({ "id": id, "kind": kind.as_str() })).into_response()
        }
        Ok(Err(rejected)) => {
            let (status, message) = rejected.parts();
            err(status, message).into_response()
        }
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "upload failed").into_response(),
    }
}

/// `GET /images/{id}` — one stored cosmetic.
///
/// Unauthenticated on purpose, exactly like the card-art mirror: everyone at a
/// table sees everyone's sleeves, and the id is a content hash nobody can
/// guess their way through.
///
/// The path is the bare id, with no `.jpg` on the end — which is where this
/// differs from [`crate::art`] next door, and the difference is a decision
/// rather than an oversight. An art URL mirrors Scryfall's own path and
/// inherits the extension from it; this id is our own digest and is what a
/// deck stores in one field, so making the URL the id and nothing else means
/// a client appends the field it already has instead of building a string
/// around it. The first version did carry the suffix, and the very first
/// fetch of an uploaded sleeve answered 404: `upload` handed back an id and
/// `serve` wanted a filename. The type is named in a header, which is the
/// only place a type binds anything; the extension on disk is the store's
/// business and stays there.
pub async fn serve(State(state): State<Shared>, UrlPath(id): UrlPath<String>) -> Response {
    if !well_formed(&id) {
        return err(StatusCode::NOT_FOUND, "no such image").into_response();
    }
    let store = Arc::clone(&state.deck_images);
    let found = tokio::task::spawn_blocking(move || store.get(&id)).await;
    match found {
        Ok(Some(bytes)) => (
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (header::CACHE_CONTROL, MAX_AGE),
                (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            ],
            bytes,
        )
            .into_response(),
        _ => err(StatusCode::NOT_FOUND, "no such image").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_picture(w: u32, h: u32) -> Vec<u8> {
        let mut buf = image::RgbImage::new(w, h);
        for (x, y, pixel) in buf.enumerate_pixels_mut() {
            *pixel = image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]);
        }
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(buf)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("encode a png");
        out
    }

    fn temp_store(label: &str) -> Store {
        let dir =
            std::env::temp_dir().join(format!("baylee-cosmetics-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Store { dir: Some(dir) }
    }

    #[test]
    fn every_upload_comes_out_at_the_one_size_its_kind_is_drawn_at() {
        for (kind, expected) in [(Kind::Sleeve, (488, 680)), (Kind::Playmat, (1024, 512))] {
            // Deliberately the wrong shape *and* the wrong scale on the way in.
            let encoded = normalise(kind, &a_picture(300, 100)).expect("a png decodes");
            let out = image::load_from_memory(&encoded).expect("and comes back out");
            assert_eq!(
                (out.width(), out.height()),
                expected,
                "{} is stored at one size whatever arrives",
                kind.as_str()
            );
        }
    }

    /// The defect this half was written with: `resize_exact` alone squeezes a
    /// picture into the target's shape. A sleeve is the one place that cannot
    /// be missed — every card at the table wears it, so one squashed portrait
    /// is wrong forty times on a single screen.
    #[test]
    fn a_picture_of_the_wrong_shape_is_cut_to_shape_and_not_squashed() {
        // Far too wide: one card's height of the middle is all that survives.
        assert_eq!(centred_crop(2000, 100, 488, 680), (71, 100));
        // Far too tall: the full width, and the middle of the column.
        assert_eq!(centred_crop(100, 2000, 488, 680), (100, 139));
        // A landscape photograph onto the playmat's 2:1.
        assert_eq!(centred_crop(800, 800, 1024, 512), (800, 400));
        // And the case that matters most, because the client's own crop tool
        // produces it: a picture already of the target's shape is its own
        // centred crop, so the tool's decision is kept rather than re-taken.
        assert_eq!(centred_crop(976, 1360, 488, 680), (976, 1360));
        assert_eq!(centred_crop(488, 680, 488, 680), (488, 680));
    }

    #[test]
    fn the_id_is_the_hash_of_what_was_stored_so_the_same_picture_is_one_file() {
        let store = temp_store("dedup");
        let a = store
            .put(Kind::Sleeve, &a_picture(400, 400))
            .expect("stored");
        let b = store
            .put(Kind::Sleeve, &a_picture(400, 400))
            .expect("stored");
        assert_eq!(a, b, "the same picture twice is one id");
        assert!(well_formed(&a));
        assert!(store.get(&a).is_some(), "and it can be read back");
    }

    /// The same picture as a sleeve and as a playmat is two different files,
    /// because they are stored at two different sizes — hashing the *upload*
    /// would have collapsed them and served a card back as a mat.
    #[test]
    fn one_picture_under_two_kinds_is_two_images() {
        let store = temp_store("kinds");
        let sleeve = store
            .put(Kind::Sleeve, &a_picture(400, 400))
            .expect("stored");
        let mat = store
            .put(Kind::Playmat, &a_picture(400, 400))
            .expect("stored");
        assert_ne!(sleeve, mat);
    }

    #[test]
    fn something_that_is_not_an_image_is_refused_however_it_is_labelled() {
        let store = temp_store("garbage");
        assert_eq!(
            store.put(Kind::Sleeve, b"GIF89a this is not a picture"),
            Err(Rejected::NotAnImage)
        );
    }

    #[test]
    fn an_oversized_body_is_refused_before_it_is_decoded() {
        let store = temp_store("toobig");
        let huge = vec![0u8; MAX_UPLOAD_BYTES + 1];
        assert_eq!(store.put(Kind::Sleeve, &huge), Err(Rejected::TooLarge));
    }

    #[test]
    fn a_disabled_store_accepts_nothing_and_serves_nothing() {
        let store = Store { dir: None };
        assert!(!store.enabled());
        assert_eq!(
            store.put(Kind::Sleeve, &a_picture(100, 100)),
            Err(Rejected::Disabled)
        );
        assert!(store.get(&"a".repeat(64)).is_none());
    }

    /// The id goes straight into a filename, so it has to be impossible to
    /// name anything but a file in the directory.
    #[test]
    fn an_id_cannot_climb_out_of_the_image_directory() {
        assert!(!well_formed("../../etc/passwd"));
        assert!(!well_formed(&"../".repeat(21)));
        assert!(
            !well_formed(&"A".repeat(64)),
            "uppercase is not what we write"
        );
        assert!(!well_formed(&"a".repeat(63)), "and the length is exact");
        assert!(well_formed(&"0123456789abcdef".repeat(4)));
    }

    #[test]
    fn a_kind_survives_the_round_trip_through_a_url() {
        for kind in [Kind::Sleeve, Kind::Playmat] {
            assert_eq!(Kind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(Kind::parse("avatar"), None);
    }
}
