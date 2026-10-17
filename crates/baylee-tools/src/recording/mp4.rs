//! Pure-Rust H.264 (`avc1`) ISO-BMFF (`.mp4`) encoder without external tools.
//!
//! Uses Baseline Profile (`profile_idc = 66`) with `I_PCM` macroblocks and
//! `P_Skip` runs across unchanged macroblocks between frames, plus periodic
//! IDR sync samples (`stss`) and exact per-frame durations (`stts`).
use anyhow::{Result, ensure};
use image::RgbImage;
use std::io::{Seek, SeekFrom, Write};

const KEYFRAME_INTERVAL: usize = 30;
const TIMESCALE_MS: u32 = 1000;

pub(super) struct Animation<W> {
    output: W,
    size: (u32, u32),
    mdat_start: u64,
    offsets: Vec<u32>,
    sizes: Vec<u32>,
    delays: Vec<u32>,
    sync_samples: Vec<u32>,
    previous_mbs: Option<Vec<[u8; 384]>>,
    gop_frame_num: u32,
    idr_pic_id: u32,
}

impl<W: Write + Seek> Animation<W> {
    pub fn new(mut output: W, size: (u32, u32)) -> Result<Self> {
        ensure!(
            (1..=4096).contains(&size.0) && (1..=4096).contains(&size.1),
            "MP4 dimensions must be 1..4096"
        );
        let ftyp = mp4_box(*b"ftyp", b"isom\x00\x00\x02\x00isomiso2avc1mp41");
        output.write_all(&ftyp)?;
        let mdat_start = output.stream_position()?;
        output.write_all(&[0, 0, 0, 0, b'm', b'd', b'a', b't'])?;
        Ok(Self {
            output,
            size,
            mdat_start,
            offsets: Vec::new(),
            sizes: Vec::new(),
            delays: Vec::new(),
            sync_samples: Vec::new(),
            previous_mbs: None,
            gop_frame_num: 0,
            idr_pic_id: 0,
        })
    }

    pub fn frame(&mut self, image: &RgbImage, delay_ms: u32) -> Result<()> {
        ensure!(
            image.dimensions() == self.size && (1..=60_000).contains(&delay_ms),
            "invalid MP4 frame dimensions/duration"
        );
        let mbs = frame_macroblocks(image.as_raw(), self.size.0, self.size.1);
        let is_idr =
            self.previous_mbs.is_none() || self.offsets.len().is_multiple_of(KEYFRAME_INTERVAL);
        let sample = if is_idr {
            self.gop_frame_num = 0;
            let encoded = encode_slice(&mbs, None, 0, self.idr_pic_id);
            self.idr_pic_id = (self.idr_pic_id + 1) & 0xFF;
            self.sync_samples
                .push(u32::try_from(self.offsets.len() + 1)?);
            encoded
        } else {
            self.gop_frame_num = (self.gop_frame_num + 1) & 0x3FF;
            encode_slice(&mbs, self.previous_mbs.as_deref(), self.gop_frame_num, 0)
        };
        self.previous_mbs = Some(mbs);
        let offset = u32::try_from(self.output.stream_position()?)?;
        let length = u32::try_from(sample.len())?;
        self.output.write_all(&sample)?;
        self.offsets.push(offset);
        self.sizes.push(length);
        self.delays.push(delay_ms);
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        ensure!(!self.offsets.is_empty(), "MP4 contains no frames");
        let mdat_end = self.output.stream_position()?;
        let mdat_len = u32::try_from(mdat_end - self.mdat_start)?;
        self.output.seek(SeekFrom::Start(self.mdat_start))?;
        self.output.write_all(&mdat_len.to_be_bytes())?;
        self.output.seek(SeekFrom::Start(mdat_end))?;

        let moov = build_moov(
            self.size,
            &self.offsets,
            &self.sizes,
            &self.delays,
            &self.sync_samples,
        )?;
        self.output.write_all(&moov)?;
        Ok(())
    }
}

struct BitWriter {
    bytes: Vec<u8>,
    cur: u8,
    bits: u8,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            cur: 0,
            bits: 0,
        }
    }

    fn bit(&mut self, b: u8) {
        self.cur = (self.cur << 1) | (b & 1);
        self.bits += 1;
        if self.bits == 8 {
            self.bytes.push(self.cur);
            self.cur = 0;
            self.bits = 0;
        }
    }

    fn bits(&mut self, val: u32, count: u8) {
        for shift in (0..count).rev() {
            self.bit(u8::try_from((val >> shift) & 1).unwrap_or(0));
        }
    }

    fn ue(&mut self, val: u32) {
        let v = val + 1;
        let zeros = v.ilog2();
        for _ in 0..zeros {
            self.bit(0);
        }
        self.bits(v, u8::try_from(zeros + 1).expect("<= 32 bits"));
    }

    fn align_zero(&mut self) {
        while self.bits != 0 {
            self.bit(0);
        }
    }

    fn rbsp_trailing(&mut self) {
        self.bit(1);
        self.align_zero();
    }

    fn raw_bytes(&mut self, data: &[u8]) {
        debug_assert_eq!(self.bits, 0);
        self.bytes.extend_from_slice(data);
    }
}

fn ebsp_from_rbsp(nal_header: u8, rbsp: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rbsp.len() + 16);
    out.push(nal_header);
    let mut zeros = 0_u32;
    for &b in rbsp {
        if zeros >= 2 && b <= 3 {
            out.push(3);
            zeros = 0;
        }
        out.push(b);
        if b == 0 {
            zeros += 1;
        } else {
            zeros = 0;
        }
    }
    out
}

fn make_sps(width: u32, height: u32) -> Vec<u8> {
    let even_w = width + (width & 1);
    let even_h = height + (height & 1);
    let mb_w = even_w.div_ceil(16);
    let mb_h = even_h.div_ceil(16);
    let crop_r = (mb_w * 16 - even_w) / 2;
    let crop_b = (mb_h * 16 - even_h) / 2;
    let mut bw = BitWriter::new();
    bw.bits(66, 8); // Baseline profile
    bw.bits(0xC0, 8); // constraint_set0 + constraint_set1
    bw.bits(40, 8); // Level 4.0
    bw.ue(0); // seq_parameter_set_id
    bw.ue(6); // log2_max_frame_num_minus4 (10-bit frame_num)
    bw.ue(2); // pic_order_cnt_type = 2
    bw.ue(1); // max_num_ref_frames = 1
    bw.bit(0); // gaps_in_frame_num_value_allowed_flag
    bw.ue(mb_w - 1);
    bw.ue(mb_h - 1);
    bw.bit(1); // frame_mbs_only_flag
    bw.bit(0); // direct_8x8_inference_flag
    if crop_r > 0 || crop_b > 0 {
        bw.bit(1);
        bw.ue(0);
        bw.ue(crop_r);
        bw.ue(0);
        bw.ue(crop_b);
    } else {
        bw.bit(0);
    }
    bw.bit(0); // vui_parameters_present_flag
    bw.rbsp_trailing();
    ebsp_from_rbsp(0x67, &bw.bytes)
}

fn make_pps() -> Vec<u8> {
    let mut bw = BitWriter::new();
    bw.ue(0); // pic_parameter_set_id
    bw.ue(0); // seq_parameter_set_id
    bw.bit(0); // entropy_coding_mode_flag (CAVLC)
    bw.bit(0); // bottom_field_pic_order_in_frame_present_flag
    bw.ue(0); // num_slice_groups_minus1
    bw.ue(0); // num_ref_idx_l0_default_active_minus1
    bw.ue(0); // num_ref_idx_l1_default_active_minus1
    bw.bit(0); // weighted_pred_flag
    bw.bits(0, 2); // weighted_bipred_idc
    bw.ue(0); // pic_init_qp_minus26 = se(0)
    bw.ue(0); // pic_init_qs_minus26 = se(0)
    bw.ue(0); // chroma_qp_index_offset = se(0)
    bw.bit(1); // deblocking_filter_control_present_flag
    bw.bit(0); // constrained_intra_pred_flag
    bw.bit(0); // redundant_pic_cnt_present_flag
    bw.rbsp_trailing();
    ebsp_from_rbsp(0x68, &bw.bytes)
}

fn frame_macroblocks(rgb: &[u8], width: u32, height: u32) -> Vec<[u8; 384]> {
    let even_w = width + (width & 1);
    let even_h = height + (height & 1);
    let mb_w = even_w.div_ceil(16);
    let mb_h = even_h.div_ceil(16);
    let get_rgb = |x: u32, y: u32| -> (i32, i32, i32) {
        let cx = x.min(width - 1) as usize;
        let cy = y.min(height - 1) as usize;
        let idx = (cy * width as usize + cx) * 3;
        (
            i32::from(rgb[idx]),
            i32::from(rgb[idx + 1]),
            i32::from(rgb[idx + 2]),
        )
    };
    let mut mbs = Vec::with_capacity((mb_w * mb_h) as usize);
    for mby in 0..mb_h {
        for mbx in 0..mb_w {
            let mut pcm = [0_u8; 384];
            for dy in 0..16 {
                for dx in 0..16 {
                    let (red, green, blue) = get_rgb(mbx * 16 + dx, mby * 16 + dy);
                    let luma = ((66 * red + 129 * green + 25 * blue + 128) >> 8) + 16;
                    pcm[(dy * 16 + dx) as usize] =
                        u8::try_from(luma.clamp(16, 235)).expect("clamped 16..=235");
                }
            }
            for dy in 0..8 {
                for dx in 0..8 {
                    let mut r_sum = 0;
                    let mut g_sum = 0;
                    let mut b_sum = 0;
                    for sy in 0..2 {
                        for sx in 0..2 {
                            let (red, green, blue) =
                                get_rgb(mbx * 16 + dx * 2 + sx, mby * 16 + dy * 2 + sy);
                            r_sum += red;
                            g_sum += green;
                            b_sum += blue;
                        }
                    }
                    let red = r_sum >> 2;
                    let green = g_sum >> 2;
                    let blue = b_sum >> 2;
                    let cb = ((-38 * red - 74 * green + 112 * blue + 128) >> 8) + 128;
                    let cr = ((112 * red - 94 * green - 18 * blue + 128) >> 8) + 128;
                    pcm[256 + (dy * 8 + dx) as usize] =
                        u8::try_from(cb.clamp(16, 240)).expect("clamped 16..=240");
                    pcm[320 + (dy * 8 + dx) as usize] =
                        u8::try_from(cr.clamp(16, 240)).expect("clamped 16..=240");
                }
            }
            mbs.push(pcm);
        }
    }
    mbs
}

fn encode_slice(
    mbs: &[[u8; 384]],
    previous: Option<&[[u8; 384]]>,
    frame_num: u32,
    idr_pic_id: u32,
) -> Vec<u8> {
    let mut bw = BitWriter::new();
    bw.ue(0); // first_mb_in_slice
    let nal_header = if let Some(prev) = previous {
        bw.ue(5); // slice_type = P
        bw.ue(0); // pic_parameter_set_id
        bw.bits(frame_num, 10);
        bw.bit(0); // num_ref_idx_active_override_flag
        bw.bit(0); // ref_pic_list_modification_flag_l0
        bw.bit(0); // adaptive_ref_pic_marking_mode_flag
        bw.ue(0); // slice_qp_delta
        bw.ue(1); // disable_deblocking_filter_idc = 1
        let mut skip_run = 0_u32;
        for (pcm, old) in mbs.iter().zip(prev.iter()) {
            if pcm == old {
                skip_run += 1;
            } else {
                bw.ue(skip_run);
                skip_run = 0;
                bw.ue(30); // I_PCM in P-slice
                bw.align_zero();
                bw.raw_bytes(pcm);
            }
        }
        if skip_run > 0 {
            bw.ue(skip_run);
        }
        0x41
    } else {
        bw.ue(7); // slice_type = I
        bw.ue(0); // pic_parameter_set_id
        bw.bits(0, 10); // frame_num = 0
        bw.ue(idr_pic_id);
        bw.bit(0); // no_output_of_prior_pics_flag
        bw.bit(0); // long_term_reference_flag
        bw.ue(0); // slice_qp_delta
        bw.ue(1); // disable_deblocking_filter_idc = 1
        for pcm in mbs {
            bw.ue(25); // I_PCM in I-slice
            bw.align_zero();
            bw.raw_bytes(pcm);
        }
        0x65
    };
    bw.rbsp_trailing();
    let nal = ebsp_from_rbsp(nal_header, &bw.bytes);
    let mut sample = Vec::with_capacity(nal.len() + 4);
    sample.extend_from_slice(
        &u32::try_from(nal.len())
            .expect("NAL fits in u32")
            .to_be_bytes(),
    );
    sample.extend_from_slice(&nal);
    sample
}

fn mp4_box(tag: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let len = u32::try_from(payload.len() + 8).expect("MP4 box fits in u32");
    let mut out = Vec::with_capacity(len as usize);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&tag);
    out.extend_from_slice(payload);
    out
}

fn full_box(tag: [u8; 4], version: u8, flags: u32, payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(payload.len() + 4);
    body.push(version);
    body.extend_from_slice(&flags.to_be_bytes()[1..4]);
    body.extend_from_slice(payload);
    mp4_box(tag, &body)
}

fn build_moov(
    size: (u32, u32),
    offsets: &[u32],
    sizes: &[u32],
    delays: &[u32],
    sync_samples: &[u32],
) -> Result<Vec<u8>> {
    let even_w = size.0 + (size.0 & 1);
    let even_h = size.1 + (size.1 & 1);
    let total_ms = u32::try_from(delays.iter().map(|&d| u64::from(d)).sum::<u64>())?;

    let mut mvhd_body = Vec::with_capacity(96);
    mvhd_body.extend_from_slice(&[0_u8; 8]);
    mvhd_body.extend_from_slice(&TIMESCALE_MS.to_be_bytes());
    mvhd_body.extend_from_slice(&total_ms.to_be_bytes());
    mvhd_body.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    mvhd_body.extend_from_slice(&0x0100_0000_u32.to_be_bytes());
    mvhd_body.extend_from_slice(&[0_u8; 8]);
    for &v in &[0x0001_0000_u32, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000] {
        mvhd_body.extend_from_slice(&v.to_be_bytes());
    }
    mvhd_body.extend_from_slice(&[0_u8; 24]);
    mvhd_body.extend_from_slice(&2_u32.to_be_bytes());
    let mvhd = full_box(*b"mvhd", 0, 0, &mvhd_body);

    let mut tkhd_body = Vec::with_capacity(80);
    tkhd_body.extend_from_slice(&[0_u8; 8]);
    tkhd_body.extend_from_slice(&1_u32.to_be_bytes());
    tkhd_body.extend_from_slice(&0_u32.to_be_bytes());
    tkhd_body.extend_from_slice(&total_ms.to_be_bytes());
    tkhd_body.extend_from_slice(&[0_u8; 16]);
    for &v in &[0x0001_0000_u32, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000] {
        tkhd_body.extend_from_slice(&v.to_be_bytes());
    }
    tkhd_body.extend_from_slice(&(even_w << 16).to_be_bytes());
    tkhd_body.extend_from_slice(&(even_h << 16).to_be_bytes());
    let tkhd = full_box(*b"tkhd", 0, 3, &tkhd_body);

    let mut mdhd_body = Vec::with_capacity(20);
    mdhd_body.extend_from_slice(&[0_u8; 8]);
    mdhd_body.extend_from_slice(&TIMESCALE_MS.to_be_bytes());
    mdhd_body.extend_from_slice(&total_ms.to_be_bytes());
    mdhd_body.extend_from_slice(&0x55c4_0000_u32.to_be_bytes());
    let mdhd = full_box(*b"mdhd", 0, 0, &mdhd_body);

    let mut hdlr_body = Vec::with_capacity(29);
    hdlr_body.extend_from_slice(&0_u32.to_be_bytes());
    hdlr_body.extend_from_slice(b"vide");
    hdlr_body.extend_from_slice(&[0_u8; 12]);
    hdlr_body.extend_from_slice(b"VideoHandler\0");
    let hdlr = full_box(*b"hdlr", 0, 0, &hdlr_body);

    let vmhd = full_box(*b"vmhd", 0, 1, &[0_u8; 8]);
    let url = full_box(*b"url ", 0, 1, &[]);
    let dref = full_box(
        *b"dref",
        0,
        0,
        &[&1_u32.to_be_bytes()[..], &url[..]].concat(),
    );
    let dinf = mp4_box(*b"dinf", &dref);
    let stbl = build_stbl(size, offsets, sizes, delays, sync_samples)?;
    let minf = mp4_box(*b"minf", &[vmhd, dinf, stbl].concat());
    let mdia = mp4_box(*b"mdia", &[mdhd, hdlr, minf].concat());
    let trak = mp4_box(*b"trak", &[tkhd, mdia].concat());
    Ok(mp4_box(*b"moov", &[mvhd, trak].concat()))
}

fn build_stbl(
    size: (u32, u32),
    offsets: &[u32],
    sizes: &[u32],
    delays: &[u32],
    sync_samples: &[u32],
) -> Result<Vec<u8>> {
    let even_w = size.0 + (size.0 & 1);
    let even_h = size.1 + (size.1 & 1);
    let sps = make_sps(size.0, size.1);
    let pps = make_pps();
    let mut avcc_body = vec![1, sps[1], sps[2], sps[3], 0xFF, 0xE1];
    avcc_body.extend_from_slice(&u16::try_from(sps.len())?.to_be_bytes());
    avcc_body.extend_from_slice(&sps);
    avcc_body.push(1);
    avcc_body.extend_from_slice(&u16::try_from(pps.len())?.to_be_bytes());
    avcc_body.extend_from_slice(&pps);
    let avcc = mp4_box(*b"avcC", &avcc_body);

    let mut avc1_body = vec![0_u8; 6];
    avc1_body.extend_from_slice(&1_u16.to_be_bytes());
    avc1_body.extend_from_slice(&[0_u8; 16]);
    avc1_body.extend_from_slice(&u16::try_from(even_w)?.to_be_bytes());
    avc1_body.extend_from_slice(&u16::try_from(even_h)?.to_be_bytes());
    avc1_body.extend_from_slice(&0x0048_0000_u32.to_be_bytes());
    avc1_body.extend_from_slice(&0x0048_0000_u32.to_be_bytes());
    avc1_body.extend_from_slice(&0_u32.to_be_bytes());
    avc1_body.extend_from_slice(&1_u16.to_be_bytes());
    avc1_body.extend_from_slice(&[0_u8; 32]);
    avc1_body.extend_from_slice(&0x0018_u16.to_be_bytes());
    avc1_body.extend_from_slice(&0xFFFF_u16.to_be_bytes());
    avc1_body.extend_from_slice(&avcc);
    let avc1 = mp4_box(*b"avc1", &avc1_body);
    let stsd = full_box(
        *b"stsd",
        0,
        0,
        &[&1_u32.to_be_bytes()[..], &avc1[..]].concat(),
    );

    let mut stts_body = u32::try_from(delays.len())?.to_be_bytes().to_vec();
    for &delay in delays {
        stts_body.extend_from_slice(&1_u32.to_be_bytes());
        stts_body.extend_from_slice(&delay.to_be_bytes());
    }
    let stts = full_box(*b"stts", 0, 0, &stts_body);

    let mut stss_body = u32::try_from(sync_samples.len())?.to_be_bytes().to_vec();
    for &sample_idx in sync_samples {
        stss_body.extend_from_slice(&sample_idx.to_be_bytes());
    }
    let stss = full_box(*b"stss", 0, 0, &stss_body);

    let stsc = full_box(
        *b"stsc",
        0,
        0,
        &[0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1],
    );

    let mut stsz_body = 0_u32.to_be_bytes().to_vec();
    stsz_body.extend_from_slice(&u32::try_from(sizes.len())?.to_be_bytes());
    for &sz in sizes {
        stsz_body.extend_from_slice(&sz.to_be_bytes());
    }
    let stsz = full_box(*b"stsz", 0, 0, &stsz_body);

    let mut stco_body = u32::try_from(offsets.len())?.to_be_bytes().to_vec();
    for &off in offsets {
        stco_body.extend_from_slice(&off.to_be_bytes());
    }
    let stco = full_box(*b"stco", 0, 0, &stco_body);

    Ok(mp4_box(
        *b"stbl",
        &[stsd, stts, stss, stsc, stsz, stco].concat(),
    ))
}
