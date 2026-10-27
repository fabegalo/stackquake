<script lang="ts">
  import { api, errorText, type SetupStatus } from "../lib/api";

  let { status, onDone }: { status: SetupStatus; onDone: () => void } = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  async function signIn() {
    busy = true;
    error = null;
    try {
      await api.signIn();
      onDone();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

<main>
  <div class="panel stack card">
    <div class="brand">Stackquake</div>
    <p class="muted">
      Falhas e ANRs do Play Console e do Firebase Crashlytics em um só lugar.
    </p>

    {#if status.hasClient}
      <button class="primary big" onclick={signIn} disabled={busy}>
        {busy ? "Continue no navegador…" : "Entrar com Google"}
      </button>
      <p class="small muted">
        Use a mesma conta do Play Console. O acesso é só de leitura e os dados ficam apenas neste
        computador, criptografados.
      </p>
    {:else}
      <p class="error">
        Este build não tem o login do Google configurado. Se você compilou o Stackquake, veja
        docs/DESENVOLVIMENTO.md.
      </p>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}
  </div>
</main>

<style>
  main { height: 100%; display: grid; place-items: center; padding: 1rem; }
  .card { max-width: 420px; text-align: center; align-items: center; padding: 2rem; }
  .brand { font-weight: 700; font-size: 1.6rem; color: var(--accent); }
  .big { font-size: 1rem; padding: 0.7rem 1.4rem; }
</style>
