export type ProfileStatus = "connected" | "disconnected" | "connecting";

export type ProxyType = "socks5" | "http";

export interface Profile {
  id: string;
  name: string;
  configContent: string;
  proxyType: ProxyType;
  port: number;
  status: ProfileStatus;
  dns?: string;
  endpoint?: string;
  lastUsed?: string;
}

export interface AppSettings {
  autoStart: boolean;
  startOnBoot: boolean;
  defaultPort: number;
  defaultProxyType: ProxyType;
}
