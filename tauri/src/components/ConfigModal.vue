<template>
    <Transition name="modal-fade">
        <div v-if="show"
            class="position-fixed top-0 start-0 w-100 h-100 d-flex justify-content-center align-items-center z-config-modal"
            style="background: rgba(0, 0, 0, 0.5); backdrop-filter: blur(5px);">

            <div class="card border-0 shadow-lg rounded-4 overflow-hidden animate-pop"
                style="width: 480px; max-height: 90vh;">
                <div
                    class="card-header bg-white border-bottom border-light p-4 d-flex justify-content-between align-items-center">
                    <h5 class="mb-0 fw-bold text-success d-flex align-items-center gap-2">
                        <i class="bi bi-sliders"></i>
                        <span>系统设置</span>
                    </h5>
                    <button @click="$emit('close')"
                        class="btn btn-sm btn-light rounded-circle d-flex align-items-center justify-content-center hover-scale"
                        style="width: 32px; height: 32px;">
                        <i class="bi bi-x-lg"></i>
                    </button>
                </div>

                <div class="card-body p-4 overflow-auto">
                    <form @submit.prevent="saveConfig" class="d-flex flex-column gap-4">
                        <div>
                            <label class="form-label small fw-bold text-secondary mb-1">服务器地址 URL</label>
                            <div class="input-group">
                                <span class="input-group-text bg-light border-0 text-secondary"><i
                                        class="bi bi-hdd-network"></i></span>
                                <input v-model="config.serverBase" type="text" class="form-control bg-light border-0"
                                    placeholder="https://example.com" required>
                            </div>
                            <div class="form-text x-small text-muted mt-1">
                                后端 API 服务的基地址，请包含 http/https 协议头。
                            </div>
                        </div>

                        <div>
                            <label class="form-label small fw-bold text-secondary mb-1">电量阈值 (度)</label>
                            <div class="input-group">
                                <span class="input-group-text bg-light border-0 text-secondary"><i
                                        class="bi bi-lightning-charge"></i></span>
                                <input v-model.number="config.degreeThreshold" type="number" step="0.1" min="0"
                                    max="999" class="form-control bg-light border-0" placeholder="10.0" required>
                            </div>
                            <div class="form-text x-small text-muted mt-1">
                                当宿舍剩余电量低于此值时，将会发送系统通知提醒。
                            </div>
                        </div>

                        <div class="p-3 bg-light bg-opacity-50 rounded-4 border border-secondary border-opacity-10">
                            <div class="d-flex justify-content-between align-items-center cursor-pointer user-select-none"
                                @click="toggleShowTls">
                                <div class="d-flex align-items-center gap-2">
                                    <i class="bi bi-shield-lock text-success"></i>
                                    <label class="form-label small fw-bold text-secondary mb-0 cursor-pointer">TLS
                                        证书配置</label>
                                    <span
                                        class="badge bg-secondary bg-opacity-10 text-secondary border border-secondary border-opacity-25 fw-normal rounded-pill"
                                        style="font-size: 0.65rem;">可选</span>
                                </div>
                                <i class="bi bi-chevron-down small transition-transform text-muted"
                                    :style="{ transform: showTls ? 'rotate(180deg)' : 'rotate(0deg)' }"></i>
                            </div>

                            <div v-show="showTls"> <!-- v-show 似乎不能和 d-flex 放一起, 因此需要额外套一层 div -->
                                <div
                                    class="d-flex flex-column gap-3 mt-3 pt-3 border-top border-secondary border-opacity-10">
                                    <div>
                                        <div class="d-flex justify-content-between align-items-center mb-1">
                                            <label class="form-label x-small fw-bold text-muted mb-0">客户端证书
                                                (client.crt)</label>
                                            <button type="button" @click="importFromFile('clientCert')"
                                                class="btn btn-link p-0 x-small text-success text-decoration-none">
                                                <i class="bi bi-file-earmark-arrow-up"></i> 导入
                                            </button>
                                        </div>
                                        <textarea v-model="config.clientCert"
                                            class="form-control form-control-sm font-monospace x-small bg-white"
                                            rows="3"></textarea>
                                    </div>

                                    <div>
                                        <div class="d-flex justify-content-between align-items-center mb-1">
                                            <label class="form-label x-small fw-bold text-muted mb-0">客户端私钥
                                                (client.key)</label>
                                            <button type="button" @click="importFromFile('clientKey')"
                                                class="btn btn-link p-0 x-small text-success text-decoration-none">
                                                <i class="bi bi-file-earmark-arrow-up"></i> 导入
                                            </button>
                                        </div>
                                        <textarea v-model="config.clientKey"
                                            class="form-control form-control-sm font-monospace x-small bg-white"
                                            rows="3"></textarea>
                                    </div>

                                    <div>
                                        <div class="d-flex justify-content-between align-items-center mb-1">
                                            <label class="form-label x-small fw-bold text-muted mb-0">根证书
                                                (root-ca.crt)</label>
                                            <button type="button" @click="importFromFile('rootCA')"
                                                class="btn btn-link p-0 x-small text-success text-decoration-none">
                                                <i class="bi bi-file-earmark-arrow-up"></i> 导入
                                            </button>
                                        </div>
                                        <textarea v-model="config.rootCA"
                                            class="form-control form-control-sm font-monospace x-small bg-white"
                                            rows="3"></textarea>
                                    </div>
                                </div>
                                <div class="mt-3 pt-3 border-top border-secondary border-opacity-10">
                                    <div class="form-check form-switch d-flex align-items-center gap-2">
                                        <input class="form-check-input cursor-pointer" type="checkbox" role="switch"
                                            id="useSelfSignedTls" v-model="config.useSelfSignedTls"
                                            style="width: 2.5em; height: 1.25em;">
                                        <label class="form-check-label small fw-bold text-secondary cursor-pointer"
                                            for="useSelfSignedTls">
                                            启用客户端证书验证 (mTLS)
                                        </label>
                                    </div>

                                    <div class="mt-2 p-2 rounded-3"
                                        :class="isTlsReady ? 'bg-success bg-opacity-10 text-success' : 'bg-warning bg-opacity-10 text-warning'"
                                        style="font-size: 0.7rem;">
                                        <i class="bi"
                                            :class="isTlsReady ? 'bi-check-circle-fill' : 'bi-exclamation-triangle-fill'"></i>
                                        <span class="ms-1">
                                            提示：只有在开启按钮且设置前三个属性都不为空时，该配置才会生效。
                                            <strong v-if="config.useSelfSignedTls && !isTlsReady">(当前证书不完整)</strong>
                                        </span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </form>

                    <div class="mt-4 pt-3 border-top border-light">
                        <label class="form-label x-small fw-bold text-secondary mb-2 d-flex align-items-center gap-2">
                            <i class="bi bi-hdd-stack"></i>
                            数据维护 (点击即刻生效)
                        </label>

                        <div class="d-flex gap-2">
                            <button type="button" @click="handleClearCookies" :disabled="loading"
                                class="btn btn-light btn-sm flex-grow-1 rounded-3 d-flex align-items-center justify-content-center gap-2 py-1 x-small text-secondary btn-action-hover">
                                <span v-if="loading" class="spinner-border spinner-border-sm"
                                    style="width: 0.8rem; height: 0.8rem;"></span>
                                <i v-else class="bi bi-eraser"></i>
                                <span>清除 Cookies</span>
                            </button>

                            <button type="button" @click="handleClearRoom" :disabled="loading"
                                class="btn btn-light btn-sm flex-grow-1 rounded-3 d-flex align-items-center justify-content-center gap-2 py-1 x-small text-secondary btn-action-hover">
                                <span v-if="loading" class="spinner-border spinner-border-sm"
                                    style="width: 0.8rem; height: 0.8rem;"></span>
                                <i v-else class="bi bi-arrow-counterclockwise"></i>
                                <span>重置房间信息</span>
                            </button>
                        </div>
                    </div>


                    <div class="mt-4 pt-3 border-top border-light">
                        <label class="form-label x-small fw-bold text-secondary mb-2 d-flex align-items-center gap-2">
                            <i class="bi bi-house-door"></i>
                            当前房间信息
                        </label>

                        <div class="p-3 bg-light bg-opacity-50 rounded-3 border border-secondary border-opacity-10">
                            <div v-if="roomInfo && !roomInfoError" class="d-flex flex-column gap-1">
                                <div class="d-flex align-items-center gap-2 x-small">
                                    <span class="text-muted">校区:</span>
                                    <span class="fw-bold text-dark">{{ roomInfo.area.areaName }}</span>
                                </div>
                                <div class="d-flex align-items-center gap-2 x-small">
                                    <span class="text-muted">园区:</span>
                                    <span class="fw-bold text-dark">{{ roomInfo.district.districtName }}</span>
                                </div>
                                <div class="d-flex align-items-center gap-2 x-small">
                                    <span class="text-muted">楼栋:</span>
                                    <span class="fw-bold text-dark">{{ roomInfo.building.buiName }}</span>
                                </div>
                                <div class="d-flex align-items-center gap-2 x-small">
                                    <span class="text-muted">楼层:</span>
                                    <span class="fw-bold text-dark">{{ roomInfo.floor.floorName }}</span>
                                </div>
                                <div class="d-flex align-items-center gap-2 x-small">
                                    <span class="text-muted">房间:</span>
                                    <span class="fw-bold text-dark">{{ roomInfo.room.roomName }}</span>
                                </div>
                            </div>
                            <div v-else class="text-center py-2">
                                <span class="x-small text-muted text-left d-block">
                                    <i class="bi bi-exclamation-circle"></i>
                                    房间信息获取失败, 可能原因: <br>
                                    &nbsp;&nbsp;&nbsp;&nbsp;1. 没有设置房间信息 <br>
                                    &nbsp;&nbsp;&nbsp;&nbsp;2. 房间信息无效 <br>
                                    &nbsp;&nbsp;&nbsp;&nbsp;3. 校园房间信息数据库变更导致信息失效
                                </span>
                            </div>
                        </div>
                    </div>

                    <div class="mt-4 pt-3 border-top border-light">
                        <label class="form-label x-small fw-bold text-secondary mb-2 d-flex align-items-center gap-2">
                            <i class="bi bi-hand-index"></i>
                            应用操作
                        </label>

                        <button type="button" @click="handleQuitApp" :disabled="loading"
                            class="btn btn-light btn-sm rounded-pill d-inline-flex align-items-center gap-2 py-1 x-small text-secondary btn-quit-hover">
                            <i class="bi bi-power"></i>
                            <span>退出应用</span>
                        </button>
                    </div>
                </div>

                <div class="card-footer bg-white border-top border-light p-3 d-flex gap-2 justify-content-end">
                    <button type="button" @click="$emit('close')"
                        class="btn btn-light text-secondary rounded-pill px-4 fw-bold small hover-scale">取消</button>
                    <button @click="saveConfig" :disabled="loading"
                        class="btn btn-success rounded-pill px-4 d-flex align-items-center gap-2 fw-bold small hover-scale shadow-sm">
                        <span v-if="loading" class="spinner-border spinner-border-sm" role="status"
                            aria-hidden="true"></span>
                        保存并应用
                    </button>
                </div>
            </div>
        </div>
    </Transition>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { clearCookiesCmd, clearRoomCmd, getConfigCmd, getRoomInfoCmd, GuiConfig, pickCertCmd, quitAppCmd, RoomInfo, updateConfigCmd } from '../utils/config';

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{
    close: [],
    save: [any]
    error: [string, string]
}>();

const loading = ref(false);
const showTls = ref(false);
const roomInfo = ref<RoomInfo | null>(null);
const roomInfoError = ref(false);

const config = ref<GuiConfig>({
    serverBase: '',
    useSelfSignedTls: false,
    degreeThreshold: 10.0
});

// 判断证书是否全部填写
const isTlsReady = computed(() => {
    return !!(config.value.useSelfSignedTls &&
        config.value.clientCert?.trim() &&
        config.value.clientKey?.trim() &&
        config.value.rootCA?.trim());
});

// 加载宿舍信息
async function loadRoomInfo() {
    try {
        roomInfo.value = await getRoomInfoCmd();
        roomInfoError.value = false;
    } catch (e) {
        console.error('Failed to load room info', e);
        roomInfo.value = null;
        roomInfoError.value = true;
        // emit('error', '获取宿舍信息失败', String(e));
    }
}

// 打开时加载配置
watch(() => props.show, async (newVal) => {
    if (newVal) {
        showTls.value = false;
        try {
            loading.value = true;
            const savedConfig = await getConfigCmd();
            config.value = savedConfig;
            await loadRoomInfo();
        } catch (e) {
            console.error('Failed to load config', e);
            emit('error', '加载配置失败', String(e));
        } finally {
            loading.value = false;
        }
    }
});

function toggleShowTls() {
    showTls.value = !showTls.value;
}

async function saveConfig() {
    loading.value = true;
    try {
        await updateConfigCmd(config.value);
        emit('save', config.value);
        emit('close');
    } catch (e) {
        console.error(e);
        emit('error', '保存配置失败', String(e));
    } finally {
        loading.value = false;
    }
}

async function importFromFile(field: 'clientCert' | 'clientKey' | 'rootCA') {
    try {
        const certPath = await pickCertCmd();
        config.value[field] = certPath;
    } catch (e) {
        if (e !== 'cancelled') {
            console.error(e);
            emit('error', '读取文件失败', String(e));
        }
    }
}

async function handleClearCookies() {
    loading.value = true;
    try {
        await clearCookiesCmd();
    } catch (e) {
        emit('error', '清除登录 Cookies 失败', String(e));
    } finally {
        loading.value = false;
    }
}

async function handleClearRoom() {
    loading.value = true;
    try {
        await clearRoomCmd();
        await loadRoomInfo();
    } catch (e) {
        emit('error', '清除房间信息失败', String(e));
    } finally {
        loading.value = false;
    }
}

async function handleQuitApp() {
    loading.value = true;
    try {
        await quitAppCmd();
    } catch (e) {
        emit('error', '退出应用失败', String(e));
        loading.value = false;
    }
}
</script>

<style scoped>
.z-config-modal {
    /* 必须比 HealthModal 的 50 高, 比 notification 100 低 */
    z-index: 75;
}

.x-small {
    font-size: 0.75rem;
}

.transition-transform {
    transition: transform 0.3s ease;
}

.hover-scale {
    transition: transform 0.2s ease;
}

.hover-scale:hover {
    transform: scale(1.05);
}

.cursor-pointer {
    cursor: pointer;
}

/* Modal animations */
.modal-fade-enter-active,
.modal-fade-leave-active {
    transition: opacity 0.3s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
    opacity: 0;
}

.modal-fade-enter-active .animate-pop {
    animation: popIn 0.4s cubic-bezier(0.175, 0.885, 0.32, 1.275);
}

.modal-fade-leave-active .animate-pop {
    animation: popOut 0.3s ease-in;
}

@keyframes popIn {
    0% {
        transform: scale(0.8) translateY(20px);
        opacity: 0;
    }

    100% {
        transform: scale(1) translateY(0);
        opacity: 1;
    }
}

@keyframes popOut {
    0% {
        transform: scale(1);
        opacity: 1;
    }

    100% {
        transform: scale(0.9);
        opacity: 0;
    }
}

.form-check-input:checked {
    background-color: #198754;
    border-color: #198754;
}

.form-check-input:focus {
    box-shadow: 0 0 0 0.25rem rgba(25, 135, 84, 0.25);
}

/* 提示文字过渡 */
.text-success,
.text-warning {
    transition: color 0.3s ease;
}

.btn-link.x-small {
    font-size: 0.7rem;
    opacity: 0.8;
    transition: opacity 0.2s;
}

.btn-link.x-small:hover {
    opacity: 1;
}

/* 自定义按钮悬停效果 */
.btn-action-hover {
    border: 1px solid transparent;
    /* 预留边框位置防止抖动 */
    transition: all 0.2s ease;
    background-color: #f8f9fa;
    /* 对应 bootstrap 的 bg-light */
}

/* 悬停时：背景变淡红，文字变红，边框变红 */
.btn-action-hover:hover:not(:disabled) {
    background-color: #fff5f5;
    /* 极淡的红色背景 */
    color: #dc3545 !important;
    /* text-danger */
    border-color: #ffcdd2;
    transform: translateY(-1px);
    /* 微微上浮，增加点击欲望 */
    box-shadow: 0 2px 4px rgba(220, 53, 69, 0.1);
}

/* 点击时（Active） */
.btn-action-hover:active:not(:disabled) {
    transform: translateY(0);
    background-color: #ffe6e6;
}

/* 禁用状态 */
.btn-action-hover:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}

/* 退出按钮样式 */
.btn-quit-hover {
    border: 1px solid transparent;
    transition: all 0.2s ease;
    background-color: #f8f9fa;
}

.btn-quit-hover:hover:not(:disabled) {
    background-color: #fff5f5;
    color: #dc3545 !important;
    border-color: #ffcdd2;
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(220, 53, 69, 0.1);
}

.btn-quit-hover:active:not(:disabled) {
    transform: translateY(0);
    background-color: #ffe6e6;
}

.btn-quit-hover:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}
</style>