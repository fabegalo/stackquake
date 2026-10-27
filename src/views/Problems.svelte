<script lang="ts">
  import { api, errorText, type Kind, type ProblemRow, type Status } from "../lib/api";
  import { kindLabel, num, sourceLabel, statusLabel } from "../lib/format";
  import ProblemDetail from "./ProblemDetail.svelte";

  let { packageName, days }: { packageName: string; days: number } = $props();

  let rows = $state<ProblemRow[]>([]);
  let status = $state<Status | "">("");
  let kind = $state<Kind | "">("");
  let search = $state("");
  let openId = $state<number | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);

  const filtered = $derived(
    search.trim() === ""
      ? rows
      : rows.filter((r) => r.title.toLowerCase().includes(search.trim().toLowerCase())),
  );

  async function load() {
    loading = true;
    error = null;
    try {
      rows = await api.listProblems(packageName, days, status || null, kind || null);
    } catch (e) {
      error = errorText(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Recarrega quando filtros ou janela mudam.
    void [status, kind, days];
    load();
  });
</script>

{#if openId !== null}
  <ProblemDetail id={openId} onClose={() => { openId = null; load(); }} />
{:else}
  <div class="panel stack">
    <div class="row">
      <input type="search" placeholder="Buscar por título…" bind:value={search} />
      <select bind:value={kind} aria-label="Tipo">
        <option value="">Todos os tipos</option>
        <option value="crash">Falhas</option>
        <option value="anr">ANRs</option>
        <option value="nonfatal">Não fatais</option>
      </select>
      <select bind:value={status} aria-label="Status">
        <option value="">Todos os status</option>
        {#each Object.entries(statusLabel) as [value, label]}<option {value}>{label}</option>{/each}
      </select>
      <div class="spacer"></div>
      <span class="small muted">{filtered.length} problema(s) · ordenados por usuários afetados</span>
    </div>

    {#if error}
      <p class="error">{error}</p>
    {:else if loading}
      <p class="muted">Carregando…</p>
    {:else if filtered.length === 0}
      <p class="muted">Nenhum problema. Sincronize o app para trazer os dados.</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>Tipo</th>
              <th>Problema</th>
              <th>Status</th>
              <th>Fontes</th>
              <th class="num">Usuários (Play)</th>
              <th class="num">Usuários (Crashlytics)</th>
              <th class="num">Eventos (Crashlytics)</th>
              <th class="num">Versões</th>
            </tr>
          </thead>
          <tbody>
            {#each filtered as r (r.id)}
              <tr class="clickable" onclick={() => (openId = r.id)}>
                <td><span class="badge {r.kind}">{kindLabel[r.kind]}</span></td>
                <td>
                  <div>{r.title}</div>
                  {#if r.assignee}<div class="small muted">{r.assignee}</div>{/if}
                </td>
                <td><span class="badge {r.status}">{statusLabel[r.status]}</span></td>
                <td class="small">{sourceLabel[r.sources]}</td>
                <td class="num">{num(r.playUsers)}</td>
                <td class="num">{num(r.clUsers)}</td>
                <td class="num">{num(r.clEvents)}</td>
                <td class="num small">
                  {r.firstVersion ?? "?"}{r.lastVersion !== null && r.lastVersion !== r.firstVersion ? ` → ${r.lastVersion}` : ""}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
{/if}
