# CLAUDE.md — Regras de Engenharia para AIRPG

> Contrato do projeto: não é um PR, é como o projeto se organiza. Leia com seriedade.

## Estrutura do projeto

```
airpg/
├── .claude/          # config de harness (launch.json, hooks)
├── airpg-engine/     # Rust: Orquestrador, Estado Rígido, LLM, guardrails
├── airpg-frontend/   # React/TS: UI/UX, chat
├── airpg-world/      # Elixir/OTP: simulação de mundo autônomo, geopolítica
├── docker-compose.yml # dev local
└── litellm.{config,docker.config}.yaml  # modelo de IA (dev via Ollama, prod configurável)
```

Três runtimes separados, sincronizados via NATS (Rust ↔ Elixir) e HTTP/SSE (Rust ↔ React). Cada um tem seu próprio Dockerfile, Cargo.toml/mix.exs, testes isolados.

## Documentação: Obsidian é a fonte de verdade

**Localização**: `/Users/victor/Documents/documentacao/obsidian/airpg/`

Toda decisão de arquitetura, todo plano de mudança, e **todo status de trabalho em andamento** vive em Markdown dentro do Obsidian vault. Se não está aqui, não é realidade do projeto ainda.

### Estrutura do vault

- `00-Overview/`: visão geral, glossário
- `01-Arquitetura/`: design das 4 camadas, orquestrador, guardrails, agentes
- `02-Agentes/` até `05-Fluxos/`: subsistemas
- `06-Decisoes-Tecnicas/`: stack, benchmarks, critérios
- `07-OpenSpec/`: roadmap e changes (fetures/bugs em escopo fechado)
- `08-Tarefas/`: **coordenação entre sessões e painel de execução** — VER ABAIXO

### Antes de qualquer sessão de código: leia

1. **Roadmap**: `07-OpenSpec/Roadmap-Lancamento.md` — o que está pronto, o que está bloqueado
2. **Status de execução**: `08-Tarefas/Tarefas-Pendentes.md` — o que cada módulo/frente fez, o que falta
3. **Coordenação ativa**: `08-Tarefas/Coordenacao-Entre-Sessoes.md` — **isto é crítico se há múltiplas frentes rodando em paralelo** — veja qual frente você está trabalhando, qual é o status, qual é o bloqueador atual

## Regras de trabalho paralelo

Se você está numa sessão de código:

- **Toda frente tem um documento `Change-*.md` no `07-OpenSpec/`** descrevendo o problema, a solução, e o status.
- **Atualize o documento** ao iniciar ("Sessão iniciada às 15:30, objetivo: implementar X"), durante (bloqueadores encontrados), e ao finalizar (commits, próximos passos).
- **Sincronize em `08-Tarefas/Coordenacao-Entre-Sessoes.md`** — é o painel de coordenação entre chats paralelos. Se você descobre um bloqueador que afeta outra frente, registre lá imediatamente.
- **Nenhum código antes da doc aprovada** — planos vão pra `Change-*.md`, o usuário lê, aprova ou redireciona, e só daí você implementa.

## Git

- **Branch única**: `main` — tudo vai pra main, sem feature branches (repo é um monorepo com 3 subrepos como submódulos, branch switching complica demais).
- **Commits frequentes**: mínimo viável (uma feature = um commit, não "tudo num commit só").
- **Mensagens em português** (repo histórico usa português): imperativo simples, 1 linha tipo "feat: adiciona RAG ao engine" ou "fix: guardrail rejeita 40% validos".
- **Antes de commit**: `cargo test` (engine), `cargo fmt` (style), `npm test` no frontend, `mix test` no world — não mergear código vermelho.
- **Não force-push** pra main a menos que o usuário peça explicitamente.

## Decisões já tomadas (não rediscuta)

Esses itens foram decididos **pelo usuário** e estão consignados. Se parecem questionáveis, leia a justificativa no documento antes de reabrir.

| Decisão | Documento | Por quê |
|---|---|---|
| Stack: Rust + Elixir/OTP + React | `06-Decisoes-Tecnicas/Stack-Escolhida.md` | Rust é rápido/determinístico pro núcleo, Elixir é nativo em supervisão de processos autônomos |
| Estado Rígido tem dono único (Rust) | `01-Arquitetura/Orquestrador-Central.md` | Evita estado fantasma em múltiplas fontes |
| LLM nunca decide número (dano, CD, etc.) | `01-Arquitetura/Guardrail-Narrativo.md` | Alucinação de regra é o bug mais grave do sistema |
| Preço de item é fixo por NPC | `07-OpenSpec/Change-Economia-Viva-e-Consistencia.md` | IA inventava números arbitrários; fixo é previsível e funciona melhor |
| Temperamento evolutivo é Fase 2 | `07-OpenSpec/Change-Temperamento-Evolutivo.md` | MVP atual (economia + diálogo consistente) é bom o suficiente; humor é enfeite, não bloqueador |

## Padrões de código esperados

### Rust (airpg-engine)

- `sqlx` para SQL com type-checking em compile time; migrations em `src/schema/migrations/` (numeradas sequencialmente).
- Estrutura de função LLM: `prompt` → `LlmClient::call` → `jsonutil::extrair_json` → validação de campos obrigatórios → retorno estruturado (nunca retornar JSON solto).
- Nomes de função com underscores, variáveis private por padrão, pub só se genuinamente necessário — compilador é seu amigo.
- Modules em `mod.rs` (não `lib.rs` monolítico), hierarquia reflete o código.

### React/TypeScript (airpg-frontend)

- Componentes por papel: `Chat.tsx`, `Inventory.tsx`, `Bestiary.tsx` (substantivos plurais de feature, não "ChatBox" ou "InventoryManager").
- Estado local via `useState`, global (sessão de jogo) via Context + hook customizado.
- Tailwind + Design System Terra (`tailwind.config.js` já tem as cores/tipografia definidas).
- Nenhum `any` sem comentário explicando por quê (tipos importam para o produto).

### Elixir (airpg-world)

- GenServer por entidade autônoma (NPC, facção, evento de mundo).
- Logs via `Logger` padrão (não `IO.puts`); estrutura de log já está configurada.
- Testes via `ExUnit`; fixtures em `test/support/`.

## LLM: como usar

Todo LLM call vai por `airpg-engine/src/llm.rs:LlmClient`. Três regras:

1. **Nunca deixar a IA decidir números**: dano, CD, iniciativa, quantidade, preço. Isso é skill determinística em Rust (ver `skills/mod.rs`).
2. **Sempre injetar contexto de "verdade"**: se o guardrail precisa saber quem está vivo, a prompt deve listar quem está vivo. Não confiar na IA se lembrar de contexto passado.
3. **Sempre validar o JSON retornado**: campo obrigatório vazio ou tipo errado → rejeitar e reforçar a prompt. Ver `guardrail.rs` para o padrão de "retry com feedback".

## Quando você fica preso

1. **Leia a documentação** — a resposta provavelmente está em `01-Arquitetura` ou `06-Decisoes-Tecnicas`.
2. **Registre o bloqueador** em `08-Tarefas/Coordenacao-Entre-Sessoes.md` — não é fraqueza, é comunicação.
3. **Se é decisão de produto**: não tome sozinho. Documente a opção no `Change-*.md` e peça aprovação do usuário.

## Estrutura de uma Change

Se você é responsável por uma mudança grande:

1. Crie `07-OpenSpec/Change-NomedaMudanca.md` (ou estenda uma existente).
2. Seções: **Why** (problema), **What Changes** (concretamente), **Impact** (quais arquivos), **Tasks** (checklist), **Design** (decisões técnicas).
3. Status: 🔴 bloqueador / 🟡 planejado / 🟢 implementado / ⚪ fase 2.
4. Registre em `08-Tarefas/Coordenacao-Entre-Sessoes.md` apontando pra sua Change.
5. Atualize conforme você implementa.

## Relacionamento com o usuário

- **O usuário decide arquitetura e priorização**. Você executa com qualidade.
- **Evidência antes de conclusão**: não diga "implementado", diga "implementado e validado contra [X]" (testes, execução real, etc.).
- **Transparência sobre trade-offs**: se você tem que escolher entre A e B, diga qual é a perda em cada uma e peça confirmação.

## Checklist antes de fazer commit

- [ ] Código compila / testes passam
- [ ] Documentação atualizada no vault (seja `Change-*.md` ou `Tarefas-Pendentes.md`)
- [ ] Não há `TODO` comentário solto sem rastreamento em `Change-*.md`
- [ ] Nenhuma constante hard-coded que pertença em config/DB (preço, prompt template, etc.)
- [ ] Se toca estado: validação de schema confirmada em `db.rs`
- [ ] Commit message reflete o que foi feito (não "trabalho em progresso" genérico)

---

Tl;dr: **Obsidian é a fonte de verdade, `Change-*.md` é o contrato, `Tarefas-Pendentes.md` e `Coordenacao-Entre-Sessoes.md` são o painel vivo**. Leia antes de mexer.
