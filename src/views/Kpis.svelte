<script lang="ts">
  import { api, errorText, type KpiInfo, type KpiResult } from "../lib/api";
  import { cell } from "../lib/format";

  let { packageName, days, version }: { packageName: string; days: number; version: number } = $props();

  let kpis = $state<KpiInfo[]>([]);
  let selected = $state<string | null>(null);
  let result = $state<KpiResult | null>(null);
  let error = $state<string | null>(null);
  let running = $state(false);

  const current = $derived(kpis.find((k) => k.id === selected) ?? null);

  async function loadList() {
    try {
      kpis = await api.listKpis();
      if (!selected || !kpis.some((k) => k.id === selected)) selected = kpis[0]?.id ?? null;
    } catch (e) {
      error = errorText(e);
    }
  }

  async function run() {
    if (!selected) return;
    running = true;
    error = null;
    try {
      result = await api.runKpi(selected, packageName, days);
    } catch (e) {
      result = null;
      error = errorText(e);
    } finally {
      running = false;
    }
  }

  $effect(() => {
    void [selected, packageName, days, version];
    run();
  });

  loadList();
</script>

<div class="layout">
  <div class="panel list">
    <div class="row">
      <h3>KPIs</h3>
      <div class="spacer"></div>
      <button class="link small" onclick={loadList}>recarregar</button>
    </div>
    {#each kpis as k (k.id)}
      <button class="item" class:active={k.id === selected} onclick={() => (selected = k.id)}>
        {k.name}
        {#if k.source === "user"}<span class="badge small">seu</span>{/if}
      </button>
    {/each}
    <button class="small" onclick={() => api.openKpiFolder().catch((e) => (error = errorText(e)))}>
      Abrir pasta de KPIs
    </button>
    <p class="small muted">
      Cada KPI é um arquivo .sql. Copie um daqui do repositório (src-tauri/kpis) para a pasta, edite e
      clique em recarregar. Não precisa recompilar.
    </p>
  </div>

  <div class="panel stack">
    {#if current}
      <div>
        <h2>{current.name}</h2>
        {#if current.description}<p class="muted small">{current.description}</p>{/if}
      </div>
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {:else if running && !result}
      <p class="muted">Executando…</p>
    {:else if result}
      {#if result.rows.length === 0}
        <p class="muted">Sem resultados.</p>
      {:else}
        <div class="table-wrap">
          <table>
            <thead><tr>{#each result.columns as c}<th>{c}</th>{/each}</tr></thead>
            <tbody>
              {#each result.rows as row}
                <tr>
                  {#each row as v, i}
                    <td class:num={typeof v === "number"}>{cell(v, result.columns[i] ?? "")}</td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        {#if result.truncated}<p class="small warn">Mostrando as primeiras 5.000 linhas.</p>{/if}
      {/if}
    {/if}
  </div>
</div>

<style>
  .layout { display: grid; grid-template-columns: 240px 1fr; gap: 1rem; align-items: start; }
  .list { display: flex; flex-direction: column; gap: 0.3rem; }
  .item { text-align: left; border-color: transparent; background: transparent; }
  .item.active { background: var(--panel-2); border-color: var(--border); font-weight: 600; }
</style>
