use std::sync::atomic::{AtomicI32, AtomicU32};
use windows::Win32::Foundation::MAX_PATH;
// Hand-generated from RTSSSharedMemory.h
// DO edit if something is missing

// RTSS v2.0 memory structure
#[repr(C)]
pub struct RtssSharedMemory {
    // signature allows applications to verify status of shared memory
    // The signature can be set to:
    // 'RTSS'     - statistics server's memory is initialized and contains valid data
    // 0xDEAD     - statistics server's memory is marked for deallocation and
    //              no longer contain valid data
    // otherwise  - the memory is not initialized
    pub signature: [u8; 4],

    // structure version ((major<<16) + minor)
    // must be set to 0x0002xxxx for v2.x structure
    pub version: u32,

    // size of RTSS_SHARED_MEMORY_APP_ENTRY for compatibility with future versions
    pub app_entry_size: u32,

    // offset of arrApp array for compatibility with future versions
    pub app_arr_offset: u32,

    // size of arrApp array for compatibility with future versions
    pub app_arr_size: u32,

    // size of RTSS_SHARED_MEMORY_OSD_ENTRY for compatibility with future versions
    pub osd_entry_size: u32,

    // offset of arrOSD array for compatibility with future versions
    pub osd_arr_offset: u32,

    // size of arrOSD array for compatibility with future versions
    pub osd_arr_size: u32,

    // Global OSD frame ID. Increment it to force the server to update OSD for all currently active
    // 3D applications.
    pub osd_frame: AtomicU32,

    // set bit 0 when you're writing to shared memory and reset it when done
    // WARNING: do not forget to reset it, otherwise you'll completely lock OSD updates for
    // all clients
    pub busy: AtomicI32,
}

pub const RTSS_SIGNATURE: [u8; 4] = ['S' as u8, 'S' as u8, 'T' as u8, 'R' as u8];

// OSD slot descriptor structure
#[repr(C)]
pub struct RtssSharedMemoryOsdEntry {
    //OSD slot text
    pub osd: [u8; 256],

    //OSD slot owner ID
    pub osd_owner: [u8; 256],

    //next fields are valid for v2.7 and newer shared memory format only

    //extended OSD slot text
    pub osd_ex: [u8; 4096],

    //next fields are valid for v2.12 and newer shared memory format only

    //OSD slot data buffer
    pub buffer: [u8; 262144],
}

//application descriptor structure
#[repr(C)]
pub struct RtssSharedMemoryAppEntry {
    //application identification related fields

    //process ID
    pub process_id: u32,
    //process executable name
    pub name: [u8; MAX_PATH as usize],
    //application specific flags
    pub flags: u32,

    //instantaneous framerate related fields

    //start time of framerate measurement period (in milliseconds)
    pub time0: u32,
    //end time of framerate measurement period (in milliseconds)
    pub time1: u32,
    //amount of frames rendered during (time1 - time0) period
    pub frames: u32,
    //frame time (in microseconds)
    pub frame_time: u32,
}

#[repr(C)]
pub struct RtssEmbeddedObject {
    //embedded object signature
    pub signature: [u8; 4],

    //embedded object size in bytes
    pub size: u32,

    //embedded object width in pixels (if positive) or in chars (if negative)
    pub width: i32,

    //embedded object height in pixels (if positive) or in chars (if negative)
    pub height: i32,

    //embedded object margin in pixels
    pub margin: i32,
}

pub const RTSS_EMBEDDED_OBJECT_GRAPH_SIGNATURE: [u8; 4] =
    ['0' as u8, '0' as u8, 'R' as u8, 'G' as u8];

pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_FILLED: u32 = 1;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_FRAMERATE: u32 = 2;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_FRAMETIME: u32 = 4;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_BAR: u32 = 8;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_BGND: u32 = 16;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_VERTICAL: u32 = 32;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_MIRRORED: u32 = 64;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_AUTOSCALE: u32 = 128;

pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_FRAMERATE_MIN: u32 = 256;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_FRAMERATE_AVG: u32 = 512;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_FRAMERATE_MAX: u32 = 1024;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_FRAMERATE_1DOT0_PERCENT_LOW: u32 = 2048;
pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_FRAMERATE_0DOT1_PERCENT_LOW: u32 = 4096;

pub const RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_BAR_RANGE: u32 = 8192;

#[repr(C)]
pub struct RtssEmbeddedObjectGraph {
    //embedded object header
    pub header: RtssEmbeddedObject,

    //bitmask containing RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_XXX flags
    pub flags: u32,

    //graph mininum value
    pub min: f32,

    //graph maximum value
    pub max: f32,

    //count of data samples immediately following the struct
    pub data_count: u32,
}
