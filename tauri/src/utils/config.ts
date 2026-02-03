import { invoke } from "@tauri-apps/api/core";

export interface GuiConfig {
    serverBase: string;
    clientCert?: string;
    clientKey?: string;
    rootCA?: string;
    useSelfSignedTls: boolean
}

export interface Area {
    areaId: string;
    areaName: string;
}

export interface District {
    districtId: string;
    districtName: string;
}

export interface Building {
    buiId: string;
    buiName: string;
}

export interface Floor {
    floorId: string;
    floorName: string;
}

export interface Room {
    roomId: string;
    roomName: string;
}

export interface Districts {
    areas: Area[];
    districts: District[];
    buils: Building[];
    floors: Floor[];
    rooms: Room[];
}

export interface Buildings {
    buils: Building[];
}

export interface Floors {
    floors: Floor[];
}

export interface Rooms {
    rooms: Room[];
}

export interface RoomInfo {
    area: Area;
    district: District;
    building: Building;
    floor: Floor;
    room: Room;
}

export async function updateConfigCmd(config: GuiConfig) {
    return await invoke("update_config", { config });
}

export async function getConfigCmd(): Promise<GuiConfig> {
    return await invoke<GuiConfig>("get_config");
}

export async function pickCertCmd(): Promise<string> {
    return await invoke<string>("pick_cert");
}

export async function clearRoomCmd() {
    await invoke("clear_room");
}

export async function clearCookiesCmd() {
    await invoke("clear_cookies");
}

export async function getRoomInfoCmd(): Promise<RoomInfo> {
    return await invoke("get_room_info");
}