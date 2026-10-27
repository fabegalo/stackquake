<script lang="ts">
  // Gráfico de linhas em SVG puro (sem biblioteca externa): taxas percebidas
  // de falha e ANR por dia, com as linhas de limite do Play.
  import type { Cell } from "../lib/api";
  import { pct } from "../lib/format";

  let { rows }: { rows: Record<string, Cell>[] } = $props();

  const W = 760;
  const H = 220;
  const PAD = { l: 52, r: 12, t: 12, b: 26 };
  const CRASH_LIMIT = 0.0109;
  const ANR_LIMIT = 0.0047;

  const series = $derived(
    rows.map((r) => ({
      day: String(r.day ?? ""),
      crash: typeof r.crash_percebida === "number" ? r.crash_percebida : null,
      anr: typeof r.anr_percebida === "number" ? r.anr_percebida : null,
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
  <p class="muted">Sem dados diários do Play ainda. Clique em Sincronizar.</p>
{:else}
  <svg viewBox="0 0 {W} {H}" role="img" aria-label="Taxas diárias percebidas de falha e ANR">
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
    <span class="crash">━ Falhas percebidas</span>
    <span class="anr">━ ANRs percebidos</span>
    <span class="muted">┄ limites do Play (1,09% e 0,47%)</span>
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
