<script lang="ts">
  // Plain SVG line chart (no external library): daily user-perceived crash
  // and ANR rates, with Play's threshold lines.
  import type { Cell } from "../lib/api";
  import { pct } from "../lib/format";
  import { t } from "../lib/i18n/index.svelte";

  let { rows }: { rows: Record<string, Cell>[] } = $props();

  const W = 760;
  const H = 220;
  const PAD = { l: 52, r: 12, t: 12, b: 26 };
  const CRASH_LIMIT = 0.0109;
  const ANR_LIMIT = 0.0047;

  const series = $derived(
    rows.map((r) => ({
      day: String(r.day ?? ""),
      crash: typeof r.perceived_crash_rate === "number" ? r.perceived_crash_rate : null,
      anr: typeof r.perceived_anr_rate === "number" ? r.perceived_anr_rate : null,
    })),
  );

  const maxY = $derived(
    Math.max(CRASH_LIMIT * 1.2, ...series.flatMap((s) => [s.crash ?? 0, s.anr ?? 0])) * 1.05,
  );

  const x = (i: number) => PAD.l + (series.length <= 1 ? 0 : (i / (series.length - 1)) * (W - PAD.l - PAD.r));
  const y = (v: number) => PAD.t + (1 - v / maxY) * (H - PAD.t - PAD.b);

  function path(key: "crash" | "anr"): string {
    let d = "";
    let pen = false;
    series.forEach((s, i) => {
      const v = s[key];
      if (v === null) {
        pen = false;
        return;
      }
      d += `${pen ? "L" : "M"}${x(i).toFixed(1)},${y(v).toFixed(1)}`;
      pen = true;
    });
    return d;
  }

  const ticks = $derived([0, 0.25, 0.5, 0.75, 1].map((f) => f * maxY));
  const labelEvery = $derived(Math.max(1, Math.ceil(series.length / 8)));
</script>

{#if series.length === 0}
  <p class="muted">{t("chart.empty")}</p>
{:else}
  <svg viewBox="0 0 {W} {H}" role="img" aria-label={t("chart.aria")}>
    {#each ticks as t}
      <line class="grid" x1={PAD.l} x2={W - PAD.r} y1={y(t)} y2={y(t)} />
      <text class="axis" x={PAD.l - 6} y={y(t) + 4} text-anchor="end">{pct(t, 1)}</text>
    {/each}
    <line class="limit crash" x1={PAD.l} x2={W - PAD.r} y1={y(CRASH_LIMIT)} y2={y(CRASH_LIMIT)} />
    <line class="limit anr" x1={PAD.l} x2={W - PAD.r} y1={y(ANR_LIMIT)} y2={y(ANR_LIMIT)} />
    <path class="line crash" d={path("crash")} />
    <path class="line anr" d={path("anr")} />
    {#each series as s, i}
      {#if i % labelEvery === 0}
        <text class="axis" x={x(i)} y={H - 6} text-anchor="middle">{s.day.slice(5)}</text>
      {/if}
    {/each}
  </svg>
  <div class="row small legend">
    <span class="crash">{t("chart.crashes")}</span>
    <span class="anr">{t("chart.anrs")}</span>
    <span class="muted">{t("chart.thresholds")}</span>
  </div>
{/if}

<style>
  svg { width: 100%; height: auto; display: block; }
  .grid { stroke: var(--border); stroke-width: 1; }
  .axis { fill: var(--muted); font-size: 11px; }
  .line { fill: none; stroke-width: 2; }
  .line.crash { stroke: var(--crash); }
  .line.anr { stroke: var(--anr); }
  .limit { stroke-dasharray: 4 4; stroke-width: 1; opacity: 0.7; }
  .limit.crash { stroke: var(--crash); }
  .limit.anr { stroke: var(--anr); }
  .legend .crash { color: var(--crash); }
  .legend .anr { color: var(--anr); }
</style>
