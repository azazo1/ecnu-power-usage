<template>
    <div
        class="flex h-screen w-screen bg-green-50 text-gray-700 font-sans selection:bg-emerald-200"
    >
        <aside
            class="w-64 bg-white border-r border-green-100 flex flex-col shadow-sm z-20"
        >
            <div class="p-6">
                <h1
                    class="text-2xl font-bold text-emerald-600 tracking-tight flex items-center gap-2"
                >
                    <span class="text-3xl">⚡</span> 宿舍电量
                </h1>
            </div>

            <nav class="flex-1 px-4 space-y-2">
                <button
                    @click="
                        currentTab = 'records';
                        selectedArchive = null;
                    "
                    class="w-full flex items-center gap-3 px-4 py-3 rounded-xl transition-all duration-200"
                    :class="
                        currentTab === 'records'
                            ? 'bg-emerald-100 text-emerald-800 font-semibold shadow-inner'
                            : 'hover:bg-gray-50 text-gray-600'
                    "
                >
                    <svg
                        class="w-5 h-5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
                        ></path>
                    </svg>
                    当前记录 (Records)
                </button>

                <button
                    @click="currentTab = 'archives'"
                    class="w-full flex items-center gap-3 px-4 py-3 rounded-xl transition-all duration-200"
                    :class="
                        currentTab === 'archives'
                            ? 'bg-emerald-100 text-emerald-800 font-semibold shadow-inner'
                            : 'hover:bg-gray-50 text-gray-600'
                    "
                >
                    <svg
                        class="w-5 h-5"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"
                        ></path>
                    </svg>
                    历史归档 (Archives)
                </button>
            </nav>

            <div class="p-4 text-xs text-center text-gray-400">
                v1.0.0 Stable
            </div>
        </aside>

        <main class="flex-1 p-4 overflow-hidden relative">
            <transition name="fade" mode="out-in">
                <DataVisualizer
                    v-if="currentTab === 'records'"
                    title="当前周期记录"
                    :data="currentRecords"
                />

                <div
                    v-else-if="currentTab === 'archives' && !selectedArchive"
                    class="h-full bg-white rounded-2xl shadow-sm border border-emerald-100 flex flex-col"
                >
                    <div
                        class="p-4 border-b border-emerald-50 flex justify-between items-center"
                    >
                        <h2 class="text-lg font-semibold text-emerald-800">
                            归档列表
                        </h2>
                        <button
                            @click="refreshArchives"
                            class="p-2 text-emerald-600 hover:bg-emerald-50 rounded-lg transition"
                            title="刷新"
                        >
                            <svg
                                class="w-5 h-5"
                                fill="none"
                                stroke="currentColor"
                                viewBox="0 0 24 24"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                                ></path>
                            </svg>
                        </button>
                    </div>
                    <div
                        class="p-6 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 overflow-auto"
                    >
                        <div
                            v-for="arc in archiveList"
                            :key="arc.id"
                            @click="openArchive(arc)"
                            class="group cursor-pointer border border-emerald-100 bg-emerald-50/50 hover:bg-emerald-100 hover:shadow-md rounded-xl p-4 transition-all"
                        >
                            <div class="flex items-center gap-3 mb-2">
                                <div
                                    class="bg-white p-2 rounded-lg text-emerald-500"
                                >
                                    <svg
                                        class="w-6 h-6"
                                        fill="none"
                                        stroke="currentColor"
                                        viewBox="0 0 24 24"
                                    >
                                        <path
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                            stroke-width="2"
                                            d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
                                        ></path>
                                    </svg>
                                </div>
                                <div>
                                    <h3
                                        class="font-bold text-gray-800 group-hover:text-emerald-800"
                                    >
                                        {{ arc.name }}
                                    </h3>
                                    <p class="text-xs text-gray-500">
                                        {{ arc.dateRange }}
                                    </p>
                                </div>
                            </div>
                            <div
                                class="text-right text-xs text-emerald-600 font-medium"
                            >
                                点击查看详情 &rarr;
                            </div>
                        </div>
                    </div>
                </div>

                <DataVisualizer
                    v-else-if="currentTab === 'archives' && selectedArchive"
                    :title="selectedArchive.name"
                    :data="selectedArchiveData"
                    is-archive-mode
                    @back="selectedArchive = null"
                />
            </transition>
        </main>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import DataVisualizer from "./components/DataVisualizer.vue";
import { parseCsvData, type ElectricityRecord } from "./utils/electricity";

// --- State ---
const currentTab = ref<"records" | "archives">("records");
const currentRecords = ref<ElectricityRecord[]>([]);
const archiveList = ref<any[]>([]);
const selectedArchive = ref<any>(null);
const selectedArchiveData = ref<ElectricityRecord[]>([]);

// --- Mock Data Loaders (todo 替换为 Tauri invoke) ---

// 1. 模拟加载 Records
onMounted(() => {
    // 模拟 CSV 数据
    const mockCsv = `
2026-01-24T14:35:32+08:00,33.43
2026-01-24T18:00:00+08:00,32.10
2026-01-25T00:00:00+08:00,30.50
2026-01-25T08:00:00+08:00,28.00
2026-01-25T12:00:00+08:00,27.50
2026-01-25T18:00:00+08:00,25.00
  `;
    currentRecords.value = parseCsvData(mockCsv);

    refreshArchives();
});

// 2. 模拟加载 Archive List
const refreshArchives = () => {
    archiveList.value = [
        { id: 1, name: "2025年秋季学期", dateRange: "2025.09.01 - 2026.01.15" },
        { id: 2, name: "2025年春季学期", dateRange: "2025.03.01 - 2025.07.15" },
    ];
};

// 3. 模拟加载单个 Archive
const openArchive = (archive: any) => {
    // 这里应该调用 Tauri 后端读取文件
    const mockArchiveCsv = `
2025-09-01T00:00:00+08:00,100.00
2025-09-02T00:00:00+08:00,95.00
2025-09-03T00:00:00+08:00,90.00
  `;
    selectedArchiveData.value = parseCsvData(mockArchiveCsv);
    selectedArchive.value = archive;
};
</script>

<style>
/* 简单的淡入淡出过渡 */
.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>
