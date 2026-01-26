<template>
    <div class="d-flex vh-100 bg-gradient"
        style="background: linear-gradient(135deg, #e8f5e9 0%, #c8e6c9 50%, #a5d6a7 100%);">
        <div class="position-fixed top-0 end-0 p-4 z-100 d-flex flex-column gap-2" style="max-width: 400px;">
            <TransitionGroup name="notification-list">
                <div v-for="note in notifications" :key="note.id"
                    class="notification-item shadow-lg rounded-4 bg-white border border-opacity-25 px-4 py-3 d-flex align-items-start gap-3"
                    :class="note.type === 'success' ? 'border-success' : 'border-danger'">

                    <i
                        :class="['bi fs-4', note.type === 'success' ? 'bi-check-circle-fill text-success' : 'bi-exclamation-circle-fill text-danger']"></i>

                    <div class="d-flex flex-column flex-grow-1">
                        <span class="fw-bold text-dark">{{ note.title }}</span>
                        <span v-if="note.message" class="text-muted small mt-1">{{ note.message }}</span>
                    </div>

                    <button @click="removeNotification(note.id)" class="btn btn-sm btn-link text-secondary p-0">
                        <i class="bi bi-x-lg"></i>
                    </button>
                </div>
            </TransitionGroup>
        </div>

        <!-- Sidebar -->
        <aside class="bg-white shadow-lg d-flex flex-column" style="width: 80px; min-width: 80px;">
            <!-- Logo -->
            <div class="p-4 d-flex justify-content-center align-items-center">
                <button @click="openUrl('https://github.com/azazo1/ecnu-power-usage')"
                    class="btn p-0 border-0 d-flex align-items-center justify-content-center hover-logo position-relative"
                    data-bs-toggle="tooltip" data-bs-placement="right" title="宿舍电量监控">
                    <div class="bg-success bg-gradient text-white rounded-circle shadow-sm d-flex align-items-center justify-content-center"
                        style="width: 48px; height: 48px;">
                        <i class="bi bi-lightning-charge-fill fs-3"></i>
                    </div>
                    <div class="position-absolute top-50 start-50 translate-middle bg-success rounded-circle pulse-ring"
                        style="width: 60px; height: 60px;"></div>
                </button>
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
            <div class="p-3 text-center text-muted small cursor-pointer hover-opacity-100"
                @click="openUrl('https://github.com/azazo1/ecnu-power-usage/releases')" data-bs-toggle="tooltip"
                data-bs-placement="right" :title="'查看更新日志 v' + crateVersion">
                <span style="opacity: 0.6;">v{{ crateVersion }}</span>
            </div>
        </aside>

        <!-- Main Content -->
        <main class="flex-grow-1 p-4 h-100 overflow-hidden">
            <transition name="fade" mode="out-in">
                <!-- Current Records View -->
                <DataVisualizer v-if="currentTab === 'records'" @refresh="refreshRecords" :data="currentRecords"
                    @create-archive="handleCreateArchive" :archive-path="null">
                    <template #title><i class="bi bi-journal-text me-2"></i> 当前周期记录</template>
                </DataVisualizer>

                <!-- Archives List View -->
                <ArchiveList v-else-if="currentTab === 'archives' && !selectedArchive" :archive-list="archiveList"
                    @open="openArchive" @refresh="refreshSelectedArchive" />

                <!-- Archive Detail View -->
                <DataVisualizer v-else-if="currentTab === 'archives' && selectedArchive"
                    :data="selectedArchiveData.content" is-archive-mode @archive-back="selectedArchive = null"
                    @refresh="refreshSelectedArchive" :archive-path="selectedArchiveData.path">
                    <template #title>
                        {{ selectedArchive.length > 13 ? selectedArchive.slice(0, 13) + '...' : selectedArchive }}
                    </template>
                </DataVisualizer>
            </transition>
        </main>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import DataVisualizer from "./components/DataVisualizer.vue";
import { getRecords, type ElectricityRecord } from "./utils/records";
import { invoke } from "@tauri-apps/api/core";
import { Archive, ArchiveMeta, createArchive, downloadArchive, listArchives } from "./utils/archive";
import { openUrl } from "@tauri-apps/plugin-opener";
import ArchiveList from "./components/ArchiveList.vue";

// --- State ---
const currentTab = ref<"records" | "archives">("records");
const currentRecords = ref<ElectricityRecord[]>([]);
const archiveList = ref<ArchiveMeta[]>([]);
const selectedArchive = ref<string | null>(null);
const selectedArchiveData = ref<Archive>({ path: "", content: [] });
const crateVersion = ref<string>('');

// --- Data ---

onMounted(async () => {
    crateVersion.value = await getCrateVersion()
})

// 加载 Records
onMounted(() => {
    refreshRecords();
    refreshArchives();
});

async function refreshRecords() {
    try {
        currentRecords.value = await getRecords();
    } catch (e) {
        notifyError("获取当前记录失败", `获取内容失败: ${e}`);
    }
}

async function refreshArchives() {
    try {
        archiveList.value = await listArchives();
    } catch (e) {
        notifyError("获取归档列表失败", `获取内容失败: ${e}`);
    }
}

async function refreshSelectedArchive() {
    if (selectedArchive.value != null) {
        try {
            selectedArchiveData.value = await downloadArchive(selectedArchive.value);
        } catch (e) {
            notifyError("获取归档记录失败", `${e}`);
        }
    }
}

async function openArchive(archiveName: string) {
    try {
        selectedArchiveData.value = await downloadArchive(archiveName);
        selectedArchive.value = archiveName;
    } catch (e) {
        notifyError("打开归档失败", `${e}`);
    }
};

async function getCrateVersion(): Promise<string> {
    return await invoke("crate_version");
}

async function handleCreateArchive(startTime: Date | null, endTime: Date | null, name: string | null) {
    try {
        let meta = await createArchive(startTime, endTime, name);
        console.log(meta);
        // 触发成功通知
        notifySuccess('归档创建成功', `已归档 ${meta.recordsNum} 条记录`);

        await refreshRecords();
        await refreshArchives();
    } catch (error) {
        console.error(error);
        notifyError('归档创建失败', `${error}\n请检查后端日志获取更详细内容`);
    }
}

// --- Notification ---

interface Notification {
    id: number;
    title: string;
    message?: string;
    type: 'success' | 'error';
}

const notifications = ref<Notification[]>([]);
let nextId = 0;

function notifySuccess(title: string, message: string = '') {
    console.log("emit info", { title, message })
    addNotification(title, message, 'success');
}

function notifyError(title: string, message: string = '') {
    console.log("emit error", { title, message })
    addNotification(title, message, 'error');
}

function addNotification(title: string, message: string, type: 'success' | 'error') {
    const id = nextId++;
    const newNotif = { id, title, message, type };

    // 堆叠在数组开头（显示在最上方）或结尾（显示在最下方）
    notifications.value.push(newNotif);

    // 3秒后自动移除
    setTimeout(() => {
        removeNotification(id);
    }, 4000);
}

function removeNotification(id: number) {
    notifications.value = notifications.value.filter(n => n.id !== id);
}

// --- health check ---
type HealthKind = 'NoNet' | 'ServerDown' | 'NotLogin' | 'NoRoom' | 'Ok' | 'Unknown';
interface HealthStatus {
    kind: HealthKind,
    message: string | null,
}

async function healthCheck(): Promise<HealthStatus> {
    try {
        let kind: HealthKind = await invoke('health_check');
        return {
            kind,
            message: null,
        }
    } catch (error) {
        return {
            kind: 'Unknown',
            message: String(error)
        }
    }
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

.cursor-pointer {
    cursor: pointer;
}

/* 标签页切换动画 Fade transition */
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

/* 通知列表动画 */
.notification-list-enter-active,
.notification-list-leave-active {
    transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.notification-list-enter-from {
    opacity: 0;
    transform: translateX(50px) scale(0.9);
}

.notification-list-leave-to {
    opacity: 0;
    transform: translateX(20px);
}

/* 关键：处理列表项移动动画 */
.notification-list-move {
    transition: transform 0.4s ease;
}

/* 确保离开时脱离文档流，让其他元素平滑滑动 */
.notification-list-leave-active {
    position: absolute;
    width: 100%;
    /* 保持原有宽度防止坍塌 */
}

.notification-item {
    pointer-events: auto;
    /* 确保按钮可点击 */
    z-index: 1000;
}

/* Logo Animation */
.hover-logo {
    transition: transform 0.3s ease;
}

.hover-logo:hover {
    transform: scale(1.05);
}

.pulse-ring {
    opacity: 0;
}

.pulse-ring:hover {
    animation: pulse 1.3s infinite forwards;
}

@keyframes pulse {
    0% {
        transform: translate(-50%, -50%) scale(0.95);
        opacity: 0.3;
    }

    70% {
        transform: translate(-50%, -50%) scale(1.2);
        opacity: 0;
    }

    100% {
        transform: translate(-50%, -50%) scale(0.95);
        opacity: 0;
    }
}
</style>
