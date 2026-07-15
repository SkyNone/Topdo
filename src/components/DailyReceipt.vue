<template>
  <Teleport to="body">
    <Transition name="receipt-fade">
      <div v-if="modelValue" class="receipt-overlay" @click.self="close">
        <section class="receipt-dialog task-scrollbar" role="dialog" aria-modal="true" aria-labelledby="daily-receipt-title">
          <button type="button" class="receipt-close" aria-label="关闭今日小票" @click="close">
            <Icon name="x" :size="16" />
          </button>

          <div class="receipt-paper">
            <p class="receipt-brand">TOPDO</p>
            <h2 id="daily-receipt-title">今日小票</h2>
            <p class="receipt-date">{{ receipt.dateLabel }}</p>

            <div class="receipt-total">
              <span>今天完成</span>
              <strong>{{ receipt.totalCount }}<small>项</small></strong>
            </div>

            <div v-if="receipt.items.length" class="receipt-items">
              <div v-for="item in receipt.items" :key="`${item.name}-${item.time}`" class="receipt-item">
                <span class="receipt-check"><Icon name="check-circle" :size="17" :stroke-width="2" /></span>
                <span class="receipt-item__name">{{ item.name }}</span>
                <time v-if="item.time">{{ item.time }}</time>
              </div>
              <p v-if="receipt.remainingCount" class="receipt-more">另有 {{ receipt.remainingCount }} 项已完成</p>
            </div>
            <div v-else class="receipt-empty">
              <strong>今天还没有完成记录</strong>
              <span>完成一项任务或习惯后，就能生成今日小票。</span>
            </div>

            <div class="receipt-metrics">
              <div><span>重点任务</span><strong>{{ receipt.focusCount }}</strong></div>
              <div><span>习惯打卡</span><strong>{{ receipt.habitChecked }}/{{ receipt.habitTotal }}</strong></div>
              <div><span>连续完成</span><strong>{{ receipt.streak }}<small>天</small></strong></div>
            </div>

            <div v-if="receipt.tags.length" class="receipt-tags">
              <span v-for="tag in receipt.tags" :key="tag">{{ tag }}</span>
            </div>
            <p class="receipt-quote">今天推进的每一步，都算数。</p>
          </div>

          <p v-if="feedback" class="receipt-feedback" role="status">{{ feedback }}</p>
          <div class="receipt-actions">
            <button type="button" class="receipt-primary" :disabled="saving || receipt.totalCount === 0" @click="saveImage">
              <Icon name="file-text" :size="16" />
              {{ saving ? '正在保存...' : '保存为图片' }}
            </button>
            <button type="button" :disabled="receipt.totalCount === 0" @click="copyText">
              <Icon name="task" :size="16" />
              复制文字
            </button>
          </div>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useHabitStore } from '../stores/habitStore';
import { useTaskStore } from '../stores/taskStore';
import { buildDailyReceiptData, buildDailyReceiptText, exportDailyReceiptImage } from '../services/dailyReceiptService';
import Icon from './Icon.vue';

const props = defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ (event: 'update:modelValue', value: boolean): void }>();
const taskStore = useTaskStore();
const habitStore = useHabitStore();
const saving = ref(false);
const feedback = ref('');
let feedbackTimer: ReturnType<typeof setTimeout> | null = null;

const receipt = computed(() => buildDailyReceiptData(
  taskStore.tasks,
  habitStore.habitsWithStats,
  taskStore.completionStreak
));

function close() {
  emit('update:modelValue', false);
}

function showFeedback(message: string) {
  feedback.value = message;
  if (feedbackTimer) clearTimeout(feedbackTimer);
  feedbackTimer = setTimeout(() => { feedback.value = ''; }, 3200);
}

async function saveImage() {
  if (saving.value || receipt.value.totalCount === 0) return;
  saving.value = true;
  try {
    await exportDailyReceiptImage(receipt.value);
    showFeedback('小票已保存到 Topdo 导出目录');
  } catch (error) {
    showFeedback(`保存失败：${String(error)}`);
  } finally {
    saving.value = false;
  }
}

async function copyText() {
  if (receipt.value.totalCount === 0) return;
  const text = buildDailyReceiptText(receipt.value);
  try {
    await copyToClipboard(text);
    showFeedback('今日小票文字已复制');
  } catch (error) {
    showFeedback(`复制失败：${String(error)}`);
  }
}

async function copyToClipboard(text: string) {
  if (navigator.clipboard?.writeText) {
    try {
      await Promise.race([
        navigator.clipboard.writeText(text),
        new Promise<never>((_, reject) => setTimeout(() => reject(new Error('clipboard timeout')), 800))
      ]);
      return;
    } catch {
      // Some WebView contexts expose Clipboard API without granting write access.
    }
  }

  const textarea = document.createElement('textarea');
  textarea.value = text;
  textarea.setAttribute('readonly', '');
  textarea.style.position = 'fixed';
  textarea.style.opacity = '0';
  document.body.appendChild(textarea);
  textarea.select();
  const copied = document.execCommand('copy');
  textarea.remove();
  if (!copied) throw new Error('当前系统不允许写入剪贴板');
}

function onKeydown(event: KeyboardEvent) {
  if (props.modelValue && event.key === 'Escape') close();
}

onMounted(() => window.addEventListener('keydown', onKeydown));
onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown);
  if (feedbackTimer) clearTimeout(feedbackTimer);
});
</script>

<style scoped>
.receipt-overlay {
  position: fixed;
  inset: 0;
  z-index: 90;
  padding: 12px;
  display: grid;
  place-items: center;
  background: color-mix(in srgb, #111827 32%, transparent);
  backdrop-filter: blur(8px);
}

.receipt-dialog {
  --receipt-paper: #fffefa;
  --receipt-ink: #1d1d1f;
  --receipt-muted: #8e8e93;
  --receipt-line: #d8d8dd;
  position: relative;
  width: min(100%, 360px);
  max-height: calc(100vh - 24px);
  overflow-x: hidden;
  overflow-y: auto;
  padding: 18px 10px 10px;
  background: transparent;
}

.receipt-close {
  position: sticky;
  top: 0;
  z-index: 3;
  float: right;
  margin: -10px 1px 5px 0;
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border: 1px solid var(--border);
  border-radius: 50%;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--bg-solid) 88%, transparent);
  box-shadow: 0 4px 14px rgba(15, 23, 42, 0.12);
  backdrop-filter: blur(10px);
  cursor: pointer;
}

.receipt-paper {
  position: relative;
  clear: both;
  margin: 8px 7px 13px;
  padding: 28px 22px 25px;
  color: var(--receipt-ink);
  background-color: var(--receipt-paper);
  background-image: repeating-linear-gradient(
    0deg,
    rgba(90, 78, 58, 0.014) 0,
    rgba(90, 78, 58, 0.014) 1px,
    transparent 1px,
    transparent 4px
  );
  filter: drop-shadow(0 16px 22px rgba(15, 23, 42, 0.16));
}

.receipt-paper::before,
.receipt-paper::after {
  position: absolute;
  left: 0;
  width: 100%;
  height: 12px;
  content: '';
  background: radial-gradient(circle at 6px 6px, var(--receipt-paper) 0 5.7px, transparent 6.2px) 0 0 / 12px 12px repeat-x;
}

.receipt-paper::before {
  top: -6px;
}

.receipt-paper::after {
  bottom: -6px;
}

.receipt-brand {
  margin: 0 0 5px;
  color: var(--primary);
  font-size: 10px;
  font-weight: 750;
  letter-spacing: 0.16em;
  text-align: center;
}

.receipt-paper h2 {
  margin: 0;
  font-size: 27px;
  line-height: 1.2;
  font-weight: 750;
  letter-spacing: 0.02em;
  text-align: center;
}

.receipt-date {
  margin: 10px 0 0;
  padding-bottom: 17px;
  border-bottom: 1px dashed var(--receipt-line);
  color: var(--receipt-muted);
  font-size: 11px;
  text-align: center;
}

.receipt-total {
  padding: 18px 0 13px;
  display: grid;
  justify-items: center;
  gap: 1px;
}

.receipt-total > span {
  color: var(--receipt-muted);
  font-size: 12px;
}

.receipt-total strong {
  color: var(--primary);
  font-size: 62px;
  line-height: 1;
  font-variant-numeric: tabular-nums;
}

.receipt-total small,
.receipt-metrics small {
  margin-left: 3px;
  font-size: 12px;
  font-weight: 650;
}

.receipt-items {
  border-top: 1px solid #ececef;
}

.receipt-item {
  min-height: 44px;
  display: grid;
  grid-template-columns: 20px minmax(0, 1fr) auto;
  align-items: center;
  gap: 8px;
  border-bottom: 1px dashed #dedee2;
  font-size: 12px;
}

.receipt-check {
  display: inline-flex;
  color: var(--primary);
}

.receipt-item__name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.receipt-item time,
.receipt-more {
  color: #aeaeb2;
  font-size: 10px;
  font-variant-numeric: tabular-nums;
}

.receipt-more {
  margin: 9px 0 0 28px;
}

.receipt-empty {
  padding: 20px 10px;
  display: grid;
  gap: 5px;
  text-align: center;
}

.receipt-empty strong {
  font-size: 13px;
}

.receipt-empty span {
  color: var(--receipt-muted);
  font-size: 11px;
}

.receipt-metrics {
  margin-top: 17px;
  padding: 16px 0;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  border-top: 1px dashed var(--receipt-line);
  border-bottom: 1px dashed var(--receipt-line);
}

.receipt-metrics > div {
  min-width: 0;
  display: grid;
  justify-items: center;
  gap: 4px;
  border-right: 1px solid #e6e6e9;
}

.receipt-metrics > div:last-child {
  border-right: 0;
}

.receipt-metrics span {
  color: var(--receipt-muted);
  font-size: 10px;
  white-space: nowrap;
}

.receipt-metrics strong {
  color: var(--primary);
  font-size: 20px;
  line-height: 1;
  font-variant-numeric: tabular-nums;
}

.receipt-tags {
  padding-top: 12px;
  display: flex;
  justify-content: center;
  flex-wrap: wrap;
  gap: 6px;
}

.receipt-tags span {
  padding: 4px 9px;
  border-radius: 999px;
  color: var(--primary);
  background: #edf5ff;
  font-size: 10px;
}

.receipt-quote {
  margin: 18px 0 0;
  color: #55555a;
  font-size: 11px;
  font-weight: 500;
  text-align: center;
}

.receipt-feedback {
  margin: 8px 0 0;
  color: color-mix(in srgb, var(--text-primary) 84%, transparent);
  font-size: 10px;
  text-align: center;
}

.receipt-actions {
  margin: 10px 7px 0;
  display: grid;
  grid-template-columns: 1.2fr 1fr;
  gap: 8px;
}

.receipt-actions button {
  min-height: 42px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: 1px solid var(--primary);
  border-radius: 10px;
  color: var(--primary);
  background: var(--bg-solid);
  font-family: var(--font-family);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.receipt-actions .receipt-primary {
  color: white;
  background: var(--primary);
}

.receipt-actions button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.receipt-fade-enter-active,
.receipt-fade-leave-active {
  transition: opacity 0.16s ease;
}

.receipt-fade-enter-active .receipt-dialog,
.receipt-fade-leave-active .receipt-dialog {
  transition: transform 0.2s cubic-bezier(0.34, 1.2, 0.64, 1), opacity 0.16s ease;
}

.receipt-fade-enter-from,
.receipt-fade-leave-to {
  opacity: 0;
}

.receipt-fade-enter-from .receipt-dialog,
.receipt-fade-leave-to .receipt-dialog {
  opacity: 0;
  transform: translateY(8px) scale(0.98);
}

@media (max-height: 600px) {
  .receipt-overlay { align-items: start; }
  .receipt-dialog { max-height: calc(100vh - 16px); padding: 15px 8px 8px; }
  .receipt-paper { padding: 22px 18px 20px; }
  .receipt-paper h2 { font-size: 23px; }
  .receipt-total { padding: 12px 0 9px; }
  .receipt-total strong { font-size: 48px; }
  .receipt-item { min-height: 37px; }
}
</style>
