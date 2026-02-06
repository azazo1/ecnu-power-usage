import { invoke } from "@tauri-apps/api/core";
import {
    parseISO,
    differenceInSeconds,
} from "date-fns";

export type RawRecord = [string, number];

export interface ElectricityRecord {
    timestamp: Date;
    kwh: number; // 剩余电量
    diff: number; // 相比上一条的变化量
    speed: number; // 消耗速度 (度/小时)
}

export function fromRawRecords(records: RawRecord[]): ElectricityRecord[] {
    let eRecords: ElectricityRecord[] = [];
    let timestamps: Date[] = [];
    for (let i = 0; i < records.length; ++i) {
        timestamps.push(parseISO(records[i][0]));
    }
    for (let i = 0; i < records.length; ++i) {
        const kwh = records[i][1];
        const timestamp = timestamps[i];

        // 计算 Diff (与上一个对比)
        let diff = 0;
        if (i > 0) {
            diff = kwh - records[i - 1][1];
        }

        // 计算 Speed
        let prev = records[i > 0 ? i - 1 : 0];
        let next = records[i < records.length - 1 ? i + 1 : records.length - 1];
        let timestampPrev = timestamps[i > 0 ? i - 1 : 0];
        let timestampNext = timestamps[i < records.length - 1 ? i + 1 : records.length - 1];
        const hoursDiff =
            Math.abs(differenceInSeconds(timestampNext, timestampPrev)) / 3600;
        const kwhDiff = Math.max(prev[1] - next[1], 0);
        let speed = 0;
        if (hoursDiff > 0) {
            speed = kwhDiff / hoursDiff;
        }

        eRecords.push({
            timestamp,
            kwh,
            diff,
            speed,
        });
    }
    return eRecords;
}

// 解析 CSV 数据的核心逻辑
export function parseCsvData(csvContent: string): ElectricityRecord[] {
    const lines = csvContent.trim().split("\n");
    let rawRecords: RawRecord[] = [];
    for (let i = 0; i < lines.length; i++) {
        const [timeStr, kwhStr] = lines[i].split(",");
        if (!timeStr || !kwhStr) continue;

        const kwh = parseFloat(kwhStr);
        rawRecords.push([timeStr, kwh]);
    }

    return fromRawRecords(rawRecords);
}


export async function getRecords(): Promise<ElectricityRecord[]> {
    return fromRawRecords(await invoke("get_records"));
}