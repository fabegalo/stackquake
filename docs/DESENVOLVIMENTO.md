# Desenvolvimento e publicação

## Rodando

Pré-requisitos: [Rust](https://rustup.rs) estável, Node.js 20+ e as [dependências do Tauri](https://v2.tauri.app/start/prerequisites/) da sua plataforma. No Linux também `libdbus-1-dev` e um chaveiro (GNOME Keyring ou KWallet).

```bash
npm install
npm run tauri dev
```

## Login com Google (feito uma única vez, por quem publica o app)

O Google exige que todo app com "Entrar com Google" seja registrado por quem o desenvolve. Isso é feito **uma vez**. O registro vai embutido no app, e os usuários só clicam no botão.

1. No [Google Cloud Console](https://console.cloud.google.com), crie um projeto para o Stackquake (ex.: `stackquake-app`). Não precisa de cartão.
2. Em **APIs e serviços → Biblioteca**, ative:
   - Google Play Developer Reporting API
   - Firebase Management API
   - Firebase Crashlytics API
3. Em **Google Auth Platform**:
   - **Branding**: nome "Stackquake", e-mail de suporte.
   - **Público-alvo**: *Externo*, depois **Publicar app** ("Em produção"). No modo "Teste", o login expira a cada 7 dias.
   - **Acesso a dados**: adicione os escopos `.../auth/playdeveloperreporting`, `.../auth/firebase`, `openid` e `.../auth/userinfo.email`.
4. Em **Clientes → Criar cliente → App para computador**, baixe o JSON e salve como:

   ```
   src-tauri/google-oauth-client.json
   ```

   Esse arquivo está no `.gitignore` e nunca vai para o repositório. O `build.rs` lê o arquivo e embute o cliente no binário.

Na CI, use os secrets `STACKQUAKE_GOOGLE_CLIENT_ID` e `STACKQUAKE_GOOGLE_CLIENT_SECRET`. O workflow de release já os repassa.

### Sobre o "client secret" embutido

Em apps instalados, o Google e a [RFC 8252 §8.5](https://datatracker.ietf.org/doc/html/rfc8252#section-8.5) **não consideram o `client_secret` confidencial**, porque qualquer binário pode ser inspecionado. A segurança do login vem do PKCE (S256), do redirecionamento só para `127.0.0.1` e do parâmetro `state`. É o mesmo modelo de ferramentas como `gcloud` e `rclone`. Mesmo assim, o valor fica fora do repositório para que forks usem o próprio cliente.

### Verificação do Google (para distribuir publicamente)

Enquanto o app não for verificado pelo Google, a tela de login mostra o aviso "app não verificado" (*Avançado → Acessar*) e o app aceita até **100 usuários**. Para tirar o aviso e o limite, peça a verificação em *Google Auth Platform → Central de verificação*. Ela é gratuita e pede uma página inicial, uma política de privacidade e um vídeo curto do fluxo de login. Os escopos usados são "sensíveis", não "restritos", então não há auditoria de segurança paga.

### Por que o escopo `firebase`

A API do Crashlytics só aceita `firebase` ou `cloud-platform`; não existe uma versão "somente leitura". O Stackquake usa o menor dos dois e, no núcleo (`src-tauri/src/http.rs`), **bloqueia qualquer método que não seja GET** para os servidores do Firebase. O app não consegue alterar nada no Firebase, mesmo que um bug tente.

## Publicando uma versão

```bash
git tag v0.1.0 && git push --tags
```

O workflow `release.yml` gera os instaladores de Windows, macOS e Linux como rascunho de release, com atestado de proveniência.
