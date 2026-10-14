//! Container checks decode independently; geometry assertions use literal
//! output coordinates rather than reusing the rendering constants/formulas.
use super::*;

fn le16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}
fn le32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn be32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap())
}

#[test]
fn windows_icons_have_all_seven_bmp_sizes_alpha_and_bottom_up_rows() {
    let sizes = [16, 24, 32, 48, 64, 128, 256];
    let images: Vec<_> = sizes
        .iter()
        .map(|&n| {
            RgbaImage::from_fn(n, n, |_, y| {
                if y == 0 {
                    Rgba([10, 20, 30, 0])
                } else {
                    Rgba([40, 50, 60, 255])
                }
            })
        })
        .collect();
    let bytes = formats::ico(&images).unwrap();
    assert_eq!(&bytes[..4], &[0, 0, 1, 0]);
    assert_eq!(le16(&bytes, 4), 7);
    let mut end = 6 + 7 * 16;
    for (i, size) in sizes.into_iter().enumerate() {
        let row = 6 + i * 16;
        let side = if size == 256 {
            0
        } else {
            u8::try_from(size).unwrap()
        };
        assert_eq!(&bytes[row..row + 4], &[side, side, 0, 0]);
        assert_eq!(le16(&bytes, row + 4), 1);
        assert_eq!(le16(&bytes, row + 6), 32);
        let n = usize::try_from(size).unwrap();
        let offset = usize::try_from(le32(&bytes, row + 12)).unwrap();
        assert_eq!(offset, end);
        let length = usize::try_from(le32(&bytes, row + 8)).unwrap();
        assert_eq!(length, 40 + n * n * 4 + n.div_ceil(32) * 4 * n);
        assert_eq!(le32(&bytes, offset), 40);
        assert_eq!(le32(&bytes, offset + 4), size);
        assert_eq!(le32(&bytes, offset + 8), size * 2);
        assert_eq!(&bytes[offset + 40..offset + 44], &[60, 50, 40, 255]);
        let top = offset + 40 + (n - 1) * n * 4;
        assert_eq!(&bytes[top..top + 4], &[30, 20, 10, 0]);
        let mask = offset + 40 + n * n * 4;
        assert_eq!(bytes[mask], 0);
        assert_eq!(bytes[mask + (n - 1) * n.div_ceil(32) * 4], 255);
        end = offset + length;
    }
    assert_eq!(end, bytes.len());
    assert!(formats::ico(&[RgbaImage::new(257, 257)]).is_err());
    assert!(formats::ico(&[RgbaImage::new(16, 15)]).is_err());
}

#[test]
fn mac_icons_contain_every_point_size_and_retina_representation() {
    let bytes = formats::icns(|n| RgbaImage::from_pixel(n, n, Rgba([12, 34, 56, 255]))).unwrap();
    assert_eq!(&bytes[..4], b"icns");
    assert_eq!(usize::try_from(be32(&bytes, 4)).unwrap(), bytes.len());
    let mut seen = std::collections::BTreeMap::new();
    let mut at = 8;
    while at < bytes.len() {
        let kind = std::str::from_utf8(&bytes[at..at + 4]).unwrap().to_owned();
        let end = at + usize::try_from(be32(&bytes, at + 4)).unwrap();
        let png = image::load_from_memory_with_format(&bytes[at + 8..end], image::ImageFormat::Png)
            .unwrap()
            .into_rgba8();
        assert_eq!(png.width(), png.height());
        assert_eq!(png.get_pixel(0, 0).0, [12, 34, 56, 255]);
        assert!(seen.insert(kind, png.width()).is_none());
        at = end;
    }
    // ICNS type codes distinguish points from pixels (ic13 is 128pt@2x).
    let expected = [
        ("icp4", 16),
        ("ic11", 32),
        ("icp5", 32),
        ("ic12", 64),
        ("ic07", 128),
        ("ic13", 256),
        ("ic08", 256),
        ("ic14", 512),
        ("ic09", 512),
        ("ic10", 1024),
    ]
    .map(|(k, n)| (k.to_owned(), n))
    .into_iter()
    .collect();
    assert_eq!(seen, expected);
    assert_eq!(at, bytes.len());
}

#[test]
fn icon_geometry_preserves_cropped_small_art_insets_and_macos_shadow() {
    let painting = RgbaImage::from_pixel(100, 100, Rgba([200, 80, 40, 255]));
    for size in [16, 24, 32] {
        let flat = icon(&painting, size, false);
        assert_eq!(flat, icon(&painting, size, true));
        assert_eq!(flat.get_pixel(0, 0)[3], 0);
        assert_eq!(flat.get_pixel(size / 2, 0).0, [200, 80, 40, 255]);
    }
    let flat = icon(&painting, 128, false);
    assert_eq!(flat.get_pixel(64, 3)[3], 0);
    assert_eq!(flat.get_pixel(64, 4).0, [200, 80, 40, 255]);
    let mac = icon(&painting, 128, true);
    assert_eq!(mac.get_pixel(64, 13).0, [200, 80, 40, 255]);
    assert_eq!(mac.get_pixel(64, 0)[3], 0);
    assert!(mac.get_pixel(64, 117)[3] > 0 && mac.get_pixel(64, 117)[3] < 128);
    // Crop excludes a bright outer border at small sizes, but large icons
    // keep it: a square-only fixture would not detect a lost crop.
    let bordered = RgbaImage::from_fn(100, 100, |x, y| {
        if !(4..96).contains(&x) || !(4..96).contains(&y) {
            Rgba([255, 255, 255, 255])
        } else {
            Rgba([0, 0, 0, 255])
        }
    });
    assert!(icon(&bordered, 16, false).get_pixel(8, 0)[0] < 20);
    assert!(icon(&bordered, 128, false).get_pixel(64, 4)[0] > 230);
}

#[test]
fn installer_background_keeps_retina_dimensions_gradient_arrow_and_lettering() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../baylee-client/assets/fonts");
    let fonts = Fonts::load(&root).unwrap();
    for scale in [1, 2] {
        let image = background(scale, &fonts);
        assert_eq!(image.dimensions(), (660 * scale, 420 * scale));
        assert_eq!(image.get_pixel(0, 0).0, [20, 27, 52, 255]);
        assert_eq!(image.get_pixel(0, 420 * scale - 1).0, [128, 140, 182, 255]);
        assert_eq!(
            image.get_pixel(330 * scale, 205 * scale).0,
            [243, 213, 138, 255]
        );
        assert_ne!(
            image.get_pixel(170 * scale, 205 * scale).0,
            [243, 213, 138, 255]
        );
        let title =
            imageops::crop_imm(&image, 180 * scale, 40 * scale, 300 * scale, 30 * scale).to_image();
        assert!(title.pixels().filter(|p| p[0] > 200).count() > 100);
        let legal = imageops::crop_imm(&image, 100 * scale, 389 * scale, 460 * scale, 20 * scale)
            .to_image();
        assert!(legal.pixels().filter(|p| p[0] < 50).count() > 100);
        assert!(image.pixels().all(|p| p[3] == 255));
    }
}
