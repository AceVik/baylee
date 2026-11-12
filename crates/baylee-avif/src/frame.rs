//! A decoded frame's planes, packed without the decoder's row padding, and
//! their conversion to RGBA for a picture drawn as an ordinary image.

/// How luma and chroma were mixed from RGB (the sequence header's matrix
/// coefficients, ITU-T H.273 table 4). Central writes BT.601 coefficients;
/// anything this crate does not know is read as BT.601 as well.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Matrix {
    /// BT.601 (H.273 values 5 and 6).
    #[default]
    Bt601,
    /// BT.709 (H.273 value 1).
    Bt709,
}

impl Matrix {
    /// The matrix an H.273 `matrix_coefficients` value names.
    #[must_use]
    pub fn from_h273(value: u32) -> Self {
        if value == 1 { Self::Bt709 } else { Self::Bt601 }
    }

    /// `(kr, kb)`: the red and blue weights of luma.
    fn weights(self) -> (f32, f32) {
        match self {
            Self::Bt601 => (0.299, 0.114),
            Self::Bt709 => (0.2126, 0.0722),
        }
    }

    /// The four coefficients a shader turns centred chroma into RGB with:
    /// `r = y + a·v`, `g = y − b·u − c·v`, `b = y + d·u`.
    #[must_use]
    pub fn coefficients(self) -> [f32; 4] {
        let (kr, kb) = self.weights();
        let kg = 1.0 - kr - kb;
        [
            2.0 * (1.0 - kr),
            2.0 * (1.0 - kb) * kb / kg,
            2.0 * (1.0 - kr) * kr / kg,
            2.0 * (1.0 - kb),
        ]
    }
}

/// One 8-bit 4:2:0 frame: full-size luma, half-size chroma (rounded up), and
/// an alpha plane of luma's size when the picture is transparent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Luma, `width × height` bytes, row after row.
    pub y: Vec<u8>,
    /// Blue-difference chroma, `chroma_width × chroma_height` bytes.
    pub u: Vec<u8>,
    /// Red-difference chroma, the same size as `u`.
    pub v: Vec<u8>,
    /// Opacity, `width × height` bytes, when the picture has an alpha track.
    pub alpha: Option<Vec<u8>>,
    /// How chroma was mixed.
    pub matrix: Matrix,
    /// Whether the samples use the full 0–255 range rather than 16–235.
    pub full_range: bool,
}

impl Frame {
    /// The chroma planes' width.
    #[must_use]
    pub fn chroma_width(&self) -> u32 {
        self.width.div_ceil(2)
    }

    /// The chroma planes' height.
    #[must_use]
    pub fn chroma_height(&self) -> u32 {
        self.height.div_ceil(2)
    }

    /// The frame as 8-bit sRGB-encoded RGBA, row after row, into `out`
    /// (resized to fit, so a buffer kept between frames is reused). Chroma is
    /// taken from the nearest sample, which at a picture's own size is what
    /// a 4:2:0 frame holds; an opaque frame's alpha is 255.
    pub fn to_rgba(&self, out: &mut Vec<u8>) {
        let (w, h) = (self.width as usize, self.height as usize);
        let cw = self.chroma_width() as usize;
        out.resize(w * h * 4, 0);
        // What each chroma sample adds to each colour: red takes Cr, green
        // gives up some Cb and some Cr, blue takes Cb.
        let [red_cr, green_cb, green_cr, blue_cb] = self.matrix.coefficients();
        let (y_off, y_scale, c_scale) = if self.full_range {
            (0.0, 1.0 / 255.0, 1.0 / 255.0)
        } else {
            (16.0, 1.0 / 219.0, 1.0 / 224.0)
        };
        for row in 0..h {
            let crow = (row / 2) * cw;
            for col in 0..w {
                let at = row * w + col;
                let luma = (f32::from(self.y[at]) - y_off) * y_scale;
                let cu = (f32::from(self.u[crow + col / 2]) - 128.0) * c_scale;
                let cv = (f32::from(self.v[crow + col / 2]) - 128.0) * c_scale;
                let px = &mut out[at * 4..at * 4 + 4];
                px[0] = to_byte(luma + red_cr * cv);
                px[1] = to_byte(luma - green_cb * cu - green_cr * cv);
                px[2] = to_byte(luma + blue_cb * cu);
                px[3] = self.alpha.as_ref().map_or(255, |alpha| alpha[at]);
            }
        }
    }
}

fn to_byte(x: f32) -> u8 {
    (x * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(y: u8, u: u8, v: u8, full_range: bool) -> Frame {
        Frame {
            width: 3,
            height: 3,
            y: vec![y; 9],
            u: vec![u; 4],
            v: vec![v; 4],
            alpha: None,
            matrix: Matrix::Bt601,
            full_range,
        }
    }

    #[test]
    fn grey_stays_grey_and_the_limited_range_is_stretched() {
        let mut out = Vec::new();
        flat(128, 128, 128, true).to_rgba(&mut out);
        assert_eq!(&out[..4], &[128, 128, 128, 255]);
        flat(16, 128, 128, false).to_rgba(&mut out);
        assert_eq!(&out[..4], &[0, 0, 0, 255]);
        flat(235, 128, 128, false).to_rgba(&mut out);
        assert_eq!(&out[..4], &[255, 255, 255, 255]);
        assert_eq!(out.len(), 3 * 3 * 4);
    }

    #[test]
    fn a_pure_red_comes_back_red_under_both_matrices() {
        for matrix in [Matrix::Bt601, Matrix::Bt709] {
            let (kr, kb) = matrix.weights();
            let y = kr;
            let u = (0.0 - y) / (2.0 * (1.0 - kb));
            let v = (1.0 - y) / (2.0 * (1.0 - kr));
            let byte = |x: f32| (x * 255.0).round() as u8;
            let mut frame = flat(byte(y), byte(u + 0.5), byte(v + 0.5), true);
            frame.matrix = matrix;
            let mut out = Vec::new();
            frame.to_rgba(&mut out);
            assert!(
                out[0] >= 252 && out[1] <= 3 && out[2] <= 3,
                "{matrix:?}: {:?}",
                &out[..4]
            );
        }
    }

    #[test]
    fn alpha_is_carried_through_and_odd_sizes_round_chroma_up() {
        let mut frame = flat(128, 128, 128, true);
        frame.alpha = Some((0..9).collect());
        assert_eq!((frame.chroma_width(), frame.chroma_height()), (2, 2));
        let mut out = Vec::new();
        frame.to_rgba(&mut out);
        let alphas: Vec<u8> = out.chunks(4).map(|p| p[3]).collect();
        assert_eq!(alphas, (0..9).collect::<Vec<u8>>());
    }
}
