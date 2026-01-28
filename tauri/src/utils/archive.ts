import { invoke } from "@tauri-apps/api/core";
import { parseISO, formatRFC3339 } from "date-fns";
import { ElectricityRecord, fromRawRecords, RawRecord } from "./records";

interface RawArchiveMeta {
    start_time: string,
    end_time: string,
    archive_name: string,
    records_num: number,
}

export interface ArchiveMeta {
    startTime: Date,
    endTime: Date,
    name: string,
    recordsNum: number,
}

function fromRawMeta(rawMeta: RawArchiveMeta): ArchiveMeta {
    return {
        endTime: parseISO(rawMeta.end_time),
        startTime: parseISO(rawMeta.start_time),
        name: rawMeta.archive_name,
        recordsNum: rawMeta.records_num
    };
}

export async function listArchives(): Promise<ArchiveMeta[]> {
    let rawMetas: RawArchiveMeta[] = await invoke("list_archives");
    let metas: ArchiveMeta[] = [];
    for (let i = 0; i < rawMetas.length; ++i) {
        const rawMeta = rawMetas[i];
        metas.push(fromRawMeta(rawMeta))
    }
    return metas;
}

export interface Archive {
    path: string,
    content: ElectricityRecord[]
}

export async function downloadArchive(name: string): Promise<Archive> {
    let arc: [string, RawRecord[]] = await invoke("download_archive", { archiveName: name });
    return { path: arc[0], content: fromRawRecords(arc[1]) };
}

export async function createArchive(startTime: Date | null, endTime: Date | null, name: string | null): Promise<ArchiveMeta> {
    let rawMeta: RawArchiveMeta = await invoke("create_archive", { startTime: startTime && formatRFC3339(startTime), endTime: endTime && formatRFC3339(endTime), name });
    return fromRawMeta(rawMeta);
}

export async function deleteArchive(name: string) {
    await invoke("delete_archive", { name });
}