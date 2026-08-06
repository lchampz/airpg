# AIRPG

RPG de texto orquestrado por IA — engine em Rust, mundo vivo autônomo em Elixir/OTP, frontend em React/TypeScript. Monorepo com submódulos independentes.

## Submódulos

| Repositório | Linguagem | Papel |
|---|---|---|
| [airpg-engine](https://github.com/lchampz/airpg-engine) | Rust (Axum/Tokio) | Orquestrador Central, Guardrails Narrativos, Pool de Agentes reativo, Skills, Estado Rígido |
| [airpg-world](https://github.com/lchampz/airpg-world) | Elixir/OTP | Mundo Vivo: NPCs autônomos, supervisão de falha, detecção de colisão com o jogador |
| [airpg-frontend](https://github.com/lchampz/airpg-frontend) | React + TypeScript | Consumidor puro do contrato de eventos do engine |

Comunicação entre `airpg-engine` e `airpg-world` via **NATS** (ainda não conectado no esqueleto atual — ver README de cada submódulo para o status real da integração).

## Clonando

```bash
git clone --recurse-submodules https://github.com/lchampz/airpg.git
```

Se já clonou sem `--recurse-submodules`:

```bash
git submodule update --init --recursive
```

## Rodando localmente (esqueleto atual)

```bash
# terminal 1 — engine
cd airpg-engine && cargo run

# terminal 2 — frontend
cd airpg-frontend && npm install && npm run dev

# terminal 3 — mundo vivo (opcional no MVP atual, ainda não conectado ao engine via NATS)
cd airpg-world && mix deps.get && iex -S mix
```

## Documentação da arquitetura

A documentação completa (camadas, eventos, fluxo de turno, decisões técnicas e sua justificativa) vive no vault Obsidian do projeto — arquitetura, guardrails, modelo de agentes, catálogo de eventos, e o registro de todas as decisões técnicas tomadas (`Stack-Escolhida`, `Benchmark-Linguagens`, `Decisoes-Resolvidas`).

## Status

Esqueleto inicial (walking skeleton) dos três serviços, cada um validado individualmente (testes + smoke test) e o fluxo `frontend → engine` confirmado ponta a ponta em browser. Ainda não implementados: integração NATS real entre `airpg-engine` e `airpg-world`, chamadas a LLM (Guardrails, diálogo de agentes), Guardrail de Saída, e persistência efetiva do Estado Rígido nos endpoints.
