//! IEEE-754 half-precision (`binary16`) tensor quantization and bulk byte framing.
//!
//! Baylee AI (`PlayerNet4` / `PlayerNet5`) executes in `FP16` internally, while
//! the outer ONNX Runtime graph boundary passes `f32` slices. Storing recurrent
//! hidden states (`h_states`, `9 × d_model` per seat) as `u16` (`binary16`) in
//! Hot RAM and Cold SSD swap cuts memory bandwidth and footprint by 50% with
//! zero loss relative to the `FP16` model weights.

use anyhow::bail;

/// Converts a finite 32-bit IEEE-754 `f32` into a 16-bit IEEE-754 `binary16` (`u16`)
/// with round-to-nearest-even semantics, clamping finite overflow to `±65504.0` (`0x7bff`)
/// to match ONNX Runtime's `convert_float_to_float16`.
#[inline]
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Explicit IEEE-754 binary32 -> binary16 bit manipulation"
)]
pub const fn f32_to_f16(val: f32) -> u16 {
    let bits = val.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let raw_exp = (bits >> 23) & 0xff;
    let mant = bits & 0x007f_ffff;

    if raw_exp == 0xff {
        let nan_bits = if mant != 0 {
            0x0200 | ((mant >> 13) as u16)
        } else {
            0
        };
        return sign | 0x7c00 | nan_bits;
    }

    let exp = (raw_exp as i32) - 127 + 15;
    if exp <= 0 {
        if exp < -10 {
            return sign;
        }
        let full_mant = mant | 0x0080_0000;
        let shift = (14 - exp) as u32;
        let round_bit = 1_u32 << (shift - 1);
        let sticky = (full_mant & (round_bit - 1)) != 0;
        let lsb = (full_mant >> shift) & 1;
        let mut h_mant = (full_mant >> shift) as u16;
        if (full_mant & round_bit) != 0 && (sticky || lsb != 0) {
            h_mant += 1;
        }
        sign | h_mant
    } else if exp >= 31 {
        sign | 0x7bff
    } else {
        let round_bit = 0x1000_u32;
        let sticky = (mant & 0x0fff) != 0;
        let lsb = (mant >> 13) & 1;
        let mut h_bits = sign | ((exp as u16) << 10) | ((mant >> 13) as u16);
        if (mant & round_bit) != 0 && (sticky || lsb != 0) {
            h_bits += 1;
            if (h_bits & 0x7c00) == 0x7c00 {
                h_bits = sign | 0x7bff;
            }
        }
        h_bits
    }
}

/// Fast branch-minimal conversion of a 16-bit IEEE-754 `binary16` (`u16`) into `f32`.
#[inline]
#[must_use]
pub const fn f16_to_f32(bits: u16) -> f32 {
    let sign = ((bits & 0x8000) as u32) << 16;
    let exp = ((bits >> 10) & 0x1f) as u32;
    let mant = (bits & 0x03ff) as u32;
    if exp == 0 {
        if mant == 0 {
            return f32::from_bits(sign);
        }
        let shift = mant.leading_zeros() - 21;
        let norm_mant = (mant << shift) & 0x03ff;
        let f32_exp = (127 - 15 + 1 - shift) << 23;
        f32::from_bits(sign | f32_exp | (norm_mant << 13))
    } else if exp == 31 {
        f32::from_bits(sign | 0x7f80_0000 | (mant << 13))
    } else {
        let f32_exp = (exp + 127 - 15) << 23;
        f32::from_bits(sign | f32_exp | (mant << 13))
    }
}

/// Converts a 16-bit IEEE-754 `binary16` (`u16`) into `f32`, rejecting `NaN`/`Inf`.
///
/// # Errors
/// Returns an error if `bits` encodes `NaN` or `±Inf` (`(bits & 0x7c00) == 0x7c00`).
#[inline]
pub fn f16_to_f32_checked(bits: u16) -> anyhow::Result<f32> {
    if (bits & 0x7c00) == 0x7c00 {
        bail!("snapshot h_state contains non-finite FP16 value (0x{bits:04x})");
    }
    Ok(f16_to_f32(bits))
}

/// Quantizes `src` (`f32`) into `dst` (`u16` `binary16`), reusing `dst`'s
/// existing heap allocation whenever possible.
#[inline]
pub fn pack_f32_slice_to_f16(src: &[f32], dst: &mut Vec<u16>) {
    dst.resize(src.len(), 0);
    for (out, &val) in dst.iter_mut().zip(src.iter()) {
        *out = f32_to_f16(val);
    }
}

/// Dequantizes `src` (`u16` `binary16`) into `dst` (`f32`), reusing `dst`'s
/// existing heap allocation whenever possible.
#[inline]
pub fn unpack_f16_slice_to_f32(src: &[u16], dst: &mut Vec<f32>) {
    dst.resize(src.len(), 0.0);
    for (out, &word) in dst.iter_mut().zip(src.iter()) {
        *out = f16_to_f32(word);
    }
}

/// Appends a slice of `u16` values as little-endian bytes into `out` using
/// 8-byte bulk writes.
#[inline]
pub fn extend_u16_le_bytes(words: &[u16], out: &mut Vec<u8>) {
    out.reserve(words.len() * 2);
    let (quads, rem) = words.as_chunks::<4>();
    for quad in quads {
        let packed = u64::from(quad[0])
            | (u64::from(quad[1]) << 16)
            | (u64::from(quad[2]) << 32)
            | (u64::from(quad[3]) << 48);
        out.extend_from_slice(&packed.to_le_bytes());
    }
    for &r in rem {
        out.extend_from_slice(&r.to_le_bytes());
    }
}

/// Unpacks a little-endian byte slice (whose length must be a multiple of 2)
/// into `dst` (`Vec<u16>`), validating that every `FP16` word is finite.
///
/// # Errors
/// Returns an error if any unpacked `u16` is `NaN` or `±Inf`.
#[inline]
#[allow(
    clippy::cast_possible_truncation,
    reason = "Explicit 16-bit lane extraction from 64-bit little-endian word"
)]
pub fn unpack_u16_le_bytes_into(raw_bytes: &[u8], dst: &mut Vec<u16>) -> anyhow::Result<()> {
    let count = raw_bytes.len() / 2;
    dst.resize(count, 0);
    let (quads, rem_words) = dst.as_chunks_mut::<4>();
    let (byte_octs, byte_rem) = raw_bytes.as_chunks::<8>();
    for (quad, oct) in quads.iter_mut().zip(byte_octs.iter()) {
        let word = u64::from_le_bytes(*oct);
        let w0 = word as u16;
        let w1 = (word >> 16) as u16;
        let w2 = (word >> 32) as u16;
        let w3 = (word >> 48) as u16;
        if (w0 & 0x7c00) == 0x7c00
            || (w1 & 0x7c00) == 0x7c00
            || (w2 & 0x7c00) == 0x7c00
            || (w3 & 0x7c00) == 0x7c00
        {
            bail!("snapshot cold h_state contains non-finite FP16 value");
        }
        *quad = [w0, w1, w2, w3];
    }
    let (byte_pairs, _) = byte_rem.as_chunks::<2>();
    for (out, &pair) in rem_words.iter_mut().zip(byte_pairs.iter()) {
        let bits = u16::from_le_bytes(pair);
        if (bits & 0x7c00) == 0x7c00 {
            bail!("snapshot cold h_state contains non-finite FP16 value");
        }
        *out = bits;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fp16_roundtrip_and_bulk_le_framing() {
        let values = [-12.5_f32, -1.0, -0.25, 0.0, 0.125, 0.5, 1.0, 42.0, 256.0];
        let mut half = Vec::new();
        pack_f32_slice_to_f16(&values, &mut half);
        let mut back = Vec::new();
        unpack_f16_slice_to_f32(&half, &mut back);
        assert_eq!(back, values);

        let mut bytes = Vec::new();
        extend_u16_le_bytes(&half, &mut bytes);
        assert_eq!(bytes.len(), half.len() * 2);

        let mut unpacked = Vec::new();
        unpack_u16_le_bytes_into(&bytes, &mut unpacked).unwrap();
        assert_eq!(unpacked, half);
    }
}
