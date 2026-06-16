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
}

export interface AppSettings {
  wireproxyBinaryPath: string;
}

export interface GeneratedConfigMeta {
  content: string;
  path: string;
  generatedAt: string;
  proxyType: ProxyType;
  port: number;
}
