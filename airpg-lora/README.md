# airpg-lora

Fine-tuning de **estilo de narração** via LoRA (ver Change-RAG-SRD-e-Desktop,
Fase 6 — opcional). Objetivo: reforçar o formato `*ação*`/`-fala` já exigido
pelo prompt de produção (ver `airpg-engine/src/reacoes.rs::montar_system_prompt`)
e reduzir a taxa de resposta fora do formato registrada em `Tarefas-Pendentes.md`.

**Isto não ensina regras de D&D.** Isso é papel do RAG sobre o SRD (ver
`airpg-desktop`/`Change-RAG-SRD-e-Desktop`, Fases 2-3) e das skills
determinísticas — LoRA aqui é só estilo/formato de narração.

## Dataset

`dataset/gerar_dataset.py` gera `train.jsonl`/`valid.jsonl`/`test.jsonl` a
partir de exemplos escritos à mão **pra este projeto especificamente** —
nenhuma linha vem de livros de D&D (SRD ou não). Formato chat (`messages`),
o mesmo shape que o `LlmClient::complete_with_history` já monta em produção,
pra treinar no formato exato em que o modelo vai ser usado.

**Estado atual: 30 exemplos** (24 treino / 3 validação / 3 teste). O plano
original (`Change-RAG-SRD-e-Desktop`) previa 200-500 — isto é um conjunto
inicial pra provar o pipeline, não o dataset de produção. Pra escalar,
edite `EXEMPLOS`/`VARIACOES` em `gerar_dataset.py` e rode de novo.

```bash
python3 dataset/gerar_dataset.py
```

## Ferramenta: `mlx-lm` (Apple Silicon, não Unsloth)

O plano original citava Unsloth — não funciona bem em Mac sem GPU NVIDIA.
`mlx-lm` (framework MLX da própria Apple) faz LoRA/QLoRA nativamente via
Metal, sem CUDA. Setup:

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install mlx-lm
```

## Rodando o treino

**Este repo já tem um treino de prova rodado e verificado** contra um
modelo pequeno (`mlx-community/Llama-3.2-1B-Instruct-4bit`, ~700MB) — não
o modelo de produção (`llama3` 8B), por tempo/banda desta sessão. Comando
usado:

```bash
mlx_lm.lora \
  --model mlx-community/Llama-3.2-1B-Instruct-4bit \
  --train \
  --data dataset \
  --iters 60 \
  --batch-size 2 \
  --num-layers 8 \
  --adapter-path adapters_teste
```

Resultado real: loss de treino caiu de 5.3 → 0.5 em 60 iterações.
Comparação de saída pro mesmo prompt confirma a mudança de estilo:

- **Base** (sem adapter): `*Olha para o vinho e faço um gesto...* - Fala para o dono do vinho: "Ei, pessoal! Quero saber o preço..."` — fora do formato, quebra tom.
- **Com adapter**: `*se inclina pra cerveja* -Fala pra comer, se for pisco, serve pra 10 moedas.` — formato `*ação* -fala` seguido à risca, tom mais terso.

Conteúdo ainda sai um pouco confuso (dataset pequeno + modelo de 1B) — a
mudança de **formato** é real e mensurável, a de **qualidade de conteúdo**
exige o dataset maior e o modelo de produção.

## Para rodar contra o modelo de produção (`llama3` 8B) — próximo passo

Não executado nesta sessão (mais tempo/banda do que cabia aqui). Passos:

```bash
# 1. Baixar/converter llama3 8B pra MLX (ou usar uma versão já convertida
#    da comunidade, ex: mlx-community/Meta-Llama-3-8B-Instruct-4bit)
mlx_lm.lora \
  --model mlx-community/Meta-Llama-3-8B-Instruct-4bit \
  --train \
  --data dataset \
  --iters 300 \
  --batch-size 4 \
  --num-layers 16 \
  --adapter-path adapters_producao

# 2. Testar antes de decidir usar em produção
mlx_lm.generate --model mlx-community/Meta-Llama-3-8B-Instruct-4bit \
  --adapter-path adapters_producao \
  --prompt "..."

# 3. Se aprovado, fundir o adapter no modelo base pra distribuir um GGUF
#    único (ver mlx_lm.fuse) e apontar o Ollama pra esse modelo fundido
#    em vez do llama3 stock.
```

**Antes de rodar em produção**: escalar o dataset pra pelo menos 100-200
exemplos (o de 30 aqui é só prova de conceito), e validar contra uma
amostra real de turnos (mesmo processo usado na Fase 4 — ver
Change-RAG-SRD-e-Desktop) comparando taxa de resposta fora do formato
antes/depois.
