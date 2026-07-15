import { invoke } from '@tauri-apps/api/core';
import type { HabitWithStats } from '../types/habit';
import type { Task } from '../types';

export interface DailyReceiptItem {
  name: string;
  time: string;
}

export interface DailyReceiptData {
  dateKey: string;
  dateLabel: string;
  totalCount: number;
  focusCount: number;
  habitChecked: number;
  habitTotal: number;
  streak: number;
  items: DailyReceiptItem[];
  remainingCount: number;
  tags: string[];
}

const MAX_VISIBLE_ITEMS = 5;

function localDateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

function dateKeyFromTimestamp(raw: string): string {
  const value = String(raw || '').trim();
  if (!value) return '';
  if (/^\d{4}-\d{2}-\d{2}/.test(value) && !/(Z|[+-]\d{2}:?\d{2})$/i.test(value)) {
    return value.slice(0, 10);
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value.slice(0, 10) : localDateKey(date);
}

function timeFromTimestamp(raw: string): string {
  const value = String(raw || '').trim();
  if (!value) return '';
  if (/^\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}/.test(value) && !/(Z|[+-]\d{2}:?\d{2})$/i.test(value)) {
    return value.slice(11, 16);
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '';
  return new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false }).format(date);
}

function isCompleted(task: Task): boolean {
  return String(task.status || '').includes('已完成');
}

function isFocusPriority(priority: string): boolean {
  const value = String(priority || '').trim();
  return value === '紧急' || value === '重要' || value.includes('今日必做') || value.includes('本周完成');
}

function normalizeTags(tags: unknown): string[] {
  if (!Array.isArray(tags)) return [];
  return tags.map((tag) => String(tag || '').trim()).filter(Boolean);
}

export function buildDailyReceiptData(
  tasks: Task[],
  habits: HabitWithStats[],
  streak: number,
  now = new Date()
): DailyReceiptData {
  const todayKey = localDateKey(now);
  const completedTasks = tasks.filter(
    (task) => isCompleted(task) && dateKeyFromTimestamp(task.completed_at || '') === todayKey
  );
  const checkedHabits = habits.filter((habit) => habit.todayChecked);
  const taskItems = completedTasks.map((task) => ({
    name: task.name,
    time: timeFromTimestamp(task.completed_at || '')
  }));
  const habitItems = checkedHabits.map((habit) => ({
    name: `${habit.name}打卡`,
    time: habit.todayCheckTime || ''
  }));
  const allItems = [...taskItems, ...habitItems].sort((a, b) => a.time.localeCompare(b.time));
  const tags = Array.from(new Set(completedTasks.flatMap((task) => normalizeTags(task.tags)))).slice(0, 3);
  const weekday = new Intl.DateTimeFormat('zh-CN', { weekday: 'long' }).format(now);

  return {
    dateKey: todayKey,
    dateLabel: `${todayKey.replace(/-/g, '.')} · ${weekday}`,
    totalCount: allItems.length,
    focusCount: completedTasks.filter((task) => isFocusPriority(task.priority)).length,
    habitChecked: checkedHabits.length,
    habitTotal: habits.filter((habit) => !habit.is_archived).length,
    streak,
    items: allItems.slice(0, MAX_VISIBLE_ITEMS),
    remainingCount: Math.max(0, allItems.length - MAX_VISIBLE_ITEMS),
    tags
  };
}

export function buildDailyReceiptText(data: DailyReceiptData): string {
  const lines = [
    `Topdo 今日小票 · ${data.dateLabel}`,
    '',
    `今天完成 ${data.totalCount} 项`,
    ...data.items.map((item) => `✓ ${item.name}${item.time ? `  ${item.time}` : ''}`)
  ];
  if (data.remainingCount > 0) lines.push(`… 另有 ${data.remainingCount} 项已完成`);
  lines.push('', `重点任务 ${data.focusCount} · 习惯打卡 ${data.habitChecked}/${data.habitTotal} · 连续完成 ${data.streak} 天`);
  if (data.tags.length) lines.push(`今日标签：${data.tags.join(' · ')}`);
  lines.push('', '今天推进的每一步，都算数。');
  return lines.join('\n');
}

function roundedRect(context: CanvasRenderingContext2D, x: number, y: number, width: number, height: number, radius: number) {
  context.beginPath();
  context.roundRect(x, y, width, height, radius);
  context.fill();
}

function drawReceiptPaper(context: CanvasRenderingContext2D, x: number, y: number, width: number, height: number) {
  const toothRadius = 8;
  context.beginPath();
  context.rect(x, y, width, height);
  for (let centerX = x + toothRadius; centerX < x + width; centerX += toothRadius * 2) {
    context.moveTo(centerX + toothRadius, y);
    context.arc(centerX, y, toothRadius, 0, Math.PI * 2);
    context.moveTo(centerX + toothRadius, y + height);
    context.arc(centerX, y + height, toothRadius, 0, Math.PI * 2);
  }
  context.fill();
}

function drawText(
  context: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  options: { font: string; color?: string; align?: CanvasTextAlign }
) {
  context.font = options.font;
  context.fillStyle = options.color || '#1d1d1f';
  context.textAlign = options.align || 'left';
  context.textBaseline = 'middle';
  context.fillText(text, x, y);
}

function drawDashedLine(context: CanvasRenderingContext2D, x1: number, y: number, x2: number) {
  context.save();
  context.strokeStyle = '#d9d9de';
  context.lineWidth = 2;
  context.setLineDash([7, 7]);
  context.beginPath();
  context.moveTo(x1, y);
  context.lineTo(x2, y);
  context.stroke();
  context.restore();
}

function canvasToBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error('生成小票图片失败'))), 'image/png');
  });
}

export async function renderDailyReceiptPng(data: DailyReceiptData): Promise<Blob> {
  const canvas = document.createElement('canvas');
  canvas.width = 750;
  canvas.height = 1200;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('当前环境无法生成图片');

  context.fillStyle = '#f1f3f7';
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.shadowColor = 'rgba(15, 23, 42, 0.10)';
  context.shadowBlur = 28;
  context.shadowOffsetY = 10;
  context.fillStyle = '#fffefa';
  drawReceiptPaper(context, 45, 52, 660, 1096);
  context.shadowColor = 'transparent';

  context.save();
  context.globalAlpha = 0.035;
  context.fillStyle = '#776b58';
  for (let y = 70; y < 1135; y += 8) {
    for (let x = 64 + (y % 16); x < 690; x += 18) context.fillRect(x, y, 1, 1);
  }
  context.restore();

  drawText(context, 'TOPDO', 375, 82, { font: '700 22px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#007aff', align: 'center' });
  drawText(context, '今日小票', 375, 150, { font: '750 48px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', align: 'center' });
  drawDashedLine(context, 95, 210, 655);
  drawText(context, data.dateLabel, 375, 245, { font: '400 20px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#86868b', align: 'center' });
  drawText(context, '今天完成', 375, 310, { font: '400 22px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#86868b', align: 'center' });
  drawText(context, String(data.totalCount), 356, 385, { font: '750 94px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#007aff', align: 'center' });
  drawText(context, '项', 414, 410, { font: '650 24px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#007aff' });

  let rowY = 485;
  for (const item of data.items) {
    context.fillStyle = '#007aff';
    context.beginPath();
    context.arc(112, rowY, 15, 0, Math.PI * 2);
    context.fill();
    context.strokeStyle = '#ffffff';
    context.lineWidth = 3;
    context.beginPath();
    context.moveTo(105, rowY);
    context.lineTo(110, rowY + 5);
    context.lineTo(119, rowY - 6);
    context.stroke();
    drawText(context, item.name.slice(0, 18), 150, rowY, { font: '500 23px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif' });
    if (item.time) drawText(context, item.time, 630, rowY, { font: '400 18px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#aeaeb2', align: 'right' });
    context.strokeStyle = '#e5e5ea';
    context.lineWidth = 1;
    context.beginPath();
    context.moveTo(96, rowY + 34);
    context.lineTo(654, rowY + 34);
    context.stroke();
    rowY += 68;
  }
  if (data.remainingCount > 0) {
    drawText(context, `另有 ${data.remainingCount} 项已完成`, 150, rowY - 4, { font: '400 18px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#86868b' });
  }

  const summaryY = 875;
  drawDashedLine(context, 95, summaryY - 56, 655);
  const summaries = [
    ['重点任务', String(data.focusCount)],
    ['习惯打卡', `${data.habitChecked}/${data.habitTotal}`],
    ['连续完成', `${data.streak} 天`]
  ];
  summaries.forEach(([label, value], index) => {
    const x = 160 + index * 215;
    drawText(context, label, x, summaryY, { font: '400 18px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#86868b', align: 'center' });
    drawText(context, value, x, summaryY + 48, { font: '650 40px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#007aff', align: 'center' });
  });

  if (data.tags.length) {
    context.font = '500 19px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif';
    const widths = data.tags.map((tag) => Math.max(110, context.measureText(tag).width + 48));
    const totalWidth = widths.reduce((sum, width) => sum + width, 0) + (widths.length - 1) * 16;
    let x = (750 - totalWidth) / 2;
    data.tags.forEach((tag, index) => {
      context.fillStyle = '#eff6ff';
      roundedRect(context, x, 990, widths[index], 52, 26);
      drawText(context, tag, x + widths[index] / 2, 1016, { font: '500 19px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#007aff', align: 'center' });
      x += widths[index] + 16;
    });
  }

  drawDashedLine(context, 95, 1075, 655);
  drawText(context, '今天推进的每一步，都算数。', 375, 1115, { font: '500 21px -apple-system, BlinkMacSystemFont, "PingFang SC", sans-serif', color: '#55555a', align: 'center' });
  return canvasToBlob(canvas);
}

export async function exportDailyReceiptImage(data: DailyReceiptData): Promise<string> {
  const blob = await renderDailyReceiptPng(data);
  const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
  return invoke<string>('export_daily_receipt_image', { bytes });
}
