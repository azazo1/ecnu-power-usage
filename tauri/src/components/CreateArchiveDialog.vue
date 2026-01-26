<template>
    <Transition name="fade">
        <div v-if="show" class="modal-backdrop show backdrop-blur"></div>
    </Transition>

    <Transition name="zoom">
        <div v-if="show" class="modal d-block" tabindex="-1" @click.self="$emit('close')">
            <div class="modal-dialog modal-dialog-centered">
                <div class="modal-content shadow-2xl border-0 rounded-4 overflow-hidden">
                    <div class="modal-header bg-success text-white py-3 border-0">
                        <h5 class="modal-title d-flex align-items-center gap-2">
                            <i class="bi bi-archive-fill animate-bounce-subtle"></i>
                            创建新归档
                        </h5>
                        <button type="button" class="btn-close btn-close-white" @click="$emit('close')"></button>
                    </div>

                    <div class="modal-body p-4 bg-white">
                        <div class="mb-4">
                            <label
                                class="form-label text-secondary small fw-bold text-uppercase tracking-wider">确认时间范围</label>
                            <div
                                class="p-3 bg-light rounded-3 border border-dashed border-success border-opacity-50 text-dark font-monospace position-relative overflow-hidden">
                                <div class="d-flex align-items-center gap-2 mb-2">
                                    <i class="bi bi-calendar-event text-success"></i>
                                    <span>{{ startTime ? format(startTime, 'yyyy-MM-dd HH:mm:ss') : '最早记录' }}</span>
                                </div>
                                <div class="d-flex align-items-center gap-2">
                                    <i class="bi bi-arrow-down-short text-muted ms-1"></i>
                                    <span class="text-muted small">至</span>
                                </div>
                                <div class="d-flex align-items-center gap-2 mt-2">
                                    <i class="bi bi-calendar-check text-danger"></i>
                                    <span>{{ endTime ? format(endTime, 'yyyy-MM-dd HH:mm:ss') : '最新记录' }}</span>
                                </div>
                                <i class="bi bi-clock-history position-absolute end-0 bottom-0 m-2 text-success opacity-10"
                                    style="font-size: 3rem;"></i>
                            </div>
                        </div>

                        <div class="mb-2">
                            <label for="archiveName"
                                class="form-label text-secondary small fw-bold text-uppercase tracking-wider">归档名称</label>
                            <input type="text"
                                class="form-control form-control-lg rounded-3 border-success border-opacity-25 focus-ring custom-placeholder"
                                id="archiveName" v-model="localName" placeholder="留空则自动生成..."
                                @keyup.enter="handleConfirm" autofocus />
                            <div class="form-text mt-2 d-flex align-items-start gap-2">
                                <i class="bi bi-exclamation-triangle-fill text-warning"></i>
                                <span>提示：归档后的数据将从主历史记录中移除。</span>
                            </div>
                        </div>
                    </div>

                    <div class="modal-footer bg-light border-0 p-3">
                        <button type="button" class="btn btn-link text-secondary text-decoration-none hover-text-dark"
                            @click="$emit('close')">取消</button>
                        <button type="button" class="btn btn-success px-5 rounded-3 fw-bold shadow-sm hover-scale"
                            @click="handleConfirm">
                            确认归档
                        </button>
                    </div>
                </div>
            </div>
        </div>
    </Transition>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { format } from 'date-fns';

const props = defineProps<{
    show: boolean;
    startTime: Date | null;
    endTime: Date | null;
}>();

const emit = defineEmits<{
    close: [];
    confirm: [name: string | null];
}>();

const localName = ref('');

watch(() => props.show, (val) => {
    if (val) localName.value = '';
});

const handleConfirm = () => {
    emit('confirm', localName.value.trim() || null);
};
</script>

<style scoped>
/* 修改 placeholder 的颜色和大小 */
.custom-placeholder::placeholder {
    color: #adb5bd;
    /* Bootstrap 的 secondary 浅色，或者使用 #ccc */
    font-size: 0.85rem;
    /* 比 input 本身的文字小一号 */
    opacity: 0.7;
    /* 增加透明度让它看起来更“淡” */
    font-weight: 400;
    /* 确保不是粗体 */
}

/* 兼容不同浏览器的写法 (可选，现代 Tauri 环境通常只需上面的标准写法) */
.custom-placeholder::-webkit-input-placeholder {
    color: #adb5bd;
    font-size: 0.85rem;
}

/* 焦点时的状态优化 */
.focus-ring:focus::placeholder {
    opacity: 0.4;
    /* 输入时让 placeholder 变得更淡，减少干扰 */
    transition: opacity 0.2s ease;
}

.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}

.backdrop-blur {
    backdrop-filter: blur(4px);
    background-color: rgba(0, 0, 0, 0.4) !important;
}

/* 2. 面板弹入缩放过渡 */
.zoom-enter-active {
    transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.zoom-leave-active {
    transition: all 0.2s ease-in;
}

.zoom-enter-from {
    opacity: 0;
    transform: scale(0.9) translateY(-20px);
}

.zoom-leave-to {
    opacity: 0;
    transform: scale(0.95);
}

/* 3. 微交互动画 */
.animate-bounce-subtle {
    animation: bounce 2s infinite;
}

@keyframes bounce {

    0%,
    100% {
        transform: translateY(0);
    }

    50% {
        transform: translateY(-3px);
    }
}

.shadow-2xl {
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
}

.border-dashed {
    border-style: dashed !important;
}

.focus-ring:focus {
    box-shadow: 0 0 0 0.25rem rgba(25, 135, 84, 0.15);
    border-color: #198754;
    outline: 0;
}

.hover-text-dark:hover {
    color: #212529 !important;
}

.tracking-wider {
    letter-spacing: 0.05em;
}
</style>