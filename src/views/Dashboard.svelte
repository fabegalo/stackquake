<script lang="ts">
  import { api, errorText, rowsOf, type Cell } from "../lib/api";
  import { cell, num, pct } from "../lib/format";
  import TrendChart from "./TrendChart.svelte";

  let { packageName, days }: { packageName: string; days: number } = $props();

  let summary = $state<Record<string, Cell> | null>(null);
  let trend = $state<Record<string, Cell>[]>([]);
  let versions = $state<Record<string, Cell>[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);

  const VERSION_COLUMNS: [string, string][] = [
    ["version_code", "versionCode"],
    ["versao", "Versão"],
    ["crash_percebida", "Falhas percebidas"],
    ["anr_percebida", "ANRs percebidos"],
    ["usuarios_sem_falha", "Usuários sem falha"],
    ["pico_usuarios_dia", "Usuários/dia (pico)"],
    ["crashes_crashlytics", "Falhas (Crashlytics)"],
    ["anrs_crashlytics", "ANRs (Crashlytics)"],
    ["situacao", "Situação"],
  ];

  async function load() {
    loading = true;
    error = null;
    try {
      const [s, t, v] = await Promise.all([
        api.runKpi("01_resumo", packageName, days),
        api.runKpi("03_tendencia_diaria", packageName, days),
        api.runKpi("02_saude_por_versao", packageName, days),
      ]);
      summary = rowsOf(s)[0] ?? null;
      trend = rowsOf(t);
      versions = rowsOf(v);
    } catch (e) {
      error = errorText(e);
    } finally {
      loading = false;
    }
  }

  function over(value: Cell | undefined, limit: Cell | undefined): boolean {
    return typeof value === "number" && typeof limit === "number" && value >= limit;
  }

  load();
</script>

{#if error}
  <div class="panel error">{error}</div>
{:else if loading}
  <p class="muted">Calculando KPIs…</p>
{:else}
  <div class="cards">
    <div class="panel card">
      <h3>Falhas percebidas</h3>
      <div class="big" class:bad={over(summary?.crash_percebida, summary?.limite_crash)}>
        {pct(summary?.crash_percebida)}
      </div>
      <div class="small muted">limite do Play: {pct(summary?.limite_crash)} · geral {pct(summary?.crash_geral)}</div>
    </div>
    <div class="panel card">
      <h3>ANRs percebidos</h3>
      <div class="big" class:bad={over(summary?.anr_percebida, summary?.limite_anr)}>
        {pct(summary?.anr_percebida)}
      </div>
      <div class="small muted">limite do Play: {pct(summary?.limite_anr)} · geral {pct(summary?.anr_geral)}</div>
    </div>
    <div class="panel card">
      <h3>Usuários sem falha</h3>
      <div class="big">{pct(summary?.usuarios_sem_falha_crashlytics)}</div>
      <div class="small muted">Crashlytics · {num(summary?.usuarios_dia)} usuários/dia (Play)</div>
    </div>
    <div class="panel card">
      <h3>Problemas</h3>
      <div class="big">{num(summary?.problemas_abertos)}</div>
      <div class="small" class:bad={Number(summary?.regressoes ?? 0) > 0}>
        abertos · {num(summary?.regressoes)} regressão(ões)
      </div>
    </div>
  </div>

  <div class="panel">
    <h2>Tendência diária</h2>
    <TrendChart rows={trend} />
  </div>

  <div class="panel">
    <h2>Saúde por versão</h2>
    {#if versions.length === 0}
      <p class="muted">Sem dados por versão ainda.</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              {#each VERSION_COLUMNS as [, label]}<th class:num={label !== "Situação" && label !== "Versão"}>{label}</th>{/each}
            </tr>
          </thead>
          <tbody>
            {#each versions as v}
              <tr>
                {#each VERSION_COLUMNS as [key, label]}
                  {#if key === "situacao"}
                    <td>
                      <span class="badge" class:bad={v[key] === "acima do limite"} class:ok={v[key] === "ok"}>{v[key]}</span>
                    </td>
                  {:else}
                    <td class:num={label !== "Versão"}>{cell(v[key], key)}</td>
                  {/if}
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
{/if}

<style>
  .cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1rem; }
  .card h3 { margin-bottom: 0.3rem; }
  .big { font-size: 1.8rem; font-weight: 700; font-variant-numeric: tabular-nums; }
  .big.bad { color: var(--bad); }
</style>
