//! 系统通知
//! todo: windows, linux

use core::slice;
use std::{
    ffi::{CStr, CString, NulError},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicI32, Ordering},
    },
    task::{Poll, Waker},
};

use dashmap::DashMap;
use tracing::warn;

static NOTIFICATIONS: OnceLock<DashMap<i32, i32>> = OnceLock::new();
static WAKERS: OnceLock<DashMap<i32, Waker>> = OnceLock::new();

fn notifications() -> &'static DashMap<i32, i32> {
    NOTIFICATIONS.get_or_init(DashMap::new)
}

fn wakers() -> &'static DashMap<i32, Waker> {
    WAKERS.get_or_init(DashMap::new)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NotifyPermission {
    Granted,
    Declined,
    /// swift 处请求权限的时候发生了错误.
    Error(String),
}

static PERMISSION: OnceLock<NotifyPermission> = OnceLock::new();
static PERMISSION_NOTIFY: OnceLock<tokio::sync::Notify> = OnceLock::new();

#[cfg(target_os = "macos")]
use std::ffi::c_char;
#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn init_macos_notification_delegate() -> i32;
    /// (通知标识 ident, 状态代码 code),
    /// 通知标识为 0 的时候, 为未在这里注册的通知被触发.
    /// 状态代码:
    /// - 0 -> 用户跳过通知.
    /// - 1 -> 用户点击触发通知.
    fn register_notification_callback(callback: extern "C" fn(ident: i32, code: i32));
    fn send_notification(ident: i32, title: *const c_char, message: *const c_char);
    /// 申请通知权限.
    /// - `code``: 0 正常, 1 拒绝, 2 错误.
    fn request_notification_permission(callback: extern "C" fn(code: i32, message: *const c_char));
}

extern "C" fn notification_callback(ident: i32, code: i32) {
    let map = notifications();
    map.insert(ident, code);
    let wakers = wakers();
    if let Some(waker) = wakers.get(&ident) {
        waker.wake_by_ref();
    }
}

unsafe fn bounded_strlen(ptr: *const c_char, max_len: usize) -> Option<usize> {
    if ptr.is_null() {
        return None;
    }

    // 使用 slice 进行有限范围的查找
    // 这样即便没有 \0, 扫描也会在 max_len 处停止
    (0..max_len).find(|&i| unsafe { *ptr.add(i) } == 0)
}

extern "C" fn request_permission_callback(code: i32, message: *const c_char) {
    let message = unsafe {
        CStr::from_bytes_with_nul_unchecked(slice::from_raw_parts(
            message.cast(),
            bounded_strlen(message, 512).unwrap_or(0),
        ))
    }
    .to_string_lossy()
    .to_string();
    let perm = match code {
        0 => NotifyPermission::Granted,
        1 => NotifyPermission::Declined,
        2 => NotifyPermission::Error(message),
        _ => {
            warn!("unknown permission callback code: {code}.");
            NotifyPermission::Error(message)
        }
    };
    PERMISSION.set(perm).ok();
    if let Some(notify) = PERMISSION_NOTIFY.get() {
        notify.notify_waiters();
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum Error {
    #[error("notification system init failed")]
    NotificationSystemInitFailed,
    #[error("notification sending failed")]
    NotificationSendFailed,
    #[error("unknown notification response code: {0}")]
    UnknownNotificationResponse(i32),
    #[error(transparent)]
    Nul(#[from] NulError),
}

struct NotificationFuture {
    ident: i32,
    response: Option<Result<bool, Error>>,
}

impl NotificationFuture {
    fn new(ident: i32) -> Self {
        Self {
            ident,
            response: None,
        }
    }

    fn make_response(code: i32) -> Result<bool, Error> {
        match code {
            // 用户跳过通知.
            0 => Ok(false),
            // 用户点击通知.
            1 => Ok(true),
            // 通知发送失败, 用户没有看到通知.
            -1 => Err(Error::NotificationSendFailed),
            // 未知的回复.
            _ => Err(Error::UnknownNotificationResponse(code)),
        }
    }
}

impl Future for NotificationFuture {
    type Output = Result<bool, Error>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        // 如果已经有缓存的响应，直接返回
        if let Some(response) = &self.response {
            return Poll::Ready(response.clone());
        }

        let map = notifications();
        let wakers = wakers();

        // 第一次尝试获取结果
        if let Some((_, v)) = map.remove(&self.ident) {
            let response = Self::make_response(v);
            self.response = Some(response.clone());
            wakers.remove(&self.ident);
            return Poll::Ready(response);
        }

        // <-- 运行到此处可能会触发当前通知的回调 (notification_callback), 可能同时 waker 没注册,
        // 存在漏过的可能性, 所以要在下面再次检查 Map.

        // 没拿到结果, 注册 Waker
        // 使用 will_wake 检查是否需要更新, 避免不必要的克隆
        let needs_update = match wakers.get(&self.ident) {
            Some(existing_waker) => !existing_waker.will_wake(cx.waker()),
            None => true,
        };

        if needs_update {
            wakers.insert(self.ident, cx.waker().clone());
        }

        // 重要: 再次检查 Map, 防止在步骤 2 和 3 之间发生的并发回调 (notification_callback) 被漏掉
        if let Some((_, v)) = map.remove(&self.ident) {
            let response = Self::make_response(v);
            self.response = Some(response.clone());
            wakers.remove(&self.ident);
            return Poll::Ready(response);
        }

        Poll::Pending
    }
}

impl Drop for NotificationFuture {
    fn drop(&mut self) {
        // 确保 Future 销毁时清理掉对应的 waker
        wakers().remove(&self.ident);
    }
}

pub(crate) struct NotifyManager {
    counter: AtomicI32,
    request_perm_lock: tokio::sync::Mutex<()>,
}

impl NotifyManager {
    fn get_next_code(&self) -> i32 {
        let mut current = self.counter.load(Ordering::Relaxed);
        loop {
            let next = if current >= 1_000_000 { 1 } else { current + 1 };

            // 原子性地检查: 如果当前值还是 current，就换成 next
            match self.counter.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return next,
                Err(actual) => current = actual, // 换失败了, 说明被别的线程抢先了, 更新 current 重试
            }
        }
    }

    /// 初始化通知系统.
    /// 如果通知系统初始化失败, 那么返回 Err(())
    pub(crate) fn init() -> Result<Self, Error> {
        #[cfg(target_os = "macos")]
        unsafe {
            if init_macos_notification_delegate() != 0 {
                return Err(Error::NotificationSystemInitFailed);
            }
            register_notification_callback(notification_callback);
        }
        Ok(Self {
            counter: AtomicI32::new(1),
            request_perm_lock: tokio::sync::Mutex::new(()),
        })
    }

    /// 请求通知权限
    pub(crate) async fn request_permission(&self) -> &'static NotifyPermission {
        if let Some(perm) = PERMISSION.get() {
            return perm;
        }
        let notify = PERMISSION_NOTIFY.get_or_init(tokio::sync::Notify::new);
        if let Ok(_lck) = self.request_perm_lock.try_lock()
            && PERMISSION.get().is_none()
        {
            #[cfg(target_os = "macos")]
            unsafe {
                request_notification_permission(request_permission_callback);
            }
        }

        if let Some(perm) = PERMISSION.get() {
            return perm;
        }
        let notified = notify.notified();
        if let Some(perm) = PERMISSION.get() {
            return perm;
        }
        notified.await;
        PERMISSION
            .get()
            .expect("PERMISSION should be set after being notified")
    }

    /// 创建通知.
    /// ```
    /// let clicked = manager.notify(title, message).await.expect("通知发送失败");
    /// if clicked {
    ///     println!("通知被触发");
    /// } else {
    ///     println!("通知被取消");
    /// }
    /// ```
    ///
    /// # Error
    /// 当 title 或者 message 中含有 `\0` 字符时, 返回 [`NulError`].
    pub(crate) async fn notify(&self, title: &str, message: &str) -> Result<bool, Error> {
        let ident = self.get_next_code();
        let title = CString::new(title)?;
        let message = CString::new(message)?;
        #[cfg(target_os = "macos")]
        unsafe {
            send_notification(ident, title.as_ptr(), message.as_ptr());
        }
        NotificationFuture::new(ident).await
    }
}
