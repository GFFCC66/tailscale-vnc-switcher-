export type QualityLevel = "great" | "good" | "fair" | "poor" | "offline" | "unknown";

export interface Quality {
  level: QualityLevel;
  label: string;
}

/** 延迟只看 TCP 握手，用来给侧边栏一个粗略的质量提示。 */
export function qualityFromLatency(online: boolean | null, latencyMs: number | null): Quality {
  if (online === null) return { level: "unknown", label: "检测中" };
  if (!online || latencyMs === null) return { level: "offline", label: "离线" };
  if (latencyMs < 40) return { level: "great", label: "极佳" };
  if (latencyMs < 120) return { level: "good", label: "良好" };
  if (latencyMs < 300) return { level: "fair", label: "一般" };
  return { level: "poor", label: "较差" };
}
