<template>
    <div class="h-full flex flex-col bg-white/80 backdrop-blur-sm rounded-3xl shadow-xl overflow-hidden border border-emerald-100/50">
        <div class="flex justify-between items-center p-5 border-b border-emerald-100/50 bg-gradient-to-r from-emerald-50/50 to-green-50/50">
            <div class="flex items-center gap-3">
                <button v-if="isArchiveMode" @click="$emit('back')"
                    class="group p-2.5 hover:bg-emerald-100 rounded-xl text-emerald-600 transition-all duration-300 hover:scale-110 hover:-translate-x-0.5">
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none"
                        stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="transition-transform duration-300 group-hover:-translate-x-1">
                        <path d="m15 18-6-6 6-6" />
                    </svg>
                </button>
                <h2 class="text-xl font-bold text-emerald-800">
                    {{ title }}
                </h2>
            </div>

            <div class="bg-gradient-to-r from-emerald-100 to-green-100 p-1.5 rounded-2xl flex relative w-36 cursor-pointer shadow-inner" @click="toggleView">
                <div class="absolute top-1.5 bottom-1.5 w-1/2 bg-gradient-to-br from-emerald-400 to-green-500 rounded-xl shadow-lg transition-all duration-300 ease-out"
                    :class="viewMode === 'list'
                        ? 'left-1.5'
                        : 'left-[calc(50%-6px)] translate-x-full'
                        "></div>
                <div class="flex-1 text-center text-sm font-semibold z-10 py-1.5 transition-colors duration-300" :class="viewMode === 'list'
                    ? 'text-white'
                    : 'text-emerald-700'
                    ">
                    📋 列表
                </div>
                <div class="flex-1 text-center text-sm font-semibold z-10 py-1.5 transition-colors duration-300" :class="viewMode === 'chart'
                    ? 'text-white'
                    : 'text-emerald-700'
                    ">
                    📈 图表
                </div>
            </div>
        </div>

        <div class="flex-1 overflow-hidden relative">
            <div v-if="viewMode === 'list'" class="h-full overflow-auto select-none" ref="listContainer">
                <table class="w-full text-sm text-left">
                    <thead class="text-xs text-emerald-700 uppercase bg-gradient-to-r from-emerald-50 to-green-50 sticky top-0 z-10 shadow-sm">
                        <tr>
                            <th class="px-6 py-4 font-bold">⏰ 时间</th>
                            <th class="px-6 py-4 font-bold">🔋 剩余电量 (kWh)</th>
                            <th class="px-6 py-4 font-bold">📊 变化</th>
                        </tr>
                    </thead>
                    <tbody @mouseleave="endSelection">
                        <tr v-for="(row, index) in data" :key="index"
                            class="border-b border-emerald-50/50 transition-all duration-200 cursor-crosshair hover:shadow-sm" :class="{
                                'bg-gradient-to-r from-emerald-100 to-green-100 shadow-inner': isSelected(index),
                                'hover:bg-gradient-to-r hover:from-emerald-50/50 hover:to-green-50/30': !isSelected(index),
                            }" @mousedown="startSelection(index)" @mouseenter="updateSelection(index)"
                            @mouseup="endSelection">
                            <td class="px-6 py-3 font-mono text-gray-600">
                                {{ formatTime(row.timestamp) }}
                            </td>
                            <td class="px-6 py-3 font-medium text-gray-800">
                                {{ row.kwh.toFixed(2) }}
                            </td>
                            <td class="px-6 py-3">
                                <span v-if="row.diff === 0" class="text-gray-400">-</span>
                                <span v-else :class="row.diff > 0
                                    ? 'text-green-600'
                                    : 'text-red-500'
                                    ">
                                    {{ row.diff > 0 ? "+" : ""
                                    }}{{ row.diff.toFixed(2) }}
                                </span>
                            </td>
                        </tr>
                    </tbody>
                </table>

                <div v-if="selectionStats.count > 0"
                    class="absolute bottom-6 right-6 bg-white/95 backdrop-blur-xl border-2 border-emerald-200 p-5 rounded-2xl shadow-2xl shadow-emerald-100/50 z-20 text-sm animate-in">
                    <h4 class="font-bold text-emerald-800 mb-3 text-base flex items-center gap-2">
                        📊 统计信息
                        <span class="text-xs bg-emerald-100 text-emerald-700 px-2 py-0.5 rounded-full">{{ selectionStats.count }} 项</span>
                    </h4>
                    <div class="grid grid-cols-2 gap-x-4 gap-y-2 text-gray-600">
                        <span class="font-medium">⚡ 累计消耗:</span>
                        <span class="font-mono text-red-500 font-bold">{{
                            selectionStats.totalConsumed.toFixed(2)
                            }}
                            kWh</span>
                        <span class="font-medium">🚀 平均速度:</span>
                        <span class="font-mono text-emerald-600 font-bold">{{
                            selectionStats.avgSpeed.toFixed(2)
                            }}
                            kWh/h</span>
                        <span class="font-medium">⏱️ 时间跨度:</span>
                        <span class="font-mono text-gray-700 font-bold">{{ selectionStats.timeSpan }} h</span>
                    </div>
                    <div class="mt-3 pt-3 border-t border-emerald-100 text-xs text-gray-400 text-center">
                        💡 点击空白处取消选择
                    </div>
                </div>

                <div v-if="selectionStats.count > 0" class="absolute inset-0 -z-10" @click="clearSelection"></div>
            </div>

            <div v-else class="h-full w-full p-6">
                <v-chart class="h-full w-full" :option="chartOption" autoresize />
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
        color: ["#10b981", "#14b8a6"], // 翠绿色(电量), 青绿色(速度)
        tooltip: {
            trigger: "axis",
            axisPointer: { type: "cross" },
            backgroundColor: "rgba(255, 255, 255, 0.95)",
            borderColor: "#10b981",
            borderWidth: 2,
            textStyle: {
                color: "#374151",
            },
        },
        legend: { 
            data: ["剩余电量", "消耗速度"],
            textStyle: {
                color: "#047857",
                fontWeight: "bold",
            },
        },
        grid: { left: "3%", right: "4%", bottom: "15%", containLabel: true },
        dataZoom: [
            { type: "inside", start: 0, end: 100 },
            { type: "slider", start: 0, end: 100, bottom: 10 },
        ],
        xAxis: {
            type: "category",
            data: timestamps,
            boundaryGap: false,
            axisLine: { lineStyle: { color: "#10b981", width: 2 } },
            axisLabel: {
                color: "#047857",
                fontWeight: "500",
            },
        },
        yAxis: [
            {
                type: "value",
                name: "剩余电量 (度)",
                scale: true, // 自动缩放，不从0开始
                nameTextStyle: { color: "#047857", fontWeight: "bold" },
                axisLabel: {
                    color: "#059669",
                    fontWeight: "500",
                },
                axisLine: { lineStyle: { color: "#10b981", width: 2 } },
                splitLine: { show: true, lineStyle: { type: "dashed", color: "#d1fae5" } },
                min: (value: any) =>
                    (value.min - (value.max - value.min) * 0.1).toFixed(1), // 留白
                max: (value: any) =>
                    (value.max + (value.max - value.min) * 0.1).toFixed(1),
            },
            {
                type: "value",
                name: "消耗速度 (度/h)",
                scale: true,
                nameTextStyle: { color: "#0d9488", fontWeight: "bold" },
                axisLabel: {
                    color: "#14b8a6",
                    fontWeight: "500",
                },
                axisLine: { lineStyle: { color: "#14b8a6", width: 2 } },
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
                lineStyle: { width: 3, shadowBlur: 8, shadowColor: "rgba(16, 185, 129, 0.3)" },
                areaStyle: {
                    color: {
                        type: "linear",
                        x: 0,
                        y: 0,
                        x2: 0,
                        y2: 1,
                        colorStops: [
                            { offset: 0, color: "rgba(16, 185, 129, 0.4)" },
                            { offset: 0.5, color: "rgba(16, 185, 129, 0.2)" },
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
                lineStyle: { width: 2, type: "solid", shadowBlur: 6, shadowColor: "rgba(20, 184, 166, 0.3)" },
            },
        ],
    };
});
</script>
<style scoped>
.logo.vite:hover {
    filter: drop-shadow(0 0 2em #10b981);
}

.logo.vue:hover {
    filter: drop-shadow(0 0 2em #10b981);
}

@keyframes slideIn {
    from {
        opacity: 0;
        transform: scale(0.95) translateY(10px);
    }
    to {
        opacity: 1;
        transform: scale(1) translateY(0);
    }
}

.animate-in {
    animation: slideIn 0.3s cubic-bezier(0.4, 0, 0.2, 1);
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
