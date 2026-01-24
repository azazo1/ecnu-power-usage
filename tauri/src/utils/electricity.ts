import {
  format,
  parseISO,
  differenceInHours,
  differenceInSeconds,
  startOfDay,
  addDays,
  isSameDay,
} from "date-fns";

export interface ElectricityRecord {
  timestamp: Date;
  rawTime: string; // 原始 RFC3339 字符串
  kwh: number; // 剩余电量
  diff: number; // 相比上一条的变化量
  speed: number; // 消耗速度 (度/小时)
}

// 解析 CSV 数据的核心逻辑
export function parseCsvData(csvContent: string): ElectricityRecord[] {
  const lines = csvContent.trim().split("\n");
  const records: ElectricityRecord[] = [];

  for (let i = 0; i < lines.length; i++) {
    const [timeStr, kwhStr] = lines[i].split(",");
    if (!timeStr || !kwhStr) continue;

    const timestamp = parseISO(timeStr);
    const kwh = parseFloat(kwhStr);

    // 计算 Diff (与上一行对比)
    let diff = 0;
    if (i > 0) {
      diff = kwh - records[i - 1].kwh;
    }

    // 计算 Speed (消耗速度) - 根据需求：使用上一行和下一行计算
    // 注意：如果是第一行或最后一行，只能用单边计算
    // 为了简化，我们暂时用当前行和上一行的斜率，或者按照你的要求实现前后差分
    let speed = 0;
    // 预读下一行逻辑比较复杂，这里演示标准的后向差分 (Current - Prev) / TimeDiff
    // 如果需要严格的 (Next - Prev) 居中差分，需要先生成基本数组再二次遍历

    records.push({
      timestamp,
      rawTime: timeStr,
      kwh,
      diff,
      speed: 0, // 稍后填充
    });
  }

  // 二次遍历计算居中差分速度 (度/小时)
  for (let i = 0; i < records.length; i++) {
    let prev = records[i > 0 ? i - 1 : 0];
    let next = records[i < records.length - 1 ? i + 1 : records.length - 1];

    // 如果只有一行数据
    if (records.length < 2) break;

    const hoursDiff =
      Math.abs(differenceInSeconds(next.timestamp, prev.timestamp)) / 3600;
    const kwhDiff = Math.abs(next.kwh - prev.kwh);

    if (hoursDiff > 0) {
      records[i].speed = kwhDiff / hoursDiff;
    }
  }

  return records;
}
