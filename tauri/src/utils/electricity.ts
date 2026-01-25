import {
  parseISO,
  differenceInSeconds,
} from "date-fns";

export interface RawRecord {
  time: Date;
  degree: number;
}

export interface ElectricityRecord {
  timestamp: Date;
  kwh: number; // 剩余电量
  diff: number; // 相比上一条的变化量
  speed: number; // 消耗速度 (度/小时)
}

export function fromRawRecords(records: RawRecord[]): ElectricityRecord[] {
  let eRecords: ElectricityRecord[] = [];
  for (let i = 0; i < records.length; ++i) {
    const kwh = records[i].degree;
    const timestamp = records[i].time;

    // 计算 Diff (与上一个对比)
    let diff = 0;
    if (i > 0) {
      diff = kwh - records[i - 1].degree;
    }

    // 计算 Speed
    let prev = records[i > 0 ? i - 1 : 0];
    let next = records[i < records.length - 1 ? i + 1 : records.length - 1];
    const hoursDiff =
      Math.abs(differenceInSeconds(next.time, prev.time)) / 3600;
    const kwhDiff = Math.abs(next.degree - prev.degree);
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

    const timestamp = parseISO(timeStr);
    const kwh = parseFloat(kwhStr);
    rawRecords.push({ time: timestamp, degree: kwh });
  }

  return fromRawRecords(rawRecords);
}
