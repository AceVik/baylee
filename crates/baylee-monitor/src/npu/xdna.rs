//! The questions the `amdxdna` driver answers, from `<drm/amdxdna_accel.h>`.
//! Only the two query calls are used, never `DRM_AMDXDNA_SET_STATE`, and the
//! node is opened read-only.
//!
//! The node is opened for each refresh and closed again. Opening binds this
//! process to the device, which took 0.11 ms; a descriptor held for good would
//! make a monitor one more thing keeping the module from being reloaded.

use super::{Npu, NpuContext};
use rustix::{
    fd::{AsFd, BorrowedFd},
    io::Errno,
    ioctl::{Opcode, Updater, ioctl, opcode},
};
use std::{fs::File, path::Path};

/// `DRM_COMMAND_BASE` and the driver's numbers above it.
const COMMAND_BASE: u8 = 0x40;
const GET_INFO: Opcode = opcode::read_write::<GetInfo>(b'd', COMMAND_BASE + 7);
const GET_ARRAY: Opcode = opcode::read_write::<GetArray>(b'd', COMMAND_BASE + 10);

/// Values of `enum amdxdna_drm_get_param`.
const QUERY_AIE_METADATA: u32 = 1;
const QUERY_CLOCK_METADATA: u32 = 3;
const QUERY_SENSORS: u32 = 4;
const GET_POWER_MODE: u32 = 9;
const QUERY_RESOURCE_INFO: u32 = 12;
/// `DRM_AMDXDNA_HW_CONTEXT_ALL`.
const ALL_CONTEXTS: u32 = 0;

/// Sizes of the structures returned.
const AIE_METADATA_SIZE: usize = 64;
const CLOCK_METADATA_SIZE: usize = 48;
const POWER_MODE_SIZE: usize = 8;
const RESOURCE_INFO_SIZE: usize = 40;
const SENSOR_SIZE: usize = 168;
const CONTEXT_SIZE: usize = 144;

/// Sensors asked for at once; one per column and one for power is plenty.
const MAX_SENSORS: usize = 16;
/// `npu_task_max` is 6 on this hardware.
const MAX_CONTEXTS: usize = 32;

const SENSOR_POWER: u8 = 0;
const SENSOR_COLUMN_UTILIZATION: u8 = 1;

/// `struct amdxdna_drm_get_info`.
#[repr(C)]
struct GetInfo {
    param: u32,
    buffer_size: u32,
    buffer: u64,
}

/// `struct amdxdna_drm_get_array`.
#[repr(C)]
struct GetArray {
    param: u32,
    element_size: u32,
    num_element: u32,
    pad: u32,
    buffer: u64,
}

/// Fills `buffer` with the answer to `param` and returns how much of it.
fn get_info(fd: BorrowedFd<'_>, param: u32, buffer: &mut [u8]) -> Result<usize, Errno> {
    let capacity = u32::try_from(buffer.len()).map_err(|_| Errno::INVAL)?;
    let mut info = GetInfo {
        param,
        buffer_size: capacity,
        buffer: buffer.as_mut_ptr() as u64,
    };
    // SAFETY: `GetInfo` has the layout of `struct amdxdna_drm_get_info` and
    // `GET_INFO` is that structure's call, which the tests pin to the number
    // the header defines. The kernel writes at most `buffer_size` bytes through
    // `buffer`, which is a live, exclusively borrowed slice of that length for
    // the whole call. `read_device` sends this only to the `amdxdna` driver.
    unsafe { ioctl(fd, Updater::<GET_INFO, GetInfo>::new(&mut info)) }?;
    Ok((info.buffer_size as usize).min(buffer.len()))
}

/// Fills `buffer` with whole elements of the driver's size and returns how
/// large one is and how many there are.
fn get_array(fd: BorrowedFd<'_>, param: u32, buffer: &mut [u8]) -> Result<(usize, usize), Errno> {
    let capacity = u32::try_from(buffer.len() / CONTEXT_SIZE).map_err(|_| Errno::INVAL)?;
    let mut array = GetArray {
        param,
        element_size: CONTEXT_SIZE as u32,
        num_element: capacity,
        pad: 0,
        buffer: buffer.as_mut_ptr() as u64,
    };
    // SAFETY: as in `get_info`, for `struct amdxdna_drm_get_array`. The kernel
    // writes at most `element_size * num_element` bytes, which is no more than
    // `buffer` holds, and the slice stays borrowed for the call.
    unsafe { ioctl(fd, Updater::<GET_ARRAY, GetArray>::new(&mut array)) }?;
    Ok((array.element_size as usize, array.num_element as usize))
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u32> {
    let raw = bytes.get(at..at + 2)?;
    Some(u32::from(u16::from_ne_bytes([raw[0], raw[1]])))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_ne_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_ne_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// What the driver said. Each answer stands on its own: an older driver knows
/// fewer questions, and this chip's firmware has no sensors at all.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Answers {
    pub columns: Option<u32>,
    pub rows: Option<u32>,
    pub clock_mhz: Option<u32>,
    pub clock_control_mhz: Option<u32>,
    pub clock_max_mhz: Option<u32>,
    pub tops_max: Option<u32>,
    pub tasks_max: Option<u32>,
    pub tasks: Option<u32>,
    pub power_mode: Option<&'static str>,
    pub busy: Option<f32>,
    pub watts: Option<f64>,
    pub contexts: Option<Vec<NpuContext>>,
}

impl Answers {
    pub fn apply(self, npu: &mut Npu) {
        npu.queried = true;
        npu.columns = self.columns;
        npu.rows = self.rows;
        npu.clock_mhz = self.clock_mhz;
        npu.clock_control_mhz = self.clock_control_mhz;
        npu.clock_max_mhz = self.clock_max_mhz;
        npu.tops_max = self.tops_max;
        npu.tasks_max = self.tasks_max;
        npu.tasks = self.tasks;
        npu.power_mode = self.power_mode;
        npu.busy = self.busy;
        npu.watts = self.watts;
        // Without an answer the columns in use stay unknown, not zero.
        if let Some(contexts) = self.contexts {
            npu.columns_used = Some(contexts.iter().map(|context| context.columns).sum());
            npu.contexts = contexts;
        } else if self.tasks == Some(0) {
            npu.columns_used = Some(0);
        }
    }
}

/// Opens the node and asks everything. `None` if it cannot be opened, as for
/// a user outside the `render` group.
pub(super) fn ask(node: &Path) -> Option<Answers> {
    let file = File::open(node).ok()?;
    let fd = file.as_fd();
    let mut answers = Answers::default();
    let mut metadata = [0_u8; AIE_METADATA_SIZE];
    if get_info(fd, QUERY_AIE_METADATA, &mut metadata).is_ok() {
        (answers.columns, answers.rows) = (u16_at(&metadata, 4), u16_at(&metadata, 6));
    }
    let mut clocks = [0_u8; CLOCK_METADATA_SIZE];
    if get_info(fd, QUERY_CLOCK_METADATA, &mut clocks).is_ok() {
        (answers.clock_control_mhz, answers.clock_mhz) = parse_clocks(&clocks);
    }
    let mut mode = [0_u8; POWER_MODE_SIZE];
    if get_info(fd, GET_POWER_MODE, &mut mode).is_ok() {
        answers.power_mode = power_mode(mode[0]);
    }
    let mut resources = [0_u8; RESOURCE_INFO_SIZE];
    if get_info(fd, QUERY_RESOURCE_INFO, &mut resources).is_ok() {
        apply_resources(&mut answers, &resources);
    }
    let mut sensors = [0_u8; SENSOR_SIZE * MAX_SENSORS];
    if let Ok(written) = get_info(fd, QUERY_SENSORS, &mut sensors) {
        (answers.busy, answers.watts) = parse_sensors(&sensors[..written]);
    }
    let mut contexts = vec![0_u8; CONTEXT_SIZE * MAX_CONTEXTS];
    if let Ok((size, count)) = get_array(fd, ALL_CONTEXTS, &mut contexts) {
        answers.contexts = Some(parse_contexts(&contexts, size, count));
    }
    Some(answers)
}

/// `struct amdxdna_drm_query_clock_metadata`: two of a 16-byte name, a
/// frequency and padding; the control processor's clock, then the array's.
fn parse_clocks(bytes: &[u8]) -> (Option<u32>, Option<u32>) {
    (u32_at(bytes, 16), u32_at(bytes, 40))
}

fn power_mode(value: u8) -> Option<&'static str> {
    ["default", "low", "medium", "high", "turbo"]
        .get(usize::from(value))
        .copied()
}

/// `struct amdxdna_drm_get_resource_info`, five `u64`. `npu_tops_curr` is left
/// out: it reads the maximum on an idle device, so it rates the clock and says
/// nothing of load, and beside the maximum it would look like 100 %.
fn apply_resources(answers: &mut Answers, bytes: &[u8]) {
    let field = |at| u64_at(bytes, at).and_then(|value| u32::try_from(value).ok());
    answers.clock_max_mhz = field(0);
    answers.tops_max = field(8);
    answers.tasks_max = field(16);
    answers.tasks = field(32);
}

/// `struct amdxdna_drm_query_sensor`: `input` at 64, `unitm` at 160, `type`
/// at 161. Columns are averaged into one figure for the device.
#[allow(clippy::cast_precision_loss)] // A handful of columns.
fn parse_sensors(bytes: &[u8]) -> (Option<f32>, Option<f64>) {
    let mut watts = None;
    let mut busy: Vec<f64> = Vec::new();
    let (sensors, _) = bytes.as_chunks::<SENSOR_SIZE>();
    for sensor in sensors {
        let Some(input) = u32_at(sensor, 64) else {
            continue;
        };
        let scale = 10_f64.powi(i32::from(sensor[160].cast_signed()));
        let value = f64::from(input) * scale;
        match sensor[161] {
            SENSOR_POWER => watts = Some(value),
            SENSOR_COLUMN_UTILIZATION => busy.push(value),
            _ => {}
        }
    }
    let mean = (!busy.is_empty()).then(|| busy.iter().sum::<f64>() / busy.len() as f64);
    (mean.map(|value| value as f32), watts)
}

/// `struct amdxdna_drm_hwctx_entry`, as many as the driver says fit. A newer
/// driver may send larger elements; the fields read here stay where they are.
fn parse_contexts(bytes: &[u8], size: usize, count: usize) -> Vec<NpuContext> {
    if size < 92 {
        return Vec::new();
    }
    bytes
        .chunks_exact(size)
        .take(count)
        .filter_map(|entry| {
            Some(NpuContext {
                columns: u32_at(entry, 8)?,
                pid: u32::try_from(u64_at(entry, 16)?).unwrap_or(0),
                submitted: u64_at(entry, 24)?,
                completed: u64_at(entry, 32)?,
                errors: u64_at(entry, 56)?,
                active: u32_at(entry, 88)? == 1,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    /// The numbers `DRM_IOCTL_AMDXDNA_GET_INFO` and `_GET_ARRAY` expand to in
    /// the C header. They were proven on a real device; a changed structure
    /// must not silently become another call.
    #[test]
    // The request number's type is the platform's: `u64` on some, narrower on others.
    #[allow(clippy::useless_conversion)]
    fn the_calls_are_the_headers_calls() {
        assert_eq!(u64::from(GET_INFO), 0xC010_6447);
        assert_eq!(u64::from(GET_ARRAY), 0xC018_644A);
    }

    #[test]
    fn the_structures_have_the_headers_layout() {
        assert_eq!(size_of::<GetInfo>(), 16);
        assert_eq!(offset_of!(GetInfo, buffer), 8);
        assert_eq!(size_of::<GetArray>(), 24);
        assert_eq!(offset_of!(GetArray, num_element), 8);
        assert_eq!(offset_of!(GetArray, buffer), 16);
    }

    /// On anything but the driver the call must fail and touch nothing.
    #[test]
    fn a_node_of_another_driver_refuses_and_leaves_the_buffer_alone() {
        let file = File::open("/dev/null").unwrap();
        let mut buffer = [7_u8; 16];
        assert!(get_info(file.as_fd(), QUERY_AIE_METADATA, &mut buffer).is_err());
        assert_eq!(buffer, [7; 16]);
        let mut buffer = vec![7_u8; CONTEXT_SIZE];
        assert!(get_array(file.as_fd(), ALL_CONTEXTS, &mut buffer).is_err());
        assert!(buffer.iter().all(|byte| *byte == 7));
    }

    #[test]
    fn a_missing_node_asks_nothing() {
        assert_eq!(ask(Path::new("/nonexistent/accel0")), None);
    }

    #[test]
    fn clocks_are_read_past_their_names() {
        let mut bytes = [0_u8; CLOCK_METADATA_SIZE];
        bytes[..12].copy_from_slice(b"MP-NPU Clock");
        bytes[16..20].copy_from_slice(&600_u32.to_ne_bytes());
        bytes[24..31].copy_from_slice(b"H Clock");
        bytes[40..44].copy_from_slice(&1028_u32.to_ne_bytes());
        assert_eq!(parse_clocks(&bytes), (Some(600), Some(1028)));
    }

    #[test]
    fn resources_give_limits_and_the_tasks_in_use_but_no_tops_in_use() {
        let mut bytes = [0_u8; RESOURCE_INFO_SIZE];
        for (at, value) in [1600_u64, 10, 6, 10, 2].iter().enumerate() {
            bytes[at * 8..at * 8 + 8].copy_from_slice(&value.to_ne_bytes());
        }
        let mut answers = Answers::default();
        apply_resources(&mut answers, &bytes);
        assert_eq!(answers.clock_max_mhz, Some(1600));
        assert_eq!(answers.tops_max, Some(10));
        assert_eq!(answers.tasks_max, Some(6));
        assert_eq!(answers.tasks, Some(2));
    }

    #[test]
    fn power_modes_have_names_and_an_unknown_one_has_none() {
        assert_eq!(power_mode(0), Some("default"));
        assert_eq!(power_mode(4), Some("turbo"));
        assert_eq!(power_mode(5), None);
    }

    fn sensor(input: u32, unitm: i8, kind: u8) -> Vec<u8> {
        let mut bytes = vec![0_u8; SENSOR_SIZE];
        bytes[64..68].copy_from_slice(&input.to_ne_bytes());
        bytes[160] = unitm.cast_unsigned();
        bytes[161] = kind;
        bytes
    }

    #[test]
    fn sensors_scale_by_their_unit_and_columns_become_one_figure() {
        let bytes = [
            sensor(1500, -3, SENSOR_POWER),
            sensor(20, 0, SENSOR_COLUMN_UTILIZATION),
            sensor(60, 0, SENSOR_COLUMN_UTILIZATION),
            sensor(9, 0, 99),
        ]
        .concat();
        let (busy, watts) = parse_sensors(&bytes);
        assert_eq!(busy, Some(40.0));
        assert_eq!(watts, Some(1.5));
    }

    /// The chip this was written on answers `ENODEV`, so nothing is parsed
    /// and both stay unknown. Silence must never become a zero.
    #[test]
    fn no_sensors_are_unknown_not_zero() {
        assert_eq!(parse_sensors(&[]), (None, None));
    }

    fn context(pid: u64, columns: u32, completed: u64, state: u32) -> Vec<u8> {
        let mut bytes = vec![0_u8; CONTEXT_SIZE];
        bytes[8..12].copy_from_slice(&columns.to_ne_bytes());
        bytes[16..24].copy_from_slice(&pid.to_ne_bytes());
        bytes[24..32].copy_from_slice(&(completed + 1).to_ne_bytes());
        bytes[32..40].copy_from_slice(&completed.to_ne_bytes());
        bytes[56..64].copy_from_slice(&3_u64.to_ne_bytes());
        bytes[88..92].copy_from_slice(&state.to_ne_bytes());
        bytes
    }

    #[test]
    fn contexts_are_read_up_to_the_count_the_driver_gave() {
        let bytes = [
            context(42, 4, 100, 1),
            context(43, 1, 7, 0),
            context(99, 9, 9, 1),
        ]
        .concat();
        let contexts = parse_contexts(&bytes, CONTEXT_SIZE, 2);
        assert_eq!(
            contexts,
            [
                NpuContext {
                    pid: 42,
                    columns: 4,
                    submitted: 101,
                    completed: 100,
                    errors: 3,
                    active: true,
                },
                NpuContext {
                    pid: 43,
                    columns: 1,
                    submitted: 8,
                    completed: 7,
                    errors: 3,
                    active: false,
                },
            ]
        );
    }

    #[test]
    fn a_larger_element_keeps_its_fields_and_a_tiny_one_is_refused() {
        let mut entry = context(42, 4, 100, 1);
        entry.resize(CONTEXT_SIZE + 16, 0);
        assert_eq!(parse_contexts(&entry, CONTEXT_SIZE + 16, 1)[0].pid, 42);
        assert!(parse_contexts(&entry, 8, 1).is_empty());
        assert!(parse_contexts(&[], CONTEXT_SIZE, 0).is_empty());
    }

    #[test]
    fn answers_without_contexts_leave_the_columns_unknown() {
        let mut npu = Npu::default();
        Answers {
            tasks: None,
            contexts: None,
            ..Answers::default()
        }
        .apply(&mut npu);
        assert_eq!(npu.columns_used, None);
        let mut npu = Npu::default();
        Answers {
            contexts: Some(vec![
                NpuContext {
                    columns: 4,
                    ..NpuContext::default()
                },
                NpuContext {
                    columns: 1,
                    ..NpuContext::default()
                },
            ]),
            ..Answers::default()
        }
        .apply(&mut npu);
        assert_eq!(npu.columns_used, Some(5));
    }
}
