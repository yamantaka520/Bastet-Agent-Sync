import type { Locale } from "./i18n";
export type Settings = {
  portable?: import("./PortablePanel").PortableOptions;
  resources?: import("./OperationsPanel").Resources;
  schema: number;
  locale: Locale;
  deviceName: string;
  selectedAgents: string[];
  customPaths: Record<string, string>;
  folder: string;
  direction: "bidirectional" | "upload" | "download";
  schedule: "near-realtime" | "interval" | "manual";
  intervalSeconds: number;
  closeToTray: boolean;
};
export type Agent = {
  id: string;
  path: string;
  detected: boolean;
  custom: boolean;
};
export const names: Record<string, string> = {
  claude: "Claude",
  "claude-code": "Claude Code",
  codex: "Codex",
  "chatgpt-work": "ChatGPT Work",
  agy: "Google Agy CLI",
  grok: "Grok Build CLI",
  pi: "Pi Agent",
  "agent-memory-os": "Agent Memory OS",
};
export const defaults = (locale: Locale): Settings => ({
  schema: 1,
  locale,
  deviceName: "",
  selectedAgents: [],
  customPaths: {},
  folder: "",
  direction: "bidirectional",
  schedule: "near-realtime",
  intervalSeconds: 60,
  closeToTray: false,
});

export type Diagnostic = {
  verified: boolean;
  transferred: number;
  preservedBranches: number;
  repeatTransfers: number;
  recoveredObjects: number;
};
