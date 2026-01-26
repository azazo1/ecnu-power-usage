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
                <button v-if="archivePath" @click="openArchiveFile"
                    class="btn btn-outline-secondary btn-sm rounded-circle d-flex align-items-center justify-content-center hover-scale border-1 p-2"
                    style="width: 32px; height: 32px; --bs-btn-border-color: rgba(0,0,0,0.1);" title="在系统中打开文件">
                    <i class="bi bi-box-arrow-up-right" style="font-size: 0.9rem;"></i>
                </button>
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
            <div v-if="viewMode === 'list'" class="h-100 overflow-auto user-select-none position-relative"
                ref="listContainer" @contextmenu.prevent>
                <table class="table table-hover table-sm mb-0 user-select-none">
                    <thead class="table-primary sticky-top" style="top: 0; z-index: 10;">
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
                    <tbody>
                        <tr v-for="(row, index) in data" :key="index" class="cursor-pointer transition-all" :class="{
                            'table-success': isSelected(index),
                            'bg-success bg-opacity-10': index === selectionStart || index === selectionEnd
                        }" @click="setStartPoint(index)" @contextmenu.prevent="setEndPoint(index)">
                            <td class="px-4 py-3 font-monospace text-secondary">
                                {{ formatTime(row.timestamp) }}
                                <span v-if="index === selectionStart" class="badge bg-success ms-2 scale-in">起点</span>
                                <span v-if="index === selectionEnd"
                                    class="badge bg-warning text-dark ms-2 scale-in">终点</span>
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
            </div>

            <!-- Statistics Card -->
            <div v-if="viewMode === 'list' && selectionStats.count > 0"
                class="position-absolute bottom-0 end-0 m-3 bg-white border border-success border-opacity-50 rounded-3 shadow p-2 z-3 animate-slide-in"
                style="min-width: 200px; font-size: 0.8rem;">

                <div
                    class="d-flex justify-content-between align-items-center mb-2 pb-1 border-bottom border-success border-opacity-25">
                    <div class="fw-bold text-success d-flex align-items-center gap-1">
                        <i class="bi bi-graph-up-arrow"></i>
                        <span>统计</span>
                        <span class="badge bg-success bg-opacity-10 text-success rounded-pill px-2 py-0"
                            style="font-size: 0.75rem;">
                            {{ selectionStats.count }}
                        </span>
                    </div>
                    <i class="bi bi-x-lg text-secondary cursor-pointer hover-text-danger" style="font-size: 0.7rem;"
                        @click="clearSelection" title="取消选择"></i>
                </div>

                <div class="d-flex flex-column gap-1 mb-2">
                    <div class="d-flex justify-content-between align-items-center">
                        <span class="text-secondary text-nowrap me-3">
                            <i class="bi bi-lightning-charge me-1"></i>消耗
                        </span>
                        <span class="font-monospace text-danger fw-bold">
                            {{ selectionStats.totalConsumed.toFixed(2) }}
                            <small class="text-muted fw-normal" style="font-size: 0.7em;">kWh</small>
                        </span>
                    </div>

                    <div class="d-flex justify-content-between align-items-center">
                        <span class="text-secondary text-nowrap me-3">
                            <i class="bi bi-speedometer2 me-1"></i>速度
                        </span>
                        <span class="font-monospace text-success fw-bold">
                            {{ selectionStats.avgSpeed.toFixed(2) }}
                            <small class="text-muted fw-normal" style="font-size: 0.7em;">kWh/天</small>
                        </span>
                    </div>

                    <div class="d-flex justify-content-between align-items-center">
                        <span class="text-secondary text-nowrap me-3">
                            <i class="bi bi-hourglass-split me-1"></i>时长
                        </span>
                        <span class="font-monospace fw-bold text-dark">
                            {{ selectionStats.timeSpan }}
                            <small class="text-muted fw-normal" style="font-size: 0.7em;">h</small>
                        </span>
                    </div>
                </div>

                <div class="d-flex gap-2 pt-2 border-top border-success border-opacity-10">
                    <button
                        class="btn btn-sm btn-outline-success border-0 bg-success bg-opacity-10 flex-grow-1 py-0 d-flex justify-content-center align-items-center"
                        style="height: 24px;" @click="extendToStart" title="选中从开头到当前">
                        <i class="bi bi-skip-backward-fill"></i>
                    </button>

                    <button
                        class="btn btn-sm btn-success flex-grow-1 py-1 d-flex justify-content-center align-items-center gap-1"
                        style="height: 24px; font-size: 0.75rem;" @click="handleArchiveClick">
                        <i class="bi bi-archive"></i>
                        <span>归档选中项</span>
                    </button>

                    <button
                        class="btn btn-sm btn-outline-success border-0 bg-success bg-opacity-10 flex-grow-1 py-0 d-flex justify-content-center align-items-center"
                        style="height: 24px;" @click="extendToEnd" title="选中从当前到末尾">
                        <i class="bi bi-skip-forward-fill"></i>
                    </button>
                </div>
            </div>

            <!-- 创建归档 对话框 -->
            <CreateArchiveDialog :show="isCreateArchiveDialogShow"
                :startTime="selectionStart !== null ? data[selectionStart].timestamp : null"
                :endTime="selectionEnd !== null ? data[selectionEnd].timestamp : null"
                @close="isCreateArchiveDialogShow = false" @confirm="onDialogConfirm" />

            <!-- Chart View -->
            <div v-if="viewMode === 'chart'" class="h-100 w-100 p-3 d-flex flex-column">
                <div class="d-flex align-items-center gap-2 mb-2 px-1 animate-slide-in" style="font-size: 0.75rem;">
                    <div class="d-flex align-items-center gap-2 text-secondary border-end pe-2">
                        <span class="fw-bold text-success"><i class="bi bi-globe2 me-1"></i>总:</span>
                        <span>{{ totalStats?.consumed }}<small>kWh</small></span>
                        <span class="opacity-50">|</span>
                        <span>{{ totalStats?.speed }}<small>kWh/天</small></span>
                    </div>

                    <div class="d-flex align-items-center gap-2 flex-grow-1">
                        <span class="fw-bold text-primary"><i class="bi bi-zoom-in me-1"></i>视图:</span>
                        <span class="text-danger fw-bold">{{ windowStats?.consumed }}<small
                                class="fw-normal text-muted">kWh</small></span>
                        <span class="text-success fw-bold">{{ windowStats?.speed }}<small
                                class="fw-normal text-muted">kWh/天</small></span>
                        <span class="text-dark fw-bold">{{ windowStats?.hours }}<small
                                class="fw-normal text-muted">h</small></span>
                    </div>
                </div>

                <div class="flex-grow-1 position-relative">
                    <v-chart class="h-100 w-100" :option="chartOption" autoresize @datazoom="handleDataZoom" />
                </div>
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
import { format, differenceInMinutes } from "date-fns";
import type { ElectricityRecord } from "../utils/records";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import CreateArchiveDialog from "./CreateArchiveDialog.vue";

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
    /// archive path
    archivePath: string | null;
}>();

const emit = defineEmits<{ back: [], createArchive: [startTime: Date | null, endTime: Date | null, name: string | null] }>();

// --- 视图切换逻辑 ---
const viewMode = ref<"list" | "chart">("list");

const formatTime = (d: Date) => format(d, "yyyy-MM-dd HH:mm:ss");

// --- 列表选择逻辑 (左键起点，右键终点) ---
const selectionStart = ref<number | null>(null);
const selectionEnd = ref<number | null>(null);

const correctifyBoundary = () => {
    if (selectionEnd.value === null || selectionStart.value === null) return;
    if (selectionEnd.value < selectionStart.value) {
        let tmp = selectionStart.value;
        selectionStart.value = selectionEnd.value;
        selectionEnd.value = tmp;
    }
}

// 左键点击：设置起点
const setStartPoint = (index: number) => {
    selectionStart.value = index;
    // 逻辑选择：设置新起点时，是重置终点，还是保留终点以调整范围？
    // 为了防止混乱，这里设定：如果还没有终点，就把终点也设为当前点（即选中单行）
    // 如果已有终点，则只更新起点，实现范围调整
    if (selectionEnd.value === null) {
        selectionEnd.value = index;
    }
    correctifyBoundary();
};

// 右键点击：设置终点
const setEndPoint = (index: number) => {
    selectionEnd.value = index;
    // 同理，如果还没有起点，把起点也设为这个点
    if (selectionStart.value === null) {
        selectionStart.value = index;
    }
    correctifyBoundary();
};

const clearSelection = () => {
    selectionStart.value = null;
    selectionEnd.value = null;
};

const extendToStart = () => {
    if (!selectedRange.value) return;
    // 保持当前的结束点不变，将开始点设为 0
    // 注意：这里利用 selectedRange 计算好的 end，确保逻辑方向正确
    const currentEnd = selectedRange.value.end;
    selectionStart.value = 0;
    selectionEnd.value = currentEnd;
};

const extendToEnd = () => {
    if (!selectedRange.value || !props.data.length) return;
    // 保持当前的开始点不变，将结束点设为最后一条数据的索引
    const currentStart = selectedRange.value.start;
    selectionStart.value = currentStart;
    selectionEnd.value = props.data.length - 1;
};

// 计算有效范围（自动处理 start > end 的情况）
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
        Number(timeSpan) > 0 ? (totalConsumed / Number(timeSpan)) * 24 : 0;

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
    const speedsData = props.data.map((d) => [d.timestamp.getTime(), parseFloat((d.speed * 24).toFixed(3))]);

    return {
        color: ["#10b981", "#f97316"],
        tooltip: {
            trigger: "axis",
            axisPointer: { type: "cross" },
            backgroundColor: "rgba(255, 255, 255, 0.95)",
            borderColor: "#10b981",
            borderWidth: 2,
            textStyle: { color: "#374151" },
            formatter: (params: any) => {
                if (!params || params.length === 0) return "";
                const date = new Date(params[0].value[0]);
                const timeStr = format(date, "yyyy年MM月dd日 HH:mm:ss");
                let result = `<div style="font-weight: bold; margin-bottom: 8px;">${timeStr}</div>`;
                params.forEach((param: any) => {
                    const value = typeof param.value[1] === "number" ? param.value[1].toFixed(2) : param.value[1];
                    const unit = param.seriesName === "剩余电量" ? "kWh" : "kWh/天";
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
            top: "0%",
            left: "center",
            orient: "horizontal",
            data: ["剩余电量", "消耗速度"],
            textStyle: { color: "#047857", fontWeight: "bold" },
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
                formatter: (value: number) => format(new Date(value), "MM-dd HH:mm"),
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
                axisLabel: { color: "#059669", fontWeight: "500" },
                axisLine: { lineStyle: { color: "#10b981", width: 2 } },
                splitLine: { show: true, lineStyle: { type: "dashed", color: "#d1fae5" } },
                min: (value: any) => (value.min - (value.max - value.min) * 0.1).toFixed(1),
                max: (value: any) => (value.max + (value.max - value.min) * 0.1).toFixed(1),
            },
            {
                type: "value",
                name: "消耗速度 (度/天)",
                scale: true,
                nameTextStyle: { color: "#ea580c", fontWeight: "bold" },
                axisLabel: { color: "#f97316", fontWeight: "500" },
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
                        x: 0, y: 0, x2: 0, y2: 1,
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

// --- 实时图表窗口统计逻辑 ---
const chartWindowRange = ref({ startValue: 0, endValue: 0 });

// 处理 ECharts 的 dataZoom 事件
const handleDataZoom = (params: any) => {
    let startPercent, endPercent;

    // 兼容不同的触发方式（缩放或平移）
    if (params.batch) {
        startPercent = params.batch[0].start;
        endPercent = params.batch[0].end;
    } else {
        startPercent = params.start;
        endPercent = params.end;
    }

    // 只有百分比有效时才更新
    if (startPercent !== undefined && endPercent !== undefined) {
        const totalStart = props.data[0].timestamp.getTime();
        const totalEnd = props.data[props.data.length - 1].timestamp.getTime();
        const totalDuration = totalEnd - totalStart;

        // 将百分比映射回真实的时间戳
        chartWindowRange.value = {
            startValue: totalStart + (totalDuration * startPercent) / 100,
            endValue: totalStart + (totalDuration * endPercent) / 100
        };
    }
};

// 计算当前窗口的统计信息
const windowStats = computed(() => {
    if (!props.data.length) return null;

    // 如果还没触发过缩放，默认显示全部
    const start = chartWindowRange.value.startValue || props.data[0].timestamp.getTime();
    const end = chartWindowRange.value.endValue || props.data[props.data.length - 1].timestamp.getTime();

    const subset = props.data.filter(d => {
        const t = d.timestamp.getTime();
        return t >= start && t <= end;
    });

    if (subset.length < 2) return null;

    const first = subset[0];
    const last = subset[subset.length - 1];
    const consumed = Math.abs(first.kwh - last.kwh);
    const hours = Math.abs(differenceInMinutes(last.timestamp, first.timestamp)) / 60;

    return {
        consumed: consumed.toFixed(2),
        speed: hours > 0 ? ((consumed / hours) * 24).toFixed(2) : "0.00",
        hours: hours.toFixed(1)
    };
});

// 计算全部数据的统计信息
const totalStats = computed(() => {
    if (props.data.length < 2) return null;
    const first = props.data[0];
    const last = props.data[props.data.length - 1];
    const consumed = Math.abs(first.kwh - last.kwh);
    const hours = Math.abs(differenceInMinutes(last.timestamp, first.timestamp)) / 60;

    return {
        consumed: consumed.toFixed(2),
        speed: hours > 0 ? ((consumed / hours) * 24).toFixed(2) : "0.00",
        hours: hours.toFixed(1)
    };
});

const openArchiveFile = async () => {
    if (props.archivePath) {
        try {
            await revealItemInDir(props.archivePath);
        } catch (err) {
            console.error("无法打开 Archive:", err);
        }
    }
}

const isCreateArchiveDialogShow = ref(false);

const handleArchiveClick = () => {
    isCreateArchiveDialogShow.value = true;
};

const onDialogConfirm = (name: string | null) => {
    isCreateArchiveDialogShow.value = false;
    const start = selectionStart.value;
    const end = selectionEnd.value;
    const startTime = start !== null ? props.data[start].timestamp : null;
    const endTime = end !== null ? props.data[end].timestamp : null;

    emit('createArchive', startTime, endTime, name);

    clearSelection();
};
</script>

<style scoped>
.hover-scale {
    transition: transform 0.3s ease;
}

.hover-scale:hover {
    transform: scale(1.05);
}

.cursor-pointer {
    cursor: pointer;
}

.transition-all {
    transition: all 0.2s ease;
}

.animate-slide-in {
    animation: slideIn 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.scale-in {
    animation: scaleIn 0.2s cubic-bezier(0.4, 0, 0.2, 1);
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

@keyframes scaleIn {
    from {
        transform: scale(0);
    }

    to {
        transform: scale(1);
    }
}

.user-select-none {
    user-select: none;
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
}

.sticky-top {
    background-color: var(--bs-primary-bg-subtle) !important;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);
}

/* 优化滚动条样式，使其更美观 */
.overflow-auto::-webkit-scrollbar {
    width: 6px;
}

.overflow-auto::-webkit-scrollbar-thumb {
    background: rgba(16, 185, 129, 0.2);
    border-radius: 10px;
}
</style>