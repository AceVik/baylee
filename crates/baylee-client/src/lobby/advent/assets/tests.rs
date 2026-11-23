use super::{AssetMeta, AssetScope, Library, Pending, decode};
use baylee_account::advent::{Finish, Kind, Mime};
use sha2::{Digest as _, Sha256};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

fn synthetic() -> (AssetMeta, Vec<u8>) {
    let mut bytes = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut bytes)
        .encode(&[78; 18], 3, 2, image::ExtendedColorType::Rgb8)
        .unwrap();
    (
        AssetMeta {
            size: bytes.len() as u32,
            digest: Sha256::digest(&bytes).into(),
            width: 3,
            height: 2,
            tier: Finish::Special,
            mime: Mime::Webp,
        },
        bytes,
    )
}
#[test]
fn only_a_verified_static_image_becomes_a_gpu_texture() {
    let (meta, bytes) = synthetic();
    let image = decode(&bytes, meta).unwrap();
    assert_eq!(image.texture_descriptor.size.width, 3);
    assert_eq!(image.texture_descriptor.size.height, 2);
    assert!(decode(&bytes, AssetMeta { height: 3, ..meta }).is_err());
    assert!(
        decode(
            &bytes,
            AssetMeta {
                digest: [0; 32],
                ..meta
            }
        )
        .is_err()
    );
    assert!(decode(&bytes[..bytes.len() - 1], meta).is_err());
}
#[test]
fn replacing_or_revoking_a_scope_cancels_pending_pages_without_crossing_table_or_owner() {
    let (meta, _) = synthetic();
    let mut library = Library::default();
    let stop = Arc::new(AtomicBool::new(false));
    let scope = AssetScope::Equipped {
        match_id: [1; 16],
        seat: 0,
        kind: Kind::Sleeve,
    };
    let (_send, rx) = mpsc::sync_channel(1);
    library.pending = Some(Pending {
        scope,
        meta,
        cancelled: Arc::clone(&stop),
        rx: Mutex::new(rx),
    });
    library.want(scope, meta);
    assert!(!stop.load(Ordering::Acquire));
    assert!(library.queue.is_empty());
    library.want(
        scope,
        AssetMeta {
            digest: [3; 32],
            ..meta
        },
    );
    assert!(stop.load(Ordering::Acquire));
    assert_eq!(library.queue.len(), 1);
    let owned = AssetScope::Owned { item: [4; 16] };
    library.want(owned, meta);
    library.retain(|s| *s != scope);
    assert_eq!(library.queue.len(), 1);
    assert_eq!(library.queue[0].scope, owned);
}
