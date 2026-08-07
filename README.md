# AIRPG

RPG de texto orquestrado por IA — engine em Rust, mundo vivo autônomo em Elixir/OTP, frontend em React/TypeScript. Monorepo com submódulos independentes.

## Submódulos

| Repositório | Linguagem | Papel |
|---|---|---|
| [airpg-engine](https://github.com/lchampz/airpg-engine) | Rust (Axum/Tokio) | Orquestrador Central, Guardrails Narrativos, Pool de Agentes reativo, Skills, Estado Rígido |
| [airpg-world](https://github.com/lchampz/airpg-world) | Elixir/OTP | Mundo Vivo: NPCs autônomos, supervisão de falha, detecção de colisão com o jogador |
| [airpg-frontend](https://github.com/lchampz/airpg-frontend) | React + TypeScript | Consumidor puro do contrato de eventos do engine |

`airpg-engine` e `airpg-world` se comunicam via **NATS**, conectado e testado ponta a ponta (colisão detectada no Elixir → engine roteia o Pool de Agentes → resposta gerada por LLM → Elixir retoma autonomia do NPC).

## Clonando

```bash
git clone --recurse-submodules https://github.com/lchampz/airpg.git
```

Se já clonou sem `--recurse-submodules`:

```bash
git submodule update --init --recursive
```

## Rodando via Docker (recomendado)

Sobe todos os serviços com um único comando — engine, mundo vivo, frontend, NATS e o gateway LiteLLM. **Requer Ollama já rodando no host** (`ollama serve`, com o modelo `llama3.2` baixado via `ollama pull llama3.2`) — o Ollama não é containerizado de propósito, para reaproveitar o modelo já baixado e não conflitar com o Ollama.app do host. O container do LiteLLM alcança o host via `host.docker.internal`.

```bash
docker compose build
docker compose up -d
docker compose ps        # confirma os 5 serviços de pé
docker compose logs -f airpg-engine   # acompanhar o comportamento da IA em tempo real
```

Serviços expostos no host:
- `airpg-engine` → `localhost:8080`
- `airpg-frontend` → `localhost:5173`
- `litellm` → `localhost:4000`
- `nats` → `localhost:4222`

Trocar de LLM local para um provedor hospedado depois é só editar `litellm.docker.config.yaml` — nenhum código do engine muda (ver `Stack-Escolhida` no vault).

Testando o mundo vivo dentro do Docker (dispara um NPC autônomo e observa a colisão de dentro do container):

```bash
docker compose exec airpg-world mix run -e '
AirpgWorld.PlayerTracker.set_location("taverna_porto_velho")
AirpgWorld.NpcSupervisor.start_npc("npc_taverneiro_bram", "taverna_porto_velho", :atender_clientes)
Process.sleep(10000)
IO.inspect(AirpgWorld.Npc.get_state("npc_taverneiro_bram"))
'
```

Parar tudo: `docker compose down` (os dados do SQLite ficam no volume `airpg_engine_data`, sobrevivem ao `down` a menos que use `down -v`).

## Rodando localmente sem Docker

```bash
# infra
brew install nats-server && nats-server
pipx install 'litellm[proxy]==1.55.0' --python python3.12   # 1.55.0 evita o build do bridge Rust nativo de versões mais novas
litellm --config litellm.config.yaml --port 4000
ollama serve   # se ainda não estiver rodando

# serviços
cd airpg-engine && cargo run
cd airpg-world && mix deps.get && iex -S mix
cd airpg-frontend && npm install && npm run dev
```

## Documentação da arquitetura

A documentação completa (camadas, eventos, fluxo de turno, decisões técnicas e sua justificativa) vive no vault Obsidian do projeto — arquitetura, guardrails, modelo de agentes, catálogo de eventos, e o registro de todas as decisões técnicas tomadas (`Stack-Escolhida`, `Benchmark-Linguagens`, `Decisoes-Resolvidas`).

## Status

Testado ponta a ponta (local e via Docker):
- Roteamento real do Pool de Agentes: até 4 NPCs por turno, filtrados por `location_id`, respostas geradas em paralelo via LLM (LiteLLM → Ollama local)
- Guardrail de Saída real (segunda chamada de LLM, veredito estruturado em JSON)
- **Persistência de mudança de estado**: o LLM propõe (dano/cura, itens), o engine valida contra whitelist + bounds (HP nunca sai de `[0, máximo]`, item precisa existir para ser removido) e persiste no SQLite — confirmado que o estado sobrevive entre requisições (dano num turno, cura persistida no próximo) e que diálogo comum não gera mudança nenhuma (sem alucinação de efeito mecânico)
- Mundo Vivo (Elixir/OTP): NPC autônomo detecta colisão com o jogador **na borda** (não repete a cada tick — bug encontrado e corrigido em teste), publica no NATS, engine assume a interação reativa, e devolve `interacao_finalizada` para o NPC retomar autonomia
- Stack inteira reproduzível via `docker compose up`

Ainda não implementado: Guardrail de Entrada não consulta o Estado Rígido para validar viabilidade mecânica da ação, sem detecção de combate para ordem de iniciativa sequencial, e reidratação dos NPCs autônomos do Mundo Vivo entre reinícios do container.
