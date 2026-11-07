//! `/images`: what the image assets hold on the GPU, largest first.
//!
//! The render world's own targets are not assets: a view's colour, depth and
//! multisample textures follow from the window, and `docs/perf-client.md` adds
//! them up. This is the rest, for telling a texture budget from a render target.

use bevy::prelude::*;

/// How many images an answer names.
const LISTED: usize = 16;

/// Answers one `/images`: a count, a sum in bytes and the largest.
pub(super) fn answer(images: &Assets<Image>) -> String {
    let mut sizes: Vec<(u64, u32, u32, u32, String)> = images
        .iter()
        .map(|(_, image)| {
            let size = image.texture_descriptor.size;
            let bytes = image.data.as_ref().map_or(0, Vec::len) as u64;
            let block = u64::from(
                image
                    .texture_descriptor
                    .format
                    .block_copy_size(None)
                    .unwrap_or(4),
            );
            let drawn = u64::from(size.width)
                * u64::from(size.height)
                * u64::from(size.depth_or_array_layers)
                * block;
            (
                bytes.max(drawn),
                size.width,
                size.height,
                image.texture_descriptor.mip_level_count,
                format!("{:?}", image.texture_descriptor.format),
            )
        })
        .collect();
    let total: u64 = sizes.iter().map(|size| size.0).sum();
    let count = sizes.len();
    sizes.sort_by_key(|size| std::cmp::Reverse(size.0));
    let largest: Vec<String> = sizes
        .iter()
        .take(LISTED)
        .map(|(bytes, width, height, mips, format)| {
            format!(
                "{{\"bytes\":{bytes},\"size\":\"{width}x{height}\",\"mips\":{mips},\
                 \"format\":\"{format}\"}}"
            )
        })
        .collect();
    format!(
        "{{\"ok\":true,\"images\":{count},\"bytes\":{total},\"largest\":[{}]}}",
        largest.join(",")
    )
}
