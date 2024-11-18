use windows::core::{Owned, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CheckMenuItem, CreatePopupMenu, SetForegroundWindow, TrackPopupMenu, HMENU,
    MF_BYCOMMAND, MF_CHECKED, MF_ENABLED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    TPM_LEFTBUTTON,
};

pub struct PopupMenu {
    handle: Owned<HMENU>,
    submenus: Vec<PopupMenu>,
}

impl PopupMenu {
    /// Constructs a new popup menu.
    pub fn new() -> Self {
        // SAFETY: The call is always sound, we don't expect it to fail
        let handle = unsafe { Owned::new(CreatePopupMenu().unwrap()) };
        PopupMenu {
            handle,
            submenus: vec![],
        }
    }

    /// Appends a separator to the menu.
    pub fn append_separator(&mut self) {
        // SAFETY: Menu handle is owned by `self` and stays valid until drop
        unsafe { AppendMenuW(*self.handle, MF_SEPARATOR, 0, None).unwrap() };
    }

    /// Appends a menu item to the menu.
    pub fn append_menu_item(&mut self, title: &str, id: u32) {
        let mut buf: Vec<u16> = title.encode_utf16().collect();
        buf.push(0); // null-terminate
        let flags = MF_ENABLED | MF_STRING | MF_UNCHECKED;
        // SAFETY: Menu handle is owned by `self` and stays valid until drop
        unsafe { AppendMenuW(*self.handle, flags, id as usize, PCWSTR(buf.as_ptr())).unwrap() };
    }

    /// Appends a submenu to the menu, taking ownership of the `PopupMenu` instance
    pub fn append_submenu(&mut self, title: &str, menu: PopupMenu) {
        let mut buf: Vec<u16> = title.encode_utf16().collect();
        buf.push(0); // null-terminate
        let submenu = *menu.handle;
        self.submenus.push(menu);
        let flags = MF_ENABLED | MF_POPUP | MF_STRING | MF_UNCHECKED;
        // SAFETY: Both menu handles (safe and submenu) are owned by `self` and stay valid until drop
        unsafe { AppendMenuW(*self.handle, flags, submenu.0 as _, PCWSTR(buf.as_ptr())).unwrap() };
    }

    /// Tries to set the checked state of a menu item and returns the previous state.
    /// `true` means checked, and `false` means unchecked.
    pub fn check_menu_item(&mut self, id: u32, checked: bool) -> Option<bool> {
        let flags = MF_BYCOMMAND | if checked { MF_CHECKED } else { MF_UNCHECKED };
        // SAFETY: Menu handle is owned by `self` and stays valid until drop
        let result = unsafe { CheckMenuItem(*self.handle, id, flags.0) };
        match result {
            r if r == MF_CHECKED.0 => Some(false),
            r if r == MF_UNCHECKED.0 => Some(true),
            u32::MAX => None,
            r => panic!("Unexpected response from CheckMenuItem: {}", r),
        }
    }

    /// Shows the popup menu at the given coordinates, sending events to the specified window.
    ///
    /// # Notes
    ///
    /// The call does not return until the menu is dismissed,
    /// i.e. it starts a nested Windows event loop and could unintentionally result in recursion.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the handle will stay valid for the duration of the call.
    pub unsafe fn show(&self, x: i32, y: i32, window: HWND) -> bool {
        // We set foreground window to ensure the menu will be dismissed on focus lost.
        // SAFETY: The call is sound with a valid handle (guaranteed by the caller).
        // The call is expected to fail in some cases (e.g. another menu is already displayed).
        _ = unsafe { SetForegroundWindow(window) };
        // SAFETY: The call is sound with valid handles.
        unsafe { TrackPopupMenu(*self.handle, TPM_LEFTBUTTON, x, y, 0, window, None) }.0 != 0
    }
}
