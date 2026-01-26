import { invoke } from "@tauri-apps/api/core";

export type HealthKind = 'NoNet' | 'ServerDown' | 'NotLogin' | 'NoRoom' | 'Ok' | 'Unknown';
export interface HealthStatus {
    kind: HealthKind,
    message: string | null,
}

/// 检查状态
export async function healthCheck(): Promise<HealthStatus> {
    try {
        let kind: HealthKind = await invoke('health_check');
        return {
            kind,
            message: null,
        }
    } catch (error) {
        return {
            kind: 'Unknown',
            message: String(error)
        }
    }
}