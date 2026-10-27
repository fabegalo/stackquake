<script lang="ts">
  import { api, errorText, type AppRow, type SetupStatus, type SyncReport } from "./lib/api";
  import { dateTime } from "./lib/format";
  import { t } from "./lib/i18n/index.svelte";
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
  // Raw error, translated at render time (follows language changes).
  let error = $state<unknown>(null);
  // Bumped on every sync so views reload.
  let version = $state(0);

  const current = $derived(apps.find((a) => a.packageName === selected) ?? null);
  const enabledApps = $derived(apps.filter((a) => a.enabled));

  async function loadStatus() {
    try {
      status = await api.setupStatus();
      if (status.signedIn) await loadApps();
    } catch (e) {
      error = e;
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
      error = e;
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
    {#if error}<p class="error">{errorText(error)}</p>{:else}<p class="muted">{t("app.loading")}</p>{/if}
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
        <button class:active={view === "dashboard"} onclick={() => (view = "dashboard")}>{t("nav.dashboard")}</button>
        <button class:active={view === "problems"} onclick={() => (view = "problems")}>{t("nav.problems")}</button>
        <button class:active={view === "kpis"} onclick={() => (view = "kpis")}>{t("nav.kpis")}</button>
        <button class:active={view === "settings"} onclick={() => (view = "settings")}>{t("nav.settings")}</button>
      </nav>
      <div class="spacer"></div>
      {#if status.email}<p class="small muted account">{status.email}</p>{/if}
      <p class="small muted">{t("app.privacyNote")}</p>
    </aside>

    <section class="main">
      <header class="row">
        <div>
          <h1>{current?.displayName ?? current?.packageName ?? t("header.noApp")}</h1>
          {#if current}
            <span class="small muted mono">{current.packageName}</span>
            <span class="small muted"> · {t("header.lastSync", { when: dateTime(current.lastSyncAt) })}</span>
          {/if}
        </div>
        <div class="spacer"></div>
        <label class="inline">
          {t("header.window")}
          <select bind:value={days}>
            {#each [7, 28, 90] as n}<option value={n}>{t("header.days", { n })}</option>{/each}
          </select>
        </label>
        <button class="primary" onclick={sync} disabled={syncing || !selected}>
          {syncing ? t("header.syncing") : t("header.sync")}
        </button>
      </header>

      {#if error}<div class="panel error">{errorText(error)}</div>{/if}
      {#if report}
        <div class="panel small">
          <div class="row">
            <span>{t("sync.vitals", { n: report.vitalsRows })}</span>
            <span>{t("sync.playIssues", { n: report.playIssues })}</span>
            <span>
              {t("sync.crashlytics", { issues: report.crashlyticsIssues, versions: report.crashlyticsVersions })}
            </span>
            {#if report.regressions > 0}<span class="bad">{t("sync.regressions", { n: report.regressions })}</span>{/if}
            <div class="spacer"></div>
            <button class="link" onclick={() => (report = null)}>{t("header.close")}</button>
          </div>
          {#each report.warnings as w}
            <p class="warn">{t(`sync.source.${w.source}`)}: {errorText(w.error)}</p>
          {/each}
        </div>
      {/if}

      {#if !selected}
        <div class="panel">
          <p>{t("empty.noApps")}</p>
          <button onclick={() => (view = "settings")}>{t("empty.openSettings")}</button>
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
