export type GenerationEvent =
  | { type: "piece"; text: string }
  | { type: "done" }
  | { type: "error"; message: string };

export interface StoredMessage {
  role: string;
  content: string;
  createdAt: number;
}

export interface ConversationSummary {
  id: string;
  title: string;
  flowName: string;
  createdAt: number;
  updatedAt: number;
}

export interface PermissionRequest {
  requestId: number;
  tool: string;
  args: Record<string, unknown>;
}

export interface CatalogModel {
  name: string;
  kind: "chat" | "embedding" | "image" | "asr" | "tts";
  sizeBytes: number;
  quantization: string;
  requiredEngine: string;
  alreadyDownloaded: boolean;
  fitsInAvailableMemory: boolean;
}

export interface SystemStats {
  cpuUsagePercent: number;
  memoryUsedBytes: number;
  memoryTotalBytes: number;
  processMemoryBytes: number;
  workspaceDiskBytes: number;
}

export interface FlowStateInfo {
  flowName: string;
  stateName: string;
  systemPrompt: string;
}

export interface ProviderInfo {
  name: string;
  envVar: string;
  configured: boolean;
}

export interface CloudModel {
  id: string;
  provider: string;
  label: string;
}

export interface CustomProviderConfig {
  name: string;
  baseUrl: string;
  envVar: string;
  models: string[];
}

export type McpTransportConfig =
  | { transport: "stdio"; command: string; args: string[] }
  | { transport: "http"; url: string; bearerToken?: string | null };

export interface McpServerConfig {
  name: string;
  transport: McpTransportConfig;
}

export interface McpToolDescriptor {
  name: string;
  description: string | null;
  inputSchema: unknown;
}

export interface ToolSpec {
  name: string;
  description: string;
  inputSchema: unknown;
}

export interface LocalModelInfo {
  name: string;
  sizeBytes: number;
  loaded: boolean;
  kind: "chat" | "embedding" | "image" | "asr" | "tts" | null;
}

export interface ScheduledJob {
  id: string;
  name: string;
  cronExpr: string;
  conversationId: string;
  prompt: string;
  model: string | null;
}

export function formatBytes(bytes: number): string {
  const gb = bytes / 1024 / 1024 / 1024;
  if (gb >= 1) return `${gb.toFixed(1)} GB`;
  return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
}
