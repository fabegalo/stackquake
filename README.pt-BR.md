# Stackquake

**Falhas e ANRs do Google Play Console e do Firebase Crashlytics unificados em um app desktop** para Windows, macOS e Linux, com gestão de problemas e KPIs por app e versão.

[English](README.md)

- **Só "Entrar com Google"**: sem chaves, JSON ou IDs para configurar. Apps e projetos Firebase são descobertos automaticamente.
- **Funciona no plano gratuito Spark do Firebase, sem BigQuery**: usa a Play Developer Reporting API e a API do Crashlytics, ambas gratuitas.
- **Somente leitura por design**: o núcleo só permite leituras no Firebase, os tokens ficam no chaveiro do sistema e o banco local é criptografado. Veja [SECURITY.md](SECURITY.md).
- **Local e privado**: sem servidor, sem telemetria. Cada conta Google tem o próprio banco criptografado.
- **Fácil de mexer**: cada KPI é um arquivo `.sql`; cada fonte de dados é um módulo isolado.
- **Multilíngue**: inglês e português (Brasil), fácil de estender.

## Usando

Abra o app e clique em **Entrar com Google**. O idioma segue o do sistema e pode ser trocado em **Configurações → Idioma**. Detalhes em [docs/SETUP.md](docs/SETUP.md) (em inglês).

## Desenvolvendo

```bash
npm install
npm run tauri dev
```

Para o login funcionar no seu build, registre o app no Google uma vez: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) (em inglês).

## Contribuindo

Contribuições são bem-vindas. A documentação técnica, o código e as mensagens de commit são em inglês e seguem o [Conventional Commits](https://www.conventionalcommits.org/). Veja [CONTRIBUTING.md](CONTRIBUTING.md).

## Privacidade

O Stackquake não tem servidor nem telemetria; seus dados ficam no seu computador. Veja [PRIVACY.md](PRIVACY.md) (em inglês).

## Licença

[Apache-2.0](LICENSE).
