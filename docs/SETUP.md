# Usando o Stackquake

1. Abra o Stackquake e clique em **Entrar com Google**. Use a conta que tem acesso ao Play Console.
2. Pronto. Os apps da conta aparecem sozinhos, e o projeto Firebase de cada um também é encontrado automaticamente.

Nenhuma chave, JSON, ID ou integração para configurar. Funciona no plano **Spark** (gratuito) do Firebase e **não usa BigQuery**.

## De onde vêm os dados

| Fonte | O que traz | Período |
|---|---|---|
| **Play Console** (Android Vitals) | Taxas de falha e ANR percebidas, usuários por dia, issues e relatórios | 90 dias na primeira sincronização |
| **Firebase Crashlytics** (API do Crashlytics) | Issues, eventos e usuários afetados por dia, % de usuários sem falha por versão, stack traces | Últimos 90 dias (limite da API) |

O Stackquake guarda tudo localmente, então o histórico continua crescendo além dos 90 dias.

## Várias contas Google

Cada conta tem os próprios dados, separados e criptografados com uma chave própria. Ao trocar de conta, você só vê os apps da conta conectada. Ao voltar para uma conta, os dados dela (status, responsáveis, notas) continuam lá.

## Permissões da conta

- **Play Console**: *Ver informações do app* (somente leitura) nos apps desejados.
- **Firebase**: acesso ao projeto Firebase do app (qualquer papel que permita ver o Crashlytics, como *Visualizador*).

## Problemas comuns

| Mensagem | O que fazer |
|---|---|
| `nenhum projeto Firebase com este app foi encontrado` | A conta não tem acesso ao projeto Firebase do app. Peça acesso a quem administra o projeto. |
| `nenhum dado do Crashlytics para este app` | O SDK do Crashlytics não está no app, ou ainda não houve nenhum evento. |
| `sem permissão no Firebase` | A conta não tem um papel no projeto Firebase que permita ver o Crashlytics. |
| `permissões não concedidas` | Na tela do Google alguma caixa ficou desmarcada. Entre de novo e marque todas. |
