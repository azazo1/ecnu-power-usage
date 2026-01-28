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

                    <div class="d-flex flex-column flex-grow-1 text-break pe-2">
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

            <div class="border-top border-light my-2 mx-2"></div>

            <!-- 配置按钮 -->
            <button @click="showConfigModal = true"
                class="btn-gear d-flex align-items-center justify-content-center border-0 shadow-sm transition-all"
                :class="{ 'btn-gear-active': showConfigModal }" data-bs-toggle="tooltip" data-bs-placement="right"
                title="系统设置">
                <i class="bi bi-gear-fill fs-4"></i>
            </button>

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

        <ConfigModal :show="showConfigModal" @close="showConfigModal = false" @save="handleConfigSave"
            @error="notifyError" />

        <HealthModal :show="currentHealth.kind !== 'Ok'" :health-status="currentHealth" @retry="manualHealthCheck"
            @error="notifyError" @open-config="showConfigModal = true" />
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import DataVisualizer from "./components/DataVisualizer.vue";
import { getRecords, type ElectricityRecord } from "./utils/records";
import { invoke } from "@tauri-apps/api/core";
import { Archive, ArchiveMeta, createArchive, downloadArchive, listArchives } from "./utils/archive";
import { openUrl } from "@tauri-apps/plugin-opener";
import ArchiveList from "./components/ArchiveList.vue";
import HealthModal from "./components/HealthModal.vue";
import { healthCheck, HealthStatus } from "./utils/health";
import ConfigModal from "./components/ConfigModal.vue";
import { GuiConfig } from "./utils/config";
import { sysNotify } from "./utils/notify";

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
});

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

        refreshRecords();
        refreshArchives();
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
const currentHealth = ref<HealthStatus>({ kind: 'Ok', message: null });
let healthCheckTimer: number | null = null;

onMounted(async () => {
    // 启动轮询，每 5 秒检查一次
    healthCheckTimer = window.setInterval(async () => {
        // 如果当前已经在显示模态框，我们可以暂停轮询，或者继续轮询以自动恢复
        // 这里选择继续轮询，这样网络恢复后模态框会自动消失
        await performHealthCheck();
    }, 5000);
});

const notifyConfig = {
    NoNet: { title: '网络已断开', msg: '请检查网络设置。' },
    ServerDown: { title: '服务器连接失败', msg: '后端服务暂时无法访问。' },
    NotLogin: { title: '登录已过期', msg: '请重新登录以继续。' },
    NoRoom: { title: '房间未绑定', msg: '请先配置您的宿舍房间号。' },
    TlsError: { title: '安全连接失败', msg: '证书校验无效，连接已终止。' },
    Unknown: { title: '系统错误', msg: '发生未知异常。' }
};

// 执行检查并更新状态
async function performHealthCheck(): Promise<HealthStatus> {
    const rawStatus = currentHealth.value;
    const status = await healthCheck();
    // 只有状态发生改变时才更新，避免不必要的响应式触发（虽然 Vue 会处理基本类型的 diff）
    if (status.kind !== rawStatus.kind) {
        console.log(`Health status changed: ${rawStatus.kind} -> ${status.kind}`);
        currentHealth.value = status;

        // 如果状态恢复正常，顺便刷新一下数据
        if (status.kind === 'Ok') {
            refreshRecords();
            refreshArchives();
            refreshSelectedArchive();
        } else {
            const config = notifyConfig[status.kind] || notifyConfig.Unknown;
            sysNotify(config.title, config.msg);
        }
    } else if (status.message !== rawStatus.message) {
        // 如果 kind 没变但 message 变了也更新一下
        currentHealth.value = status;
    }
    return rawStatus;
}

// 手动触发（用于点击“重试”按钮）
async function manualHealthCheck() {
    // 设置为 loading 状态或者是保留当前状态，这里简单直接调逻辑
    await performHealthCheck();
    if (currentHealth.value.kind !== 'Ok') {
        notifyError("状态异常", "请检查相关设置或网络。");
    } else {
        notifySuccess("连接成功", "状态正常。");
    }
}

onUnmounted(() => {
    if (healthCheckTimer) {
        clearInterval(healthCheckTimer);
    }
});

// --- Config ---

const showConfigModal = ref(false);

function handleConfigSave(_config: GuiConfig) {
    notifySuccess('设置已保存', '配置更新成功，正在重试连接...');
    // 保存后立即尝试重连，给用户即时反馈
    manualHealthCheck();
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

/* 齿轮按钮基础样式 */
.btn-gear {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    /* 强制圆形 */
    background-color: #ffffff;
    color: #9e9e9e;
    cursor: pointer;
    transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
    margin: 0 auto;
    /* 侧边栏居中 */
}

/* 激活状态 (Modal 打开时) */
.btn-gear-active {
    background: linear-gradient(135deg, #66bb6a 0%, #43a047 100%);
    color: white !important;
    box-shadow: 0 4px 12px rgba(67, 160, 71, 0.4);
    transform: rotate(90deg);
    /* 激活时旋转 90 度 */
}

/* 悬停效果：旋转 + 绿色背光 */
.btn-gear:hover {
    color: #43a047;
    background-color: #f1f8e9;
    transform: scale(1.1) rotate(45deg);
    /* 悬停时轻微旋转 */
    box-shadow: 0 0 15px rgba(67, 160, 71, 0.2);
}

/* 内部图标旋转动画 */
.btn-gear:hover i {
    filter: drop-shadow(0 0 2px rgba(67, 160, 71, 0.3));
}

/* 点击反馈 */
.btn-gear:active {
    transform: scale(0.9) rotate(0deg);
    transition: all 0.1s;
}

/* 更有“设置”感，加一个微小的呼吸灯 */
.btn-gear-active::after {
    content: '';
    position: absolute;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    border: 2px solid #66bb6a;
    animation: gear-pulse 2s infinite;
}

@keyframes gear-pulse {
    0% {
        transform: scale(1);
        opacity: 0.5;
    }

    100% {
        transform: scale(1.4);
        opacity: 0;
    }
}
</style>
