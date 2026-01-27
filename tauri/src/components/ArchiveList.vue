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
                <div v-for="arc in archiveList" :key="arc.name" class="col-md-6 col-lg-4">
                    <div @click="$emit('open', arc.name)"
                        class="card h-100 border-success border-opacity-50 hover-card cursor-pointer rounded-4 shadow-sm">
                        <div class="card-body d-flex flex-column">
                            <div class="d-flex align-items-center gap-3 mb-3">
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
</style>

<script setup lang="ts">
import { ArchiveMeta } from '../utils/archive';

defineProps<{ archiveList: ArchiveMeta[] }>();

defineEmits<{
    // 刷新 archive 列表
    refresh: [],
    // 打开 archive
    open: [string]
}>();


const timeFormatter = new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false // 使用24小时制
});
</script>