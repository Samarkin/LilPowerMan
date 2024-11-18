use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use windows::core::{w, Error, Owned, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS,
};
use windows::Win32::System::Registry::{
    RegCreateKeyExW, RegDeleteValueW, RegEnumValueW, RegGetValueW, RegQueryInfoKeyW,
    RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_ALL_ACCESS, REG_DWORD_LITTLE_ENDIAN,
    REG_OPTION_NON_VOLATILE, RRF_RT_REG_DWORD, RRF_ZEROONFAILURE,
};

#[derive(Copy, Clone, Default, PartialEq)]
pub enum TdpSetting {
    #[default]
    Tracking,
    Forcing(u32),
}

#[derive(Clone, Default, PartialEq)]
pub struct Settings {
    app_limits: HashMap<OsString, u32>,
    tdp: TdpSetting,
}

impl Settings {
    pub fn get_app_limit(&self, app: &OsStr) -> Option<u32> {
        self.app_limits.get(app).copied()
    }

    pub fn get_tdp_setting(&self) -> TdpSetting {
        self.tdp
    }
}

pub struct SettingsStorage {
    root_key: Owned<HKEY>,
    app_key: Owned<HKEY>,
}

impl SettingsStorage {
    pub fn new() -> Self {
        let root_key = Self::create_subkey(HKEY_CURRENT_USER, w!("Software\\LilPowerMan")).unwrap();
        let app_key = Self::create_subkey(*root_key, w!("Applications")).unwrap();
        SettingsStorage { root_key, app_key }
    }

    fn create_subkey(parent: HKEY, name: PCWSTR) -> Result<Owned<HKEY>, Error> {
        let mut key = HKEY::default();
        // SAFETY: All arguments are valid, so the call is sound
        let err = unsafe {
            RegCreateKeyExW(
                parent,
                name,
                0,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_ALL_ACCESS,
                None,
                &mut key,
                None,
            )
        };
        if err != ERROR_SUCCESS {
            return Err(Error::from(err));
        }
        // SAFETY: We own the returned handle
        Ok(unsafe { Owned::new(key) })
    }

    fn load_tdp_setting(&self) -> TdpSetting {
        let mut data = 0;
        let mut data_len = size_of::<u32>() as u32;
        // SAFETY: All provided pointers reference local variables, string is null-terminated
        let result = unsafe {
            RegGetValueW(
                *self.root_key,
                None,
                w!("TdpSetting"),
                RRF_RT_REG_DWORD | RRF_ZEROONFAILURE,
                None,
                Some(&mut data as *mut _ as *mut _),
                Some(&mut data_len),
            )
        };
        if result != ERROR_SUCCESS && result != ERROR_MORE_DATA && result != ERROR_FILE_NOT_FOUND {
            panic!("{}", Error::from(result));
        }
        if data == 0 {
            TdpSetting::Tracking
        } else {
            TdpSetting::Forcing(data)
        }
    }

    pub fn load(&self) -> Settings {
        let mut values = 0;
        let mut max_value_name_len = 0;
        // SAFETY: All provided pointers reference local variables
        let result = unsafe {
            RegQueryInfoKeyW(
                *self.app_key,
                PWSTR::null(),
                None,
                None,
                None,
                None,
                None,
                Some(&mut values),
                Some(&mut max_value_name_len),
                None,
                None,
                None,
            )
        };
        if result != ERROR_SUCCESS {
            panic!("{}", Error::from(result));
        }
        let mut app_limits = HashMap::new();
        for i in 0..values {
            let mut value = vec![0; max_value_name_len as usize + 1];
            let mut value_name_len = max_value_name_len;
            let mut typ = 0;
            let mut data = 0;
            let mut data_len = size_of::<u32>() as u32;
            let result = unsafe {
                // SAFETY: All provided pointers reference local variables, lengths are correct
                RegEnumValueW(
                    *self.app_key,
                    i,
                    PWSTR::from_raw(value.as_mut_ptr()),
                    &mut value_name_len,
                    None,
                    Some(&mut typ),
                    Some(&mut data as *mut _ as *mut _),
                    Some(&mut data_len),
                )
            };
            if result != ERROR_SUCCESS && result != ERROR_NO_MORE_ITEMS && result != ERROR_MORE_DATA
            {
                panic!("{}", Error::from(result));
            }
            if typ == REG_DWORD_LITTLE_ENDIAN.0 {
                app_limits.insert(OsString::from_wide(&value[..value_name_len as usize]), data);
            }
        }
        Settings {
            app_limits,
            tdp: self.load_tdp_setting(),
        }
    }

    pub fn set_app_limit(&mut self, settings: &mut Settings, app: OsString, limit: u32) {
        let mut value: Vec<u16> = app.encode_wide().collect();
        value.push(0);
        let data: [u8; 4] = limit.to_le_bytes();
        // SAFETY: All provided pointers reference local variables, string is null-terminated
        let result = unsafe {
            RegSetValueExW(
                *self.app_key,
                PCWSTR::from_raw(value.as_ptr()),
                0,
                REG_DWORD_LITTLE_ENDIAN,
                Some(&data),
            )
        };
        if result != ERROR_SUCCESS {
            panic!("{}", Error::from(result));
        }
        settings.app_limits.insert(app, limit);
    }

    pub fn remove_app_limit(&mut self, settings: &mut Settings, app: &OsStr) {
        let mut value: Vec<u16> = app.encode_wide().collect();
        value.push(0);
        // SAFETY: String is null-terminated
        let result = unsafe { RegDeleteValueW(*self.app_key, PCWSTR::from_raw(value.as_ptr())) };
        if result != ERROR_SUCCESS {
            panic!("{}", Error::from(result));
        }
        settings.app_limits.remove(app);
    }

    pub fn set_tdp_setting(&mut self, settings: &mut Settings, tdp: TdpSetting) {
        let data = if let TdpSetting::Forcing(x) = tdp {
            x.to_le_bytes()
        } else {
            [0; 4]
        };
        // SAFETY: All provided pointers reference local variables, string is null-terminated
        let result = unsafe {
            RegSetValueExW(
                *self.root_key,
                w!("TdpSetting"),
                0,
                REG_DWORD_LITTLE_ENDIAN,
                Some(&data),
            )
        };
        if result != ERROR_SUCCESS {
            panic!("{}", Error::from(result));
        }
        settings.tdp = tdp;
    }
}
