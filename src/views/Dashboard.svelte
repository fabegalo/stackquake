<script lang="ts">
  import { api, errorText, rowsOf, type Cell } from "../lib/api";
  import { cell, columnLabel, num, pct } from "../lib/format";
  import { t } from "../lib/i18n/index.svelte";
  import TrendChart from "./TrendChart.svelte";

  let { packageName, days }: { packageName: string; days: number } = $props();

  let summary = $state<Record<string, Cell> | null>(null);
  let trend = $state<Record<string, Cell>[]>([]);
  let versions = $state<Record<string, Cell>[]>([]);
  let error = $state<unknown>(null);
  let loading = $state(true);

  const VERSION_COLUMNS = [
    "version_code",
    "version_name",
    "perceived_crash_rate",
    "perceived_anr_rate",
    "crash_free_users",
    "peak_daily_users",
    "crashlytics_crashes",
    "crashlytics_anrs",
    "health",
  ];
  const TEXT_COLUMNS = new Set(["version_name", "health"]);

  async function load() {
    loading = true;
    error = null;
    try {
      const [s, tr, v] = await Promise.all([
        api.runKpi("01_summary", packageName, days),
        api.runKpi("03_daily_trend", packageName, days),
        api.runKpi("02_version_health", packageName, days),
      ]);
      summary = rowsOf(s)[0] ?? null;
      trend = rowsOf(tr);
      versions = rowsOf(v);
    } catch (e) {
      error = e;
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
  <div class="panel error">{errorText(error)}</div>
{:else if loading}
  <p class="muted">{t("dashboard.computing")}</p>
{:else}
  <div class="cards">
    <div class="panel card">
      <h3>{t("dashboard.perceivedCrashes")}</h3>
      <div class="big" class:bad={over(summary?.perceived_crash_rate, summary?.crash_threshold)}>
        {pct(summary?.perceived_crash_rate)}
      </div>
      <div class="small muted">
        {t("dashboard.playThreshold", { threshold: pct(summary?.crash_threshold), overall: pct(summary?.overall_crash_rate) })}
      </div>
    </div>
    <div class="panel card">
      <h3>{t("dashboard.perceivedAnrs")}</h3>
      <div class="big" class:bad={over(summary?.perceived_anr_rate, summary?.anr_threshold)}>
        {pct(summary?.perceived_anr_rate)}
      </div>
      <div class="small muted">
        {t("dashboard.playThreshold", { threshold: pct(summary?.anr_threshold), overall: pct(summary?.overall_anr_rate) })}
      </div>
    </div>
    <div class="panel card">
      <h3>{t("dashboard.crashFree")}</h3>
      <div class="big">{pct(summary?.crash_free_users)}</div>
      <div class="small muted">{t("dashboard.crashFreeHint", { users: num(summary?.daily_users) })}</div>
    </div>
    <div class="panel card">
      <h3>{t("dashboard.problems")}</h3>
      <div class="big">{num(summary?.open_problems)}</div>
      <div class="small" class:bad={Number(summary?.regressions ?? 0) > 0}>
        {t("dashboard.problemsHint", { n: num(summary?.regressions) })}
      </div>
    </div>
  </div>

  <div class="panel">
    <h2>{t("dashboard.dailyTrend")}</h2>
    <TrendChart rows={trend} />
  </div>

  <div class="panel">
    <h2>{t("dashboard.versionHealth")}</h2>
    {#if versions.length === 0}
      <p class="muted">{t("dashboard.noVersionData")}</p>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              {#each VERSION_COLUMNS as key}<th class:num={!TEXT_COLUMNS.has(key)}>{columnLabel(key)}</th>{/each}
            </tr>
          </thead>
          <tbody>
            {#each versions as v}
              <tr>
                {#each VERSION_COLUMNS as key}
                  {#if key === "health"}
                    <td>
                      <span class="badge" class:bad={v[key] === "over_threshold"} class:ok={v[key] === "ok"}>
                        {cell(v[key], key)}
                      </span>
                    </td>
                  {:else}
                    <td class:num={!TEXT_COLUMNS.has(key)}>{cell(v[key], key)}</td>
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
