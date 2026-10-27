<script lang="ts">
  import { api, errorText, type AppRow, type SetupStatus, type SyncReport } from "./lib/api";
  import { dateTime } from "./lib/format";
  import Setup from "./views/Setup.svelte";
  import Dashboard from "./views/Dashboard.svelte";
  import Problems from "./views/Problems.svelte";
  import Kpis from "./views/Kpis.svelte";
  import Settings from "./views/Settings.svelte";

  type View = "dashboard" | "problems" | "kpis" | "settings";

  let status = $state<SetupStatus | null>(null);
  let apps = $state<AppRow[]>([]);
  let selected = $state<string | null>(null);
  let view = $state<View>("dashboard");
  let days = $state(28);
  let syncing = $state(false);
  let report = $state<SyncReport | null>(null);
  let error = $state<string | null>(null);
  // Incrementa a cada sincronização para as telas recarregarem.
  let version = $state(0);

  const current = $derived(apps.find((a) => a.packageName === selected) ?? null);
  const enabledApps = $derived(apps.filter((a) => a.enabled));

  async function loadStatus() {
    try {
      status = await api.setupStatus();
      if (status.signedIn) await loadApps();
    } catch (e) {
      error = errorText(e);
    }
  }

  async function loadApps() {
    apps = await api.listApps();
    if (apps.length === 0) apps = await api.refreshApps();
    if (!selected || !apps.some((a) => a.packageName === selected)) {
      selected = enabledApps[0]?.packageName ?? apps[0]?.packageName ?? null;
    }
  }

  async function sync() {
    if (!selected) return;
    syncing = true;
    error = null;
    report = null;
    try {
      report = await api.syncApp(selected);
      apps = await api.listApps();
      version++;
    } catch (e) {
      error = errorText(e);
    } finally {
      syncing = false;
    }
  }

  async function onSignedOut() {
    apps = [];
    selected = null;
    await loadStatus();
  }

  loadStatus();
</script>

{#if status === null}
  <main class="center">
    {#if error}<p class="error">{error}</p>{:else}<p class="muted">Carregando…</p>{/if}
  </main>
{:else if !status.hasClient || !status.signedIn}
  <Setup {status} onDone={loadStatus} />
{:else}
  <div class="shell">
    <aside>
      <div class="brand">Stackquake</div>
      <label>
        App
        <select bind:value={selected}>
          {#each enabledApps as a (a.packageName)}
            <option value={a.packageName}>{a.displayName ?? a.packageName}</option>
          {/each}
        </select>
      </label>
      <nav>
        <button class:active={view === "dashboard"} onclick={() => (view = "dashboard")}>Painel</button>
        <button class:active={view === "problems"} onclick={() => (view = "problems")}>Problemas</button>
        <button class:active={view === "kpis"} onclick={() => (view = "kpis")}>KPIs</button>
        <button class:active={view === "settings"} onclick={() => (view = "settings")}>Configurações</button>
      </nav>
      <div class="spacer"></div>
      {#if status.email}<p class="small muted account">{status.email}</p>{/if}
      <p class="small muted">Dados só neste computador, criptografados e separados por conta.</p>
    </aside>

    <section class="main">
      <header class="row">
        <div>
          <h1>{current?.displayName ?? current?.packageName ?? "Nenhum app"}</h1>
          {#if current}
            <span class="small muted mono">{current.packageName}</span>
            <span class="small muted"> · última sincronização: {dateTime(current.lastSyncAt)}</span>
          {/if}
        </div>
        <div class="spacer"></div>
        <label class="inline">
          Janela
          <select bind:value={days}>
            <option value={7}>7 dias</option>
            <option value={28}>28 dias</option>
            <option value={90}>90 dias</option>
          </select>
        </label>
        <button class="primary" onclick={sync} disabled={syncing || !selected}>
          {syncing ? "Sincronizando…" : "Sincronizar"}
        </button>
      </header>

      {#if error}<div class="panel error">{error}</div>{/if}
      {#if report}
        <div class="panel small">
          <div class="row">
            <span>Play Vitals: {report.vitalsRows} linhas</span>
            <span>Issues do Play: {report.playIssues}</span>
            <span>Crashlytics: {report.crashlyticsIssues} issues, {report.crashlyticsVersions} versões</span>
            {#if report.regressions > 0}<span class="bad">{report.regressions} regressão(ões)</span>{/if}
            <div class="spacer"></div>
            <button class="link" onclick={() => (report = null)}>fechar</button>
          </div>
          {#each report.warnings as w}<p class="warn">{w}</p>{/each}
        </div>
      {/if}

      {#if !selected}
        <div class="panel">
          <p>Nenhum app encontrado na sua conta do Play Console.</p>
          <button onclick={() => (view = "settings")}>Abrir configurações</button>
        </div>
      {:else if view === "dashboard"}
        {#key `${selected}-${days}-${version}`}<Dashboard packageName={selected} {days} />{/key}
      {:else if view === "problems"}
        {#key `${selected}-${version}`}<Problems packageName={selected} {days} />{/key}
      {:else if view === "kpis"}
        <Kpis packageName={selected} {days} {version} />
      {:else}
        <Settings bind:apps onSignedOut={onSignedOut} email={status.email} />
      {/if}
    </section>
  </div>
{/if}

<style>
  .center { height: 100%; display: grid; place-items: center; }
  .shell { display: grid; grid-template-columns: 220px 1fr; height: 100%; }
  aside {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    border-right: 1px solid var(--border);
    background: var(--panel);
  }
  .brand { font-weight: 700; font-size: 1.15rem; color: var(--accent); }
  .account { overflow-wrap: anywhere; margin: 0; }
  nav { display: flex; flex-direction: column; gap: 0.25rem; }
  nav button { text-align: left; border-color: transparent; background: transparent; }
  nav button.active { background: var(--panel-2); border-color: var(--border); font-weight: 600; }
  .main { overflow: auto; padding: 1.25rem; display: flex; flex-direction: column; gap: 1rem; }
  header { align-items: flex-end; }
  header h1 { margin-bottom: 0.15rem; }
  .inline { flex-direction: row; align-items: center; }
</style>
