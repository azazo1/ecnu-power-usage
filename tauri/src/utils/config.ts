import { invoke } from "@tauri-apps/api/core";

export interface GuiConfig {
    serverBase: string;
    clientCert?: string;
    clientKey?: string;
    rootCA?: string;
    useSelfSignedTls: boolean
}

export async function updateConfigCmd(config: GuiConfig) {
    return await invoke("update_config", { config });
}

export async function getConfigCmd(): Promise<GuiConfig> {
    return await invoke<GuiConfig>("get_config");
}