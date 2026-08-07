//! Shell desktop do AIRPG (ver Change-RAG-SRD-e-Desktop, Fase 5). Empacota o
//! `airpg-engine` já existente como sidecar local — nenhuma mudança de
//! código no engine em si, só configuração de ambiente diferente (aponta
//! direto pro Ollama local, sem passar pelo gateway LiteLLM que o modo
//! web/Docker usa).

use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// Ollama expõe uma API OpenAI-compatible nesse endpoint por padrão — o
/// `airpg-engine` já fala esse protocolo (`llm.rs::LlmClient`), então não
/// precisa de nenhum código novo no engine, só apontar `LITELLM_URL` pra cá
/// em vez do gateway LiteLLM (ver Stack-Escolhida: troca de provedor é só
/// configuração, por design).
const OLLAMA_URL: &str = "http://127.0.0.1:11434";
/// Ver Change-RAG-SRD-e-Desktop, Fase 4: `llama3` (8B) validado como upgrade
/// real sobre `llama3.2` (3B) — mesmo modelo usado no modo Docker/web.
const MODELO_PADRAO: &str = "llama3";

/// Mantém o processo do engine vivo enquanto o app roda — sem isso, o
/// `CommandChild` seria dropado no fim do `setup` e o sidecar morreria
/// junto (comportamento documentado do `tauri-plugin-shell`).
struct EstadoSidecar {
    engine: Mutex<Option<CommandChild>>,
}

#[derive(Clone, serde::Serialize)]
struct ModeloDisponivel {
    disponivel: bool,
}

/// Verifica se `modelo` já foi baixado no Ollama local — chamado pelo
/// frontend no primeiro carregamento pra decidir se mostra a tela de
/// download antes de liberar o jogo.
#[tauri::command]
async fn verificar_modelo_local(modelo: String) -> Result<ModeloDisponivel, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{OLLAMA_URL}/api/tags"))
        .send()
        .await
        .map_err(|e| format!("Ollama local não respondeu (está rodando?): {e}"))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let modelos = json["models"].as_array().cloned().unwrap_or_default();
    let disponivel = modelos.iter().any(|m| {
        m["name"]
            .as_str()
            .map(|nome| nome.starts_with(&modelo))
            .unwrap_or(false)
    });

    Ok(ModeloDisponivel { disponivel })
}

/// Baixa `modelo` via API do Ollama, emitindo progresso pro frontend a cada
/// linha de resposta (Ollama transmite NDJSON — um objeto JSON por linha,
/// com `completed`/`total` em bytes durante o download).
#[tauri::command]
async fn baixar_modelo(app: tauri::AppHandle, modelo: String) -> Result<(), String> {
    use futures_util::StreamExt;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{OLLAMA_URL}/api/pull"))
        .json(&serde_json::json!({ "model": modelo }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let mut stream = resp.bytes_stream();
    let mut sobra = Vec::new();

    while let Some(pedaco) = stream.next().await {
        let pedaco = pedaco.map_err(|e| e.to_string())?;
        sobra.extend_from_slice(&pedaco);

        // NDJSON: processa cada linha completa, guarda o resto pro próximo pedaço.
        while let Some(pos) = sobra.iter().position(|&b| b == b'\n') {
            let linha: Vec<u8> = sobra.drain(..=pos).collect();
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&linha) {
                let _ = app.emit("modelo-download-progresso", &json);
            }
        }
    }

    Ok(())
}

/// Sobe o `airpg-engine` como sidecar, configurado pra falar direto com o
/// Ollama local (sem LiteLLM) e persistir num SQLite dentro do diretório de
/// dados do app — nunca no diretório de instalação, que pode ser read-only.
fn iniciar_engine_sidecar(app: &tauri::AppHandle) -> anyhow::Result<CommandChild> {
    let data_dir = app
        .path()
        .app_data_dir()
        .expect("app_data_dir deveria sempre resolver em desktop");
    std::fs::create_dir_all(&data_dir).expect("falha ao criar diretório de dados do app");
    let db_path = data_dir.join("airpg.db");

    let sidecar = app
        .shell()
        .sidecar("airpg-engine")
        .expect("sidecar 'airpg-engine' precisa estar registrado em tauri.conf.json")
        .envs(std::collections::HashMap::from([
            (
                "DATABASE_URL".to_string(),
                format!("sqlite://{}?mode=rwc", db_path.display()),
            ),
            // Ollama fala o protocolo OpenAI-compatible nativamente em
            // /v1 — o LlmClient do engine não precisa saber que não está
            // mais falando com o gateway LiteLLM (ver Stack-Escolhida).
            ("LITELLM_URL".to_string(), format!("{OLLAMA_URL}/v1")),
            ("LITELLM_MODEL".to_string(), MODELO_PADRAO.to_string()),
            // Sem NATS no desktop single-player: o engine já trata isso
            // como melhor-esforço e sobe mesmo sem conseguir conectar
            // (ver main.rs — "engine seguira sem barramento cross-process").
            ("NATS_URL".to_string(), "nats://127.0.0.1:1".to_string()),
            ("RUST_LOG".to_string(), "info".to_string()),
        ]));

    let (mut rx, child) = sidecar.spawn()?;

    // Repassa stdout/stderr do engine pro log do Tauri em vez de descartar
    // silenciosamente — essencial pra debugar problema de usuário final
    // que não tem acesso a terminal.
    tauri::async_runtime::spawn(async move {
        while let Some(evento) = rx.recv().await {
            match evento {
                CommandEvent::Stdout(linha) => {
                    log::info!("[engine] {}", String::from_utf8_lossy(&linha));
                }
                CommandEvent::Stderr(linha) => {
                    log::warn!("[engine] {}", String::from_utf8_lossy(&linha));
                }
                CommandEvent::Error(err) => {
                    log::error!("[engine] erro no sidecar: {err}");
                }
                CommandEvent::Terminated(status) => {
                    log::warn!("[engine] sidecar encerrou: {status:?}");
                }
                _ => {}
            }
        }
    });

    Ok(child)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(EstadoSidecar {
            engine: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            verificar_modelo_local,
            baixar_modelo
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let handle = app.handle().clone();
            let child = iniciar_engine_sidecar(&handle)?;
            let estado = app.state::<EstadoSidecar>();
            *estado.engine.lock().unwrap() = Some(child);

            Ok(())
        })
        .on_window_event(|window, event| {
            // Mata o sidecar quando a janela principal fecha — sem isso o
            // processo do engine fica órfão rodando em background.
            if let tauri::WindowEvent::Destroyed = event {
                let estado = window.state::<EstadoSidecar>();
                let filho_encerrado = estado.engine.lock().unwrap().take();
                if let Some(child) = filho_encerrado {
                    let _ = child.kill();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
