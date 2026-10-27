<script lang="ts">
  import { api, errorText, type AppRow } from "../lib/api";

  let {
    apps = $bindable(),
    onSignedOut,
    email,
  }: { apps: AppRow[]; onSignedOut: () => void; email: string | null } = $props();

  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let busy = $state(false);
  let confirmReset = $state(false);

  async function run(action: () => Promise<void>, ok?: string) {
    busy = true;
    error = null;
    message = null;
    try {
      await action();
      if (ok) message = ok;
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  const refresh = () =>
    run(async () => {
      apps = await api.refreshApps();
    }, "Lista de apps atualizada.");

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

  // Apaga banco e chave desta conta e sai: volta para a tela de login.
  const reset = () =>
    run(async () => {
      await api.resetLocalData();
      confirmReset = false;
      onSignedOut();
    });
</script>

<div class="stack">
  {#if error}<div class="panel error">{error}</div>{/if}
  {#if message}<div class="panel ok">{message}</div>{/if}

  <div class="panel stack">
    <div class="row">
      <h2>Apps</h2>
      <div class="spacer"></div>
      <button onclick={refresh} disabled={busy}>Atualizar lista do Play Console</button>
    </div>
    <p class="small muted">
      Os apps e o projeto Firebase de cada um são encontrados automaticamente na sua conta. Desmarque
      os que não quer acompanhar.
    </p>
    <div class="table-wrap">
      <table>
        <thead>
          <tr><th>Ativo</th><th>App</th><th>Projeto Firebase</th></tr>
        </thead>
        <tbody>
          {#each apps as a (a.packageName)}
            <tr>
              <td>
                <input
                  type="checkbox"
                  checked={a.enabled}
                  disabled={busy}
                  aria-label="Acompanhar {a.packageName}"
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
                  <span class="muted">não encontrado (só dados do Play)</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>

  <div class="panel stack">
    <h2>Conta</h2>
    {#if email}<p>Conectado como <strong>{email}</strong>.</p>{/if}
    <p class="small muted">
      Cada conta Google tem os próprios dados, separados e criptografados. Sair revoga o acesso no
      Google; os dados desta conta continuam guardados para quando você entrar de novo.
    </p>
    <div><button onclick={signOut} disabled={busy}>Sair da conta Google</button></div>
  </div>

  <div class="panel stack">
    <h2>Apagar dados desta conta</h2>
    <p class="small muted">
      Apaga de vez tudo que o Stackquake guardou desta conta neste computador (apps, histórico,
      status, responsáveis e notas) e a chave de criptografia, e sai da conta. Não mexe em nada no
      Google.
    </p>
    {#if !confirmReset}
      <div>
        <button class="danger" onclick={() => (confirmReset = true)} disabled={busy}>Apagar dados…</button>
      </div>
    {:else}
      <div class="row">
        <span class="bad">Tem certeza? Não dá para desfazer.</span>
        <button class="danger" onclick={reset} disabled={busy}>Sim, apagar tudo</button>
        <button onclick={() => (confirmReset = false)}>Cancelar</button>
      </div>
    {/if}
  </div>
</div>
