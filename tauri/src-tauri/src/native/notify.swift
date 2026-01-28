import Foundation
import UserNotifications

// @convention(c) 确保它符合 C 语言调用约定
public typealias NotificationCallback = @convention(c) (Int32, Int32) -> Void
var rustCallback: NotificationCallback?

let globalNotificationDelegate = AppDelegate()

@_cdecl("register_notification_callback")
public func register_notification_callback(callback: @escaping NotificationCallback) {
    rustCallback = callback
}

@_cdecl("init_macos_notification_delegate")
public func init_macos_notification_delegate() -> Int32 {
    guard let bundleID = Bundle.main.bundleIdentifier, !bundleID.isEmpty else {
        print("Swift: 无 bundle id, 跳过通知代理初始化.")
        return 1
    }
    DispatchQueue.main.async {
        UNUserNotificationCenter.current().delegate = globalNotificationDelegate
    }
    return 0
}

@_cdecl("send_notification")
public func send_notification(identifier: Int32, title: UnsafePointer<CChar>, body: UnsafePointer<CChar>) {
    let titleStr = String(cString: title)
    let bodyStr = String(cString: body)

    let content = UNMutableNotificationContent()
    content.title = titleStr
    content.body = bodyStr
    content.sound = .default

    let request = UNNotificationRequest(
        identifier: String(identifier),
        content: content,
        trigger: nil // 立即发送
    )

    UNUserNotificationCenter.current().add(request) { error in
        if let error = error {
            let errorMsg = "Swift Error: \(error.localizedDescription)"
            print(errorMsg)
            rustCallback?(identifier, -1)
        }
    }
}

public typealias RequestNotificationPermissionCallback = @convention(c) (Int32, UnsafePointer<CChar>) -> Void
@_cdecl("request_notification_permission")
public func request_notification_permission(callback: @escaping RequestNotificationPermissionCallback) {
    let center = UNUserNotificationCenter.current()

    // 申请 声音、弹窗、角标 权限
    center.requestAuthorization(options: [.alert, .sound, .badge]) { granted, error in
        if granted {
            "granted".withCString { ptr in
                callback(0, ptr)
            }
        } else if let error = error {
            "error: \(error.localizedDescription)".withCString { ptr in
                callback(2, ptr)
            }
        } else {
            "declined".withCString { ptr in
                callback(1, ptr)
            }
        }
    }
}

class AppDelegate: NSObject, UNUserNotificationCenterDelegate {
    func userNotificationCenter(_ center: UNUserNotificationCenter,
                                didReceive response: UNNotificationResponse,
                                withCompletionHandler completionHandler: @escaping () -> Void) {

        // 从 identifier 中恢复 Int32
        let identifier = response.notification.request.identifier
        if let code = Int32(identifier) {
            rustCallback?(code, 0)
        } else {
            // 处理系统默认或其他非自定义通知
            rustCallback?(0, 0)
        }

        completionHandler()
    }

    // 确保应用在前台时也能显示通知
    func userNotificationCenter(_ center: UNUserNotificationCenter,
                                willPresent notification: UNNotification,
                                withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        completionHandler([.banner, .list, .sound])
    }
}