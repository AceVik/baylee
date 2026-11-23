//! Advent artwork is acquired with a private, account- or table-bound scope.
//! The decoder checks its own bounds again when creating the GPU texture.
use super::{Account, Error};
use crate::advent::{AssetMeta, AssetScope, Mime, Request, Response};
use image::{ImageDecoder as _, codecs::webp::WebPDecoder};
use std::io::Cursor;

impl Account {
    /// Access the current account's Advent campaign, inventory or table.
    ///
    /// # Errors
    /// Refuses mismatched operations, expired sessions and disallowed views.
    pub fn advent(&self, request: Request) -> Result<Response, Error> {
        self.session.advent(&self.http, request)
    }

    /// Fetch an authorized immutable image without writing it to a shared cache.
    ///
    /// # Errors
    /// Refuses stale or substituted metadata, missing permission and invalid image geometry.
    pub fn advent_asset(&self, scope: AssetScope, meta: &AssetMeta) -> Result<Vec<u8>, Error> {
        meta.validate()?;
        let (actual, bytes) = self.session.advent_asset(&self.http, scope, None)?;
        if actual != *meta {
            return Err(Error::Conflict);
        }
        let bytes = bytes.ok_or(Error::Rejected)?;
        // The shared SDK verified the digest while assembling the authorized pages.
        check_geometry(&actual, &bytes)?;
        Ok(bytes)
    }
}

/// Check a complete ordinary-table transfer before handing a buffer to the renderer.
///
/// # Errors
/// Refuses mismatched lengths, digests, formats or dimensions. Entitlement comes
/// from the authenticated table, so this function alone never authorizes a read.
pub fn check_advent_image(meta: &AssetMeta, bytes: &[u8]) -> Result<(), Error> {
    meta.verify(bytes)?;
    check_geometry(meta, bytes)
}

fn check_geometry(meta: &AssetMeta, bytes: &[u8]) -> Result<(), Error> {
    let Mime::Webp = meta.mime;
    let decoder = WebPDecoder::new(Cursor::new(bytes)).map_err(|_| Error::Rejected)?;
    if decoder.has_animation()
        || decoder.dimensions() != (u32::from(meta.width), u32::from(meta.height))
    {
        return Err(Error::Rejected);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advent::Finish;
    use image::codecs::webp::WebPEncoder;
    use sha2::{Digest, Sha256};

    #[test]
    fn verified_seat_asset_must_match_exact_dimensions_digest_and_static_encoding() {
        let mut bytes = vec![];
        WebPEncoder::new_lossless(&mut bytes)
            .encode(&[125; 24], 4, 2, image::ExtendedColorType::Rgb8)
            .unwrap();
        let meta = AssetMeta {
            tier: Finish::Standard,
            digest: Sha256::digest(&bytes).into(),
            size: u32::try_from(bytes.len()).unwrap(),
            mime: Mime::Webp,
            mat_framing: None,
            width: 4,
            height: 2,
        };
        assert!(check_advent_image(&meta, &bytes).is_ok());
        for bad in [
            AssetMeta {
                digest: [0; 32],
                ..meta
            },
            AssetMeta { width: 1, ..meta },
            AssetMeta {
                size: meta.size + 1,
                ..meta
            },
        ] {
            assert!(check_advent_image(&bad, &bytes).is_err());
        }
        let invalid = b"ordinary text";
        assert!(
            check_advent_image(
                &AssetMeta {
                    digest: Sha256::digest(invalid).into(),
                    size: 13,
                    ..meta
                },
                invalid
            )
            .is_err()
        );
    }
}
