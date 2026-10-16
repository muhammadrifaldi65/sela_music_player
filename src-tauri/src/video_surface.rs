// Child HWND only: libmpv renders inside the artwork bounds, never a top-level window.
#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    type Hwnd = *mut c_void;
    #[link(name = "user32")]
    extern "system" {
        fn CreateWindowExW(
            ex: u32,
            class: *const u16,
            title: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Hwnd,
            menu: Hwnd,
            instance: Hwnd,
            param: Hwnd,
        ) -> Hwnd;
        fn SetWindowPos(
            window: Hwnd,
            after: Hwnd,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
        fn ShowWindow(window: Hwnd, command: i32) -> i32;
    }
    pub fn create(parent: isize) -> Result<isize, String> {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let hwnd = unsafe {
            CreateWindowExW(
                0x08000000,
                class.as_ptr(),
                std::ptr::null(),
                0x40000000 | 0x04000000 | 0x02000000,
                0,
                0,
                1,
                1,
                parent as Hwnd,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if hwnd.is_null() {
            Err(std::io::Error::last_os_error().to_string())
        } else {
            Ok(hwnd as isize)
        }
    }
    pub fn bounds(
        hwnd: isize,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        visible: bool,
    ) -> Result<(), String> {
        let ok = unsafe {
            SetWindowPos(
                hwnd as Hwnd,
                std::ptr::null_mut(),
                x,
                y,
                width,
                height,
                0x0010,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        unsafe {
            ShowWindow(hwnd as Hwnd, if visible { 5 } else { 0 });
        }
        Ok(())
    }
}
#[cfg(windows)]
pub use windows::*;
#[cfg(not(windows))]
pub fn create(_: isize) -> Result<isize, String> {
    Err("Embedded video requires Windows".into())
}
#[cfg(not(windows))]
pub fn bounds(_: isize, _: i32, _: i32, _: i32, _: i32, _: bool) -> Result<(), String> {
    Err("Embedded video requires Windows".into())
}
