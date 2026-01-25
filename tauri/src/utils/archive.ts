import { invoke } from "@tauri-apps/api/core";
import { parseISO } from "date-fns";

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

export async function listArchives(): Promise<ArchiveMeta[]> {
    let rawMetas: RawArchiveMeta[] = await invoke("list_archives");
    let metas: ArchiveMeta[] = [];
    for (let i = 0; i < rawMetas.length; ++i) {
        const rmeta = rawMetas[i];
        metas.push({
            endTime: parseISO(rmeta.end_time),
            startTime: parseISO(rmeta.start_time),
            name: rmeta.archive_name,
            recordsNum: rmeta.records_num
        })
    }
    return metas;
}