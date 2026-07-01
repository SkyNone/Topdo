import type { RecurrenceRule, Task } from '../types';
import { buildDueDateValue, splitDueDate } from './dueDate';

export interface RecurringInstanceInput {
  template: Task;
  name: string;
  priority: string;
  status: string;
  due_date: string;
  recurrence_parent_id: string;
  recurrence_index: number;
  recurrence_rule: RecurrenceRule;
  reminder_before: number | null;
}

function dateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

function createdAtDateKey(raw: string): string {
  const n = Number(raw);
  const d = Number.isFinite(n) ? new Date(n > 1e12 ? n : n * 1000) : new Date(raw);
  if (Number.isNaN(d.getTime())) return '';
  return dateKey(d);
}

function dateFromKey(key: string): Date | null {
  const date = new Date(`${key}T00:00:00`);
  return Number.isNaN(date.getTime()) ? null : date;
}

export function parseRecurrenceRule(value: unknown): RecurrenceRule | null {
  const raw = typeof value === 'string'
    ? (() => {
        try { return JSON.parse(value); } catch { return null; }
      })()
    : value;
  if (!raw || typeof raw !== 'object') return null;
  const rule = raw as Partial<RecurrenceRule>;
  if (!['daily', 'weekly', 'monthly', 'weekdays', 'custom'].includes(String(rule.type))) return null;
  return {
    type: rule.type as RecurrenceRule['type'],
    interval: Number(rule.interval || 1) || 1,
    daysOfWeek: Array.isArray(rule.daysOfWeek) ? rule.daysOfWeek.map(Number).filter((v) => v >= 0 && v <= 6) : undefined,
    dayOfMonth: rule.dayOfMonth ? Number(rule.dayOfMonth) : undefined,
    endDate: rule.endDate || null,
    endCount: rule.endCount ? Number(rule.endCount) : null
  };
}

export function recurrenceLabel(rule: RecurrenceRule | null | undefined): string {
  if (!rule) return '';
  const days = ['日', '一', '二', '三', '四', '五', '六'];
  if (rule.type === 'daily') return rule.interval > 1 ? `每${rule.interval}天` : '每天';
  if (rule.type === 'weekdays') return rule.interval > 1 ? `每${rule.interval}周工作日` : '工作日';
  if (rule.type === 'monthly') {
    if (rule.interval === 12) return `每年同月${rule.dayOfMonth || 1}日`;
    return rule.interval > 1 ? `每${rule.interval}个月${rule.dayOfMonth || 1}日` : `每月${rule.dayOfMonth || 1}日`;
  }
  if (rule.type === 'weekly' || rule.type === 'custom') {
    const selected = (rule.daysOfWeek?.length ? rule.daysOfWeek : [new Date().getDay()]).map((d) => days[d]).join('、');
    return rule.interval > 1 ? `每${rule.interval}周${selected}` : `每周${selected}`;
  }
  return '自定义';
}

function daysBetween(anchorKey: string, targetKey: string): number {
  const anchor = dateFromKey(anchorKey);
  const target = dateFromKey(targetKey);
  if (!anchor || !target) return 0;
  return Math.floor((target.getTime() - anchor.getTime()) / 86400000);
}

function weekStart(date: Date): Date {
  const result = new Date(date);
  result.setDate(result.getDate() - result.getDay());
  return result;
}

function weeksBetween(anchorKey: string, targetKey: string): number {
  const anchor = dateFromKey(anchorKey);
  const target = dateFromKey(targetKey);
  if (!anchor || !target) return 0;
  return Math.floor((weekStart(target).getTime() - weekStart(anchor).getTime()) / (86400000 * 7));
}

function monthsBetween(anchorKey: string, targetKey: string): number {
  const anchor = dateFromKey(anchorKey);
  const target = dateFromKey(targetKey);
  if (!anchor || !target) return 0;
  return (target.getFullYear() - anchor.getFullYear()) * 12 + target.getMonth() - anchor.getMonth();
}

function matchesInterval(date: Date, rule: RecurrenceRule, anchorKey: string): boolean {
  const targetKey = dateKey(date);
  if (targetKey <= anchorKey) return false;

  const interval = Math.max(1, Number(rule.interval || 1));
  if (interval <= 1) return true;

  if (rule.type === 'daily') return daysBetween(anchorKey, targetKey) % interval === 0;
  if (rule.type === 'weekly' || rule.type === 'custom' || rule.type === 'weekdays') {
    return weeksBetween(anchorKey, targetKey) % interval === 0;
  }
  if (rule.type === 'monthly') return monthsBetween(anchorKey, targetKey) % interval === 0;
  return true;
}

export function matchesRule(date: Date, rule: RecurrenceRule, anchorKey?: string): boolean {
  const dayOfWeek = date.getDay();
  const dayOfMonth = date.getDate();
  if (anchorKey && !matchesInterval(date, rule, anchorKey)) return false;
  switch (rule.type) {
    case 'daily': return true;
    case 'weekly': return rule.daysOfWeek?.includes(dayOfWeek) ?? false;
    case 'monthly': return dayOfMonth === (rule.dayOfMonth || 1);
    case 'weekdays': return dayOfWeek >= 1 && dayOfWeek <= 5;
    case 'custom': return rule.daysOfWeek?.includes(dayOfWeek) ?? false;
    default: return false;
  }
}

function getNextIndex(tasks: Task[], parentId: string): number {
  const max = tasks
    .filter((task) => task.recurrence_parent_id === parentId)
    .reduce((acc, task) => Math.max(acc, Number(task.recurrence_index || 0)), 0);
  return max + 1;
}

function recurrenceRuleKey(rule: unknown): string {
  const parsed = parseRecurrenceRule(rule);
  return parsed ? JSON.stringify(parsed) : '';
}

function recurrenceParentCandidates(template: Task): Set<string> {
  return new Set([template.record_id, template.id].filter(Boolean) as string[]);
}

function occurrenceDateKey(task: Pick<Task, 'due_date' | 'created_at'>, fallback = ''): string {
  const due = splitDueDate(task.due_date);
  return due.date || createdAtDateKey(task.created_at || '') || fallback;
}

export function hasRecurringOccurrence(
  tasks: Task[],
  template: Task,
  occurrenceDate: string
): boolean {
  const parentIds = recurrenceParentCandidates(template);
  const templateRuleKey = recurrenceRuleKey(template.recurrence_rule);

  return tasks.some((task) => {
    if (occurrenceDateKey(task) !== occurrenceDate) return false;
    if (parentIds.has(task.record_id) || (task.id && parentIds.has(task.id))) return true;

    const parentId = (task.recurrence_parent_id || '').trim();
    if (!parentId) return false;
    if (parentIds.has(parentId)) return true;

    // 兼容早期 temp parent id 被远端 record_id 替换前创建的实例。
    return task.name === template.name && recurrenceRuleKey(task.recurrence_rule) === templateRuleKey;
  });
}

function getRecurringOccurrenceCount(tasks: Task[], template: Task): number {
  const parentIds = recurrenceParentCandidates(template);
  const templateRuleKey = recurrenceRuleKey(template.recurrence_rule);
  return tasks.filter((task) => {
    const parentId = (task.recurrence_parent_id || '').trim();
    if (!parentId) return false;
    if (parentIds.has(parentId)) return true;
    return task.name === template.name && recurrenceRuleKey(task.recurrence_rule) === templateRuleKey;
  }).length;
}

function occurrenceDueDate(templateDueDate: string | undefined, todayStr: string): string {
  const parts = splitDueDate(templateDueDate);
  if (!parts.date) return todayStr;
  return buildDueDateValue(todayStr, parts.hasTime ? parts.time : '');
}

function recurrenceAnchorKey(task: Task): string {
  const due = splitDueDate(task.due_date);
  return due.date || createdAtDateKey(task.created_at);
}

export function generateRecurringInstances(tasks: Task[], today = new Date()): RecurringInstanceInput[] {
  const todayStr = dateKey(today);
  const templates = tasks.filter((task) => task.recurrence_rule && !task.recurrence_parent_id);
  const instances: RecurringInstanceInput[] = [];

  for (const template of templates) {
    const rule = parseRecurrenceRule(template.recurrence_rule);
    if (!rule) continue;
    if (rule.endDate && rule.endDate < todayStr) continue;
    if (rule.endCount) {
      const existingCount = getRecurringOccurrenceCount(tasks, template);
      if (existingCount >= rule.endCount) continue;
    }
    const anchorKey = recurrenceAnchorKey(template);
    if (!anchorKey || !matchesRule(today, rule, anchorKey)) continue;

    const templateId = template.record_id || template.id || '';
    if (!templateId || hasRecurringOccurrence(tasks, template, todayStr)) continue;

    instances.push({
      template,
      name: template.name,
      priority: template.priority || '普通',
      status: '待处理',
      due_date: occurrenceDueDate(template.due_date, todayStr),
      recurrence_parent_id: templateId,
      recurrence_index: getNextIndex(tasks, templateId),
      recurrence_rule: rule,
      reminder_before: template.reminder_before ?? null
    });
  }
  return instances;
}
