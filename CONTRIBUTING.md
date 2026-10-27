# Contribuindo

Obrigado! O projeto é Apache-2.0 e toda contribuição entra sob essa licença.

## Onde mexer

| Quero… | Arquivo |
|---|---|
| Criar ou ajustar um KPI | `src-tauri/kpis/*.sql` (ou a pasta de KPIs do usuário, sem recompilar) |
| Mudar como Play e Crashlytics são unidos | `src-tauri/src/unify.rs`, que tem testes |
| Adicionar uma fonte de dados | novo módulo em `src-tauri/src/google/` + chamada em `sync.rs` |
| Mudar uma tela | `src/views/*.svelte` |
| Adicionar um comando IPC | `commands.rs` + `lib.rs` + **as três listas**: `build.rs`, `capabilities/default.json`, `isolation/index.js` |

## Regras de segurança (obrigatórias em PRs)

1. **Nada de rede na interface.** Toda chamada externa passa por `http.rs`. Um host novo entra em `ALLOWED_HOSTS` com justificativa no PR.
2. **Nenhum token ou segredo** sai do núcleo Rust, em retorno de comando, log ou mensagem de erro.
3. **Valide toda entrada** de comando em `validate.rs`. Nunca interpole texto do usuário em SQL ou URL.
4. **Sem HTML vindo de dados**: nada de `{@html}` no Svelte.
5. **Sem atributos `style` inline** (a CSP bloqueia); use classes.
6. Dependência nova precisa de justificativa, versão exata e licença na lista do `deny.toml`.

Para testar o login no seu build, siga [docs/DESENVOLVIMENTO.md](docs/DESENVOLVIMENTO.md).

## Checagens locais

```bash
npm run check && npm run build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```
