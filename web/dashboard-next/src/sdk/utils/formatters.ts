// Formatting utilities for HyperSDK

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';

  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

export function formatDuration(seconds: number): string {
  if (seconds < 60) {
    return `${seconds.toFixed(0)}s`;
  }

  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;

  if (minutes < 60) {
    return `${minutes}m ${remainingSeconds.toFixed(0)}s`;
  }

  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;

  if (hours < 24) {
    return `${hours}h ${remainingMinutes}m`;
  }

  const days = Math.floor(hours / 24);
  const remainingHours = hours % 24;

  return `${days}d ${remainingHours}h`;
}

export function formatTimestamp(timestamp: string): string {
  const date = new Date(timestamp);
  return date.toLocaleString();
}

export function formatRelativeTime(timestamp: string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSeconds = Math.floor(diffMs / 1000);

  if (diffSeconds < 60) {
    return `${diffSeconds}s ago`;
  }

  const diffMinutes = Math.floor(diffSeconds / 60);
  if (diffMinutes < 60) {
    return `${diffMinutes}m ago`;
  }

  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) {
    return `${diffHours}h ago`;
  }

  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
}

export function formatPercentage(value: number, decimals = 1): string {
  return `${value.toFixed(decimals)}%`;
}

export function formatNumber(value: number, decimals = 0): string {
  return value.toLocaleString(undefined, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

export function getStatusColor(status: string): string {
  const statusColors: Record<string, string> = {
    pending: '#6b7280',
    running: '#3b82f6',
    completed: '#10b981',
    failed: '#ef4444',
    cancelled: '#f59e0b',
    healthy: '#10b981',
    degraded: '#f59e0b',
    unhealthy: '#ef4444',
  };

  return statusColors[status] || '#6b7280';
}

export function getStatusIcon(status: string): string {
  const statusIcons: Record<string, string> = {
    pending: '⏳',
    running: '▶️',
    completed: '✅',
    failed: '❌',
    cancelled: '🚫',
    healthy: '✅',
    degraded: '⚠️',
    unhealthy: '❌',
  };

  return statusIcons[status] || '●';
}

export function getSeverityColor(severity: string): string {
  const severityColors: Record<string, string> = {
    info: '#3b82f6',
    warning: '#f59e0b',
    error: '#ef4444',
    critical: '#991b1b',
  };

  return severityColors[severity] || '#6b7280';
}
