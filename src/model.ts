import type { Locale } from "./i18n";
export type Settings = {
  cloudProvider: "google-drive" | "icloud-drive" | "onedrive-folder";
  portable?: import("./PortablePanel").PortableOptions;
  resources?: import("./OperationsPanel").Resources;
  schema: number;
  locale: Locale;
  deviceName: string;
  selectedAgents: string[];
  customPaths: Record<string, string>;
  projectMappings?: { source: string; target: string }[];
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
  hermes: "Hermes Agent",
};
// The Store channel excludes the external memory-service adapter, not native
// conversation adapters using user-authorized folder access.
const storeHidden = new Set(["agent-memory-os"]);
// Keep every known agent selectable even when a discovery response predates its
// backend entry, so a new source is never missing from the setup list. The Store
// build trusts discovery exactly, since it already reports the reduced set.
export function withKnownAgents(
  found: Agent[],
  memorySyncAvailable = true,
): Agent[] {
  const present = found.filter(
    (agent) => memorySyncAvailable || !storeHidden.has(agent.id),
  );
  if (!memorySyncAvailable) return present;
  const seen = new Set(present.map((agent) => agent.id));
  const known: Agent[] = Object.keys(names).map((id) => ({
    id,
    path: "",
    detected: false,
    custom: false,
  }));
  return [...present, ...known.filter((agent) => !seen.has(agent.id))];
}
export const defaults = (locale: Locale): Settings => ({
  schema: 1,
  cloudProvider: "google-drive",
  locale,
  deviceName: "",
  selectedAgents: [],
  customPaths: {},
  projectMappings: [],
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
