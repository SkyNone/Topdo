import { ref } from 'vue';

export interface LogEntry {
  timestamp: string;
  tag: string;
  message: string;
  data?: unknown;
}

const MAX_LOGS = 50;

export const logs = ref<LogEntry[]>([]);

function formatTimestamp(date: Date): string {
  const hh = String(date.getHours()).padStart(2, '0');
  const mm = String(date.getMinutes()).padStart(2, '0');
  const ss = String(date.getSeconds()).padStart(2, '0');
  const ms = String(date.getMilliseconds()).padStart(3, '0');
  return `${hh}:${mm}:${ss}.${ms}`;
}

function normalizeData(data: unknown): unknown {
  if (data === undefined) return undefined;
  try {
    const text = JSON.stringify(data);
    if (text.length > 1200) {
      return `${text.slice(0, 1200)}...`;
    }
    return JSON.parse(text);
  } catch {
    return String(data);
  }
}

export function log(tag: string, message: string, data?: unknown) {
  const entry: LogEntry = {
    timestamp: formatTimestamp(new Date()),
    tag,
    message,
    data: normalizeData(data)
  };

  logs.value = [entry, ...logs.value].slice(0, MAX_LOGS);
}

export function clearLogs() {
  logs.value = [];
}

export function formatLogLine(entry: LogEntry): string {
  const base = `[${entry.timestamp}] [${entry.tag}] ${entry.message}`;
  if (entry.data === undefined) return base;

  try {
    return `${base} ${JSON.stringify(entry.data)}`;
  } catch {
    return `${base} ${String(entry.data)}`;
  }
}
