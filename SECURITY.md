# Segurança

## Reportando uma vulnerabilidade

**Não abra issue pública.** Use o [aviso de segurança privado do GitHub](../../security/advisories/new) deste repositório. Respondemos em até 7 dias e publicamos a correção com crédito, se você quiser.

## Modelo de ameaças

O Stackquake lida com credenciais Google com acesso ao Play Console e ao Firebase, e com crash reports, que podem conter dados de usuários. O objetivo é que **nem um bug na interface nem um crash report malicioso** consigam vazar credenciais ou dados.

### Controles

| Área | Controle | Onde |
|---|---|---|
| Login | OAuth 2.0 para apps nativos (RFC 8252): navegador do sistema, loopback em `127.0.0.1` com porta aleatória, **PKCE S256**, `state` comparado em tempo constante, timeout de 5 min | `src-tauri/src/auth.rs` |
| Escopos | `playdeveloperreporting` (leitura) + `firebase` (a API do Crashlytics não tem escopo só de leitura) + `openid`/`email` para separar os dados por conta. Escopos desmarcados na tela de consentimento são detectados | `auth.rs` |
| Firebase só leitura | Mesmo com o escopo `firebase`, o núcleo **só permite GET** para `firebase.googleapis.com` e `firebasecrashlytics.googleapis.com`: nenhuma alteração no Firebase é possível | `http.rs` |
| Segredos | Refresh token e chave do banco **só no chaveiro do SO**; access token só em memória, zerado ao liberar (`zeroize`) | `secrets.rs` |
| Cliente OAuth | Embutido no build (arquivo local fora do git ou secret da CI). Em apps instalados o `client_secret` não é confidencial (RFC 8252 §8.5); a proteção vem do PKCE + loopback + `state` | `build.rs`, `docs/DESENVOLVIMENTO.md` |
| Rede | Um único cliente HTTP: **HTTPS obrigatório**, lista fixa de hosts com **métodos permitidos por host**, **sem redirecionamentos**, timeouts, limite de tamanho de resposta, TLS via rustls | `http.rs` |
| Dados em repouso | SQLite + **SQLCipher (AES-256)**, **um banco e uma chave por conta Google**, chave de 256 bits do gerador do SO; pasta de dados com permissão `0700` (Unix); `secure_delete` | `db.rs` |
| Interface isolada | CSP `default-src 'self'`, **`connect-src` só IPC** (a UI não acessa a internet), sem `unsafe-inline`/`eval`, `freezePrototype` | `tauri.conf.json` |
| IPC | **Capabilities** com uma permissão por comando; padrão **Isolation** valida cada chamada num iframe isolado; nenhum plugin exposto à UI | `capabilities/`, `isolation/` |
| Entradas | Pacotes, IDs de projeto, App IDs do Firebase, IDs de issue/evento e textos validados no Rust antes de virar parte de uma URL | `validate.rs` |
| Conteúdo não confiável | Stack traces e logs são exibidos **só como texto** (nunca HTML). A página de retorno do OAuth é estática | Svelte, `auth.rs` |
| KPIs do usuário | Conexão **somente leitura** + autorizador SQLite que só permite SELECT/leitura/funções (sem ATTACH, PRAGMA ou escrita) + timeout de 5 s + limite de linhas | `kpi.rs`, `db.rs` |
| Cadeia de suprimentos | Versões exatas e lockfiles, `cargo-deny` (RustSec, licenças, origens), `npm audit`, Actions fixadas por SHA, Dependabot, atestado de proveniência SLSA nos releases | `.github/`, `deny.toml` |
| Binário | `panic = "abort"`, LTO, símbolos removidos; sem sourcemaps no bundle | `Cargo.toml`, `vite.config.ts` |

### Fora do escopo / riscos conhecidos

- **Máquina comprometida**: quem roda código como o seu usuário pode ler o chaveiro.
- **Linux**: o webview (WebKitGTK) depende das atualizações da distribuição. A UI não carrega conteúdo remoto, o que reduz bastante a exposição.
- **Releases sem assinatura de código** até os certificados serem configurados no CI. Verifique a proveniência com `gh attestation verify`.
