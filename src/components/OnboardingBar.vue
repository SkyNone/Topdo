<template>
  <section class="mx-3 mt-2 rounded-[10px] border border-[color:var(--border)] bg-[color:var(--bg-solid)] px-3 py-2">
    <div class="mb-2 flex items-center justify-between">
      <h3 class="text-xs font-semibold text-[color:var(--text-primary)]">新手引导（3 步）</h3>
      <button
        type="button"
        class="rounded-[6px] px-2 py-0.5 text-[11px] text-[color:var(--text-secondary)] hover:bg-[color:var(--bg-hover)] hover:text-[color:var(--text-primary)]"
        @click="$emit('dismiss')"
      >
        跳过
      </button>
    </div>

    <div class="grid grid-cols-3 gap-2">
      <div class="rounded-[8px] border px-2 py-1.5 text-[11px]" :class="steps.createDone ? doneClass : waitingClass">
        <p class="font-medium">① ⌘N 新建 / ⌘K 快捷键</p>
        <p class="mt-0.5 opacity-80">创建 1 条任务</p>
      </div>
      <div class="rounded-[8px] border px-2 py-1.5 text-[11px]" :class="steps.progressDone ? doneClass : waitingClass">
        <p class="font-medium">② 推进到进行中</p>
        <p class="mt-0.5 opacity-80">点击状态圆圈</p>
      </div>
      <div class="rounded-[8px] border px-2 py-1.5 text-[11px]" :class="steps.doneDone ? doneClass : waitingClass">
        <p class="font-medium">③ 完成并沉底</p>
        <p class="mt-0.5 opacity-80">出现已完成任务</p>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
defineProps<{
  steps: {
    createDone: boolean;
    progressDone: boolean;
    doneDone: boolean;
  };
}>();

defineEmits<{
  (event: 'dismiss'): void;
}>();

const doneClass = 'border-[color:var(--status-done)] bg-[color:var(--primary-light)] text-[color:var(--status-done)]';
const waitingClass = 'border-[color:var(--border)] bg-[color:var(--bg-secondary)] text-[color:var(--text-secondary)]';
</script>
