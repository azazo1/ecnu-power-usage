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
                                        <label class="form-label x-small fw-bold text-muted mb-1">客户端证书</label>
                                        <textarea v-model="config.clientCert"
                                            class="form-control form-control-sm font-monospace x-small bg-white"
                                            rows="3"></textarea>
                                    </div>
                                    <div>
                                        <label class="form-label x-small fw-bold text-muted mb-1">客户端私钥</label>
                                        <textarea v-model="config.clientKey"
                                            class="form-control form-control-sm font-monospace x-small bg-white"
                                            rows="3"></textarea>
                                    </div>
                                    <div>
                                        <label class="form-label x-small fw-bold text-muted mb-1">根证书</label>
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
import { getConfigCmd, GuiConfig, updateConfigCmd as saveConfigCmd } from '../utils/config';

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{
    close: [],
    save: [any]
    error: [string, string]
}>();

const loading = ref(false);
const showTls = ref(false);

const config = ref<GuiConfig>({
    serverBase: '',
    useSelfSignedTls: false
});

// 判断证书是否全部填写
const isTlsReady = computed(() => {
    return !!(config.value.useSelfSignedTls &&
        config.value.clientCert?.trim() &&
        config.value.clientKey?.trim() &&
        config.value.rootCA?.trim());
});

// 打开时加载配置
watch(() => props.show, async (newVal) => {
    if (newVal) {
        showTls.value = false;
        try {
            loading.value = true;
            const savedConfig = await getConfigCmd();
            config.value = savedConfig;
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
        await saveConfigCmd(config.value);
        emit('save', config.value);
        emit('close');
    } catch (e) {
        console.error(e);
        emit('error', '保存配置失败', String(e));
    } finally {
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
</style>