<script lang="ts">
  import { api, errorText, type ProblemDetail, type Status } from "../lib/api";
  import { STATUSES, dateTime, kindLabel, num, statusLabel } from "../lib/format";
  import { intlLocale, t } from "../lib/i18n/index.svelte";

  let { id, onClose }: { id: number; onClose: () => void } = $props();

  let detail = $state<ProblemDetail | null>(null);
  let error = $state<unknown>(null);
  let saving = $state(false);
  let fetching = $state(false);
  let savedAt = $state<Date | null>(null);

  // Editable fields
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
      error = e;
    }
  }

  async function save() {
    saving = true;
    error = null;
    try {
      await api.updateProblem(id, status, assignee || null, notes || null, resolvedIn || null);
      savedAt = new Date();
      await load();
    } catch (e) {
      error = e;
    } finally {
      saving = false;
    }
  }

  async function fetchDetails() {
    fetching = true;
    error = null;
    try {
      fill(await api.fetchProblemSample(id));
    } catch (e) {
      error = e;
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
    <button onclick={onClose}>{t("detail.back")}</button>
    {#if error}<span class="error">{errorText(error)}</span>{/if}
  </div>

  {#if detail}
    {@const p = detail.problem}
    <div class="panel">
      <div class="row">
        <span class="badge {p.kind}">{kindLabel(p.kind)}</span>
        <h2 class="title">{p.title}</h2>
      </div>
      <p class="small muted">
        {t("detail.signature")}: <span class="mono">{p.signature}</span> ·
        {t("detail.created", { when: dateTime(p.createdAt) })} ·
        {t("detail.updated", { when: dateTime(p.updatedAt) })}
      </p>
    </div>

    <div class="grid">
      <div class="panel stack">
        <h3>{t("detail.management")}</h3>
        <label>
          {t("detail.status")}
          <select bind:value={status}>
            {#each STATUSES as s}<option value={s}>{statusLabel(s)}</option>{/each}
          </select>
        </label>
        <label>
          {t("detail.assignee")}
          <input bind:value={assignee} maxlength="200" placeholder={t("detail.assigneePlaceholder")} />
        </label>
        <label>
          {t("detail.fixedIn")}
          <input bind:value={resolvedIn} inputmode="numeric" maxlength="20" placeholder={t("detail.fixedInPlaceholder")} />
          <span class="small">{t("detail.fixedInHint")}</span>
        </label>
        <label>
          {t("detail.notes")}
          <textarea rows="5" bind:value={notes} maxlength="20000"></textarea>
        </label>
        <div class="row">
          <button class="primary" onclick={save} disabled={saving}>
            {saving ? t("detail.saving") : t("detail.save")}
          </button>
          {#if savedAt}
            <span class="small ok">{t("detail.savedAt", { time: savedAt.toLocaleTimeString(intlLocale()) })}</span>
          {/if}
        </div>
      </div>

      <div class="panel stack">
        <h3>{t("detail.byVersion")}</h3>
        {#if detail.byVersion.length === 0}
          <p class="muted small">
            {detail.crashlyticsIssues.length > 0 ? t("detail.byVersionFetch") : t("detail.byVersionNone")}
          </p>
        {:else}
          <table>
            <thead>
              <tr>
                <th>{t("detail.versionCode")}</th>
                <th>{t("detail.version")}</th>
                <th class="num">{t("detail.events")}</th>
                <th class="num">{t("detail.users")}</th>
              </tr>
            </thead>
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
        <h3>{t("detail.sources")}</h3>
        <div class="spacer"></div>
        {#if missingDetails}
          <button onclick={fetchDetails} disabled={fetching}>
            {fetching ? t("detail.fetching") : t("detail.fetch")}
          </button>
        {/if}
      </div>

      {#each detail.playIssues as i (i.issueId)}
        <div class="source">
          <div class="row">
            <strong>Play Console</strong>
            <span class="small muted">
              {t("detail.playSummary", {
                reports: num(i.reportCount),
                users: num(i.distinctUsers),
                first: i.firstVersion ?? "?",
                last: i.lastVersion ?? "?",
                when: dateTime(i.lastSeen),
              })}
            </span>
          </div>
          <p class="small mono">{i.cause ?? "?"} · {i.location ?? "?"}</p>
          {#if i.issueUri}
            <p class="small muted">{t("detail.openInPlay")} <span class="mono selectable">{i.issueUri}</span></p>
          {/if}
          {#if i.sampleReport}<pre class="stacktrace">{i.sampleReport}</pre>{/if}
        </div>
      {/each}

      {#each detail.crashlyticsIssues as i (i.issueId)}
        <div class="source">
          <div class="row">
            <strong>Crashlytics</strong>
            <span class="small muted">
              {t("detail.clSummary", {
                first: i.firstVersion ?? "?",
                last: i.lastVersion ?? "?",
                when: dateTime(i.lastSeen),
              })}
            </span>
          </div>
          <p class="small">{i.title}{#if i.subtitle} <span class="muted">— {i.subtitle}</span>{/if}</p>
          {#if i.rootType || i.blameSymbol}<p class="small mono">{i.rootType ?? "?"} · {i.blameSymbol ?? "?"}</p>{/if}
          {#if i.issueUri}
            <p class="small muted">{t("detail.openInFirebase")} <span class="mono selectable">{i.issueUri}</span></p>
          {/if}
          {#if i.sampleStack}<pre class="stacktrace">{i.sampleStack}</pre>{/if}
        </div>
      {/each}

      {#if detail.playIssues.length === 0 && detail.crashlyticsIssues.length === 0}
        <p class="muted">{t("detail.historical")}</p>
      {/if}
    </div>
  {:else if !error}
    <p class="muted">{t("app.loading")}</p>
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
