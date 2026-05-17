<template>
  <div
    ref="panelRef"
    class="quick-input"
    @click.stop
    @mousedown.stop
    @dblclick.stop
    @keydown.stop
  >
    <div class="create-toolbar">
      <button type="button" class="toolbar-btn toolbar-btn--muted" @click="handleCancel">取消</button>
      <strong>新建任务</strong>
      <button type="button" class="toolbar-btn toolbar-btn--primary" :disabled="!taskName.trim() || submitting" @click="handleCreate">
        {{ submitting ? '创建中...' : '创建' }}
      </button>
    </div>

    <div class="quick-input__body task-scrollbar">
      <div class="task-input-area">
        <input
          ref="inputRef"
          v-model="taskName"
          class="task-input"
          placeholder="输入任务名称..."
          @keydown.meta.s.prevent="handleCreate"
          @keydown.enter.prevent="handleCreate"
          @keydown.esc.prevent="handleCancel"
        />
        <div class="input-divider" aria-hidden="true"></div>
      </div>

      <div class="accordion-panel">
        <section v-if="expandedOption !== 'priority'" class="opt-row" @click="toggleOption('priority')">
          <div class="opt-icon opt-icon--blue"><Icon name="priority" :size="13" /></div>
          <span class="opt-title">优先级</span>
          <span class="opt-value"><span class="value-dot" :style="{ background: prioritySummary.dot }"></span>{{ prioritySummary.label }}</span>
          <span class="opt-arrow">›</span>
        </section>
        <section v-else class="opt-expanded">
          <div class="opt-exp-header">
            <div class="opt-icon opt-icon--blue"><Icon name="priority" :size="13" /></div>
            <span class="opt-exp-title">优先级</span>
            <button type="button" class="opt-collapse" @click="expandedOption = null">收起 ‹</button>
          </div>
          <div class="opt-chips">
            <button
              v-for="option in priorityOptions"
              :key="option.value"
              type="button"
              class="chip"
              :class="[`chip--${option.tone}`, { selected: selectedPriority === option.value }]"
              @click="selectPriority(option.value)"
            >
              <span class="chip-dot" :style="{ background: option.dot }"></span>{{ option.label }}
            </button>
          </div>
        </section>

        <section v-if="expandedOption !== 'date'" class="opt-row" @click="toggleOption('date')">
          <div class="opt-icon opt-icon--green"><Icon name="calendar" :size="13" /></div>
          <span class="opt-title">日期</span>
          <span class="opt-value">{{ dueDateSummary }}</span>
          <span class="opt-arrow">›</span>
        </section>
        <section v-else class="opt-expanded">
          <div class="opt-exp-header">
            <div class="opt-icon opt-icon--green"><Icon name="calendar" :size="13" /></div>
            <span class="opt-exp-title">日期</span>
            <button type="button" class="opt-collapse" @click="expandedOption = null">收起 ‹</button>
          </div>
          <div class="opt-chips">
            <button
              v-for="option in dateOptions"
              :key="String(option.value)"
              type="button"
              class="chip"
              :class="{ selected: selectedDateOption === option.value }"
              @click="onDateOptionUpdate(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
          <div class="date-time-row">
            <span>时间</span>
            <input v-model="selectedDueTime" type="time" :disabled="!selectedDueDate" />
            <small>{{ selectedDueDate ? '默认 23:59' : '先选日期' }}</small>
          </div>
          <div class="calendar-box">
            <div class="calendar-head">
              <button type="button" @click="shiftCalendarMonth(-1)">‹</button>
              <strong>{{ calendarTitle }}</strong>
              <button type="button" @click="shiftCalendarMonth(1)">›</button>
            </div>
            <div class="calendar-grid">
              <span v-for="day in weekLabels" :key="day" class="calendar-week">{{ day }}</span>
              <button
                v-for="cell in calendarCells"
                :key="cell.key"
                type="button"
                class="calendar-day"
                :class="{ muted: !cell.inMonth, today: cell.date === todayKey(), selected: cell.date === selectedDueDate }"
                @click="selectCalendarDate(cell.date)"
              >
                {{ cell.day }}
              </button>
            </div>
          </div>
        </section>

        <section v-if="expandedOption !== 'repeat'" class="opt-row" @click="toggleOption('repeat')">
          <div class="opt-icon opt-icon--purple"><Icon name="recurring" :size="13" /></div>
          <span class="opt-title">重复</span>
          <span class="opt-value">{{ repeatSummary }}</span>
          <span class="opt-arrow">›</span>
        </section>
        <section v-else class="opt-expanded">
          <div class="opt-exp-header">
            <div class="opt-icon opt-icon--purple"><Icon name="recurring" :size="13" /></div>
            <span class="opt-exp-title">重复</span>
            <button type="button" class="opt-collapse" @click="expandedOption = null">收起 ‹</button>
          </div>
          <div class="opt-chips">
            <button
              v-for="option in repeatOptions"
              :key="String(option.value)"
              type="button"
              class="chip"
              :class="{ selected: repeatValue === option.value }"
              @click="selectRepeat(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
          <div v-if="repeatValue === 'weekly' || repeatValue === 'biweekly'" class="weekday-picker">
            <button
              v-for="day in weekdayOptions"
              :key="day.value"
              type="button"
              :class="{ selected: repeatDays.includes(day.value) }"
              @click="toggleRepeatDay(day.value)"
            >
              {{ day.label }}
            </button>
          </div>
        </section>

        <section v-if="expandedOption !== 'reminder'" class="opt-row" @click="toggleOption('reminder')">
          <div class="opt-icon opt-icon--orange"><Icon name="bell" :size="13" /></div>
          <span class="opt-title">提醒</span>
          <span class="opt-value">{{ reminderSummary }}</span>
          <span class="opt-arrow">›</span>
        </section>
        <section v-else class="opt-expanded">
          <div class="opt-exp-header">
            <div class="opt-icon opt-icon--orange"><Icon name="bell" :size="13" /></div>
            <span class="opt-exp-title">提醒</span>
            <button type="button" class="opt-collapse" @click="expandedOption = null">收起 ‹</button>
          </div>
          <div class="opt-chips">
            <button
              v-for="option in reminderOptions"
              :key="String(option.value)"
              type="button"
              class="chip"
              :class="{ selected: selectedReminderBefore === option.value }"
              @click="selectReminder(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
        </section>
      </div>

      <section class="sub-section">
        <div class="section-head">
          <div class="opt-icon opt-icon--gray"><Icon name="list" :size="13" /></div>
          <span>子任务</span>
        </div>
        <ul v-if="subTasks.length" class="sub-list">
          <li v-for="item in subTasks" :key="item.id" class="sub-item">
            <span class="sub-circle" aria-hidden="true"></span>
            <input v-model="item.text" class="sub-input" placeholder="子任务名称" @keydown.enter.prevent="addSubTask" />
            <button type="button" class="sub-remove" title="删除子任务" @click="removeSubTask(item.id)">×</button>
          </li>
        </ul>
        <button type="button" class="sub-add" @click="addSubTask">+ 添加子任务</button>
      </section>

      <section class="note-section">
        <div class="section-head">
          <div class="opt-icon opt-icon--gray"><Icon name="file-text" :size="13" /></div>
          <span>备注</span>
        </div>
        <textarea v-model="note" class="note-textarea" placeholder="添加备注..." rows="2"></textarea>
      </section>
    </div>

  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import { useTaskStore } from '../stores/taskStore';
import type { RecurrenceRule, SubTask } from '../types';
import { buildDueDateValue } from '../utils/dueDate';
import Icon from './Icon.vue';

const emit = defineEmits<{
  (event: 'close'): void;
  (event: 'created'): void;
  (event: 'error', message: string): void;
}>();

type ExpandedOption = 'priority' | 'date' | 'repeat' | 'reminder' | null;
type RepeatValue = 'none' | 'daily' | 'weekly' | 'biweekly' | 'monthly' | 'yearly';

const taskStore = useTaskStore();

const panelRef = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLInputElement | null>(null);
const taskName = ref('');
const selectedPriority = ref('普通');
const selectedDateOption = ref<string | number | null>(null);
const selectedDueDate = ref('');
const selectedDueTime = ref('');
const selectedRecurrenceRule = ref<RecurrenceRule | null>(null);
const selectedReminderBefore = ref<number | null>(null);
const subTasks = ref<SubTask[]>([]);
const note = ref('');
const submitting = ref(false);
const expandedOption = ref<ExpandedOption>(null);
const calendarMonth = ref(new Date(new Date().getFullYear(), new Date().getMonth(), 1));

const priorityOptions = [
  { value: '普通', label: '普通', dot: '#8E8E93', tone: 'normal' },
  { value: '重要', label: '重要', dot: '#007AFF', tone: 'important' },
  { value: '紧急', label: '紧急', dot: '#FF3B30', tone: 'urgent' }
];

const dateOptions = [
  { value: todayKey(), label: '今天' },
  { value: offsetDateKey(1), label: '明天' },
  { value: thisFridayKey(), label: '周五' },
  { value: nextMondayKey(), label: '下周一' },
  { value: null, label: '无日期' }
];

const repeatOptions: Array<{ value: RepeatValue; label: string }> = [
  { value: 'none', label: '不重复' },
  { value: 'daily', label: '每天' },
  { value: 'weekly', label: '每周' },
  { value: 'biweekly', label: '每两周' },
  { value: 'monthly', label: '每月' },
  { value: 'yearly', label: '每年' }
];

const reminderOptions = [
  { value: null, label: '无' },
  { value: 0, label: '到期时' },
  { value: 30, label: '提前30分钟' },
  { value: 60, label: '提前1小时' },
  { value: 1440, label: '提前1天' }
];

const weekLabels = ['日', '一', '二', '三', '四', '五', '六'];
const weekdayOptions = weekLabels.map((label, value) => ({ label, value }));

const prioritySummary = computed(() => priorityOptions.find((item) => item.value === selectedPriority.value) || priorityOptions[0]);
const dueDateSummary = computed(() => {
  if (!selectedDueDate.value) return '无日期';
  const dateLabel = formatDueDateSummary(selectedDueDate.value);
  return selectedDueTime.value ? `${dateLabel} ${selectedDueTime.value}` : dateLabel;
});
const repeatValue = computed<RepeatValue>(() => {
  const rule = selectedRecurrenceRule.value;
  if (!rule) return 'none';
  if (rule.type === 'daily') return 'daily';
  if (rule.type === 'weekly' && rule.interval === 2) return 'biweekly';
  if (rule.type === 'weekly') return 'weekly';
  if (rule.type === 'monthly' && rule.interval === 12) return 'yearly';
  if (rule.type === 'monthly') return 'monthly';
  return 'weekly';
});
const repeatSummary = computed(() => repeatOptions.find((item) => item.value === repeatValue.value)?.label || '不重复');
const repeatDays = computed(() => {
  const days = selectedRecurrenceRule.value?.daysOfWeek || [];
  return days.length ? days : [new Date().getDay()];
});
const reminderSummary = computed(() => reminderOptions.find((item) => item.value === selectedReminderBefore.value)?.label || '无');
const calendarTitle = computed(() => `${calendarMonth.value.getFullYear()}年 ${calendarMonth.value.getMonth() + 1}月`);
const calendarCells = computed(() => {
  const first = calendarMonth.value;
  const start = new Date(first.getFullYear(), first.getMonth(), 1 - first.getDay());
  return Array.from({ length: 42 }, (_, index) => {
    const date = new Date(start.getFullYear(), start.getMonth(), start.getDate() + index);
    return {
      key: dateKey(date),
      date: dateKey(date),
      day: date.getDate(),
      inMonth: date.getMonth() === first.getMonth()
    };
  });
});

function dateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

function todayKey(): string {
  return dateKey(new Date());
}

function offsetDateKey(days: number): string {
  const date = new Date();
  date.setDate(date.getDate() + days);
  return dateKey(date);
}

function thisFridayKey(): string {
  const date = new Date();
  const day = date.getDay();
  const distance = (5 - day + 7) % 7;
  date.setDate(date.getDate() + distance);
  return dateKey(date);
}

function nextMondayKey(): string {
  const date = new Date();
  const day = date.getDay() || 7;
  date.setDate(date.getDate() + (8 - day));
  return dateKey(date);
}

function formatDueDateSummary(value: string): string {
  if (value === todayKey()) return '今天';
  if (value === offsetDateKey(1)) return '明天';
  const date = new Date(`${value}T00:00:00`);
  if (Number.isNaN(date.getTime())) return value;
  return `${date.getMonth() + 1}月${date.getDate()}日`;
}

function toggleOption(option: Exclude<ExpandedOption, null>) {
  expandedOption.value = expandedOption.value === option ? null : option;
}

function selectPriority(value: string) {
  selectedPriority.value = value;
  expandedOption.value = null;
}

function onDateOptionUpdate(value: string | number | null) {
  if (typeof value === 'string') {
    selectedDateOption.value = value;
    selectedDueDate.value = value;
    selectedDueTime.value = selectedDueTime.value || '23:59';
    calendarMonth.value = new Date(`${value}T00:00:00`);
    return;
  }
  selectedDateOption.value = null;
  selectedDueDate.value = '';
  selectedDueTime.value = '';
}

function selectCalendarDate(value: string) {
  selectedDateOption.value = value;
  selectedDueDate.value = value;
  selectedDueTime.value = selectedDueTime.value || '23:59';
  expandedOption.value = null;
}

function shiftCalendarMonth(offset: number) {
  calendarMonth.value = new Date(calendarMonth.value.getFullYear(), calendarMonth.value.getMonth() + offset, 1);
}

function selectRepeat(value: RepeatValue) {
  if (value === 'none') selectedRecurrenceRule.value = null;
  if (value === 'daily') selectedRecurrenceRule.value = { type: 'daily', interval: 1 };
  if (value === 'weekly') selectedRecurrenceRule.value = { type: 'weekly', interval: 1, daysOfWeek: [new Date().getDay()] };
  if (value === 'biweekly') selectedRecurrenceRule.value = { type: 'weekly', interval: 2, daysOfWeek: [new Date().getDay()] };
  if (value === 'monthly') selectedRecurrenceRule.value = { type: 'monthly', interval: 1, dayOfMonth: new Date().getDate() };
  if (value === 'yearly') selectedRecurrenceRule.value = { type: 'monthly', interval: 12, dayOfMonth: new Date().getDate() };
  if (value !== 'weekly' && value !== 'biweekly') {
    expandedOption.value = null;
  }
}

function toggleRepeatDay(day: number) {
  const rule = selectedRecurrenceRule.value;
  if (!rule || rule.type !== 'weekly') return;
  const selected = new Set(repeatDays.value);
  if (selected.has(day)) {
    if (selected.size === 1) return;
    selected.delete(day);
  } else {
    selected.add(day);
  }
  selectedRecurrenceRule.value = {
    ...rule,
    daysOfWeek: Array.from(selected).sort()
  };
}

function selectReminder(value: number | null) {
  selectedReminderBefore.value = value;
  expandedOption.value = null;
}

function reset() {
  taskName.value = '';
  selectedPriority.value = '普通';
  selectedDateOption.value = null;
  selectedDueDate.value = '';
  selectedDueTime.value = '';
  selectedRecurrenceRule.value = null;
  selectedReminderBefore.value = null;
  subTasks.value = [];
  note.value = '';
  expandedOption.value = null;
}

function newSubTask(text = ''): SubTask {
  return {
    id: `${Date.now()}-${Math.random().toString(16).slice(2)}`,
    text,
    done: false,
    created_at: Math.floor(Date.now() / 1000).toString()
  };
}

function addSubTask() {
  const last = subTasks.value[subTasks.value.length - 1];
  if (last && !last.text.trim()) return;
  subTasks.value.push(newSubTask());
  nextTick(() => {
    const inputs = panelRef.value?.querySelectorAll<HTMLInputElement>('.sub-input');
    inputs?.[inputs.length - 1]?.focus();
  });
}

function removeSubTask(id: string) {
  subTasks.value = subTasks.value.filter((item) => item.id !== id);
}

async function handleCreate() {
  const name = taskName.value.trim();
  if (!name || submitting.value) return;

  const payload = {
    name,
    priority: selectedPriority.value,
    status: '待处理',
    due_date: buildDueDateValue(selectedDueDate.value, selectedDueTime.value),
    notes: note.value.trim(),
    sub_tasks: subTasks.value.filter((item) => item.text.trim()).map((item) => ({ ...item, text: item.text.trim() })),
    recurrence_rule: selectedRecurrenceRule.value,
    reminder_before: selectedReminderBefore.value
  };

  submitting.value = true;

  try {
    await taskStore.createTask(payload);
    reset();
    emit('created');
    emit('close');
  } catch (error) {
    emit('error', `创建任务失败：${String(error)}`);
  } finally {
    submitting.value = false;
  }
}

function handleCancel() {
  reset();
  emit('close');
}

function onGlobalMouseDown(event: MouseEvent) {
  const target = event.target as Node | null;
  if (!target || !panelRef.value) return;
  if (!panelRef.value.contains(target)) {
    handleCancel();
  }
}

onMounted(() => {
  nextTick(() => inputRef.value?.focus());
  document.addEventListener('mousedown', onGlobalMouseDown);
});

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onGlobalMouseDown);
});
</script>

<style scoped>
.quick-input {
  margin: 8px 12px 10px;
  max-height: min(520px, calc(100vh - 126px));
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: #fff;
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 12px;
  box-shadow: 0 6px 28px rgba(0, 0, 0, 0.09);
}

.create-toolbar {
  height: 42px;
  padding: 0 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
  border-bottom: 1px solid #f0f0f0;
}

.create-toolbar strong {
  font-size: 14px;
  font-weight: 650;
  color: #1d1d1f;
}

.toolbar-btn {
  border: 0;
  background: transparent;
  font-family: var(--font-family);
  font-size: 13px;
  cursor: pointer;
}

.toolbar-btn--muted { color: #86868b; }
.toolbar-btn--primary { color: #007aff; font-weight: 600; }
.toolbar-btn:disabled { color: #c7c7cc; cursor: not-allowed; }

.quick-input__body {
  min-height: 0;
  flex: 1;
  overflow-y: auto;
  padding: 14px 16px 10px;
}

.task-input-area {
  margin-bottom: 12px;
}

.task-input {
  width: 100%;
  padding: 4px 0;
  border: 0;
  outline: none;
  background: transparent;
  color: #1d1d1f;
  font-family: var(--font-family);
  font-size: 15px;
  line-height: 1.35;
}

.task-input::placeholder {
  color: #c0c0c0;
}

.input-divider {
  height: 2px;
  margin-top: 4px;
  border-radius: 1px;
  background: #007aff;
}

.opt-row,
.opt-expanded {
  border-bottom: 1px solid #f5f5f5;
}

.opt-row {
  min-height: 44px;
  padding: 9px 0;
  display: flex;
  align-items: center;
  cursor: pointer;
}

.opt-expanded {
  padding: 10px 0;
  animation: option-open 0.18s ease;
}

@keyframes option-open {
  from { opacity: 0; transform: translateY(-3px); }
  to { opacity: 1; transform: translateY(0); }
}

.opt-icon {
  width: 26px;
  height: 26px;
  margin-right: 10px;
  display: grid;
  place-items: center;
  flex: 0 0 auto;
  border-radius: 13px;
}

.opt-icon--blue { color: #007aff; background: rgba(0, 122, 255, 0.08); }
.opt-icon--green { color: #34c759; background: rgba(52, 199, 89, 0.08); }
.opt-icon--purple { color: #af52de; background: rgba(175, 82, 222, 0.08); }
.opt-icon--orange { color: #ff9500; background: rgba(255, 149, 0, 0.08); }
.opt-icon--gray { color: #8e8e93; background: rgba(142, 142, 147, 0.06); }

.opt-title {
  flex: 1;
  color: #1d1d1f;
  font-size: 13px;
}

.opt-value {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: #86868b;
  font-size: 12px;
  white-space: nowrap;
}

.value-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.opt-arrow {
  margin-left: 4px;
  color: #c0c0c0;
  font-size: 12px;
}

.opt-exp-header {
  margin-bottom: 8px;
  display: flex;
  align-items: center;
}

.opt-exp-title {
  flex: 1;
  color: #1d1d1f;
  font-size: 13px;
  font-weight: 600;
}

.opt-collapse {
  border: 0;
  background: transparent;
  color: #86868b;
  font-family: var(--font-family);
  font-size: 11px;
  cursor: pointer;
}

.opt-chips {
  padding-left: 36px;
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.weekday-picker {
  margin: 8px 0 0 36px;
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.weekday-picker button {
  width: 25px;
  height: 25px;
  border: 1px solid #eee;
  border-radius: 50%;
  background: #f5f5f7;
  color: #86868b;
  font-family: var(--font-family);
  font-size: 10px;
  font-weight: 600;
  cursor: pointer;
}

.weekday-picker button.selected {
  color: #7c3aed;
  border-color: #a855f7;
  background: rgba(168, 85, 247, 0.08);
}

.chip {
  padding: 4px 9px;
  display: inline-flex;
  align-items: center;
  gap: 3px;
  border: 1px solid #eee;
  border-radius: 12px;
  background: #f5f5f7;
  color: #555;
  font-family: var(--font-family);
  font-size: 11px;
  line-height: 1;
  white-space: nowrap;
  cursor: pointer;
  transition: all 0.12s ease;
}

.chip:hover { border-color: #d0d0d0; }
.chip.selected { border-color: #007aff; background: rgba(0, 122, 255, 0.05); color: #007aff; font-weight: 600; }
.chip--urgent.selected { border-color: #ff3b30; color: #ff3b30; background: rgba(255, 59, 48, 0.04); }
.chip--important.selected { border-color: #007aff; color: #007aff; background: rgba(0, 122, 255, 0.04); }
.chip--normal.selected { border-color: #8e8e93; color: #636366; background: rgba(142, 142, 147, 0.05); }

.chip-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
}

.date-time-row {
  margin: 8px 0 0 36px;
  display: flex;
  align-items: center;
  gap: 6px;
  color: #86868b;
  font-size: 10px;
}

.date-time-row input {
  height: 24px;
  width: 86px;
  border: 1px solid #eee;
  border-radius: 6px;
  background: #f9f9fb;
  color: #3d3d3d;
  font-size: 11px;
}

.date-time-row input:disabled {
  opacity: 0.4;
}

.date-time-row small {
  color: #c0c0c0;
}

.calendar-box {
  margin: 7px 0 0 36px;
  padding: 8px 10px;
  border-radius: 7px;
  background: #f9f9fb;
}

.calendar-head {
  margin-bottom: 5px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.calendar-head button {
  border: 0;
  background: transparent;
  color: #86868b;
  font-size: 13px;
  cursor: pointer;
}

.calendar-head strong {
  color: #3d3d3d;
  font-size: 10px;
  font-weight: 600;
}

.calendar-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 1px;
  text-align: center;
}

.calendar-week {
  padding: 2px 0;
  color: #86868b;
  font-size: 8px;
}

.calendar-day {
  padding: 3px 0;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: #3d3d3d;
  font-family: var(--font-family);
  font-size: 9px;
  cursor: pointer;
}

.calendar-day:hover { background: #eee; }
.calendar-day.muted { color: #d0d0d0; }
.calendar-day.today { color: #007aff; font-weight: 700; }
.calendar-day.selected { background: #007aff; color: #fff; font-weight: 700; }

.sub-section,
.note-section {
  padding: 8px 0 6px;
  border-top: 1px solid #f5f5f5;
}

.section-head {
  margin-bottom: 6px;
  display: flex;
  align-items: center;
}

.section-head span {
  color: #1d1d1f;
  font-size: 13px;
  font-weight: 600;
}

.sub-list {
  margin: 0;
  padding: 0 0 0 36px;
  display: grid;
  gap: 4px;
  list-style: none;
}

.sub-item {
  padding: 4px 7px;
  display: flex;
  align-items: center;
  gap: 6px;
  border-radius: 5px;
  background: #f9f9fb;
}

.sub-circle {
  width: 14px;
  height: 14px;
  flex: 0 0 auto;
  border: 1.5px solid #ccc;
  border-radius: 50%;
}

.sub-input {
  min-width: 0;
  flex: 1;
  border: 0;
  outline: none;
  background: transparent;
  color: #3d3d3d;
  font-family: var(--font-family);
  font-size: 11px;
}

.sub-remove {
  border: 0;
  background: transparent;
  color: #c7c7cc;
  cursor: pointer;
}

.sub-add {
  margin-top: 4px;
  padding-left: 36px;
  border: 0;
  background: transparent;
  color: #86868b;
  font-family: var(--font-family);
  font-size: 10px;
  cursor: pointer;
}

.note-textarea {
  width: calc(100% - 36px);
  height: 34px;
  margin-left: 36px;
  padding: 6px 9px;
  border: 1px solid #f0f0f0;
  border-radius: 5px;
  background: #f9f9fb;
  color: #3d3d3d;
  font-family: var(--font-family);
  font-size: 11px;
  line-height: 1.4;
  resize: vertical;
  outline: none;
}

.note-textarea:focus {
  border-color: #007aff;
}

</style>
