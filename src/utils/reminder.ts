import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import type { Task } from '../types';
import type { Habit, HabitLog } from '../types/habit';
import { dueTimestamp } from './dueDate';

let taskReminderTimer: ReturnType<typeof setInterval> | null = null;
let habitReminderTimer: ReturnType<typeof setInterval> | null = null;
const habitNotifiedKeys = new Set<string>();

export interface InAppReminder {
  id: string;
  kind: 'task' | 'habit';
  tone: 'overdue' | 'due' | 'habit';
  title: string;
  body: string;
  actionLabel: string;
  targetId: string;
  createdAt: number;
}

type InAppReminderHandler = (reminder: InAppReminder) => void;

async function ensurePermission(): Promise<boolean> {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === 'granted';
    }
    return granted;
  } catch {
    return false;
  }
}

function normalizeStatus(status: string): 'todo' | 'in_progress' | 'completed' | 'unknown' {
  const value = status.trim();
  if (value.includes('已完成')) return 'completed';
  if (value.includes('进行中')) return 'in_progress';
  if (value.includes('待处理') || value.includes('待办') || value.includes('待启动')) return 'todo';
  return 'unknown';
}

function dateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

function reminderTitle(minutes: number): string {
  if (minutes === 0) return '现在截止';
  if (minutes === 15) return '15分钟后截止';
  if (minutes === 60) return '1小时后截止';
  if (minutes === 1440) return '明天截止';
  return `${minutes}分钟后截止`;
}

export function startReminderService(
  getTasks: () => Task[],
  markNotified: (recordId: string) => Promise<void> | void,
  onInAppReminder?: InAppReminderHandler
) {
  if (taskReminderTimer) clearInterval(taskReminderTimer);
  void ensurePermission();

  const checkReminders = async () => {
    const canSendSystemNotification = await ensurePermission();
    const now = Date.now();
    for (const task of getTasks()) {
      if (normalizeStatus(task.status) === 'completed') continue;
      if (!task.due_date || task.reminder_before === null || task.reminder_before === undefined || task.reminder_notified) continue;
      const due = dueTimestamp(task.due_date);
      if (!due) continue;
      const reminder = due - Number(task.reminder_before) * 60_000;
      const shouldNotify = (now >= reminder && now <= reminder + 120_000) || now > due;
      if (!shouldNotify) continue;
      const title = now > due ? '任务已逾期' : reminderTitle(Number(task.reminder_before));
      const systemTitle = `Topdo · ${now > due ? '已逾期' : reminderTitle(Number(task.reminder_before))}`;
      if (canSendSystemNotification) {
        try {
          await sendNotification({
            title: systemTitle,
            body: task.name
          });
        } catch (error) {
          console.warn('发送任务系统通知失败:', error);
        }
      }
      onInAppReminder?.({
        id: `task:${task.record_id}:${now}`,
        kind: 'task',
        tone: now > due ? 'overdue' : 'due',
        title,
        body: task.name,
        actionLabel: '查看',
        targetId: task.record_id,
        createdAt: now
      });
      await markNotified(task.record_id);
    }
  };

  setTimeout(() => void checkReminders(), 1_000);
  taskReminderTimer = setInterval(() => void checkReminders(), 15_000);
}

function isHabitRequiredDay(date: Date, habit: Habit): boolean {
  const day = date.getDay();
  if (habit.frequency_type === 'daily') return true;
  if (habit.frequency_type === 'weekdays') return day >= 1 && day <= 5;
  return habit.frequency_days?.includes(day) ?? false;
}

export function startHabitReminderService(getHabits: () => Habit[], getLogs: () => HabitLog[], onInAppReminder?: InAppReminderHandler) {
  if (habitReminderTimer) clearInterval(habitReminderTimer);
  void ensurePermission();

  const checkHabitReminders = async () => {
    const canSendSystemNotification = await ensurePermission();
    const now = new Date();
    const currentTime = now.toTimeString().slice(0, 5);
    const today = dateKey(now);
    for (const habit of getHabits()) {
      if (habit.is_archived || !habit.remind_time || habit.remind_time !== currentTime) continue;
      if (!isHabitRequiredDay(now, habit)) continue;
      if (getLogs().some((log) => log.habit_id === habit.id && log.checked_at === today)) continue;
      const key = `${habit.id}:${today}:${currentTime}`;
      if (habitNotifiedKeys.has(key)) continue;
      habitNotifiedKeys.add(key);
      const body = `${habit.emoji} ${habit.name}`;
      if (canSendSystemNotification) {
        try {
          await sendNotification({ title: 'Topdo · 习惯提醒', body });
        } catch (error) {
          console.warn('发送习惯系统通知失败:', error);
        }
      }
      onInAppReminder?.({
        id: `habit:${habit.id}:${today}:${currentTime}`,
        kind: 'habit',
        tone: 'habit',
        title: '该打卡了',
        body,
        actionLabel: '去打卡',
        targetId: habit.id,
        createdAt: Date.now()
      });
    }
  };

  setTimeout(() => void checkHabitReminders(), 1_000);
  habitReminderTimer = setInterval(() => void checkHabitReminders(), 15_000);
}
