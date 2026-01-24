<template>
    <div
        class="h-full flex flex-col bg-white rounded-2xl shadow-sm overflow-hidden border border-emerald-100"
    >
        <div
            class="flex justify-between items-center p-4 border-b border-emerald-50 bg-emerald-50/30"
        >
            <div class="flex items-center gap-2">
                <button
                    v-if="isArchiveMode"
                    @click="$emit('back')"
                    class="p-2 hover:bg-emerald-100 rounded-full text-emerald-600 transition"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="20"
                        height="20"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path d="m15 18-6-6 6-6" />
                    </svg>
                </button>
                <h2 class="text-lg font-semibold text-emerald-800">
                    {{ title }}
                </h2>
            </div>

            <div
                class="bg-emerald-200/50 p-1 rounded-full flex relative w-32 cursor-pointer"
                @click="toggleView"
            >
                <div
                    class="absolute top-1 bottom-1 w-1/2 bg-white rounded-full shadow-sm transition-all duration-300 ease-in-out"
                    :class="
                        viewMode === 'list'
                            ? 'left-1'
                            : 'left-[calc(50%-4px)] translate-x-full'
                    "
                ></div>
                <div
                    class="flex-1 text-center text-xs font-medium z-10 py-1"
                    :class="
                        viewMode === 'list'
                            ? 'text-emerald-800'
                            : 'text-emerald-600'
                    "
                >
                    列表
                </div>
                <div
                    class="flex-1 text-center text-xs font-medium z-10 py-1"
                    :class="
                        viewMode === 'chart'
                            ? 'text-emerald-800'
                            : 'text-emerald-600'
                    "
                >
                    图表
                </div>
            </div>
        </div>

        <div class="flex-1 overflow-hidden relative">
            <div
                v-if="viewMode === 'list'"
                class="h-full overflow-auto select-none"
                ref="listContainer"
            >
                <table class="w-full text-sm text-left">
                    <thead
                        class="text-xs text-emerald-600 uppercase bg-emerald-50 sticky top-0 z-10"
                    >
                        <tr>
                            <th class="px-6 py-3">时间</th>
                            <th class="px-6 py-3">剩余电量 (kWh)</th>
                            <th class="px-6 py-3">变化</th>
                        </tr>
                    </thead>
                    <tbody @mouseleave="endSelection">
                        <tr
                            v-for="(row, index) in data"
                            :key="index"
                            class="border-b border-emerald-50 transition-colors cursor-crosshair"
                            :class="{
                                'bg-emerald-100': isSelected(index),
                                'hover:bg-emerald-50': !isSelected(index),
                            }"
                            @mousedown="startSelection(index)"
                            @mouseenter="updateSelection(index)"
                            @mouseup="endSelection"
                        >
                            <td class="px-6 py-3 font-mono text-gray-600">
                                {{ formatTime(row.timestamp) }}
                            </td>
                            <td class="px-6 py-3 font-medium text-gray-800">
                                {{ row.kwh.toFixed(2) }}
                            </td>
                            <td class="px-6 py-3">
                                <span
                                    v-if="row.diff === 0"
                                    class="text-gray-400"
                                    >-</span
                                >
                                <span
                                    v-else
                                    :class="
                                        row.diff > 0
                                            ? 'text-green-600'
                                            : 'text-red-500'
                                    "
                                >
                                    {{ row.diff > 0 ? "+" : ""
                                    }}{{ row.diff.toFixed(2) }}
                                </span>
                            </td>
                        </tr>
                    </tbody>
                </table>

                <div
                    v-if="selectionStats.count > 0"
                    class="absolute bottom-6 right-6 bg-white/90 backdrop-blur border border-emerald-200 p-4 rounded-xl shadow-lg z-20 text-sm"
                >
                    <h4 class="font-bold text-emerald-800 mb-2">
                        统计信息 ({{ selectionStats.count }} 项)
                    </h4>
                    <div class="grid grid-cols-2 gap-x-4 gap-y-1 text-gray-600">
                        <span>累计消耗:</span>
                        <span class="font-mono text-red-500 font-medium"
                            >{{
                                selectionStats.totalConsumed.toFixed(2)
                            }}
                            kWh</span
                        >
                        <span>平均速度:</span>
                        <span class="font-mono text-emerald-600 font-medium"
                            >{{
                                selectionStats.avgSpeed.toFixed(2)
                            }}
                            kWh/h</span
                        >
                        <span>时间跨度:</span>
                        <span class="font-mono"
                            >{{ selectionStats.timeSpan }} h</span
                        >
                    </div>
                    <div class="mt-2 text-xs text-gray-400 text-center">
                        点击空白处取消
                    </div>
                </div>

                <div
                    v-if="selectionStats.count > 0"
                    class="absolute inset-0 -z-10"
                    @click="clearSelection"
                ></div>
            </div>

            <div v-else class="h-full w-full p-4">
                <v-chart
                    class="h-full w-full"
                    :option="chartOption"
                    autoresize
                />
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { use } from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";
import { LineChart } from "echarts/charts";
import {
    GridComponent,
    TooltipComponent,
    DataZoomComponent,
    MarkLineComponent,
    LegendComponent,
} from "echarts/components";
import VChart from "vue-echarts";
import { format, differenceInMinutes, startOfDay, addDays } from "date-fns";
import type { ElectricityRecord } from "./utils/electricity";

// 注册 ECharts 组件
use([
    CanvasRenderer,
    LineChart,
    GridComponent,
    TooltipComponent,
    DataZoomComponent,
    MarkLineComponent,
    LegendComponent,
]);

const props = defineProps<{
    data: ElectricityRecord[];
    title: string;
    isArchiveMode?: boolean;
}>();

const emit = defineEmits(["back"]);

// --- 视图切换逻辑 ---
const viewMode = ref<"list" | "chart">("list");
const toggleView = () => {
    viewMode.value = viewMode.value === "list" ? "chart" : "list";
};

const formatTime = (d: Date) => format(d, "yyyy-MM-dd HH:mm:ss");

// --- 列表框选逻辑 ---
const selectionStart = ref<number | null>(null);
const selectionEnd = ref<number | null>(null);
const isSelecting = ref(false);

const startSelection = (index: number) => {
    isSelecting.value = true;
    selectionStart.value = index;
    selectionEnd.value = index;
};

const updateSelection = (index: number) => {
    if (isSelecting.value) {
        selectionEnd.value = index;
    }
};

const endSelection = () => {
    isSelecting.value = false;
};

const clearSelection = () => {
    selectionStart.value = null;
    selectionEnd.value = null;
};

const selectedRange = computed(() => {
    if (selectionStart.value === null || selectionEnd.value === null)
        return null;
    const start = Math.min(selectionStart.value, selectionEnd.value);
    const end = Math.max(selectionStart.value, selectionEnd.value);
    return { start, end };
});

const isSelected = (index: number) => {
    if (!selectedRange.value) return false;
    return (
        index >= selectedRange.value.start && index <= selectedRange.value.end
    );
};

// 统计逻辑
const selectionStats = computed(() => {
    if (!selectedRange.value || !props.data.length)
        return { count: 0, totalConsumed: 0, avgSpeed: 0, timeSpan: 0 };

    const subset = props.data.slice(
        selectedRange.value.start,
        selectedRange.value.end + 1,
    );
    const first = subset[0];
    const last = subset[subset.length - 1];

    // 消耗计算 (Last - First 的反向，因为一般电量是递减的，消耗=Start-End)
    // 如果列表中时间是倒序(最新在前)，则 first 是最后时间。通常 CSV 既然是 append 应该是时间正序。
    // 假设 CSV 是时间正序 (旧 -> 新): 消耗 = First.kwh - Last.kwh
    const totalConsumed = Math.abs(first.kwh - last.kwh);
    const minutes = Math.abs(
        differenceInMinutes(last.timestamp, first.timestamp),
    );
    const timeSpan = (minutes / 60).toFixed(1);

    // 平均速度
    const avgSpeed =
        Number(timeSpan) > 0 ? totalConsumed / Number(timeSpan) : 0;

    return {
        count: subset.length,
        totalConsumed,
        avgSpeed,
        timeSpan,
    };
});

// --- 图表配置逻辑 ---
const chartOption = computed(() => {
    if (!props.data || props.data.length === 0) return {};

    const timestamps = props.data.map((d) =>
        format(d.timestamp, "MM-dd HH:mm"),
    );
    const kwhs = props.data.map((d) => d.kwh);
    const speeds = props.data.map((d) => d.speed.toFixed(3));

    // 计算午夜分割线
    const markLinesData = [];
    if (props.data.length > 0) {
        let curr = startOfDay(props.data[0].timestamp);
        const end = props.data[props.data.length - 1].timestamp;
        while (curr <= end) {
            // 找到最近的一个时间点索引或者直接用时间轴值
            // 这里简化为直接在 x 轴上画线，需要 x 轴是 time 类型或者 category 匹配
            // 为了简单适配 category 轴，我们尽量匹配字符串
            const timeStr = format(curr, "MM-dd HH:mm"); // 需与 xAxis data 格式一致
            markLinesData.push({
                xAxis: timeStr,
                lineStyle: { color: "#e5e7eb", type: "solid" },
            }); // 淡淡的灰色
            curr = addDays(curr, 1);
        }
    }

    return {
        color: ["#10b981", "#f59e0b"], // 绿色(电量), 橙色(速度)
        tooltip: {
            trigger: "axis",
            axisPointer: { type: "cross" },
        },
        legend: { data: ["剩余电量", "消耗速度"] },
        grid: { left: "3%", right: "4%", bottom: "15%", containLabel: true },
        dataZoom: [
            { type: "inside", start: 0, end: 100 },
            { type: "slider", start: 0, end: 100, bottom: 10 },
        ],
        xAxis: {
            type: "category",
            data: timestamps,
            boundaryGap: false,
            axisLine: { lineStyle: { color: "#9ca3af" } },
        },
        yAxis: [
            {
                type: "value",
                name: "剩余电量 (度)",
                scale: true, // 自动缩放，不从0开始
                nameTextStyle: { color: "#047857" },
                splitLine: { show: true, lineStyle: { type: "dashed" } },
                min: (value: any) =>
                    (value.min - (value.max - value.min) * 0.1).toFixed(1), // 留白
                max: (value: any) =>
                    (value.max + (value.max - value.min) * 0.1).toFixed(1),
            },
            {
                type: "value",
                name: "消耗速度 (度/h)",
                scale: true,
                nameTextStyle: { color: "#d97706" },
                splitLine: { show: false },
            },
        ],
        series: [
            {
                name: "剩余电量",
                type: "line",
                data: kwhs,
                smooth: true,
                showSymbol: false,
                lineStyle: { width: 3 },
                areaStyle: {
                    color: {
                        type: "linear",
                        x: 0,
                        y: 0,
                        x2: 0,
                        y2: 1,
                        colorStops: [
                            { offset: 0, color: "rgba(16, 185, 129, 0.3)" },
                            { offset: 1, color: "rgba(16, 185, 129, 0.0)" },
                        ],
                    },
                },
                markLine: {
                    symbol: "none",
                    data: markLinesData,
                    label: { show: false },
                },
            },
            {
                name: "消耗速度",
                type: "line",
                yAxisIndex: 1,
                data: speeds,
                smooth: true,
                showSymbol: false,
                lineStyle: { width: 1, type: "dashed" },
            },
        ],
    };
});
</script>
<style scoped>
.logo.vite:hover {
    filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
    filter: drop-shadow(0 0 2em #249b73);
}
</style>
<style>
:root {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 24px;
    font-weight: 400;

    color: #0f0f0f;
    background-color: #f6f6f6;

    font-synthesis: none;
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    -webkit-text-size-adjust: 100%;
}

.container {
    margin: 0;
    padding-top: 10vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    text-align: center;
}

.logo {
    height: 6em;
    padding: 1.5em;
    will-change: filter;
    transition: 0.75s;
}

.logo.tauri:hover {
    filter: drop-shadow(0 0 2em #24c8db);
}

.row {
    display: flex;
    justify-content: center;
}

a {
    font-weight: 500;
    color: #646cff;
    text-decoration: inherit;
}

a:hover {
    color: #535bf2;
}

h1 {
    text-align: center;
}

input,
button {
    border-radius: 8px;
    border: 1px solid transparent;
    padding: 0.6em 1.2em;
    font-size: 1em;
    font-weight: 500;
    font-family: inherit;
    color: #0f0f0f;
    background-color: #ffffff;
    transition: border-color 0.25s;
    box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
    cursor: pointer;
}

button:hover {
    border-color: #396cd8;
}
button:active {
    border-color: #396cd8;
    background-color: #e8e8e8;
}

input,
button {
    outline: none;
}

#greet-input {
    margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
    :root {
        color: #f6f6f6;
        background-color: #2f2f2f;
    }

    a:hover {
        color: #24c8db;
    }

    input,
    button {
        color: #ffffff;
        background-color: #0f0f0f98;
    }
    button:active {
        background-color: #0f0f0f69;
    }
}
</style>
