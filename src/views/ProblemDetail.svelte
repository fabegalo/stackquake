<script lang="ts">
  import { api, errorText, type ProblemDetail, type Status } from "../lib/api";
  import { dateTime, kindLabel, num, statusLabel } from "../lib/format";

  let { id, onClose }: { id: number; onClose: () => void } = $props();

  let detail = $state<ProblemDetail | null>(null);
  let error = $state<string | null>(null);
  let saving = $state(false);
  let fetching = $state(false);
  let savedAt = $state<string | null>(null);

  // Campos editáveis
  let status = $state<Status>("open");
  let assignee = $state("");
  let notes = $state("");
  let resolvedIn = $state("");

  function fill(d: ProblemDetail) {
    detail = d;
    status = d.problem.status;
    assignee = d.problem.assignee ?? "";
    notes = d.problem.notes ?? "";
    resolvedIn = d.problem.resolvedInVersion ?? "";
  }

  async function load() {
    try {
      fill(await api.getProblem(id));
    } catch (e) {
      error = errorText(e);
    }
  }

  async function save() {
    saving = true;
    error = null;
    try {
      await api.updateProblem(id, status, assignee || null, notes || null, resolvedIn || null);
      savedAt = new Date().toLocaleTimeString("pt-BR");
      await load();
    } catch (e) {
      error = errorText(e);
    } finally {
      saving = false;
    }
  }

  async function fetchSamples() {
    fetching = true;
    error = null;
    try {
      fill(await api.fetchProblemSample(id));
    } catch (e) {
      error = errorText(e);
    } finally {
      fetching = false;
    }
  }

  const missingDetails = $derived(
    !!detail &&
      (detail.playIssues.some((i) => !i.sampleReport) ||
        detail.crashlyticsIssues.some((i) => !i.sampleStack) ||
        (detail.crashlyticsIssues.length > 0 && detail.byVersion.length === 0)),
  );

  load();
</script>

<div class="stack">
  <div class="row">
    <button onclick={onClose}>← Voltar</button>
    {#if error}<span class="error">{error}</span>{/if}
  </div>

  {#if detail}
    {@const p = detail.problem}
    <div class="panel">
      <div class="row">
        <span class="badge {p.kind}">{kindLabel[p.kind]}</span>
        <h2 class="title">{p.title}</h2>
      </div>
      <p class="small muted">
        Assinatura: <span class="mono">{p.signature}</span> · criado {dateTime(p.createdAt)} · atualizado {dateTime(p.updatedAt)}
      </p>
    </div>

    <div class="grid">
      <div class="panel stack">
        <h3>Gestão</h3>
        <label>
          Status
          <select bind:value={status}>
            {#each Object.entries(statusLabel) as [value, label]}<option {value}>{label}</option>{/each}
          </select>
        </label>
        <label>
          Responsável
          <input bind:value={assignee} maxlength="200" placeholder="nome ou e-mail" />
        </label>
        <label>
          Corrigido no versionCode
          <input bind:value={resolvedIn} inputmode="numeric" maxlength="20" placeholder="ex.: 412" />
          <span class="small">Se aparecer numa versão maior, o status vira "Regrediu" automaticamente.</span>
        </label>
        <label>
          Notas
          <textarea rows="5" bind:value={notes} maxlength="20000"></textarea>
        </label>
        <div class="row">
          <button class="primary" onclick={save} disabled={saving}>{saving ? "Salvando…" : "Salvar"}</button>
          {#if savedAt}<span class="small ok">salvo às {savedAt}</span>{/if}
        </div>
      </div>

      <div class="panel stack">
        <h3>Por versão (Crashlytics)</h3>
        {#if detail.byVersion.length === 0}
          <p class="muted small">
            {detail.crashlyticsIssues.length > 0
              ? 'Clique em "Buscar detalhes" para ver a distribuição por versão.'
              : "Sem dados do Crashlytics para este problema."}
          </p>
        {:else}
          <table>
            <thead><tr><th>versionCode</th><th>Versão</th><th class="num">Eventos</th><th class="num">Usuários</th></tr></thead>
            <tbody>
              {#each detail.byVersion as v}
                <tr>
                  <td class="mono">{v.buildVersion}</td>
                  <td>{v.displayVersion ?? "—"}</td>
                  <td class="num">{num(v.events)}</td>
                  <td class="num">{num(v.users)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    </div>

    <div class="panel stack">
      <div class="row">
        <h3>Fontes</h3>
        <div class="spacer"></div>
        {#if missingDetails}
          <button onclick={fetchSamples} disabled={fetching}>
            {fetching ? "Buscando…" : "Buscar detalhes (stack e versões)"}
          </button>
        {/if}
      </div>

      {#each detail.playIssues as i (i.issueId)}
        <div class="source">
          <div class="row">
            <strong>Play Console</strong>
            <span class="small muted">
              {num(i.reportCount)} relatórios · {num(i.distinctUsers)} usuários · versões {i.firstVersion ?? "?"} → {i.lastVersion ?? "?"} · último {dateTime(i.lastSeen)}
            </span>
          </div>
          <p class="small mono">{i.cause ?? "?"} · {i.location ?? "?"}</p>
          {#if i.issueUri}<p class="small muted">Abrir no Play Console: <span class="mono selectable">{i.issueUri}</span></p>{/if}
          {#if i.sampleReport}<pre class="stacktrace">{i.sampleReport}</pre>{/if}
        </div>
      {/each}

      {#each detail.crashlyticsIssues as i (i.issueId)}
        <div class="source">
          <div class="row">
            <strong>Crashlytics</strong>
            <span class="small muted">
              versões {i.firstVersion ?? "?"} → {i.lastVersion ?? "?"} · último {dateTime(i.lastSeen)}
            </span>
          </div>
          <p class="small">{i.title}{#if i.subtitle} <span class="muted">— {i.subtitle}</span>{/if}</p>
          {#if i.rootType || i.blameSymbol}<p class="small mono">{i.rootType ?? "?"} · {i.blameSymbol ?? "?"}</p>{/if}
          {#if i.issueUri}<p class="small muted">Abrir no Firebase: <span class="mono selectable">{i.issueUri}</span></p>{/if}
          {#if i.sampleStack}<pre class="stacktrace">{i.sampleStack}</pre>{/if}
        </div>
      {/each}

      {#if detail.playIssues.length === 0 && detail.crashlyticsIssues.length === 0}
        <p class="muted">Este problema não aparece nos dados atuais (pode ser histórico).</p>
      {/if}
    </div>
  {:else if !error}
    <p class="muted">Carregando…</p>
  {/if}
</div>

<style>
  .title { margin: 0; }
  .grid { display: grid; grid-template-columns: minmax(260px, 1fr) minmax(260px, 1fr); gap: 1rem; }
  .source { border-top: 1px solid var(--border); padding-top: 0.75rem; display: flex; flex-direction: column; gap: 0.4rem; }
  .source p { margin: 0; }
  .selectable { user-select: all; }
  @media (max-width: 900px) { .grid { grid-template-columns: 1fr; } }
</style>
