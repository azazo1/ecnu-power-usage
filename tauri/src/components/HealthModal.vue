<template>
    <Transition name="modal-fade">
        <div v-if="show"
            class="position-fixed top-0 start-0 w-100 h-100 d-flex justify-content-center align-items-center z-modal"
            style="background: rgba(255, 255, 255, 0.85); backdrop-filter: blur(5px);">

            <div class="card border-0 shadow-lg rounded-4 overflow-hidden animate-pop position-relative"
                style="width: 380px;">
                <button @click="$emit('openConfig')"
                    class="btn btn-light position-absolute top-0 end-0 m-3 rounded-circle shadow-sm d-flex align-items-center justify-content-center hover-lift"
                    style="width: 32px; height: 32px; z-index: 10;" title="打开设置">
                    <i class="bi bi-gear-fill text-secondary small"></i>
                </button>

                <div class="h-1 w-100" :class="statusConfig.bgClass"></div>

                <div class="card-body p-4 text-center d-flex flex-column align-items-center gap-3">
                    <div class="rounded-circle p-3 d-flex align-items-center justify-content-center mb-1"
                        :class="statusConfig.iconBgClass" style="width: 80px; height: 80px;">
                        <i :class="['bi fs-1', statusConfig.icon, statusConfig.textClass]"></i>
                    </div>

                    <div>
                        <h4 class="fw-bold text-dark mb-2">{{ statusConfig.title }}</h4>
                        <p class="text-muted small mb-0 px-2 text-start" style="white-space: pre-line;">{{
                            statusConfig.desc }}</p>
                        <p v-if="healthStatus.message"
                            class="text-secondary x-small mt-2 font-monospace bg-light rounded p-1">{{
                                healthStatus.message }}</p>
                    </div>

                    <div class="w-100 mt-2">
                        <button v-if="isNotLogin" @click="uploadLoginCookies"
                            class="btn btn-primary w-100 rounded-pill py-2 fw-bold shadow-sm d-flex align-items-center justify-content-center gap-2 hover-lift">
                            <i class="bi bi-gear-fill"></i>
                            配置登录信息(打开 Chrome 浏览器)
                        </button>

                        <button v-if="isNoRoom" @click="uploadRoomConfig"
                            class="btn btn-primary w-100 rounded-pill py-2 fw-bold shadow-sm d-flex align-items-center justify-content-center gap-2 hover-lift">
                            <i class="bi bi-gear-fill"></i>
                            配置宿舍信息(打开 Chrome 浏览器)
                        </button>

                        <button v-else @click="$emit('retry')"
                            class="btn btn-outline-secondary w-100 rounded-pill py-2 fw-bold d-flex align-items-center justify-content-center gap-2 mt-2 hover-lift">
                            <i class="bi bi-arrow-clockwise"></i>
                            尝试重新连接
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </Transition>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { computed } from 'vue';
import { HealthStatus } from '../utils/health';

// 定义类型，确保和 App.vue 一致

const props = defineProps<{
    show: boolean;
    healthStatus: HealthStatus,
}>();

const emit = defineEmits<{
    retry: []
    openConfig: []
    error: [string, string]
}>();



const isNotLogin = computed(() => {
    return props.healthStatus.kind === 'NotLogin';
});

const isNoRoom = computed(() => {
    return props.healthStatus.kind === 'NoRoom';
});

// 状态映射配置
const statusConfig = computed(() => {
    switch (props.healthStatus.kind) {
        case 'NoNet':
            return {
                title: '网络连接断开',
                desc: '检测不到互联网连接，请检查您的网络设置。',
                icon: 'bi-wifi-off',
                bgClass: 'bg-secondary',
                textClass: 'text-secondary',
                iconBgClass: 'bg-secondary bg-opacity-10'
            };
        case 'ServerDown':
            return {
                title: '服务器无响应',
                desc: '电费查询后端似乎暂时无法访问，请稍后再试。',
                icon: 'bi-hdd-network',
                bgClass: 'bg-danger',
                textClass: 'text-danger',
                iconBgClass: 'bg-danger bg-opacity-10'
            };
        case 'NotLogin':
            return {
                title: '未登录公共数据库',
                desc: '您的登录凭证已过期或尚未登录。',
                icon: 'bi-person-x-fill',
                bgClass: 'bg-warning',
                textClass: 'text-warning',
                iconBgClass: 'bg-warning bg-opacity-10'
            };
        case 'NoRoom':
            return {
                title: '未绑定房间信息',
                desc: '尚未配置宿舍房间号，无法查询电量。',
                icon: 'bi-house-slash-fill',
                bgClass: 'bg-primary',
                textClass: 'text-primary',
                iconBgClass: 'bg-primary bg-opacity-10'
            };
        case 'TlsError':
            return {
                title: 'TLS 安全连接失败',
                desc: '由于安全协议不匹配或证书无效，无法建立加密连接。可能原因：\n' +
                    '1. 服务器证书 SANs 未包含当前域名或 IP 地址；\n' +
                    '2. 根证书/服务器证书/客户端证书过期；\n' +
                    '3. 域名解析错误，导致证书与域名不匹配；\n' +
                    '4. 等等。',
                icon: 'bi-shield-slash-fill',
                bgClass: 'bg-indigo',
                textClass: 'text-indigo',
                iconBgClass: 'bg-indigo bg-opacity-10'
            };
        case 'Unknown':
        default:
            return {
                title: '发生未知错误',
                desc: `系统遇到无法识别的错误状态。`,
                icon: 'bi-question-circle-fill',
                bgClass: 'bg-dark',
                textClass: 'text-dark',
                iconBgClass: 'bg-dark bg-opacity-10'
            };
    }
});

async function uploadLoginCookies() {
    try {
        await invoke("login");
        emit('retry');
    } catch (e) {
        emit('error', "登录发生错误", String(e));
    }
}

async function uploadRoomConfig() {
    try {
        await invoke("pick_room");
        emit('retry');
    } catch (e) {
        emit('error', "上传宿舍信息发生错误", String(e));
    }
}
</script>

<style scoped>
.z-modal {
    z-index: 50;
    /* 低于 notification */
}

.x-small {
    font-size: 0.75rem;
}

.hover-lift {
    transition: transform 0.2s, box-shadow 0.2s;
}

.hover-lift:active {
    transform: scale(0.98);
}

/* 模态框淡入淡出 */
.modal-fade-enter-active,
.modal-fade-leave-active {
    transition: opacity 0.3s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
    opacity: 0;
}

/* 卡片弹跳动画 */
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
</style>