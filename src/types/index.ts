export type ProfileStatus = "running" | "stopped" | "starting" | "error";

export type ProxyType = "socks5" | "http";

export interface Profile {
  id: string;
  name: string;
  proxyType: ProxyType;
  port: number;
  endpoint: string;
  dns: string;
  address: string;
  allowedIps: string;
  sourcePath?: string;
  configContent: string;
  status: ProfileStatus;
  createdAt: string;
  updatedAt: string;
  lastConnectedAt?: string;
}

export interface GeneratedConfigMeta {
  content: string;
  path: string;
  generatedAt: string;
  proxyType: ProxyType;
  port: number;
}

export interface ConnectionHealthResult {
  success: boolean;
  tunnelActive: boolean;
  exitIp: string;
  localIp: string;
  latencyMs: number;
  error: string;
}

export interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

export interface ProxyStats {
  status: string;
  uploadedBytesTotal: number;
  downloadedBytesTotal: number;
  uploadSpeedBytesPerSec: number;
  downloadSpeedBytesPerSec: number;
  lastHandshake: string;
  lastHandshakeAgeSecs: number | null;
  connectedForSecs: number;
}

/** One usage window's accumulated bytes. */
export interface UsageBucket {
  uploaded: number;
  downloaded: number;
}

/** Aggregate usage across time windows, returned by `get_usage_overview`. */
export interface UsageOverview {
  today: UsageBucket;
  last24h: UsageBucket;
  week: UsageBucket;
  month: UsageBucket;
  /** All-time total across every recorded day. */
  total: UsageBucket;
}

/** A single day's bytes, keyed by `YYYY-MM-DD` in the usage history map. */
export interface UsageDay {
  uploaded: number;
  downloaded: number;
}

/** Raw per-day usage history returned by `get_usage_history`. */
export interface UsageHistory {
  days: Record<string, UsageDay>;
  lastUpdated: string | null;
}


