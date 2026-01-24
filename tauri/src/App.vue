<template>
    <div class="flex h-screen w-screen bg-gradient-to-br from-emerald-50 via-green-50 to-teal-50 text-gray-700 font-sans selection:bg-emerald-200 antialiased">
        <aside class="w-20 bg-white/80 backdrop-blur-xl border-r border-emerald-100/50 flex flex-col shadow-lg z-20 transition-all duration-300">
            <div class="p-6 flex justify-center items-center group relative">
                <span class="text-4xl transition-transform duration-300 group-hover:scale-110">⚡</span>
                <div class="absolute left-full ml-4 px-4 py-2 bg-emerald-600 text-white text-sm font-medium rounded-lg shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 whitespace-nowrap z-50">
                    宿舍电量监控
                    <div class="absolute top-1/2 right-full -translate-y-1/2 border-8 border-transparent border-r-emerald-600"></div>
                </div>
            </div>

            <nav class="flex-1 px-3 space-y-3 py-2">
                <button @click="
                    currentTab = 'records';
                selectedArchive = null;
                " class="group relative w-full flex items-center justify-center p-4 rounded-2xl transition-all duration-300" :class="currentTab === 'records'
                    ? 'bg-gradient-to-br from-emerald-400 to-green-500 text-white shadow-lg shadow-emerald-200 scale-105'
                    : 'hover:bg-emerald-50 text-gray-500 hover:text-emerald-600 hover:scale-105'
                    ">
                    <svg class="w-6 h-6 transition-transform duration-300 group-hover:scale-110" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z">
                        </path>
                    </svg>
                    <div class="absolute left-full ml-4 px-4 py-2 bg-emerald-600 text-white text-sm font-medium rounded-lg shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 whitespace-nowrap z-50">
                        当前记录
                        <div class="absolute top-1/2 right-full -translate-y-1/2 border-8 border-transparent border-r-emerald-600"></div>
                    </div>
                </button>

                <button @click="currentTab = 'archives'"
                    class="group relative w-full flex items-center justify-center p-4 rounded-2xl transition-all duration-300" :class="currentTab === 'archives'
                        ? 'bg-gradient-to-br from-emerald-400 to-green-500 text-white shadow-lg shadow-emerald-200 scale-105'
                        : 'hover:bg-emerald-50 text-gray-500 hover:text-emerald-600 hover:scale-105'
                        ">
                    <svg class="w-6 h-6 transition-transform duration-300 group-hover:scale-110" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4">
                        </path>
                    </svg>
                    <div class="absolute left-full ml-4 px-4 py-2 bg-emerald-600 text-white text-sm font-medium rounded-lg shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 whitespace-nowrap z-50">
                        历史归档
                        <div class="absolute top-1/2 right-full -translate-y-1/2 border-8 border-transparent border-r-emerald-600"></div>
                    </div>
                </button>
            </nav>

            <div class="p-4 text-xs text-center text-gray-400 group relative">
                <span class="opacity-60 group-hover:opacity-100 transition-opacity">v1.0</span>
                <div class="absolute left-full ml-4 bottom-0 px-4 py-2 bg-gray-700 text-white text-xs rounded-lg shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-200 whitespace-nowrap z-50">
                    版本 1.0.0 Stable
                    <div class="absolute top-1/2 right-full -translate-y-1/2 border-6 border-transparent border-r-gray-700"></div>
                </div>
            </div>
        </aside>

        <main class="flex-1 p-6 overflow-hidden relative">
            <transition name="fade" mode="out-in">
                <DataVisualizer v-if="currentTab === 'records'" title="📊 当前周期记录" :data="currentRecords" />

                <div v-else-if="currentTab === 'archives' && !selectedArchive"
                    class="h-full bg-white/80 backdrop-blur-sm rounded-3xl shadow-xl border border-emerald-100/50 flex flex-col overflow-hidden">
                    <div class="p-6 border-b border-emerald-100/50 flex justify-between items-center bg-gradient-to-r from-emerald-50/50 to-green-50/50">
                        <h2 class="text-xl font-bold text-emerald-800 flex items-center gap-2">
                            📦 归档列表
                        </h2>
                        <button @click="refreshArchives"
                            class="group p-3 text-emerald-600 hover:bg-emerald-100 rounded-xl transition-all duration-300 hover:scale-110" title="刷新">
                            <svg class="w-5 h-5 transition-transform duration-500 group-hover:rotate-180" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15">
                                </path>
                            </svg>
                        </button>
                    </div>
                    <div class="p-6 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5 overflow-auto">
                        <div v-for="arc in archiveList" :key="arc.id" @click="openArchive(arc)"
                            class="group cursor-pointer border-2 border-emerald-100 bg-gradient-to-br from-white via-emerald-50/30 to-green-50/20 hover:from-emerald-50 hover:via-green-50 hover:to-teal-50 hover:border-emerald-300 hover:shadow-2xl hover:shadow-emerald-100/50 rounded-2xl p-5 transition-all duration-300 hover:-translate-y-1">
                            <div class="flex items-center gap-3 mb-3">
                                <div class="bg-gradient-to-br from-emerald-400 to-green-500 p-3 rounded-xl text-white shadow-md transition-transform duration-300 group-hover:scale-110 group-hover:rotate-3">
                                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                            d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10">
                                        </path>
                                    </svg>
                                </div>
                                <div class="flex-1">
                                    <h3 class="font-bold text-gray-800 group-hover:text-emerald-700 transition-colors duration-300">
                                        {{ arc.name }}
                                    </h3>
                                    <p class="text-xs text-gray-500 group-hover:text-emerald-600 transition-colors duration-300">
                                        {{ arc.dateRange }}
                                    </p>
                                </div>
                            </div>
                            <div class="flex items-center justify-end text-xs text-emerald-600 font-semibold group-hover:text-emerald-700 transition-all duration-300">
                                <span class="group-hover:translate-x-1 transition-transform duration-300">查看详情</span>
                                <svg class="w-4 h-4 ml-1 group-hover:translate-x-2 transition-transform duration-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path>
                                </svg>
                            </div>
                        </div>
                    </div>
                </div>

                <DataVisualizer v-else-if="currentTab === 'archives' && selectedArchive" :title="selectedArchive.name"
                    :data="selectedArchiveData" is-archive-mode @back="selectedArchive = null" />
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
/* 优化的淡入淡出和位移过渡 */
.fade-enter-active {
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.fade-leave-active {
    transition: all 0.2s cubic-bezier(0.4, 0, 1, 1);
}

.fade-enter-from {
    opacity: 0;
    transform: translateY(10px);
}

.fade-leave-to {
    opacity: 0;
    transform: translateY(-10px);
}
</style>
