use crate::winapi::device_io_control;
use std::fmt::{Debug, Display, Formatter};
use windows::core::{Error as WindowsError, Owned, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
    DIGCF_INTERFACEDEVICE, DIGCF_PRESENT, GUID_DEVCLASS_BATTERY, HDEVINFO,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows::Win32::Foundation::{
    ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS, GENERIC_READ, HANDLE,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Memory::{LocalAlloc, LPTR};
use windows::Win32::System::Power::{
    BatteryInformation, BATTERY_CAPACITY_RELATIVE, BATTERY_INFORMATION, BATTERY_IS_SHORT_TERM,
    BATTERY_QUERY_INFORMATION, BATTERY_STATUS, BATTERY_SYSTEM_BATTERY, BATTERY_WAIT_STATUS,
    IOCTL_BATTERY_QUERY_INFORMATION, IOCTL_BATTERY_QUERY_STATUS, IOCTL_BATTERY_QUERY_TAG,
};

pub enum Error {
    WindowsError(WindowsError),
    UnexpectedResponse,
}

impl From<WindowsError> for Error {
    fn from(error: WindowsError) -> Self {
        Self::WindowsError(error)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WindowsError(inner) => Display::fmt(inner, f),
            Self::UnexpectedResponse => write!(f, "Unexpected response from a WinAPI call"),
        }
    }
}

impl std::error::Error for Error {}

pub struct BatteriesIterator {
    device_info_set_handle: Owned<HDEVINFO>,
    index: u32,
}

impl BatteriesIterator {
    pub fn new() -> Self {
        // SAFETY: using hardcoded GUID and correct flags to get device info set
        // The call is not expected to fail
        let device_info_set_handle = unsafe {
            Owned::new(
                SetupDiGetClassDevsW(
                    Some(&GUID_DEVCLASS_BATTERY),
                    None,
                    None,
                    DIGCF_PRESENT | DIGCF_INTERFACEDEVICE,
                )
                .unwrap(),
            )
        };
        BatteriesIterator {
            device_info_set_handle,
            index: 0,
        }
    }

    /// # Safety
    ///
    /// `device_interface_data` must be a valid structure returned by `SetupDiEnumDeviceInterfaces`.
    unsafe fn get_battery(
        &self,
        device_interface_data: &SP_DEVICE_INTERFACE_DATA,
    ) -> Result<Battery, Error> {
        let mut bytes_required = 0;
        // SAFETY: It is safe to copy the handle as long as the copy does not outlive the `Owned` wrapper
        // SAFETY: Validity of the input structure is guaranteed by the caller
        let result = SetupDiGetDeviceInterfaceDetailW(
            *self.device_info_set_handle,
            device_interface_data,
            None,
            0,
            Some(&mut bytes_required),
            None,
        );
        // We didn't provide the output buffer, so we expect the call to fail and set the required size of the buffer
        let Err(err) = result else {
            Err(Error::UnexpectedResponse)?
        };
        if err != WindowsError::from(ERROR_INSUFFICIENT_BUFFER) {
            Err(err)?;
        }
        if bytes_required < size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32 {
            Err(Error::UnexpectedResponse)?;
        }
        // SAFETY: `Owned` wrapper ensures the memory will be freed before exiting the method
        let buffer = Owned::new(LocalAlloc(LPTR, bytes_required as usize)?);
        // SAFETY: Using the memory to store `SP_DEVICE_INTERFACE_DETAIL_DATA_W` structure
        let device_interface_detail = buffer.0 as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
        (*device_interface_detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        SetupDiGetDeviceInterfaceDetailW(
            *self.device_info_set_handle,
            device_interface_data,
            Some(device_interface_detail),
            bytes_required,
            None,
            None,
        )?;
        // SAFETY: We trust `SetupDiGetDeviceInterfaceDetailW` to set device path to a null-terminated string
        let device_path = PCWSTR::from_raw(&((*device_interface_detail).DevicePath) as *const u16);
        // SAFETY: We trust `SetupDiGetDeviceInterfaceDetailW` to set a valid device path
        let handle = Owned::new(CreateFileW(
            device_path,
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )?);
        let tag: u32 = device_io_control(&handle, IOCTL_BATTERY_QUERY_TAG, &0i32)?;
        if tag == 0 {
            Err(Error::UnexpectedResponse)?;
        }
        // SAFETY: The buffer that holds the device path will get destroyed before returning,
        //     but the created handle does not depend on it anymore
        Ok(Battery { handle, tag })
    }
}

impl Iterator for BatteriesIterator {
    type Item = Result<Battery, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut device_interface_data = SP_DEVICE_INTERFACE_DATA {
            cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        // SAFETY: It is safe to copy the handle as long as the copy does not outlive `self` that destroys the object
        let result = unsafe {
            SetupDiEnumDeviceInterfaces(
                *self.device_info_set_handle,
                None,
                &GUID_DEVCLASS_BATTERY,
                self.index,
                &mut device_interface_data,
            )
        };
        if let Err(err) = result {
            if err == WindowsError::from(ERROR_NO_MORE_ITEMS) {
                return None;
            }
            return Some(Err(err.into()));
        }
        self.index += 1;
        // SAFETY: `device_interface_data` is a valid structure
        let battery = unsafe { self.get_battery(&device_interface_data) };
        match battery {
            Ok(battery) => match battery.is_supported() {
                Ok(true) => Some(Ok(battery)),
                Ok(false) => self.next(),
                Err(err) => Some(Err(err)),
            },
            Err(err) => Some(Err(err)),
        }
    }
}

pub struct Battery {
    handle: Owned<HANDLE>,
    tag: u32,
}

impl Battery {
    pub fn get_charge_rate(&self) -> Result<i32, Error> {
        let bws = BATTERY_WAIT_STATUS {
            BatteryTag: self.tag,
            ..Default::default()
        };
        let status: BATTERY_STATUS =
            device_io_control(&self.handle, IOCTL_BATTERY_QUERY_STATUS, &bws)?;
        Ok(status.Rate)
    }

    fn is_supported(&self) -> Result<bool, Error> {
        let query = BATTERY_QUERY_INFORMATION {
            BatteryTag: self.tag,
            InformationLevel: BatteryInformation,
            ..Default::default()
        };
        let info: BATTERY_INFORMATION =
            device_io_control(&self.handle, IOCTL_BATTERY_QUERY_INFORMATION, &query)?;
        let rel_capacity =
            info.Capabilities & BATTERY_CAPACITY_RELATIVE == BATTERY_CAPACITY_RELATIVE;
        let short_term_battery = info.Capabilities & BATTERY_IS_SHORT_TERM == BATTERY_IS_SHORT_TERM;
        let system_battery = info.Capabilities & BATTERY_SYSTEM_BATTERY == BATTERY_SYSTEM_BATTERY;
        Ok(system_battery && !short_term_battery && !rel_capacity)
    }
}
