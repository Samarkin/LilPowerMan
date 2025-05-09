use super::bindings::*;
use super::Error;
use std::borrow::Cow;
use std::cmp::min;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering;
use windows::core::{w, Error as WindowsError, Owned};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, HANDLE};
use windows::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, VirtualQuery, FILE_MAP_ALL_ACCESS,
    MEMORY_BASIC_INFORMATION, MEMORY_MAPPED_VIEW_ADDRESS,
};

const RTSS_MIN_SUPPORTED_VERSION: u32 = 0x0002000e; // v2.14 is the lowest to support OSD locking
const OWNER_SIGNATURE: &str = "LilPowerMan";

struct SharedMemoryGuard<'parent> {
    mem: &'parent mut RtssSharedMemory,
}

impl<'parent> SharedMemoryGuard<'parent> {
    fn new(view: &'parent mut SharedMemoryView) -> Self {
        // SAFETY: We validated that view.addr points to a valid instance of RtssSharedMemory
        let mem = unsafe { &mut *(view.view.addr.Value as *mut RtssSharedMemory) };
        while mem
            .busy
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        SharedMemoryGuard { mem }
    }
}

impl<'parent> Deref for SharedMemoryGuard<'parent> {
    type Target = RtssSharedMemory;

    fn deref(&self) -> &Self::Target {
        self.mem
    }
}

impl<'parent> DerefMut for SharedMemoryGuard<'parent> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mem
    }
}

impl<'parent> Drop for SharedMemoryGuard<'parent> {
    fn drop(&mut self) {
        self.mem.busy.store(0, Ordering::Relaxed);
    }
}

pub fn open_shared_memory() -> Result<Owned<HANDLE>, Error> {
    // SAFETY: The call is sound as long as the arguments are valid, and they are all hardcoded
    let r = unsafe { OpenFileMappingW(FILE_MAP_ALL_ACCESS.0, false, w!("RTSSSharedMemoryV2")) };
    // SAFETY: If the call succeeded, we own the handle now
    r.map(|h| unsafe { Owned::new(h) }).map_err(|err| {
        if err == WindowsError::from(ERROR_FILE_NOT_FOUND) {
            Error::RtssV2NotRunning
        } else {
            Error::WindowsError(err)
        }
    })
}

struct OwnedMemoryMapView<'mem> {
    addr: MEMORY_MAPPED_VIEW_ADDRESS,
    _mem: PhantomData<&'mem Owned<HANDLE>>,
}

impl<'mem> Drop for OwnedMemoryMapView<'mem> {
    fn drop(&mut self) {
        // SAFETY: We verified that MapViewOfFile succeeded before instantiating the view
        let result = unsafe { UnmapViewOfFile(self.addr) };
        if let Err(err) = result {
            error!("Failed to unmap view of file: {err}");
        }
    }
}

pub struct SharedMemoryView<'mem> {
    view: OwnedMemoryMapView<'mem>,
    size: usize,
}

fn string_from_mem(mem: &[u8]) -> Cow<str> {
    let len = mem.iter().position(|&c| c == b'\0').unwrap_or(mem.len());
    String::from_utf8_lossy(&mem[..len])
}

// Safely copies the bytes of the provided string into the given memory.
// Returns `true` if the string entirely fits into the memory, incl. null-terminator.
fn string_to_mem(s: &str, mem: &mut [u8]) -> bool {
    let bytes = s.as_bytes();
    let len = min(bytes.len(), mem.len());
    mem[..len].copy_from_slice(&bytes[..len]);
    if len < mem.len() {
        mem[len] = 0;
    }
    bytes.len() < mem.len()
}

enum SharedMemoryIterationNextStep {
    Break,
    Continue,
    RememberAndBreak,
    RememberIfNeededAndContinue,
}
use SharedMemoryIterationNextStep::*;

impl<'mem> SharedMemoryView<'mem> {
    pub fn from_file(file: &'mem Owned<HANDLE>) -> Result<Self, Error> {
        // SAFETY: Lifetimes guarantee that the file handle outlives the map view
        let addr = unsafe { MapViewOfFile(**file, FILE_MAP_ALL_ACCESS, 0, 0, 0) };
        if addr.Value.is_null() {
            debug!("Failed to map file view to memory");
            return Err(Error::WindowsError(WindowsError::from_win32()));
        }
        // Instantiate the view now to ensure memory gets unmapped on any error
        let view = OwnedMemoryMapView {
            addr,
            _mem: PhantomData,
        };
        let mut info = MEMORY_BASIC_INFORMATION::default();
        // SAFETY: The call is sound as long as arguments are valid
        let r = unsafe { VirtualQuery(Some(addr.Value), &mut info, size_of_val(&info)) };
        if r != size_of_val(&info) {
            debug!("Virtual query failed");
            return Err(Error::WindowsError(WindowsError::from_win32()));
        }
        let size = info.RegionSize;
        if size < size_of::<RtssSharedMemory>() {
            error!(
                "RTSS shared memory is {size} bytes. Expected at least {}.",
                size_of::<RtssSharedMemory>()
            );
            return Err(Error::UnexpectedMemoryLayout);
        }
        debug!("RTSS shared memory is {size} bytes");
        // SAFETY: We need to be careful not to assume the memory is valid until we verified
        // signature and version
        let mem = unsafe { &*(addr.Value as *const RtssSharedMemory) };
        let signature = mem.signature;
        if signature != RTSS_SIGNATURE {
            debug!("RTSS signature mismatch: {signature:?}");
            return Err(Error::RtssV2NotRunning);
        }
        let version = format!("{}.{}", mem.version >> 16, mem.version & 0xFFFF);
        if mem.version < RTSS_MIN_SUPPORTED_VERSION {
            debug!("RTSS version: {version}, expected at least {RTSS_MIN_SUPPORTED_VERSION}");
            return Err(Error::RtssVersionNotSupported(version));
        }
        debug!("RTSS version: {version}");
        // SAFETY: It is safe to use addr as a pointer to RtssSharedMemory
        Ok(SharedMemoryView { view, size })
    }

    fn lock(&mut self) -> SharedMemoryGuard {
        SharedMemoryGuard::new(self)
    }

    fn for_each_entry<D, F>(&mut self, process: D, finalize: F) -> Result<(), Error>
    where
        D: Fn(usize, &mut RtssSharedMemoryOsdEntry) -> SharedMemoryIterationNextStep,
        F: FnOnce(Option<(usize, &mut RtssSharedMemoryOsdEntry)>) -> Result<(), Error>,
    {
        let base_addr = self.view.addr.Value as usize;
        let map_view_size = self.size;
        let mem = self.lock();
        if mem.signature != RTSS_SIGNATURE {
            return Err(Error::RtssV2NotRunning);
        }
        let n = mem.osd_arr_size as usize;
        let entry_size = mem.osd_entry_size as usize;
        if entry_size < size_of::<RtssSharedMemoryOsdEntry>() {
            error!(
                "RTSS memory is corrupted: OSD entry size is {} bytes, expected at least {}",
                entry_size,
                size_of::<RtssSharedMemoryOsdEntry>()
            );
            return Err(Error::UnexpectedMemoryLayout);
        }
        let mut remembered_entry: Option<(usize, &mut RtssSharedMemoryOsdEntry)> = None;
        for i in 1..n {
            let entry_last_byte = mem.osd_arr_offset as usize + (i + 1) * entry_size - 1;
            if entry_last_byte >= map_view_size {
                error!("RTSS memory is corrupted: offset {} is out of bounds of the shared memory ({})",
                    entry_last_byte, map_view_size);
                return Err(Error::UnexpectedMemoryLayout);
            }
            let entry_addr = base_addr + mem.osd_arr_offset as usize + i * entry_size;
            // SAFETY: entry_addr points to a complete OsdEntry, entirely within the mapped file
            let entry = unsafe { &mut *(entry_addr as *mut RtssSharedMemoryOsdEntry) };
            match process(i, entry) {
                Break => break,
                Continue => continue,
                RememberAndBreak => {
                    remembered_entry = Some((i, entry));
                    break;
                }
                RememberIfNeededAndContinue => {
                    if remembered_entry.is_none() {
                        remembered_entry = Some((i, entry));
                    }
                    continue;
                }
            }
        }
        finalize(remembered_entry)
    }

    pub fn unregister(&mut self) -> Result<(), Error> {
        self.for_each_entry(
            |i, entry| {
                let owner = string_from_mem(&entry.osd_owner);
                if owner == OWNER_SIGNATURE {
                    *entry = Default::default();
                    info!("Unregistered ourselves from slot {i}");
                    trace!(
                        "Erased {} bytes at address 0x{:016X}",
                        size_of_val(&entry),
                        entry as *const _ as usize
                    );
                }
                Continue
            },
            |_| Ok(()),
        )
    }

    fn update<F>(&mut self, f: F) -> Result<(), Error>
    where
        F: FnOnce(&mut RtssSharedMemoryOsdEntry) -> Result<(), Error>,
    {
        self.for_each_entry(
            |_i, entry| {
                let current_owner = string_from_mem(&entry.osd_owner);
                if current_owner == OWNER_SIGNATURE {
                    RememberAndBreak
                } else if current_owner == "" {
                    RememberIfNeededAndContinue
                } else {
                    Continue
                }
            },
            |target| {
                let Some((target_idx, target_entry)) = target else {
                    return Err(Error::NoEmptyOsdSlots);
                };
                let current_owner = string_from_mem(&target_entry.osd_owner);
                if current_owner != OWNER_SIGNATURE {
                    info!("Registered ourselves in slot {target_idx}");
                }
                f(target_entry)
            },
        )
        /*
            let owner = string_from_mem(&entry.osd_owner);
            if owner == OWNER_SIGNATURE {
                let graph =
                    unsafe { &mut *(&mut entry.buffer as *mut _ as *mut RtssEmbeddedObjectGraph) };
                graph.header.signature = RTSS_EMBEDDED_OBJECT_GRAPH_SIGNATURE;
                graph.header.size = size_of::<RtssEmbeddedObjectGraph>() as u32;
                graph.header.width = 50;
                graph.header.height = 15;
                graph.header.margin = 0;
                graph.min = 0.0;
                graph.max = 60.0;
                graph.flags = RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_FRAMERATE| RTSS_EMBEDDED_OBJECT_GRAPH_FLAG_FRAMERATE_AVG;
                graph.data_count = 0;
                trace!("Updated OSD in slot {}", i);
                break;
            }
        }
         */
    }
}

pub struct SharedMemoryBuilder {
    osd: String,
    buf_len: usize,
}

impl SharedMemoryBuilder {
    pub fn new() -> Self {
        SharedMemoryBuilder {
            osd: String::new(),
            buf_len: 0,
        }
    }

    pub fn add_text(&mut self, text: &str) -> &mut Self {
        self.osd.push_str(text);
        self
    }

    pub fn add_newline(&mut self) -> &mut Self {
        self.add_text("\r\n")
    }

    pub fn write(&self, view: &mut SharedMemoryView) -> Result<(), Error> {
        view.update(|entry| {
            if !string_to_mem(OWNER_SIGNATURE, &mut entry.osd_owner)
                || !string_to_mem(&self.osd, &mut entry.osd_ex)
            {
                Err(Error::EntryOverflow)
            } else {
                Ok(())
            }
        })
    }
}
