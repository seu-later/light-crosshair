use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    path::PathBuf,
    ptr::{copy_nonoverlapping, null, null_mut},
};

use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, POINT,
        RECT, SIZE, WPARAM,
    },
    Graphics::Gdi::{
        AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
        COLOR_WINDOW, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject,
        EnumDisplayMonitors, GetMonitorInfoW, GetStockObject, HDC, HFONT, HGDIOBJ, HMONITOR,
        MONITORINFO, MONITORINFOEXW, SelectObject,
    },
    System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
    UI::{
        Controls::Dialogs::{
            CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW, ChooseColorW, GetOpenFileNameW,
            OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
        },
        Controls::{
            BST_CHECKED, BST_UNCHECKED, ICC_BAR_CLASSES, INITCOMMONCONTROLSEX,
            InitCommonControlsEx, TBM_SETPOS, TBM_SETRANGE, TBS_AUTOTICKS,
        },
        HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
        Input::KeyboardAndMouse::{
            GetKeyState, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey,
            SetFocus, UnregisterHotKey, VK_CONTROL, VK_ESCAPE, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        },
        Shell::{
            DefSubclassProc, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION,
            NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, SetWindowSubclass,
            Shell_NotifyIconW,
        },
        WindowsAndMessaging::*,
    },
};

use crate::{
    config::{Config, MAX_SCALE_PERCENT, MIN_SCALE_PERCENT, parse_hex},
    hotkeys::Hotkey,
    image_processing::{OriginalImage, ProcessedImage},
};

const SETTINGS_CLASS: &str = "LightCrosshair.Settings";
const OVERLAY_CLASS: &str = "LightCrosshair.Overlay";
const MUTEX_NAME: &str = "Local\\LightCrosshair.SingleInstance";
const APP_ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

const WM_TRAY: u32 = WM_APP + 1;
const WM_SHOW_SETTINGS: u32 = WM_APP + 2;
const WM_CAPTURED_HOTKEY: u32 = WM_APP + 3;

const ID_LOAD: i32 = 1001;
const ID_TINT_ENABLED: i32 = 1002;
const ID_TINT_HEX: i32 = 1003;
const ID_TINT_PICK: i32 = 1004;
const ID_SCALE: i32 = 1005;
const ID_SCALE_VALUE: i32 = 1006;
const ID_MONITOR: i32 = 1007;
const ID_SCALE_INPUT: i32 = 1008;
const ID_HOTKEY_CENTER: i32 = 1101;
const ID_HOTKEY_HIDE: i32 = 1102;
const ID_HOTKEY_SHOW: i32 = 1103;
const ID_HOTKEY_TOGGLE: i32 = 1104;
const ID_CENTER_NOW: i32 = 1201;
const ID_HIDE_NOW: i32 = 1202;

const MENU_SETTINGS: usize = 2001;
const MENU_CENTER: usize = 2002;
const MENU_SHOW: usize = 2003;
const MENU_HIDE: usize = 2004;
const MENU_LOAD: usize = 2005;
const MENU_EXIT: usize = 2006;

const HOTKEY_CENTER: i32 = 1;
const HOTKEY_HIDE: i32 = 2;
const HOTKEY_SHOW: i32 = 3;
const HOTKEY_TOGGLE: i32 = 4;

const EN_CHANGE: u16 = 0x0300;
const EN_KILLFOCUS: u16 = 0x0200;
const CBN_SELCHANGE: u16 = 1;
const BN_CLICKED: u16 = 0;
const CB_ADDSTRING: u32 = 0x0143;
const CB_GETCURSEL: u32 = 0x0147;
const CB_RESETCONTENT: u32 = 0x014B;
const CB_SETCURSEL: u32 = 0x014E;
const TBM_GETPOS: u32 = WM_USER;

#[derive(Clone)]
struct Monitor {
    device: String,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    primary: bool,
}

struct Controls {
    image_name: HWND,
    tint_enabled: HWND,
    tint_hex: HWND,
    scale: HWND,
    scale_input: HWND,
    scale_value: HWND,
    monitor: HWND,
    hotkey_center: HWND,
    hotkey_hide: HWND,
    hotkey_show: HWND,
    hotkey_toggle: HWND,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            image_name: null_mut(),
            tint_enabled: null_mut(),
            tint_hex: null_mut(),
            scale: null_mut(),
            scale_input: null_mut(),
            scale_value: null_mut(),
            monitor: null_mut(),
            hotkey_center: null_mut(),
            hotkey_hide: null_mut(),
            hotkey_show: null_mut(),
            hotkey_toggle: null_mut(),
        }
    }
}

struct App {
    instance: HINSTANCE,
    icon: HICON,
    settings: HWND,
    overlay: HWND,
    controls: Controls,
    config: Config,
    monitors: Vec<Monitor>,
    original: Option<OriginalImage>,
    processed: Option<ProcessedImage>,
    tray: NOTIFYICONDATAW,
    initializing: bool,
    exiting: bool,
    config_error_shown: bool,
}

impl App {
    fn new(instance: HINSTANCE, icon: HICON) -> Self {
        Self {
            instance,
            icon,
            settings: null_mut(),
            overlay: null_mut(),
            controls: Controls::default(),
            config: Config::load(),
            monitors: enumerate_monitors(),
            original: None,
            processed: None,
            tray: NOTIFYICONDATAW::default(),
            initializing: true,
            exiting: false,
            config_error_shown: false,
        }
    }

    fn initialize(&mut self) {
        self.refresh_monitor_control();
        self.sync_controls();
        self.initializing = false;
        self.add_tray_icon();
        self.register_initial_hotkeys();

        let loaded = self
            .config
            .image_path
            .clone()
            .is_some_and(|path| self.load_image(path, false));
        if !loaded {
            unsafe { ShowWindow(self.settings, SW_SHOWNORMAL) };
        }
    }

    fn sync_controls(&mut self) {
        unsafe {
            set_text(
                self.controls.image_name,
                self.config
                    .image_path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or("No image selected"),
            );
            SendMessageW(
                self.controls.tint_enabled,
                BM_SETCHECK,
                if self.config.tint_enabled {
                    BST_CHECKED as usize
                } else {
                    BST_UNCHECKED as usize
                },
                0,
            );
            set_text(self.controls.tint_hex, &self.config.tint);
            SendMessageW(
                self.controls.scale,
                TBM_SETRANGE,
                1,
                make_lparam(MIN_SCALE_PERCENT as u16, MAX_SCALE_PERCENT as u16),
            );
            SendMessageW(
                self.controls.scale,
                TBM_SETPOS,
                1,
                self.config.scale as isize,
            );
            self.update_scale_display(true);
            self.update_hotkey_labels();
        }
    }

    fn update_scale_display(&mut self, update_input: bool) {
        if update_input {
            let was_initializing = self.initializing;
            self.initializing = true;
            set_text(
                self.controls.scale_input,
                &format!("{:.2}", self.config.scale as f32 / 100.0),
            );
            self.initializing = was_initializing;
        }
        set_text(
            self.controls.scale_value,
            &format!("x  ({}%)", self.config.scale),
        );
    }

    fn update_hotkey_labels(&self) {
        set_text(
            self.controls.hotkey_center,
            &self.config.center_hotkey.label(),
        );
        set_text(self.controls.hotkey_hide, &self.config.hide_hotkey.label());
        set_text(self.controls.hotkey_show, &self.config.show_hotkey.label());
        set_text(
            self.controls.hotkey_toggle,
            &self.config.toggle_hotkey.label(),
        );
    }

    fn refresh_monitors(&mut self) {
        self.monitors = enumerate_monitors();
        if self.selected_monitor_index().is_none()
            && let Some(primary) = self.monitors.iter().find(|monitor| monitor.primary)
        {
            self.config.monitor = primary.device.clone();
            self.save_config();
        }
        self.refresh_monitor_control();
        self.center_overlay();
    }

    fn refresh_monitor_control(&mut self) {
        if self.controls.monitor.is_null() {
            return;
        }
        unsafe { SendMessageW(self.controls.monitor, CB_RESETCONTENT, 0, 0) };
        let mut selected = 0;
        for (index, monitor) in self.monitors.iter().enumerate() {
            let name = if monitor.primary {
                format!(
                    "Primary Monitor — {}×{} @ ({},{})",
                    monitor.width, monitor.height, monitor.left, monitor.top
                )
            } else {
                format!(
                    "Monitor {} — {}×{} @ ({},{})",
                    index + 1,
                    monitor.width,
                    monitor.height,
                    monitor.left,
                    monitor.top
                )
            };
            let wide = wide(&name);
            unsafe {
                SendMessageW(
                    self.controls.monitor,
                    CB_ADDSTRING,
                    0,
                    wide.as_ptr() as isize,
                );
            }
            if monitor.device == self.config.monitor
                || (self.config.monitor.is_empty() && monitor.primary)
            {
                selected = index;
            }
        }
        if let Some(monitor) = self.monitors.get(selected) {
            self.config.monitor = monitor.device.clone();
        }
        unsafe {
            SendMessageW(self.controls.monitor, CB_SETCURSEL, selected, 0);
        }
    }

    fn selected_monitor_index(&self) -> Option<usize> {
        self.monitors
            .iter()
            .position(|monitor| monitor.device == self.config.monitor)
            .or_else(|| self.monitors.iter().position(|monitor| monitor.primary))
            .or((!self.monitors.is_empty()).then_some(0))
    }

    fn selected_monitor(&self) -> Option<&Monitor> {
        self.selected_monitor_index()
            .and_then(|index| self.monitors.get(index))
    }

    fn load_image(&mut self, path: PathBuf, user_selected: bool) -> bool {
        match OriginalImage::load_png(&path) {
            Ok(image) => {
                self.original = Some(image);
                self.config.image_path = Some(path);
                self.rebuild_processed();
                self.save_config();
                set_text(
                    self.controls.image_name,
                    self.config
                        .image_path
                        .as_ref()
                        .and_then(|value| value.file_name())
                        .and_then(|value| value.to_str())
                        .unwrap_or("Crosshair loaded"),
                );
                true
            }
            Err(_) => {
                self.original = None;
                self.processed = None;
                unsafe { ShowWindow(self.overlay, SW_HIDE) };
                if user_selected {
                    message(
                        self.settings,
                        "Unable to load this image.",
                        "LightCrosshair",
                    );
                } else {
                    self.config.image_path = None;
                    self.save_config();
                    message(
                        self.settings,
                        "The saved crosshair image is missing or invalid. Choose another PNG.",
                        "LightCrosshair",
                    );
                }
                false
            }
        }
    }

    fn rebuild_processed(&mut self) {
        let tint = if self.config.tint_enabled {
            parse_hex(&self.config.tint)
        } else {
            None
        };
        self.processed = if self.config.scale == 0 {
            None
        } else {
            self.original
                .as_ref()
                .map(|image| image.process(tint, self.config.scale))
        };
        if self.processed.is_none() {
            unsafe { ShowWindow(self.overlay, SW_HIDE) };
        }
        self.redraw_overlay();
    }

    fn redraw_overlay(&self) {
        let (Some(image), Some(monitor)) = (&self.processed, self.selected_monitor()) else {
            return;
        };
        let (x, y) = centered_position(monitor, image.width as i32, image.height as i32);

        unsafe {
            let memory_dc = CreateCompatibleDC(null_mut());
            if memory_dc.is_null() {
                return;
            }
            let mut bitmap_info: BITMAPINFO = zeroed();
            bitmap_info.bmiHeader = BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: image.width as i32,
                biHeight: -(image.height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..zeroed()
            };
            let mut bits: *mut c_void = null_mut();
            let bitmap = CreateDIBSection(
                memory_dc,
                &bitmap_info,
                DIB_RGB_COLORS,
                &mut bits,
                null_mut(),
                0,
            );
            if bitmap.is_null() || bits.is_null() {
                DeleteDC(memory_dc);
                return;
            }
            copy_nonoverlapping(
                image.premultiplied_bgra.as_ptr(),
                bits.cast::<u8>(),
                image.premultiplied_bgra.len(),
            );
            let old_bitmap = SelectObject(memory_dc, bitmap as HGDIOBJ);
            let destination = POINT { x, y };
            let source = POINT { x: 0, y: 0 };
            let size = SIZE {
                cx: image.width as i32,
                cy: image.height as i32,
            };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            UpdateLayeredWindow(
                self.overlay,
                null_mut(),
                &destination,
                &size,
                memory_dc,
                &source,
                0,
                &blend,
                ULW_ALPHA,
            );
            SelectObject(memory_dc, old_bitmap);
            DeleteObject(bitmap as HGDIOBJ);
            DeleteDC(memory_dc);

            if self.config.visible {
                SetWindowPos(
                    self.overlay,
                    HWND_TOPMOST,
                    x,
                    y,
                    image.width as i32,
                    image.height as i32,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            } else {
                ShowWindow(self.overlay, SW_HIDE);
            }
        }
    }

    fn center_overlay(&self) {
        let (Some(image), Some(monitor)) = (&self.processed, self.selected_monitor()) else {
            return;
        };
        let (x, y) = centered_position(monitor, image.width as i32, image.height as i32);
        unsafe {
            SetWindowPos(
                self.overlay,
                HWND_TOPMOST,
                x,
                y,
                image.width as i32,
                image.height as i32,
                SWP_NOACTIVATE
                    | if self.config.visible {
                        SWP_SHOWWINDOW
                    } else {
                        SWP_NOSIZE
                    },
            );
        }
    }

    fn set_visible(&mut self, visible: bool) {
        self.config.visible = visible;
        if visible && self.processed.is_some() {
            self.redraw_overlay();
        } else {
            unsafe { ShowWindow(self.overlay, SW_HIDE) };
        }
        self.save_config();
    }

    fn show_settings(&self) {
        unsafe {
            ShowWindow(self.settings, SW_SHOWNORMAL);
            SetForegroundWindow(self.settings);
        }
    }

    fn choose_image(&mut self) {
        let mut path = [0u16; 32768];
        let filter = wide("PNG images (*.png)\0*.png\0All files (*.*)\0*.*\0");
        let title = wide("Load Crosshair");
        let extension = wide("png");
        let mut dialog = OPENFILENAMEW {
            lStructSize: size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: self.settings,
            lpstrFilter: filter.as_ptr(),
            lpstrFile: path.as_mut_ptr(),
            nMaxFile: path.len() as u32,
            lpstrTitle: title.as_ptr(),
            lpstrDefExt: extension.as_ptr(),
            Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST,
            ..OPENFILENAMEW::default()
        };
        if unsafe { GetOpenFileNameW(&mut dialog) } != 0 {
            let length = path
                .iter()
                .position(|value| *value == 0)
                .unwrap_or(path.len());
            self.load_image(
                PathBuf::from(String::from_utf16_lossy(&path[..length])),
                true,
            );
        }
    }

    fn choose_tint(&mut self) {
        let [red, green, blue] = parse_hex(&self.config.tint).unwrap_or([0, 255, 0]);
        let mut custom = [0u32; 16];
        let mut picker = CHOOSECOLORW {
            lStructSize: size_of::<CHOOSECOLORW>() as u32,
            hwndOwner: self.settings,
            rgbResult: red as u32 | (green as u32) << 8 | (blue as u32) << 16,
            lpCustColors: custom.as_mut_ptr(),
            Flags: CC_FULLOPEN | CC_RGBINIT,
            ..CHOOSECOLORW::default()
        };
        if unsafe { ChooseColorW(&mut picker) } != 0 {
            self.config.tint = format!(
                "#{:02X}{:02X}{:02X}",
                picker.rgbResult & 0xff,
                (picker.rgbResult >> 8) & 0xff,
                (picker.rgbResult >> 16) & 0xff
            );
            self.initializing = true;
            set_text(self.controls.tint_hex, &self.config.tint);
            self.initializing = false;
            self.rebuild_processed();
            self.save_config();
        }
    }

    fn register_initial_hotkeys(&self) {
        for (id, hotkey) in [
            (HOTKEY_CENTER, self.config.center_hotkey),
            (HOTKEY_HIDE, self.config.hide_hotkey),
            (HOTKEY_SHOW, self.config.show_hotkey),
            (HOTKEY_TOGGLE, self.config.toggle_hotkey),
        ] {
            if !register_hotkey(self.settings, id, hotkey) {
                message(
                    self.settings,
                    &format!(
                        "{} could not be registered. This shortcut is already being used by Windows or another application.",
                        hotkey.label()
                    ),
                    "Hotkey conflict",
                );
            }
        }
    }

    fn rebind_hotkey(&mut self, control_id: i32, hotkey: Hotkey) {
        let (registration_id, old) = match control_id {
            ID_HOTKEY_CENTER => (HOTKEY_CENTER, self.config.center_hotkey),
            ID_HOTKEY_HIDE => (HOTKEY_HIDE, self.config.hide_hotkey),
            ID_HOTKEY_SHOW => (HOTKEY_SHOW, self.config.show_hotkey),
            ID_HOTKEY_TOGGLE => (HOTKEY_TOGGLE, self.config.toggle_hotkey),
            _ => return,
        };
        unsafe { UnregisterHotKey(self.settings, registration_id) };
        if !hotkey.valid() || !register_hotkey(self.settings, registration_id, hotkey) {
            register_hotkey(self.settings, registration_id, old);
            message(
                self.settings,
                "This shortcut is already being used by Windows or another application. Use at least one modifier plus a non-modifier key.",
                "Hotkey conflict",
            );
            self.update_hotkey_labels();
            return;
        }

        match control_id {
            ID_HOTKEY_CENTER => self.config.center_hotkey = hotkey,
            ID_HOTKEY_HIDE => self.config.hide_hotkey = hotkey,
            ID_HOTKEY_SHOW => self.config.show_hotkey = hotkey,
            ID_HOTKEY_TOGGLE => self.config.toggle_hotkey = hotkey,
            _ => {}
        }
        self.update_hotkey_labels();
        self.save_config();
    }

    fn add_tray_icon(&mut self) {
        self.tray = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.settings,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAY,
            hIcon: self.icon,
            ..NOTIFYICONDATAW::default()
        };
        copy_wide(&mut self.tray.szTip, "LightCrosshair");
        unsafe {
            Shell_NotifyIconW(NIM_ADD, &self.tray);
            self.tray.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            Shell_NotifyIconW(NIM_SETVERSION, &self.tray);
        }
    }

    fn show_tray_menu(&self) {
        unsafe {
            let menu = CreatePopupMenu();
            if menu.is_null() {
                return;
            }
            append_menu(menu, MENU_SETTINGS, "Settings");
            append_menu(menu, MENU_CENTER, "Center Crosshair");
            append_menu(menu, MENU_SHOW, "Show Crosshair");
            append_menu(menu, MENU_HIDE, "Hide Crosshair");
            append_menu(menu, MENU_LOAD, "Load Crosshair");
            AppendMenuW(menu, MF_SEPARATOR, 0, null());
            append_menu(menu, MENU_EXIT, "Exit");
            let mut point = POINT::default();
            GetCursorPos(&mut point);
            SetForegroundWindow(self.settings);
            TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                self.settings,
                null(),
            );
            DestroyMenu(menu);
        }
    }

    fn save_config(&mut self) {
        if self.config.save().is_err() && !self.config_error_shown {
            self.config_error_shown = true;
            message(
                self.settings,
                "Settings could not be saved to %APPDATA%\\LightCrosshair.",
                "LightCrosshair",
            );
        }
    }

    fn exit(&mut self) {
        if self.exiting {
            return;
        }
        self.exiting = true;
        let _ = self.config.save();
        unsafe {
            for id in HOTKEY_CENTER..=HOTKEY_TOGGLE {
                UnregisterHotKey(self.settings, id);
            }
            Shell_NotifyIconW(NIM_DELETE, &self.tray);
            if !self.overlay.is_null() {
                DestroyWindow(self.overlay);
            }
            DestroyWindow(self.settings);
        }
    }
}

pub fn run() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let mutex_name = wide(MUTEX_NAME);
        let mutex = CreateMutexW(null(), 0, mutex_name.as_ptr());
        if !mutex.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
            let class = wide(SETTINGS_CLASS);
            let existing = FindWindowW(class.as_ptr(), null());
            if !existing.is_null() {
                PostMessageW(existing, WM_SHOW_SETTINGS, 0, 0);
            }
            CloseHandle(mutex);
            return;
        }

        let instance = GetModuleHandleW(null());
        let icon = create_app_icon();
        register_window_classes(instance, icon);
        let controls = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES,
        };
        InitCommonControlsEx(&controls);

        let mut app = Box::new(App::new(instance, icon));
        let app_pointer = (&mut *app as *mut App).cast::<c_void>();
        let settings_class = wide(SETTINGS_CLASS);
        let title = wide("LightCrosshair Settings");
        app.settings = CreateWindowExW(
            0,
            settings_class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            540,
            480,
            null_mut(),
            null_mut(),
            instance,
            app_pointer,
        );
        if app.settings.is_null() {
            message(
                null_mut(),
                "Unable to create the settings window.",
                "LightCrosshair",
            );
            if !mutex.is_null() {
                CloseHandle(mutex);
            }
            return;
        }

        let overlay_class = wide(OVERLAY_CLASS);
        app.overlay = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            overlay_class.as_ptr(),
            wide("LightCrosshair Overlay").as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            null_mut(),
            null_mut(),
            instance,
            app_pointer,
        );
        if app.overlay.is_null() {
            message(
                app.settings,
                "Unable to create the overlay window.",
                "LightCrosshair",
            );
            DestroyWindow(app.settings);
            if !mutex.is_null() {
                CloseHandle(mutex);
            }
            return;
        }

        app.initialize();
        let mut message_data = MSG::default();
        while GetMessageW(&mut message_data, null_mut(), 0, 0) > 0 {
            TranslateMessage(&message_data);
            DispatchMessageW(&message_data);
        }

        if !mutex.is_null() {
            CloseHandle(mutex);
        }
    }
}

unsafe fn create_app_icon() -> HICON {
    let icon = unsafe {
        CreateIconFromResourceEx(
            APP_ICON_PNG.as_ptr(),
            APP_ICON_PNG.len() as u32,
            1,
            0x00030000,
            GetSystemMetrics(SM_CXICON),
            GetSystemMetrics(SM_CYICON),
            LR_DEFAULTCOLOR,
        )
    };
    if icon.is_null() {
        unsafe { LoadIconW(null_mut(), IDI_APPLICATION) }
    } else {
        icon
    }
}

unsafe fn register_window_classes(instance: HINSTANCE, icon: HICON) {
    let cursor = unsafe { LoadCursorW(null_mut(), IDC_ARROW) };
    let settings_name = wide(SETTINGS_CLASS);
    let settings_class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(settings_proc),
        hInstance: instance,
        hIcon: icon,
        hCursor: cursor,
        hbrBackground: (COLOR_WINDOW + 1) as _,
        lpszClassName: settings_name.as_ptr(),
        ..unsafe { zeroed() }
    };
    unsafe { RegisterClassW(&settings_class) };

    let overlay_name = wide(OVERLAY_CLASS);
    let overlay_class = WNDCLASSW {
        lpfnWndProc: Some(overlay_proc),
        hInstance: instance,
        hCursor: cursor,
        lpszClassName: overlay_name.as_ptr(),
        ..unsafe { zeroed() }
    };
    unsafe { RegisterClassW(&overlay_class) };
}

unsafe extern "system" fn settings_proc(
    hwnd: HWND,
    message_id: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message_id == WM_NCCREATE {
            let create = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
        if pointer.is_null() {
            return DefWindowProcW(hwnd, message_id, wparam, lparam);
        }
        let app = &mut *pointer;

        match message_id {
            WM_CREATE => {
                create_settings_controls(app, hwnd);
                0
            }
            WM_COMMAND => {
                let id = loword(wparam) as i32;
                let notification = hiword(wparam);
                match (id, notification) {
                    (ID_LOAD, BN_CLICKED) => app.choose_image(),
                    (ID_TINT_ENABLED, BN_CLICKED) => {
                        app.config.tint_enabled =
                            SendMessageW(app.controls.tint_enabled, BM_GETCHECK, 0, 0)
                                == BST_CHECKED as isize;
                        app.rebuild_processed();
                        app.save_config();
                    }
                    (ID_TINT_HEX, EN_CHANGE) if !app.initializing => {
                        let value = get_text(app.controls.tint_hex);
                        if parse_hex(&value).is_some() {
                            app.config.tint = value.to_uppercase();
                            app.rebuild_processed();
                            app.save_config();
                        }
                    }
                    (ID_TINT_PICK, BN_CLICKED) => app.choose_tint(),
                    (ID_SCALE_INPUT, EN_CHANGE) if !app.initializing => {
                        if let Some(scale) =
                            parse_scale_percent(&get_text(app.controls.scale_input))
                            && scale != app.config.scale
                        {
                            app.config.scale = scale;
                            SendMessageW(app.controls.scale, TBM_SETPOS, 1, scale as isize);
                            app.update_scale_display(false);
                            app.rebuild_processed();
                            app.save_config();
                        }
                    }
                    (ID_SCALE_INPUT, EN_KILLFOCUS) => app.update_scale_display(true),
                    (ID_MONITOR, CBN_SELCHANGE) => {
                        let selected = SendMessageW(app.controls.monitor, CB_GETCURSEL, 0, 0);
                        if selected >= 0
                            && let Some(monitor) = app.monitors.get(selected as usize)
                        {
                            app.config.monitor = monitor.device.clone();
                            app.center_overlay();
                            app.save_config();
                        }
                    }
                    (ID_CENTER_NOW, BN_CLICKED) => app.center_overlay(),
                    (ID_HIDE_NOW, BN_CLICKED) => app.set_visible(false),
                    (id, _) if id == MENU_SETTINGS as i32 => app.show_settings(),
                    (id, _) if id == MENU_CENTER as i32 => app.center_overlay(),
                    (id, _) if id == MENU_SHOW as i32 => app.set_visible(true),
                    (id, _) if id == MENU_HIDE as i32 => app.set_visible(false),
                    (id, _) if id == MENU_LOAD as i32 => {
                        app.show_settings();
                        app.choose_image();
                    }
                    (id, _) if id == MENU_EXIT as i32 => app.exit(),
                    _ => {}
                }
                0
            }
            WM_HSCROLL if wparam != 0 || lparam == app.controls.scale as isize => {
                app.config.scale = SendMessageW(app.controls.scale, TBM_GETPOS, 0, 0) as u32;
                app.update_scale_display(true);
                app.rebuild_processed();
                app.save_config();
                0
            }
            WM_HOTKEY => {
                match wparam as i32 {
                    HOTKEY_CENTER => app.center_overlay(),
                    HOTKEY_HIDE => app.set_visible(false),
                    HOTKEY_SHOW => app.set_visible(true),
                    HOTKEY_TOGGLE => app.set_visible(!app.config.visible),
                    _ => {}
                }
                0
            }
            WM_CAPTURED_HOTKEY => {
                if lparam == 0 {
                    app.update_hotkey_labels();
                } else {
                    let encoded = lparam as u64;
                    app.rebind_hotkey(
                        wparam as i32,
                        Hotkey {
                            modifiers: encoded as u32,
                            key: (encoded >> 32) as u32,
                        },
                    );
                }
                SetFocus(hwnd);
                0
            }
            WM_TRAY => {
                let event = lparam as u32 & 0xffff;
                if event == WM_CONTEXTMENU || event == WM_RBUTTONUP {
                    app.show_tray_menu();
                } else if event == NIN_SELECT || event == WM_LBUTTONDBLCLK {
                    app.show_settings();
                }
                0
            }
            WM_SHOW_SETTINGS => {
                app.show_settings();
                0
            }
            WM_DISPLAYCHANGE => {
                app.refresh_monitors();
                0
            }
            WM_DPICHANGED => {
                let suggested = &*(lparam as *const RECT);
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
                0
            }
            WM_CLOSE => {
                ShowWindow(hwnd, SW_HIDE);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, message_id, wparam, lparam),
        }
    }
}

unsafe extern "system" fn overlay_proc(
    hwnd: HWND,
    message_id: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message_id == WM_NCCREATE {
            let create = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        match message_id {
            WM_NCHITTEST => HTTRANSPARENT as isize,
            WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
            WM_DPICHANGED => {
                let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut App;
                if !pointer.is_null() {
                    (*pointer).center_overlay();
                }
                0
            }
            _ => DefWindowProcW(hwnd, message_id, wparam, lparam),
        }
    }
}

unsafe extern "system" fn hotkey_edit_proc(
    hwnd: HWND,
    message_id: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    control_id: usize,
) -> LRESULT {
    unsafe {
        match message_id {
            WM_SETFOCUS => {
                set_text(hwnd, "Press shortcut…");
                return 0;
            }
            WM_KILLFOCUS => {
                let parent = GetParent(hwnd);
                let pointer = GetWindowLongPtrW(parent, GWLP_USERDATA) as *mut App;
                if !pointer.is_null() {
                    (*pointer).update_hotkey_labels();
                }
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let key = wparam as u32;
                if key == VK_ESCAPE as u32 {
                    PostMessageW(GetParent(hwnd), WM_CAPTURED_HOTKEY, control_id, 0);
                    return 0;
                }
                if [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN].contains(&(key as u16)) {
                    return 0;
                }
                let mut modifiers = 0;
                if GetKeyState(VK_CONTROL as i32) < 0 {
                    modifiers |= MOD_CONTROL;
                }
                if GetKeyState(VK_MENU as i32) < 0 {
                    modifiers |= MOD_ALT;
                }
                if GetKeyState(VK_SHIFT as i32) < 0 {
                    modifiers |= MOD_SHIFT;
                }
                if GetKeyState(VK_LWIN as i32) < 0 || GetKeyState(VK_RWIN as i32) < 0 {
                    modifiers |= MOD_WIN;
                }
                let encoded = ((key as u64) << 32 | modifiers as u64) as isize;
                PostMessageW(GetParent(hwnd), WM_CAPTURED_HOTKEY, control_id, encoded);
                return 0;
            }
            _ => {}
        }
        DefSubclassProc(hwnd, message_id, wparam, lparam)
    }
}

unsafe fn create_settings_controls(app: &mut App, hwnd: HWND) {
    unsafe {
        let font = GetStockObject(17) as HFONT;
        create_label(app, hwnd, "Crosshair", 18, 14, 490, 22, font);
        create_label(app, hwnd, "Image:", 24, 49, 58, 22, font);
        app.controls.image_name = create_control(
            app,
            hwnd,
            "STATIC",
            "No image selected",
            WS_CHILD | WS_VISIBLE,
            84,
            49,
            315,
            22,
            0,
            font,
        );
        create_control(
            app,
            hwnd,
            "BUTTON",
            "Load",
            WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32,
            410,
            43,
            92,
            28,
            ID_LOAD,
            font,
        );
        app.controls.tint_enabled = create_control(
            app,
            hwnd,
            "BUTTON",
            "Tint",
            WS_CHILD | WS_VISIBLE | BS_AUTOCHECKBOX as u32,
            24,
            84,
            66,
            25,
            ID_TINT_ENABLED,
            font,
        );
        app.controls.tint_hex = create_control(
            app,
            hwnd,
            "EDIT",
            "#00FF00",
            WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL as u32,
            96,
            84,
            116,
            25,
            ID_TINT_HEX,
            font,
        );
        create_control(
            app,
            hwnd,
            "BUTTON",
            "Pick color",
            WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32,
            224,
            82,
            100,
            28,
            ID_TINT_PICK,
            font,
        );
        create_label(app, hwnd, "Scale:", 24, 125, 58, 22, font);
        app.controls.scale = create_control(
            app,
            hwnd,
            "msctls_trackbar32",
            "",
            WS_CHILD | WS_VISIBLE | TBS_AUTOTICKS,
            80,
            116,
            270,
            40,
            ID_SCALE,
            font,
        );
        app.controls.scale_input = create_control(
            app,
            hwnd,
            "EDIT",
            "1.00",
            WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL as u32 | ES_CENTER as u32,
            358,
            122,
            64,
            25,
            ID_SCALE_INPUT,
            font,
        );
        app.controls.scale_value = create_control(
            app,
            hwnd,
            "STATIC",
            "x  (100%)",
            WS_CHILD | WS_VISIBLE,
            428,
            125,
            80,
            22,
            ID_SCALE_VALUE,
            font,
        );
        create_label(app, hwnd, "Display:", 24, 170, 64, 22, font);
        app.controls.monitor = create_control(
            app,
            hwnd,
            "COMBOBOX",
            "",
            WS_CHILD | WS_VISIBLE | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
            96,
            166,
            406,
            200,
            ID_MONITOR,
            font,
        );

        create_label(app, hwnd, "Hotkeys", 18, 213, 490, 22, font);
        app.controls.hotkey_center =
            hotkey_control(app, hwnd, "Center:", 248, ID_HOTKEY_CENTER, font);
        app.controls.hotkey_hide = hotkey_control(app, hwnd, "Hide:", 283, ID_HOTKEY_HIDE, font);
        app.controls.hotkey_show = hotkey_control(app, hwnd, "Show:", 318, ID_HOTKEY_SHOW, font);
        app.controls.hotkey_toggle =
            hotkey_control(app, hwnd, "Toggle:", 353, ID_HOTKEY_TOGGLE, font);

        create_control(
            app,
            hwnd,
            "BUTTON",
            "Center Now",
            WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32,
            24,
            400,
            126,
            30,
            ID_CENTER_NOW,
            font,
        );
        create_control(
            app,
            hwnd,
            "BUTTON",
            "Hide Crosshair",
            WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON as u32,
            164,
            400,
            132,
            30,
            ID_HIDE_NOW,
            font,
        );
    }
}

unsafe fn hotkey_control(
    app: &App,
    parent: HWND,
    label: &str,
    y: i32,
    id: i32,
    font: HFONT,
) -> HWND {
    unsafe {
        create_label(app, parent, label, 24, y + 4, 66, 22, font);
        let control = create_control(
            app,
            parent,
            "EDIT",
            "",
            WS_CHILD | WS_VISIBLE | WS_BORDER | ES_READONLY as u32 | ES_CENTER as u32,
            96,
            y,
            228,
            27,
            id,
            font,
        );
        SetWindowSubclass(control, Some(hotkey_edit_proc), 1, id as usize);
        control
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn create_label(
    app: &App,
    parent: HWND,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    font: HFONT,
) -> HWND {
    unsafe {
        create_control(
            app,
            parent,
            "STATIC",
            text,
            WS_CHILD | WS_VISIBLE,
            x,
            y,
            width,
            height,
            0,
            font,
        )
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn create_control(
    app: &App,
    parent: HWND,
    class: &str,
    text: &str,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    id: i32,
    font: HFONT,
) -> HWND {
    let class = wide(class);
    let text = wide(text);
    let control = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            text.as_ptr(),
            style,
            x,
            y,
            width,
            height,
            parent,
            id as usize as HMENU,
            app.instance,
            null(),
        )
    };
    unsafe { SendMessageW(control, WM_SETFONT, font as usize, 1) };
    control
}

fn enumerate_monitors() -> Vec<Monitor> {
    unsafe extern "system" fn callback(
        monitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> i32 {
        unsafe {
            let monitors = &mut *(data as *mut Vec<Monitor>);
            let mut info: MONITORINFOEXW = zeroed();
            info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
            if GetMonitorInfoW(monitor, &mut info.monitorInfo as *mut MONITORINFO) != 0 {
                let bounds = info.monitorInfo.rcMonitor;
                let end = info
                    .szDevice
                    .iter()
                    .position(|character| *character == 0)
                    .unwrap_or(info.szDevice.len());
                monitors.push(Monitor {
                    device: String::from_utf16_lossy(&info.szDevice[..end]),
                    left: bounds.left,
                    top: bounds.top,
                    width: bounds.right - bounds.left,
                    height: bounds.bottom - bounds.top,
                    primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                });
            }
            1
        }
    }

    let mut monitors = Vec::<Monitor>::new();
    unsafe {
        EnumDisplayMonitors(
            null_mut(),
            null(),
            Some(callback),
            &mut monitors as *mut Vec<Monitor> as isize,
        );
    }
    monitors.sort_by_key(|monitor| (!monitor.primary, monitor.left, monitor.top));
    monitors
}

fn centered_position(monitor: &Monitor, width: i32, height: i32) -> (i32, i32) {
    (
        monitor.left + (monitor.width - width) / 2,
        monitor.top + (monitor.height - height) / 2,
    )
}

fn parse_scale_percent(value: &str) -> Option<u32> {
    let value = value.trim();
    let value = value
        .strip_suffix('x')
        .or_else(|| value.strip_suffix('X'))
        .unwrap_or(value)
        .trim();
    let multiplier = value.parse::<f64>().ok()?;
    if !multiplier.is_finite() || !(0.0..=5.0).contains(&multiplier) {
        return None;
    }
    Some((multiplier * 100.0).round() as u32)
}

fn register_hotkey(window: HWND, id: i32, hotkey: Hotkey) -> bool {
    unsafe { RegisterHotKey(window, id, hotkey.modifiers | MOD_NOREPEAT, hotkey.key) != 0 }
}

unsafe fn append_menu(menu: HMENU, id: usize, text: &str) {
    let text = wide(text);
    unsafe { AppendMenuW(menu, MF_STRING, id, text.as_ptr()) };
}

fn set_text(window: HWND, text: &str) {
    let text = wide(text);
    unsafe { SetWindowTextW(window, text.as_ptr()) };
}

fn get_text(window: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(window) };
    let mut text = vec![0u16; length as usize + 1];
    unsafe { GetWindowTextW(window, text.as_mut_ptr(), text.len() as i32) };
    String::from_utf16_lossy(&text[..length as usize])
}

fn message(owner: HWND, text: &str, title: &str) {
    let text = wide(text);
    let title = wide(title);
    unsafe { MessageBoxW(owner, text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONWARNING) };
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn copy_wide<const N: usize>(destination: &mut [u16; N], value: &str) {
    let value: Vec<u16> = value.encode_utf16().take(N - 1).collect();
    destination[..value.len()].copy_from_slice(&value);
}

fn loword(value: usize) -> u16 {
    value as u16
}

fn hiword(value: usize) -> u16 {
    (value >> 16) as u16
}

fn make_lparam(low: u16, high: u16) -> isize {
    (low as u32 | (high as u32) << 16) as isize
}

#[cfg(test)]
mod tests {
    use super::{Monitor, centered_position, parse_scale_percent};

    #[test]
    fn centers_even_and_odd_sizes_in_physical_pixels() {
        let monitor = Monitor {
            device: String::new(),
            left: -1920,
            top: 0,
            width: 1920,
            height: 1080,
            primary: false,
        };
        assert_eq!(centered_position(&monitor, 32, 32), (-976, 524));
        assert_eq!(centered_position(&monitor, 33, 33), (-977, 523));
    }

    #[test]
    fn parses_exact_scale_multiplier() {
        assert_eq!(parse_scale_percent("0"), Some(0));
        assert_eq!(parse_scale_percent("1.37x"), Some(137));
        assert_eq!(parse_scale_percent("5.00"), Some(500));
        assert_eq!(parse_scale_percent("5.01"), None);
        assert_eq!(parse_scale_percent("nope"), None);
    }
}
