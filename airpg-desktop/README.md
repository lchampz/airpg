# airpg-desktop

Shell desktop do AIRPG via Tauri — empacota o `airpg-engine` já existente como
sidecar local, sem depender de Docker/LiteLLM. Ver `Change-RAG-SRD-e-Desktop.md`
no vault (Fase 5) para o design completo.

## Arquitetura

- **Frontend**: reaproveita `../airpg-frontend` (React) — nenhum código duplicado,
  o `tauri.conf.json` só aponta pro `dist/` dele e roda `npm run build`/`npm run dev`
  via `beforeBuildCommand`/`beforeDevCommand`.
- **Engine**: o binário de `airpg-engine` é copiado como sidecar
  (`src-tauri/binaries/airpg-engine-<target-triple>`) e sobe automaticamente no
  `setup()` do app (`src-tauri/src/lib.rs::iniciar_engine_sidecar`), configurado
  via env vars pra:
  - Persistir em SQLite dentro do `app_data_dir` (não no diretório de instalação)
  - Falar direto com o Ollama local (`http://127.0.0.1:11434/v1`) em vez do
    gateway LiteLLM do modo Docker — **zero mudança de código no engine**, Ollama
    já expõe API OpenAI-compatible e o `LlmClient` já fala esse protocolo
  - Rodar sem NATS (o engine já trata isso como melhor-esforço)

## Rebuild do binário sidecar

Sempre que `airpg-engine` mudar, o sidecar precisa ser reconstruído e recopiado
manualmente (não há automação de CI pra isso ainda):

```bash
cd ../airpg-engine
cargo build --release --bin airpg-engine
cp target/release/airpg-engine ../airpg-desktop/src-tauri/binaries/airpg-engine-$(rustc -vV | grep host | cut -d' ' -f2)
```

## Rodar em dev

```bash
npm --prefix ../airpg-frontend install  # se ainda não instalou
npx @tauri-apps/cli dev
```

## O que está implementado e verificado

- Scaffold Tauri 2.x compilando (`cargo build` limpo)
- Sidecar do engine sobe de verdade — confirmado por log real capturado via
  `tauri-plugin-log`: engine conecta no SQLite próprio, checa RAG (vazio,
  gracioso), tenta NATS (gracioso), e só falha no bind de porta quando há
  conflito com uma instância já rodando (ambiente de teste, não bug)
- `cargo tauri build --no-bundle` produz um binário `.app`/executável real
- Portabilidade do runtime ONNX (`fastembed`) confirmada fora da árvore de
  build original — pré-requisito pro sidecar funcionar em qualquer máquina

## O que NÃO está implementado ainda (corte de escopo consciente)

- **UI de primeiro uso**: os comandos Tauri `verificar_modelo_local` e
  `baixar_modelo` existem e compilam (chamam a API real do Ollama:
  `/api/tags` pra checar, `/api/pull` com streaming NDJSON de progresso pra
  baixar), mas **nenhuma tela do `airpg-frontend` os chama ainda**. Isso é
  trabalho de frontend React, fora do escopo desta sessão (que focou no lado
  Rust/engine) — próximo passo natural de quem pegar isso depois.
- Ícone/identidade visual do app são os placeholders gerados pelo scaffold
  (`tauri init`), não arte final
- Sem assinatura de código (`.app` roda localmente sem problema, mas
  distribuição pra terceiros no macOS exigiria notarização — fora de escopo
  de infraestrutura de dev)
- Download automático do Ollama em si (hoje assume que o usuário já tem
  Ollama instalado e rodando) — poderia embutir um sidecar do próprio
  `ollama serve`, mas não foi feito
