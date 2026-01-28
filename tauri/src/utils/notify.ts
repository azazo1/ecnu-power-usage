/// 系统通知

import { invoke } from "@tauri-apps/api/core";

export async function sysNotify(title: string, message: string): Promise<boolean> {
    return await invoke<boolean>("sys_notify", { title, message });
}