export interface Task {
  id?: string;
  record_id: string;
  name: string;
  status: string;
  priority: string;
  task_type: string;
  time_spent: string;
  created_at: string;
  updated_at: string;
  completed_at?: string;
  notes: string;
  sort_order?: number;
  source?: 'local' | 'feishu' | string;
  feishu_record_id?: string;
  sync_status: string;
  last_synced_at: string;
  retry_count?: number;
  last_error?: string;
  last_retry_at?: string;
}

export interface SyncMeta {
  pending_count: number;
  failed_count: number;
  last_sync_at: string;
  last_error_summary: string;
}

export interface SyncTasksResult {
  tasks: Task[];
  sync_meta: SyncMeta;
}
