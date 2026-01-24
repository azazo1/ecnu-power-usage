<template>
    <div class="d-flex min-vh-100 bg-gradient"
        style="background: linear-gradient(135deg, #e8f5e9 0%, #c8e6c9 50%, #a5d6a7 100%);">
        <!-- Sidebar -->
        <aside class="bg-white shadow-lg d-flex flex-column" style="width: 80px; min-width: 80px;">
            <!-- Logo -->
            <div class="p-4 d-flex justify-content-center align-items-center position-relative" data-bs-toggle="tooltip"
                data-bs-placement="right" title="宿舍电量监控">
                <span style="font-size: 2.5rem; cursor: pointer;" class="hover-scale">⚡</span>
            </div>

            <!-- Navigation -->
            <nav class="flex-grow-1 px-3 py-2 d-flex flex-column gap-3">
                <button @click="currentTab = 'records'; selectedArchive = null"
                    class="btn btn-nav position-relative d-flex align-items-center justify-content-center p-3 rounded-4 border-0 transition-all"
                    :class="currentTab === 'records' ? 'btn-nav-active' : 'btn-nav-inactive'" data-bs-toggle="tooltip"
                    data-bs-placement="right" title="当前记录">
                    <i class="bi bi-bar-chart-line fs-4"></i>
                </button>

                <button @click="currentTab = 'archives'"
                    class="btn btn-nav position-relative d-flex align-items-center justify-content-center p-3 rounded-4 border-0 transition-all"
                    :class="currentTab === 'archives' ? 'btn-nav-active' : 'btn-nav-inactive'" data-bs-toggle="tooltip"
                    data-bs-placement="right" title="历史归档">
                    <i class="bi bi-archive fs-4"></i>
                </button>
            </nav>

            <!-- Version Info -->
            <div class="p-3 text-center text-muted small" data-bs-toggle="tooltip" data-bs-placement="right"
                title="版本 1.0.0 Stable">
                <span style="opacity: 0.6;">v1.0</span>
            </div>
        </aside>

        <!-- Main Content -->
        <main class="flex-grow-1 p-4 overflow-hidden">
            <transition name="fade" mode="out-in">
                <!-- Current Records View -->
                <DataVisualizer v-if="currentTab === 'records'" title="📊 当前周期记录" :data="currentRecords" />

                <!-- Archives List View -->
                <div v-else-if="currentTab === 'archives' && !selectedArchive"
                    class="h-100 bg-white rounded-4 shadow-lg border border-success border-opacity-25 d-flex flex-column overflow-hidden">
                    <div
                        class="p-4 border-bottom border-success border-opacity-25 d-flex justify-content-between align-items-center bg-light bg-opacity-50">
                        <h2 class="h4 mb-0 fw-bold text-success d-flex align-items-center gap-2">
                            <i class="bi bi-archive-fill"></i>
                            归档列表
                        </h2>
                        <button @click="refreshArchives"
                            class="btn btn-outline-success rounded-3 d-flex align-items-center gap-2 hover-scale"
                            title="刷新">
                            <i class="bi bi-arrow-clockwise"></i>
                        </button>
                    </div>
                    <div class="p-4 overflow-auto flex-grow-1">
                        <div class="row g-4">
                            <div v-for="arc in archiveList" :key="arc.id" class="col-md-6 col-lg-4">
                                <div @click="openArchive(arc)"
                                    class="card h-100 border-success border-opacity-50 hover-card cursor-pointer rounded-4 shadow-sm">
                                    <div class="card-body d-flex flex-column">
                                        <div class="d-flex align-items-center gap-3 mb-3">
                                            <div
                                                class="bg-success bg-gradient text-white rounded-3 p-3 shadow-sm hover-icon">
                                                <i class="bi bi-box-seam fs-3"></i>
                                            </div>
                                            <div class="flex-grow-1">
                                                <h5 class="card-title fw-bold mb-1 text-dark">{{ arc.name }}</h5>
                                                <p class="card-text small text-success mb-0">{{ arc.dateRange }}</p>
                                            </div>
                                        </div>
                                        <div
                                            class="mt-auto d-flex align-items-center justify-content-end text-success fw-semibold small">
                                            <span class="me-1">查看详情</span>
                                            <i class="bi bi-arrow-right"></i>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Archive Detail View -->
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
2025-12-24T14:35:32+08:00,33.43
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

<style scoped>
.hover-scale {
    transition: transform 0.3s ease;
}

.hover-scale:hover {
    transform: scale(1.1);
}

.btn-nav {
    transition: all 0.3s ease;
}

.btn-nav-active {
    background: linear-gradient(135deg, #66bb6a 0%, #43a047 100%);
    color: white;
    box-shadow: 0 4px 12px rgba(67, 160, 71, 0.3);
    transform: scale(1.05);
}

.btn-nav-inactive {
    background: transparent;
    color: #9e9e9e;
}

.btn-nav-inactive:hover {
    background: #f1f8e9;
    color: #43a047;
    transform: scale(1.05);
}

.hover-card {
    transition: all 0.3s ease;
}

.hover-card:hover {
    transform: translateY(-4px);
    box-shadow: 0 8px 24px rgba(67, 160, 71, 0.2) !important;
    border-color: #43a047 !important;
}

.hover-icon {
    transition: transform 0.3s ease;
}

.hover-card:hover .hover-icon {
    transform: rotate(5deg) scale(1.1);
}

.cursor-pointer {
    cursor: pointer;
}

/* Fade transition */
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
