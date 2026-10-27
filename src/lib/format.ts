import type { Cell, Kind, Status } from "./api";
import { intlLocale, t, tMaybe } from "./i18n/index.svelte";

export function pct(v: Cell | undefined, digits = 2): string {
  if (typeof v !== "number") return "—";
  return new Intl.NumberFormat(intlLocale(), {
    style: "percent",
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(v);
}

export function num(v: Cell | undefined): string {
  return typeof v === "number" ? new Intl.NumberFormat(intlLocale()).format(v) : "—";
}

export function dateTime(iso: string | null): string {
  if (!iso) return t("header.never");
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString(intlLocale());
}

/** KPI column header: translated when known, raw SQL name otherwise. */
export function columnLabel(column: string): string {
  return tMaybe(`columns.${column}`) ?? column;
}

/** Formats a KPI cell: rates as percentages, enumerations translated. */
export function cell(v: Cell | undefined, column: string): string {
  if (v === null || v === undefined) return "—";
  if (typeof v === "number") {
    // Rate columns (fractions) are shown as percentages.
    if (/rate|threshold|crash_free/.test(column)) return pct(v);
    return Number.isInteger(v)
      ? num(v)
      : new Intl.NumberFormat(intlLocale(), { maximumFractionDigits: 3 }).format(v);
  }
  if (column === "kind") return kindLabel(v as Kind);
  if (column === "status") return statusLabel(v as Status);
  return tMaybe(`values.${column}.${v}`) ?? v;
}

export const kindLabel = (k: Kind): string => t(`kind.${k}`);
export const statusLabel = (s: Status): string => t(`status.${s}`);
export const sourceLabel = (s: string): string => t(`source.${s}`);

export const KINDS: Kind[] = ["crash", "anr", "nonfatal"];
export const STATUSES: Status[] = ["open", "investigating", "resolved", "ignored", "regressed"];
