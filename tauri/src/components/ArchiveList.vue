<template>
    <div
        class="h-100 bg-white rounded-4 shadow-lg border border-success border-opacity-25 d-flex flex-column overflow-hidden">
        <div
            class="p-4 border-bottom border-success border-opacity-25 d-flex justify-content-between align-items-center bg-light bg-opacity-50">
            <h2 class="h4 mb-0 fw-bold text-success d-flex align-items-center gap-2">
                <i class="bi bi-archive-fill"></i>
                归档列表
            </h2>
            <button @click="$emit('refresh')"
                class="btn btn-outline-success rounded-3 d-flex align-items-center gap-2 hover-scale" title="刷新">
                <i class="bi bi-arrow-clockwise"></i>
            </button>
        </div>
        <div class="p-4 overflow-auto flex-grow-1">
            <div class="row g-4">
                <!-- 点击空白处取消确认删除对话框 -->
                <div v-if="deleteTarget" class="position-fixed top-0 start-0 w-100 h-100" style="z-index: 1040;"
                    @click.stop="deleteTarget = null"> </div>

                <div v-for="arc in archiveList" :key="arc.name" class="col-md-6 col-lg-4">
                    <div @click="$emit('open', arc.name)"
                        class="card h-100 border-success border-opacity-50 hover-card cursor-pointer rounded-4 shadow-sm"
                        :class="{ 'hover-card-disabled': deleteTarget }">
                        <!-- 删除归档按钮 -->
                        <div class="position-absolute top-0 end-0 p-3" style="z-index: 1050;">
                            <button @click.stop="deleteTarget = arc.name"
                                class="btn btn-sm btn-light text-success border-0 rounded-circle shadow-sm hover-scale d-flex align-items-center justify-content-center"
                                style="width: 32px; height: 32px;" title="删除归档">
                                <i class="bi bi-trash"></i>
                            </button>

                            <!-- 确认删除对话框 -->
                            <Transition name="pop">
                                <div v-if="deleteTarget === arc.name" @click.stop
                                    class="position-absolute top-100 end-0 mt-2 bg-white rounded-3 shadow border border-success border-opacity-25 p-3 text-center"
                                    style="width: 160px; z-index: 1060;">
                                    <div class="position-absolute bottom-100 end-0 translate-middle-x me-3"
                                        style="width: 0; height: 0; border-left: 6px solid transparent; border-right: 6px solid transparent; border-bottom: 6px solid #fff; filter: drop-shadow(0 -2px 1px rgba(0,0,0,0.05));">
                                    </div>
                                    <p class="small text-muted mb-2 fw-bold">确认删除此归档?</p>
                                    <div class="d-flex justify-content-between gap-2">
                                        <button @click="deleteTarget = null"
                                            class="btn btn-xs btn-outline-secondary w-50 small"
                                            style="font-size: 0.75rem;">取消</button>
                                        <button @click="handleDelete(arc.name)"
                                            class="btn btn-xs btn-danger text-white w-50 small shadow-sm"
                                            style="font-size:0.75rem">删除</button>
                                    </div>
                                </div>
                            </Transition>
                        </div>
                        <div class="card-body d-flex flex-column">
                            <div class="d-flex align-items-start gap-3 mb-3">
                                <div class="bg-success bg-gradient text-white rounded-3 p-3 shadow-sm hover-icon">
                                    <i class="bi bi-box-seam fs-3"></i>
                                </div>
                                <div class="flex-grow-1">
                                    <h5 class="card-title fw-bold mb-1 text-dark">{{ arc.name }}
                                    </h5>
                                    <p class="card-text small text-success mb-0">{{
                                        timeFormatter.format(arc.startTime) }}
                                        - {{ timeFormatter.format(arc.endTime) }}</p>
                                    <span
                                        class="badge bg-success bg-opacity-10 text-success border border-success border-opacity-25 rounded-pill px-3">
                                        <i class="bi bi-layers-half me-1"></i>
                                        {{ arc.recordsNum }} 条记录
                                    </span>
                                </div>
                            </div>
                            <div class="position-absolute bottom-0 end-0 m-3 text-success fw-semibold small">
                                <span class="me-1">查看详情</span>
                                <i class="bi bi-arrow-right"></i>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<style lang="css" scoped>
.hover-card {
    transition: all 0.3s ease;
}

.hover-card:not(.hover-card-disabled):hover {
    transform: translateY(-4px);
    box-shadow: 0 8px 24px rgba(67, 160, 71, 0.2) !important;
    border-color: #43a047 !important;
}

.hover-icon {
    transition: transform 0.3s ease;
}

.hover-card:not(.hover-card-disabled):hover .hover-icon {
    transform: rotate(5deg) scale(1.1);
}

.cursor-pointer {
    cursor: pointer;
}

/* 定义 Transition 动画 */
.pop-enter-active,
.pop-leave-active {
    transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
    /* 带一点弹性效果 */
}

.pop-enter-from,
.pop-leave-to {
    opacity: 0;
    transform: translateY(-10px) scale(0.95);
    /* 从上方微调位置并缩小 */
}

/* 确保遮罩层不遮挡鼠标手势 */
.position-fixed {
    cursor: default;
}

/* 辅助按钮样式 */
.btn-xs {
    padding: 0.25rem 0.4rem;
    line-height: 1.2;
    border-radius: 0.2rem;
}

/* 让删除按钮在 hover 时显红，或者保持绿色风格均可，这里为了警示效果，hover 变红 */
.btn-light:hover .bi-trash {
    color: #dc3545 !important;
    /* Bootstrap danger color */
    transition: color 0.2s;
}
</style>

<script setup lang="ts">
import { ref } from 'vue';
import { ArchiveMeta } from '../utils/archive';

defineProps<{ archiveList: ArchiveMeta[] }>();

const emit = defineEmits<{
    // 刷新 archive 列表
    refresh: [],
    // 打开 archive
    open: [string]
    // 删除 archive
    delete: [string]
}>();

const timeFormatter = new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false // 使用24小时制
});

// 记录当前哪个卡片正在显示删除确认框
const deleteTarget = ref<string | null>(null);

const handleDelete = (name: string) => {
    emit('delete', name);
    deleteTarget.value = null; // 删除后关闭弹窗
};
</script>