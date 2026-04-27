// TypeScript types for HyperSDK

export interface Metrics {
  timestamp: string;
  jobs_active: number;
  jobs_completed: number;
  jobs_failed: number;
  jobs_pending: number;
  jobs_cancelled: number;
  queue_length: number;
  http_requests: number;
  http_errors: number;
  avg_response_time: number;
  memory_usage: number;
  cpu_usage: number;
  goroutines: number;
  active_connections: number;
  websocket_clients: number;
  provider_stats: Record<string, ProviderStats>;
  recent_jobs: JobInfo[];
  system_health: HealthStatus;
  alerts: Alert[];
  uptime_seconds: number;
}

export interface ProviderStats {
  jobs_total: number;
  jobs_active: number;
  jobs_completed: number;
  jobs_failed: number;
  avg_duration_seconds: number;
  total_data_exported_bytes: number;
}

export interface JobInfo {
  id: string;
  name: string;
  status: JobStatus;
  progress: number;
  start_time: string;
  end_time?: string;
  duration_seconds: number;
  provider: string;
  vm_name: string;
  vm_path: string;
  output_dir: string;
  format: string;
  compress: boolean;
  size_bytes?: number;
  error_msg?: string;
  created_at: string;
  updated_at: string;
}

export type JobStatus =
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type HealthStatus =
  | 'healthy'
  | 'degraded'
  | 'unhealthy';

export interface Alert {
  id: string;
  severity: AlertSeverity;
  title: string;
  message: string;
  timestamp: string;
  acknowledged: boolean;
}

export type AlertSeverity =
  | 'info'
  | 'warning'
  | 'error'
  | 'critical';

export interface ScheduledJob {
  id: string;
  name: string;
  description: string;
  schedule: string;
  job_template: JobDefinition;
  enabled: boolean;
  created_at: string;
  updated_at: string;
  next_run: string;
  last_run?: string;
  run_count: number;
  tags?: string[];
}

export interface JobDefinition {
  id?: string;
  name: string;
  vm_path: string;
  output_dir: string;
  format: string;
  compress: boolean;
  provider?: string;
  provider_config?: Record<string, unknown>;
}

export interface WebhookConfig {
  url: string;
  events: string[];
  headers?: Record<string, string>;
  timeout_seconds: number;
  retry_attempts: number;
  enabled: boolean;
}

export interface SystemInfo {
  version: string;
  build_time: string;
  git_commit: string;
  go_version: string;
  platform: string;
  uptime_seconds: number;
}

export interface ConnectionPoolStats {
  max_connections: number;
  active_connections: number;
  idle_connections: number;
  total_created: number;
  total_closed: number;
  health_check_failures: number;
}

// Workflow daemon types
export interface WorkflowStatus {
  mode: 'disk' | 'manifest';
  running: boolean;
  queue_depth: number;
  active_jobs: number;
  processed_today: number;
  failed_today: number;
  max_workers: number;
  uptime_seconds: number;
}

export interface WorkflowJob {
  id: string;
  name: string;
  stage: string;
  progress: number;
  started_at: string;
  elapsed_seconds: number;
  status: 'pending' | 'processing' | 'completed' | 'failed';
}

export interface ManifestPipeline {
  load: {
    source_type: string;
    source_path: string;
  };
  inspect: {
    enabled: boolean;
    detect_os: boolean;
  };
  fix: {
    fstab: {
      enabled: boolean;
      mode: string;
    };
    grub: {
      enabled: boolean;
    };
    initramfs: {
      enabled: boolean;
      regenerate: boolean;
    };
    network: {
      enabled: boolean;
      fix_level: string;
    };
  };
  convert: {
    output_format: string;
    compress: boolean;
    output_path?: string;
  };
  validate: {
    enabled: boolean;
    boot_test: boolean;
  };
}

export interface Manifest {
  version: string;
  batch?: boolean;
  name?: string;
  pipeline?: ManifestPipeline;
  vms?: Array<{
    name: string;
    pipeline: ManifestPipeline;
  }>;
}
