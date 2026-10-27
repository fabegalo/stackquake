<script lang="ts">
  import { api, errorText, type SetupStatus } from "../lib/api";
  import { i18n, LOCALES, setLocale, t, type Locale } from "../lib/i18n/index.svelte";

  let { status, onDone }: { status: SetupStatus; onDone: () => void } = $props();

  let busy = $state(false);
  let error = $state<unknown>(null);

  async function signIn() {
    busy = true;
    error = null;
    try {
      await api.signIn();
      onDone();
    } catch (e) {
      error = e;
    } finally {
      busy = false;
    }
  }
</script>

<main>
  <div class="panel stack card">
    <div class="brand">Stackquake</div>
    <p class="muted">{t("app.tagline")}</p>

    {#if status.hasClient}
      <button class="primary big" onclick={signIn} disabled={busy}>
        {busy ? t("setup.waiting") : t("setup.signIn")}
      </button>
      <p class="small muted">{t("setup.hint")}</p>
    {:else}
      <p class="error">{t("setup.noClient")}</p>
    {/if}

    {#if error}<p class="error">{errorText(error)}</p>{/if}

    <select
      class="small"
      aria-label="Language"
      value={i18n.locale}
      onchange={(e) => setLocale(e.currentTarget.value as Locale)}
    >
      {#each LOCALES as l}<option value={l.id}>{l.label}</option>{/each}
    </select>
  </div>
</main>

<style>
  main { height: 100%; display: grid; place-items: center; padding: 1rem; }
  .card { max-width: 420px; text-align: center; align-items: center; padding: 2rem; }
  .brand { font-weight: 700; font-size: 1.6rem; color: var(--accent); }
  .big { font-size: 1rem; padding: 0.7rem 1.4rem; }
</style>
