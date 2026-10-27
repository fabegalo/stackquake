import type { Cell, Kind, Status } from "./api";

const nf = new Intl.NumberFormat("pt-BR");

export function pct(v: Cell | undefined, digits = 2): string {
  return typeof v === "number" ? `${(v * 100).toFixed(digits).replace(".", ",")}%` : "—";
}

export function num(v: Cell | undefined): string {
  return typeof v === "number" ? nf.format(v) : "—";
}

export function dateTime(iso: string | null): string {
  if (!iso) return "nunca";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("pt-BR");
}

export function cell(v: Cell | undefined, column: string): string {
  if (v === null || v === undefined) return "—";
  if (typeof v === "number") {
    // Colunas de taxa (frações) aparecem em porcentagem.
    if (/percebida|limite|_geral|rate|sem_falha/.test(column)) return pct(v);
    return Number.isInteger(v) ? nf.format(v) : v.toFixed(3).replace(".", ",");
  }
  return v;
}

export const kindLabel: Record<Kind, string> = {
  crash: "Falha",
  anr: "ANR",
  nonfatal: "Não fatal",
};

export const statusLabel: Record<Status, string> = {
  open: "Aberto",
  investigating: "Investigando",
  resolved: "Resolvido",
  ignored: "Ignorado",
  regressed: "Regrediu",
};

export const sourceLabel: Record<string, string> = {
  both: "Play + Crashlytics",
  play: "Play",
  crashlytics: "Crashlytics",
  none: "—",
};
