# Stackquake

Gestão unificada de **falhas e ANRs** de apps Android: junta o **Google Play Console (Android Vitals)** e o **Firebase Crashlytics** num único app desktop para **Windows, macOS e Linux**, com KPIs por app e por versão.

- **100% software livre** (Apache-2.0), sem servidor, sem telemetria.
- **Só "Entrar com Google"**: sem chaves, JSON ou IDs para configurar; apps e projetos Firebase são descobertos sozinhos.
- **Funciona no plano Spark (gratuito) do Firebase, sem BigQuery**: usa a Play Developer Reporting API e a API do Crashlytics, ambas gratuitas.
- **Segurança em primeiro lugar**: tokens só no chaveiro do sistema, banco local criptografado, interface sem acesso à rede. Veja [SECURITY.md](SECURITY.md).
- **Fácil de mexer**: cada KPI é um arquivo `.sql`, cada fonte de dados é um módulo isolado.

## O que ele faz

| | |
|---|---|
| **Problemas unificados** | A mesma falha do Play e do Crashlytics vira um único problema (assinatura `tipo|Exceção|Classe.método`). Impacto estimado **sem contar em dobro**. |
| **Gestão** | Status (aberto, investigando, resolvido, ignorado), responsável e notas. **Regressão automática**: um problema resolvido no versionCode N que reaparece em uma versão maior volta para "Regrediu". |
| **KPIs** | Taxas percebidas de falha e ANR comparadas aos limites de mau comportamento do Play (1,09% / 0,47%), saúde por versão, tendência diária, top problemas, novos na última versão, cobertura das fontes. |
| **Histórico** | As APIs só mostram os últimos 90 dias; o Stackquake guarda os dados localmente, então o histórico continua crescendo. |
| **Várias contas** | Cada conta Google tem os próprios dados, separados e criptografados com chave própria. |

## Como funciona

```
Interface (Svelte/TS)   ── só exibe dados; CSP sem rede; IPC filtrado (Isolation)
        │  16 comandos tipados, um por permissão
Núcleo (Rust)
  ├─ auth.rs     OAuth desktop: navegador + loopback 127.0.0.1 + PKCE S256 + state
  ├─ secrets.rs  chaveiro do SO (Keychain / Credential Manager / Secret Service)
  ├─ http.rs     HTTPS só para hosts da lista, métodos por host (Firebase: só GET)
  ├─ google/     play.rs (Reporting API) · firebase.rs (descoberta) · crashlytics.rs (API do Crashlytics)
  ├─ sync.rs     sincronização incremental, falha isolada por fonte
  ├─ unify.rs    assinatura que liga as duas fontes (heurística testada)
  ├─ db.rs       SQLite + SQLCipher (AES-256), um banco e uma chave por conta
  └─ kpi.rs      KPIs .sql em conexão só leitura com autorizador
```

## Começando

**Para usar:** abra o app e clique em **Entrar com Google**. Os apps e projetos Firebase da conta são encontrados sozinhos: [docs/SETUP.md](docs/SETUP.md).

**Para desenvolver:**

```bash
npm install
npm run tauri dev
```

Para o login funcionar no seu build, registre o app no Google uma vez: [docs/DESENVOLVIMENTO.md](docs/DESENVOLVIMENTO.md).

## Criando seus KPIs

Em **KPIs → Abrir pasta de KPIs**, crie um arquivo `.sql`:

```sql
-- name: ANRs por versão nos últimos dias
-- description: Eventos de ANR do Crashlytics agrupados por versionCode
SELECT build_version AS version_code, SUM(events) AS anrs
FROM crashlytics_daily
WHERE package_name = :package AND kind = 'anr'
  AND day >= date('now', '-' || :days || ' days')
GROUP BY build_version
ORDER BY CAST(build_version AS INTEGER) DESC
```

Clique em **recarregar**. Um arquivo com o mesmo nome de um KPI embutido (ex.: `02_saude_por_versao.sql`) o substitui. Tabelas: `app`, `vitals_daily`, `play_issue`, `crashlytics_issue`, `crashlytics_daily`, `problem` (veja `src-tauri/sql/schema.sql`).

## Limitações conhecidas

- O Play agrega por dia no fuso **America/Los_Angeles**; o Crashlytics, em **UTC**.
- A ligação entre as fontes é heurística. Se o mapping do R8 não foi enviado a uma das fontes, a falha aparece ofuscada nela e não se une. O KPI "Cobertura das fontes" ajuda a detectar isso.
- Os issues do Play vêm da janela padrão da API (os mais frequentes, até 500 por sincronização).
- Ainda não há atualização automática do app (o updater assinado do Tauri está no roadmap).

## Licença

[Apache-2.0](LICENSE). Contribuições são bem-vindas: veja [CONTRIBUTING.md](CONTRIBUTING.md).
