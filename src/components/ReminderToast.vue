<template>
  <div class="reminder-toast-stack" aria-live="polite">
    <TransitionGroup name="reminder-toast">
      <article
        v-for="item in items"
        :key="item.id"
        class="reminder-toast"
        :class="`reminder-toast--${item.tone}`"
      >
        <button type="button" class="reminder-toast__content" @click="$emit('open', item)">
          <span class="reminder-toast__icon">{{ item.kind === 'habit' ? '🎯' : item.tone === 'overdue' ? '⏰' : '🔔' }}</span>
          <span class="reminder-toast__copy">
            <strong>{{ item.title }}</strong>
            <span>{{ item.body }}</span>
          </span>
          <span class="reminder-toast__action">{{ item.actionLabel }} ›</span>
        </button>
        <button type="button" class="reminder-toast__close" aria-label="关闭提醒" @click="$emit('dismiss', item.id)">×</button>
      </article>
    </TransitionGroup>
  </div>
</template>

<script setup lang="ts">
import type { InAppReminder } from '../utils/reminder';

defineProps<{
  items: InAppReminder[];
}>();

defineEmits<{
  (event: 'open', item: InAppReminder): void;
  (event: 'dismiss', id: string): void;
}>();
</script>

<style scoped>
.reminder-toast-stack {
  position: absolute;
  top: 82px;
  left: 50%;
  z-index: 60;
  display: grid;
  width: min(320px, calc(100% - 28px));
  gap: 8px;
  pointer-events: none;
  transform: translateX(-50%);
}

.reminder-toast {
  pointer-events: auto;
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--border) 75%, transparent);
  border-radius: 18px;
  background: color-mix(in srgb, var(--bg-solid) 94%, transparent);
  box-shadow: var(--shadow-md);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.reminder-toast::before {
  content: '';
  position: absolute;
  inset: 0 auto 0 0;
  width: 3px;
  background: var(--primary);
}

.reminder-toast--overdue::before {
  background: var(--accent-red);
}

.reminder-toast--habit::before {
  background: var(--accent-green);
}

.reminder-toast__content {
  min-width: 0;
  display: grid;
  grid-template-columns: 34px minmax(0, 1fr) auto;
  align-items: center;
  gap: 9px;
  padding: 10px 8px 10px 13px;
  border: 0;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.reminder-toast__icon {
  display: grid;
  width: 34px;
  height: 34px;
  place-items: center;
  border-radius: 12px;
  background: var(--bg-secondary);
  font-size: 18px;
}

.reminder-toast__copy {
  min-width: 0;
  display: grid;
  gap: 2px;
}

.reminder-toast__copy strong {
  overflow: hidden;
  color: var(--text-primary);
  font-size: var(--font-size-md);
  font-weight: 700;
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.reminder-toast__copy span {
  overflow: hidden;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.reminder-toast__action {
  color: var(--primary);
  font-size: var(--font-size-xs);
  font-weight: 700;
  white-space: nowrap;
}

.reminder-toast__close {
  width: 28px;
  height: 28px;
  margin-right: 7px;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
}

.reminder-toast__close:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.reminder-toast-enter-active,
.reminder-toast-leave-active {
  transition:
    opacity 0.18s ease,
    transform 0.18s ease;
}

.reminder-toast-enter-from,
.reminder-toast-leave-to {
  opacity: 0;
  transform: translateY(-8px) scale(0.98);
}
</style>
