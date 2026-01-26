<template>
    <div class="d-flex vh-100 bg-gradient"
        style="background: linear-gradient(135deg, #e8f5e9 0%, #c8e6c9 50%, #a5d6a7 100%);">
        <!-- Sidebar -->
        <aside class="bg-white shadow-lg d-flex flex-column" style="width: 80px; min-width: 80px;">
            <!-- Logo (todo 替换成图标, 添加悬停提示, 单击跳转 github)-->
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

            <!-- Version Info (todo: get version from backend)-->
            <div class="p-3 text-center text-muted small" data-bs-toggle="tooltip" data-bs-placement="right"
                :title="'版本 v' + crateVersion">
                <span style="opacity: 0.6;">v{{ crateVersion }}</span>
            </div>
        </aside>

        <!-- Main Content -->
        <main class="flex-grow-1 p-4 h-100 overflow-hidden">
            <transition name="fade" mode="out-in">
                <!-- Current Records View -->
                <DataVisualizer v-if="currentTab === 'records'" title="📊 当前周期记录" :data="currentRecords"
                    :archive-path="null" />

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
                            <div v-for="arc in archiveList" :key="arc.name" class="col-md-6 col-lg-4">
                                <div @click="openArchive(arc.name)"
                                    class="card h-100 border-success border-opacity-50 hover-card cursor-pointer rounded-4 shadow-sm">
                                    <div class="card-body d-flex flex-column">
                                        <div class="d-flex align-items-center gap-3 mb-3">
                                            <div
                                                class="bg-success bg-gradient text-white rounded-3 p-3 shadow-sm hover-icon">
                                                <i class="bi bi-box-seam fs-3"></i>
                                            </div>
                                            <div class="flex-grow-1">
                                                <h5 class="card-title fw-bold mb-1 text-dark">{{ arc.name }}
                                                </h5>
                                                <p class="card-text small text-success mb-0">{{
                                                    timeFormatter.format(arc.startTime) }}
                                                    - {{ timeFormatter.format(arc.endTime) }}</p>
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
                <DataVisualizer v-else-if="currentTab === 'archives' && selectedArchive" :title="selectedArchive"
                    :data="selectedArchiveData.content" is-archive-mode @back="selectedArchive = null"
                    :archive-path="selectedArchiveData.path" />
            </transition>
        </main>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import DataVisualizer from "./components/DataVisualizer.vue";
import { getRecords, type ElectricityRecord } from "./utils/records";
import { invoke } from "@tauri-apps/api/core";
import { Archive, ArchiveMeta, downloadArchive, listArchives } from "./utils/archive";

// --- State ---
const currentTab = ref<"records" | "archives">("records");
const currentRecords = ref<ElectricityRecord[]>([]);
const archiveList = ref<ArchiveMeta[]>([]);
const selectedArchive = ref<string | null>(null);
const selectedArchiveData = ref<Archive>({ path: "", content: [] });
const crateVersion = ref<string>('');

// --- Data Loaders ---

const timeFormatter = new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false // 使用24小时制
});

onMounted(async () => {
    crateVersion.value = await getCrateVersion()
})

// todo records 刷新按钮
// 加载 Records
onMounted(async () => {
    await refreshRecords();
    await refreshArchives();
});

async function refreshRecords() {
    currentRecords.value = await getRecords();
};

async function refreshArchives() {
    archiveList.value = await listArchives();
};

async function openArchive(archiveName: string) {
    selectedArchiveData.value = await downloadArchive(archiveName);
    selectedArchive.value = archiveName;
};

async function getCrateVersion(): Promise<string> {
    return await invoke("crate_version");
}
</script>

<style scoped>
:global(html, body) {
    margin: 0;
    padding: 0;
    height: 100vh;
    width: 100vw;
    /* 核心属性：禁止滚动回弹效果 */
    overscroll-behavior: none;
    /* 彻底禁止外层溢出 */
    overflow: hidden;
    /* 禁止系统级手势选中文字导致的拖拽感 */
    user-select: none;
}

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
