//! End-to-end test for a deck's cosmetics: uploading a sleeve or a playmat,
//! and getting it back.
//!
//! It exists for two reasons that are easy to tell apart. The first is the
//! feature: a picture goes in at whatever shape a phone or a browser produced
//! and has to come out at the one size its kind is drawn at, **cut** to that
//! shape rather than squashed into it — the unit tests check that arithmetic,
//! and this checks that the route, the decoder and the store agree with it.
//!
//! The second is the seam, and it turned out to be worth more than the first.
//! Every defect this test has found so far was two halves of one seam that
//! had never actually met, and none of them was visible in either half alone.
//! `POST /images/{kind}` beside `GET /images/{file}` is one route in axum's
//! eyes wearing two parameter names, so registering both panicked while the
//! router was being built — before the port was bound, so every test in this
//! crate failed at its first request with "connection refused" and none of
//! them named a route. Then `upload` answered with a bare id while `serve`
//! wanted a filename ending in `.jpg`, so the first fetch of a stored sleeve
//! answered 404 with both halves behaving exactly as written.
//!
//! Hence the shape of the test below: it uploads and then *fetches back*,
//! rather than trusting a 200. The harness now prints the gateway's own
//! stderr when the port never binds, so the first of those reads as a panic
//! instead of a refused connection.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http_bytes, json_field, login, spawn_gateway_with};

/// A picture of a given shape, as JPEG bytes — deliberately not of the shape
/// it is being uploaded for.
fn a_picture(width: u32, height: u32) -> Vec<u8> {
    let mut img = image::RgbImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        // A gradient rather than a flat colour: a flat picture survives a
        // squash unchanged and would pass this test for the wrong reason.
        *pixel = image::Rgb([(x % 251) as u8, (y % 241) as u8, 96]);
    }
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .expect("encode a jpeg");
    out.into_inner()
}

/// The size the picture came back at.
fn shape_of(bytes: &[u8]) -> (u32, u32) {
    let decoded = image::load_from_memory(bytes).expect("the answer is an image");
    (decoded.width(), decoded.height())
}

#[test]
fn a_picture_becomes_a_sleeve_of_the_one_size_a_sleeve_is_drawn_at() {
    // Uploads are off for the whole suite (a test that writes images into the
    // working directory leaves a mess), so the one test that wants them says
    // so, and names a directory of its own.
    let dir = std::env::temp_dir().join(format!("baylee-images-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let gw = spawn_gateway_with(
        "images",
        &[("BAYLEE_DECK_IMAGE_PATH", dir.to_string_lossy().into_owned())],
    );
    let token = login(gw.port, "sleeves@example.com", "sleeve_fan");

    // A panorama: twenty to one, against a sleeve's 63:88. Nothing about it
    // fits, which is the point.
    let wide = a_picture(2000, 100);
    let (status, body) = http_bytes(
        gw.port,
        "POST",
        "/images?kind=sleeve",
        Some(&token),
        "image/jpeg",
        &wide,
    );
    let body = String::from_utf8_lossy(&body).to_string();
    assert_eq!(status, 200, "upload a sleeve: {body}");
    assert!(
        body.contains("\"kind\":\"sleeve\""),
        "the kind comes back: {body}"
    );
    let id = json_field(&body, "id").to_string();

    let (status, stored) = http_bytes(gw.port, "GET", &format!("/images/{id}"), None, "", &[]);
    assert_eq!(status, 200, "fetching it back is unauthenticated");
    assert_eq!(
        shape_of(&stored),
        (488, 680),
        "a sleeve is stored at one size, whatever shape arrived"
    );

    // The id is the hash of what was stored, so the same picture twice is one
    // file — and the second upload is the same answer, not a new one.
    let (status, again) = http_bytes(
        gw.port,
        "POST",
        "/images?kind=sleeve",
        Some(&token),
        "image/jpeg",
        &wide,
    );
    let again = String::from_utf8_lossy(&again).to_string();
    assert_eq!(status, 200, "upload it again: {again}");
    assert_eq!(json_field(&again, "id"), id, "one picture, one file");

    // The same picture under the other kind is a different picture, because
    // it is cut and scaled to a different shape.
    let (status, mat) = http_bytes(
        gw.port,
        "POST",
        "/images?kind=playmat",
        Some(&token),
        "image/jpeg",
        &wide,
    );
    let mat = String::from_utf8_lossy(&mat).to_string();
    assert_eq!(status, 200, "upload a playmat: {mat}");
    let mat_id = json_field(&mat, "id").to_string();
    assert_ne!(
        mat_id, id,
        "a sleeve and a mat of one picture are two files"
    );
    let (_, stored) = http_bytes(gw.port, "GET", &format!("/images/{mat_id}"), None, "", &[]);
    assert_eq!(shape_of(&stored), (1024, 512), "a mat is two to one");

    // A kind the gateway does not draw, and no kind at all, are the same
    // answer: there is no such picture to make.
    for path in ["/images?kind=tablecloth", "/images"] {
        let (status, _) = http_bytes(gw.port, "POST", path, Some(&token), "image/jpeg", &wide);
        assert_eq!(status, 404, "{path} names no kind of picture");
    }

    let _ = std::fs::remove_dir_all(&dir);
}
