<template>
    <div
        class="h-100 d-flex flex-column bg-white rounded-4 shadow-lg overflow-hidden border border-success border-opacity-25">
        <!-- Header -->
        <div
            class="d-flex justify-content-between align-items-center p-4 border-bottom border-success border-opacity-25 bg-light bg-opacity-50">
            <div class="d-flex align-items-center gap-3">
                <button v-if="isArchiveMode" @click="$emit('back')"
                    class="btn btn-outline-success btn-sm rounded-3 d-flex align-items-center hover-scale border-2">
                    <i class="bi bi-arrow-left me-1"></i>
                </button>
                <h2 class="h4 mb-0 fw-bold text-success">{{ title }}</h2>
            </div>

            <!-- View Toggle -->
            <div class="btn-group shadow-sm" role="group">
                <button type="button" @click="viewMode = 'list'" class="btn d-flex align-items-center gap-2"
                    :class="viewMode === 'list' ? 'btn-success' : 'btn-outline-success'">
                    <i class="bi bi-list-ul"></i>
                    <span>列表</span>
                </button>
                <button type="button" @click="viewMode = 'chart'" class="btn d-flex align-items-center gap-2"
                    :class="viewMode === 'chart' ? 'btn-success' : 'btn-outline-success'">
                    <i class="bi bi-graph-up"></i>
                    <span>图表</span>
                </button>
            </div>
        </div>

        <!-- Content -->
        <div class="flex-grow-1 overflow-hidden position-relative">
            <!-- List View -->
            <div v-if="viewMode === 'list'" class="h-100 overflow-auto" ref="listContainer">
                <table class="table table-hover table-sm mb-0 user-select-none">
                    <thead class="table-success sticky-top">
                        <tr>
                            <th class="px-4 py-3 fw-bold">
                                <i class="bi bi-clock me-2"></i>时间
                            </th>
                            <th class="px-4 py-3 fw-bold">
                                <i class="bi bi-battery-half me-2"></i>剩余电量 (kWh)
                            </th>
                            <th class="px-4 py-3 fw-bold">
                                <i class="bi bi-activity me-2"></i>变化
                            </th>
                        </tr>
                    </thead>
                    <tbody @mouseleave="endSelection">
                        <tr v-for="(row, index) in data" :key="index" class="cursor-crosshair transition-all" :class="{
                            'table-success': isSelected(index),
                        }" @mousedown="startSelection(index)" @mouseenter="updateSelection(index)"
                            @mouseup="endSelection">
                            <td class="px-4 py-3 font-monospace text-secondary">
                                {{ formatTime(row.timestamp) }}
                            </td>
                            <td class="px-4 py-3 fw-medium">
                                {{ row.kwh.toFixed(2) }}
                            </td>
                            <td class="px-4 py-3">
                                <span v-if="row.diff === 0" class="text-muted">-</span>
                                <span v-else :class="row.diff > 0 ? 'text-success' : 'text-danger'">
                                    {{ row.diff > 0 ? "+" : "" }}{{ row.diff.toFixed(2) }}
                                </span>
                            </td>
                        </tr>
                    </tbody>
                </table>

                <!-- Statistics Card -->
                <div v-if="selectionStats.count > 0"
                    class="position-absolute bottom-0 end-0 m-4 bg-white border-2 border-success rounded-4 shadow-lg p-4 z-3 animate-slide-in"
                    style="min-width: 280px;">
                    <h5 class="fw-bold text-success mb-3 d-flex align-items-center gap-2">
                        <i class="bi bi-graph-up-arrow"></i>
                        统计信息
                        <span class="badge bg-success rounded-pill fs-6">{{ selectionStats.count }} 项</span>
                    </h5>
                    <div class="row g-3 small">
                        <div class="col-12">
                            <div class="d-flex justify-content-between align-items-center">
                                <span class="text-secondary">
                                    <i class="bi bi-lightning-charge me-1"></i>累计消耗:
                                </span>
                                <span class="font-monospace text-danger fw-bold fs-6">
                                    {{ selectionStats.totalConsumed.toFixed(2) }} kWh
                                </span>
                            </div>
                        </div>
                        <div class="col-12">
                            <div class="d-flex justify-content-between align-items-center">
                                <span class="text-secondary">
                                    <i class="bi bi-speedometer2 me-1"></i>平均速度:
                                </span>
                                <span class="font-monospace text-success fw-bold fs-6">
                                    {{ selectionStats.avgSpeed.toFixed(2) }} kWh/h
                                </span>
                            </div>
                        </div>
                        <div class="col-12">
                            <div class="d-flex justify-content-between align-items-center">
                                <span class="text-secondary">
                                    <i class="bi bi-hourglass-split me-1"></i>时间跨度:
                                </span>
                                <span class="font-monospace fw-bold fs-6">
                                    {{ selectionStats.timeSpan }} h
                                </span>
                            </div>
                        </div>
                    </div>
                    <div class="mt-3 pt-3 border-top border-success border-opacity-25 text-center text-muted small">
                        <i class="bi bi-info-circle me-1"></i>点击空白处取消选择
                    </div>
                </div>

                <!-- Overlay for clearing selection -->
                <div v-if="selectionStats.count > 0" class="position-absolute top-0 start-0 w-100 h-100"
                    style="z-index: 2;" @click="clearSelection"></div>
            </div>

            <!-- Chart View -->
            <div v-else class="h-100 w-100 p-4">
                <v-chart class="h-100 w-100" :option="chartOption" autoresize />
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
import type { ElectricityRecord } from "../utils/electricity";

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

    const totalConsumed = Math.abs(first.kwh - last.kwh);
    const minutes = Math.abs(
        differenceInMinutes(last.timestamp, first.timestamp),
    );
    const timeSpan = (minutes / 60).toFixed(1);

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

    // 使用时间戳作为x轴，数据格式为 [timestamp, value]
    const kwhsData = props.data.map((d) => [d.timestamp.getTime(), d.kwh]);
    const speedsData = props.data.map((d) => [d.timestamp.getTime(), parseFloat(d.speed.toFixed(3))]);

    return {
        color: ["#10b981", "#f97316"],
        tooltip: {
            trigger: "axis",
            axisPointer: { type: "cross" },
            backgroundColor: "rgba(255, 255, 255, 0.95)",
            borderColor: "#10b981",
            borderWidth: 2,
            textStyle: {
                color: "#374151",
            },
            formatter: (params: any) => {
                if (!params || params.length === 0) return "";
                const date = new Date(params[0].value[0]);
                const timeStr = format(date, "yyyy年MM月dd日 HH:mm:ss");
                let result = `<div style="font-weight: bold; margin-bottom: 8px;">${timeStr}</div>`;
                params.forEach((param: any) => {
                    const value = typeof param.value[1] === "number" ? param.value[1].toFixed(2) : param.value[1];
                    const unit = param.seriesName === "剩余电量" ? "kWh" : "kWh/h";
                    result += `<div style="margin: 4px 0;">
                        <span style="display:inline-block;width:10px;height:10px;border-radius:50%;background:${param.color};margin-right:8px;"></span>
                        <span style="font-weight: 500;">${param.seriesName}:</span>
                        <span style="font-weight: bold;">${value} ${unit}</span>
                    </div>`;
                });
                return result;
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
            type: "time",
            boundaryGap: false,
            axisLine: { lineStyle: { color: "#10b981", width: 2 } },
            axisLabel: {
                color: "#047857",
                fontWeight: "500",
                formatter: (value: number) => {
                    return format(new Date(value), "MM-dd\nHH:mm");
                },
                rotate: 0,
                hideOverlap: true,
            },
        },
        yAxis: [
            {
                type: "value",
                name: "剩余电量 (度)",
                scale: true,
                nameTextStyle: { color: "#047857", fontWeight: "bold" },
                axisLabel: {
                    color: "#059669",
                    fontWeight: "500",
                },
                axisLine: { lineStyle: { color: "#10b981", width: 2 } },
                splitLine: { show: true, lineStyle: { type: "dashed", color: "#d1fae5" } },
                min: (value: any) =>
                    (value.min - (value.max - value.min) * 0.1).toFixed(1),
                max: (value: any) =>
                    (value.max + (value.max - value.min) * 0.1).toFixed(1),
            },
            {
                type: "value",
                name: "消耗速度 (度/h)",
                scale: true,
                nameTextStyle: { color: "#ea580c", fontWeight: "bold" },
                axisLabel: {
                    color: "#f97316",
                    fontWeight: "500",
                },
                axisLine: { lineStyle: { color: "#f97316", width: 2 } },
                splitLine: { show: false },
            },
        ],
        series: [
            {
                name: "剩余电量",
                type: "line",
                data: kwhsData,
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
            },
            {
                name: "消耗速度",
                type: "line",
                yAxisIndex: 1,
                data: speedsData,
                smooth: true,
                showSymbol: false,
                lineStyle: { width: 3, type: "solid", shadowBlur: 8, shadowColor: "rgba(249, 115, 22, 0.4)" },
            },
        ],
    };
});
</script>

<style scoped>
.hover-scale {
    transition: transform 0.3s ease;
}

.hover-scale:hover {
    transform: scale(1.05);
}

.cursor-crosshair {
    cursor: crosshair;
}

.transition-all {
    transition: all 0.2s ease;
}

.animate-slide-in {
    animation: slideIn 0.3s cubic-bezier(0.4, 0, 0.2, 1);
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

.user-select-none {
    user-select: none;
}
</style>
