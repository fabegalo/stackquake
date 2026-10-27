<script lang="ts">
  import { api, errorText, type Kind, type ProblemRow, type Status } from "../lib/api";
  import { KINDS, STATUSES, kindLabel, num, sourceLabel, statusLabel } from "../lib/format";
  import { t } from "../lib/i18n/index.svelte";
  import ProblemDetail from "./ProblemDetail.svelte";

  let { packageName, days }: { packageName: string; days: number } = $props();

  let rows = $state<ProblemRow[]>([]);
  let status = $state<Status | "">("");
  let kind = $state<Kind | "">("");
  let search = $state("");
  let openId = $state<number | null>(null);
  let error = $state<unknown>(null);
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
      error = e;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Reload when filters or the window change.
    void [status, kind, days];
    load();
  });
</script>

{#if openId !== null}
  <ProblemDetail id={openId} onClose={() => { openId = null; load(); }} />
{:else}
  <div class="panel stack">
    <div class="row">
      <input type="search" placeholder={t("problems.search")} bind:value={search} />
      <select bind:value={kind} aria-label={t("problems.col.kind")}>
        <option value="">{t("problems.allKinds")}</option>
        {#each KINDS as k}<option value={k}>{kindLabel(k)}</option>{/each}
      </select>
      <select bind:value={status} aria-label={t("problems.col.status")}>
        <option value="">{t("problems.allStatuses")}</option>
        {#each STATUSES as s}<option value={s}>{statusLabel(s)}</option>{/each}
      </select>
      <div class="spacer"></div>
      <span class="small muted">{t("problems.count", { n: filtered.length })}</span>
    </div>

    {#if error}
      <p class="error">{errorText(error)}</p>
    {:else if loading}
      <p class="muted">{t("problems.loading")}</p>
    {:else if filtered.length === 0}
      <p class="muted">{t("problems.empty")}</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>{t("problems.col.kind")}</th>
              <th>{t("problems.col.problem")}</th>
              <th>{t("problems.col.status")}</th>
              <th>{t("problems.col.sources")}</th>
              <th class="num">{t("problems.col.playUsers")}</th>
              <th class="num">{t("problems.col.clUsers")}</th>
              <th class="num">{t("problems.col.clEvents")}</th>
              <th class="num">{t("problems.col.versions")}</th>
            </tr>
          </thead>
          <tbody>
            {#each filtered as r (r.id)}
              <tr class="clickable" onclick={() => (openId = r.id)}>
                <td><span class="badge {r.kind}">{kindLabel(r.kind)}</span></td>
                <td>
                  <div>{r.title}</div>
                  {#if r.assignee}<div class="small muted">{r.assignee}</div>{/if}
                </td>
                <td><span class="badge {r.status}">{statusLabel(r.status)}</span></td>
                <td class="small">{sourceLabel(r.sources)}</td>
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
