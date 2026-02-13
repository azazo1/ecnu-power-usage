//! 检测屏幕休眠状态

#[cfg(target_os = "macos")]
mod macos {
    use core::ffi::c_uint;

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGDisplayIsAsleep(display: c_uint) -> bool;
        // 新增：获取所有在线显示器
        fn CGGetOnlineDisplayList(
            max_displays: u32,
            displays: *mut c_uint,
            display_count: *mut u32,
        ) -> i32;
    }

    /// 检查所有在线显示器是否都处于休眠状态
    /// 如果获取显示器列表失败, 返回 false 并记录错误.
    pub(crate) fn all_displays_asleep() -> crate::Result<bool> {
        const MAX_DISPLAYS: u32 = 32; // 通常足够，可动态调整
        let mut displays = [0u32; MAX_DISPLAYS as usize];
        let mut count: u32 = 0;
        unsafe {
            let error = CGGetOnlineDisplayList(MAX_DISPLAYS, displays.as_mut_ptr(), &mut count);
            if error != 0 {
                return Err(crate::Error::Display(error));
            }
        }
        if count == 0 {
            // 没有在线显示器 → 视为“所有显示器休眠”
            return Ok(true);
        }
        // 遍历每个显示器，只要有一个未休眠即返回 false
        for &disp in displays[..count as usize].iter() {
            if unsafe { !CGDisplayIsAsleep(disp) } {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(target_os = "linux")]
mod linux {
    pub(crate) fn all_displays_asleep() -> crate::Result<bool> {
        Ok(false)
    }
}

#[cfg(target_os = "windows")]
mod windows {
    pub(crate) fn all_displays_asleep() -> crate::Result<bool> {
        Ok(false)
    }
}

#[cfg(target_os = "macos")]
pub(crate) use macos::*;

#[cfg(target_os = "linux")]
pub(crate) use linux::*;

#[cfg(target_os = "windows")]
pub(crate) use windows::*;
