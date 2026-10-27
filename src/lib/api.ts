// Wrappers tipados dos comandos do núcleo Rust. A interface não tem acesso a
// rede, arquivos ou tokens: tudo passa por estes comandos.
import { invoke } from "@tauri-apps/api/core";

export type Kind = "crash" | "anr" | "nonfatal";
export type Status = "open" | "investigating" | "resolved" | "ignored" | "regressed";

export interface SetupStatus {
  hasClient: boolean;
  signedIn: boolean;
  email: string | null;
}

export interface AppRow {
  packageName: string;
  displayName: string | null;
  firebaseProject: string | null;
  enabled: boolean;
  lastSyncAt: string | null;
}

export interface SyncReport {
  packageName: string;
  vitalsRows: number;
  playIssues: number;
  crashlyticsIssues: number;
  crashlyticsVersions: number;
  regressions: number;
  warnings: string[];
}

export interface KpiInfo {
  id: string;
  name: string;
  description: string;
  source: "builtin" | "user";
}

export type Cell = string | number | null;

export interface KpiResult {
  columns: string[];
  rows: Cell[][];
  truncated: boolean;
}

export interface ProblemRow {
  id: number;
  kind: Kind;
  title: string;
  status: Status;
  assignee: string | null;
  sources: "play" | "crashlytics" | "both" | "none";
  playReports: number | null;
  playUsers: number | null;
  clEvents: number | null;
  clUsers: number | null;
  firstVersion: number | null;
  lastVersion: number | null;
  lastSeen: string | null;
  impact: number;
}

export interface ProblemDetail {
  problem: {
    id: number;
    packageName: string;
    signature: string;
    kind: Kind;
    title: string;
    status: Status;
    assignee: string | null;
    notes: string | null;
    resolvedInVersion: string | null;
    createdAt: string;
    updatedAt: string;
  };
  playIssues: {
    issueId: string;
    cause: string | null;
    location: string | null;
    reportCount: number | null;
    distinctUsers: number | null;
    firstVersion: string | null;
    lastVersion: string | null;
    lastSeen: string | null;
    issueUri: string | null;
    sampleReport: string | null;
  }[];
  crashlyticsIssues: {
    issueId: string;
    title: string;
    subtitle: string | null;
    rootType: string | null;
    blameSymbol: string | null;
    firstVersion: string | null;
    lastVersion: string | null;
    firstSeen: string | null;
    lastSeen: string | null;
    issueUri: string | null;
    sampleStack: string | null;
  }[];
  byVersion: { buildVersion: string; displayVersion: string | null; events: number; users: number }[];
}

export const api = {
  setupStatus: () => invoke<SetupStatus>("get_setup_status"),
  signIn: () => invoke<void>("sign_in"),
  signOut: () => invoke<void>("sign_out"),
  listApps: () => invoke<AppRow[]>("list_apps"),
  refreshApps: () => invoke<AppRow[]>("refresh_apps"),
  updateAppSettings: (packageName: string, enabled: boolean) =>
    invoke<void>("update_app_settings", { packageName, enabled }),
  syncApp: (packageName: string) => invoke<SyncReport>("sync_app", { packageName }),
  listKpis: () => invoke<KpiInfo[]>("list_kpis"),
  runKpi: (id: string, packageName: string, days: number) => invoke<KpiResult>("run_kpi", { id, packageName, days }),
  listProblems: (packageName: string, days: number, status: Status | null, kind: Kind | null) =>
    invoke<ProblemRow[]>("list_problems", { packageName, days, status, kind }),
  getProblem: (id: number) => invoke<ProblemDetail>("get_problem", { id }),
  updateProblem: (
    id: number,
    status: Status,
    assignee: string | null,
    notes: string | null,
    resolvedInVersion: string | null,
  ) => invoke<void>("update_problem", { id, status, assignee, notes, resolvedInVersion }),
  fetchProblemSample: (id: number) => invoke<ProblemDetail>("fetch_problem_sample", { id }),
  openKpiFolder: () => invoke<void>("open_kpi_folder"),
  resetLocalData: () => invoke<void>("reset_local_data"),
};

/** Converte um resultado de KPI em objetos (coluna -> valor). */
export function rowsOf(r: KpiResult): Record<string, Cell>[] {
  return r.rows.map((row) => Object.fromEntries(r.columns.map((c, i) => [c, row[i] ?? null])));
}

export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : "Erro inesperado";
}
