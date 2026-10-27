<script lang="ts">
  import { api, errorText, type AppRow } from "../lib/api";
  import { i18n, LOCALES, setLocale, t, type Locale } from "../lib/i18n/index.svelte";

  let {
    apps = $bindable(),
    onSignedOut,
    email,
  }: { apps: AppRow[]; onSignedOut: () => void; email: string | null } = $props();

  let error = $state<unknown>(null);
  let message = $state<string | null>(null);
  let busy = $state(false);
  let confirmReset = $state(false);

  async function run(action: () => Promise<void>, okKey?: string) {
    busy = true;
    error = null;
    message = null;
    try {
      await action();
      if (okKey) message = okKey;
    } catch (e) {
      error = e;
    } finally {
      busy = false;
    }
  }

  const refresh = () =>
    run(async () => {
      apps = await api.refreshApps();
    }, "settings.refreshed");

  const toggle = (a: AppRow, enabled: boolean) =>
    run(async () => {
      await api.updateAppSettings(a.packageName, enabled);
      apps = await api.listApps();
    });

  const signOut = () =>
    run(async () => {
      await api.signOut();
      onSignedOut();
    });

  // Deletes this account's database and key, then signs out (back to sign-in).
  const reset = () =>
    run(async () => {
      await api.resetLocalData();
      confirmReset = false;
      onSignedOut();
    });
</script>

<div class="stack">
  {#if error}<div class="panel error">{errorText(error)}</div>{/if}
  {#if message}<div class="panel ok">{t(message)}</div>{/if}

  <div class="panel stack">
    <div class="row">
      <h2>{t("settings.apps")}</h2>
      <div class="spacer"></div>
      <button onclick={refresh} disabled={busy}>{t("settings.refresh")}</button>
    </div>
    <p class="small muted">{t("settings.appsHint")}</p>
    <div class="table-wrap">
      <table>
        <thead>
          <tr><th>{t("settings.active")}</th><th>{t("settings.app")}</th><th>{t("settings.firebaseProject")}</th></tr>
        </thead>
        <tbody>
          {#each apps as a (a.packageName)}
            <tr>
              <td>
                <input
                  type="checkbox"
                  checked={a.enabled}
                  disabled={busy}
                  aria-label={t("settings.track", { pkg: a.packageName })}
                  onchange={(e) => toggle(a, e.currentTarget.checked)}
                />
              </td>
              <td>
                <div>{a.displayName ?? a.packageName}</div>
                <div class="small muted mono">{a.packageName}</div>
              </td>
              <td class="small">
                {#if a.firebaseProject}
                  <span class="mono">{a.firebaseProject}</span>
                {:else}
                  <span class="muted">{t("settings.notFound")}</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>

  <div class="panel stack">
    <h2>{t("settings.language")}</h2>
    <div>
      <select
        aria-label={t("settings.language")}
        value={i18n.locale}
        onchange={(e) => setLocale(e.currentTarget.value as Locale)}
      >
        {#each LOCALES as l}<option value={l.id}>{l.label}</option>{/each}
      </select>
    </div>
  </div>

  <div class="panel stack">
    <h2>{t("settings.account")}</h2>
    {#if email}<p>{t("settings.signedInAs", { email })}</p>{/if}
    <p class="small muted">{t("settings.accountHint")}</p>
    <div><button onclick={signOut} disabled={busy}>{t("settings.signOut")}</button></div>
  </div>

  <div class="panel stack">
    <h2>{t("settings.deleteTitle")}</h2>
    <p class="small muted">{t("settings.deleteHint")}</p>
    {#if !confirmReset}
      <div>
        <button class="danger" onclick={() => (confirmReset = true)} disabled={busy}>{t("settings.delete")}</button>
      </div>
    {:else}
      <div class="row">
        <span class="bad">{t("settings.deleteConfirm")}</span>
        <button class="danger" onclick={reset} disabled={busy}>{t("settings.deleteYes")}</button>
        <button onclick={() => (confirmReset = false)}>{t("settings.cancel")}</button>
      </div>
    {/if}
  </div>
</div>
