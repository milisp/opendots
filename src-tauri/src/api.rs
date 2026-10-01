use axum::{
    Json, Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use opendots_acp::{AcpAgentDef, AcpState};
use opendots_automation::{AutomationHandle, AutomationInput, AutomationSchedule, CwdMode};
use opendots_shared::agent_runner::AgentRunner;
use opendots_shared::event_sink::EventSink;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct ApiState {
    pub acp: AcpState,
    pub automation: AutomationHandle,
    pub events: broadcast::Sender<(String, Value)>,
}

struct ApiEventSink(broadcast::Sender<(String, Value)>);
impl EventSink for ApiEventSink {
    fn emit(&self, name: &str, value: Value) {
        let _ = self.0.send((name.into(), value));
    }
}

pub fn init() -> Result<(), String> {
    // DB access initializes schemas lazily; this fails early if the app data dir is unavailable.
    opendots_db::bots::list_bots(true).map(|_| ())?;
    Ok(())
}

/// Default loopback port for the local API; override with `OPENDOTS_API_PORT`.
const DEFAULT_API_PORT: u16 = 26929;

/// Bind the loopback listener. An explicit `OPENDOTS_API_PORT` is strict (0 = random);
/// the default port falls back to a random one if it is already taken.
async fn bind_listener() -> Result<tokio::net::TcpListener, String> {
    if let Ok(raw) = std::env::var("OPENDOTS_API_PORT") {
        let port: u16 = raw
            .trim()
            .parse()
            .map_err(|_| format!("invalid OPENDOTS_API_PORT: {raw}"))?;
        return tokio::net::TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|e| format!("failed to bind 127.0.0.1:{port}: {e}"));
    }
    match tokio::net::TcpListener::bind(("127.0.0.1", DEFAULT_API_PORT)).await {
        Ok(listener) => Ok(listener),
        Err(e) => {
            log::warn!("default API port {DEFAULT_API_PORT} unavailable ({e}); using a random port");
            tokio::net::TcpListener::bind(("127.0.0.1", 0))
                .await
                .map_err(|e| e.to_string())
        }
    }
}

pub async fn start() -> Result<(tokio::net::TcpListener, Router, u16), String> {
    let listener = bind_listener().await?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let (events, _) = broadcast::channel(2048);
    let sink: Arc<dyn EventSink> = Arc::new(ApiEventSink(events.clone()));
    let acp = AcpState::new(sink.clone());
    let bot_runner = Arc::new(opendots_acp::BotAgentRunner::new(acp.clone()));
    let runners: Vec<Arc<dyn AgentRunner>> = vec![bot_runner];
    let automation = AutomationHandle::start(runners, sink).await?;
    acp.set_api_port(port);
    let state = ApiState {
        acp,
        automation,
        events,
    };
    let app = Router::new()
        .route("/api/bots/list", post(bots_list))
        .route("/api/bots/create", post(bots_create))
        .route("/api/bots/update", post(bots_update))
        .route("/api/bots/delete", post(bots_delete))
        .route("/api/bots/sessions", post(bot_sessions))
        .route("/api/mcp/servers", get(mcp_servers))
        .route("/api/acp/agents", get(acp_agents))
        .route("/api/acp/install-agent", post(acp_install_agent))
        .route("/api/acp/start", post(acp_start))
        .route("/api/acp/prompt", post(acp_prompt))
        .route("/api/acp/cancel", post(acp_cancel))
        .route("/api/acp/new-session", post(acp_new_session))
        .route("/api/acp/load-session", post(acp_load_session))
        .route("/api/acp/session", post(acp_get_session))
        .route("/api/acp/sessions", post(acp_list_sessions))
        .route("/api/acp/delete-session", post(acp_delete_session))
        .route("/api/acp/respond-permission", post(acp_permission))
        .route("/api/acp/authenticate", post(acp_authenticate))
        .route("/api/acp/set-mode", post(acp_set_mode))
        .route("/api/acp/set-model", post(acp_set_model))
        .route("/api/acp/set-config-option", post(acp_set_config))
        .route("/api/acp/stop", post(acp_stop))
        .route("/api/automation/list", post(automation_list))
        .route("/api/automation/create", post(automation_create))
        .route("/api/automation/update", post(automation_update))
        .route("/api/automation/set-paused", post(automation_pause))
        .route("/api/automation/delete", post(automation_delete))
        .route("/api/automation/run-now", post(automation_run))
        .route("/api/automation/runs/list", post(automation_runs))
        .route("/api/events", get(events_sse))
        .route("/mcp/bots", post(bot_mcp))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers([header::CONTENT_TYPE]),
        )
        .with_state(state);
    Ok((listener, app, port))
}

pub async fn serve(listener: tokio::net::TcpListener, app: Router) -> Result<(), String> {
    axum::serve(listener, app).await.map_err(|e| e.to_string())
}

fn err(message: String) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error":message})),
    )
}
#[derive(Deserialize)]
struct ListBots {
    #[serde(default)]
    include_archived: bool,
}
async fn bots_list(Json(p): Json<ListBots>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_db::bots::list_bots(p.include_archived)
        .map(|v| Json(json!(v)))
        .map_err(err)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBot {
    id: String,
    name: String,
    avatar: String,
    color: String,
    cwd: String,
    agent_id: Option<String>,
    trust_level: Option<String>,
}
async fn bots_create(Json(p): Json<CreateBot>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_db::bots::create_bot(
        &p.id,
        &p.name,
        &p.avatar,
        &p.color,
        p.agent_id.as_deref().unwrap_or("keke"),
        &p.cwd,
        p.trust_level.as_deref().unwrap_or("ask"),
    )
    .map(|v| Json(json!(v)))
    .map_err(err)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateBot {
    id: String,
    #[serde(flatten)]
    patch: opendots_db::bots::BotPatch,
}
async fn bots_update(Json(p): Json<UpdateBot>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_db::bots::update_bot(&p.id, &p.patch)
        .map(|v| Json(json!(v)))
        .map_err(err)
}
#[derive(Deserialize)]
struct Id {
    id: String,
}
async fn bots_delete(
    State(state): State<ApiState>,
    Json(p): Json<Id>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Remove routines first so a deleted bot is not repeatedly scheduled for
    // unattended runs that can no longer resolve it.
    for task in opendots_automation::list_automations(&state.automation)
        .await
        .map_err(err)?
        .into_iter()
        .filter(|task| task.bot_id.as_deref() == Some(p.id.as_str()))
    {
        opendots_automation::delete_automation(&state.automation, task.id)
            .await
            .map_err(err)?;
    }
    opendots_db::bots::delete_bot(&p.id)
        .map(|_| Json(json!(null)))
        .map_err(err)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BotSessions {
    bot_id: String,
    limit: Option<usize>,
}
async fn bot_sessions(
    Json(p): Json<BotSessions>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_db::acp_sessions::list_bot_sessions(&p.bot_id, p.limit.unwrap_or(100))
        .map(|v| Json(json!(v)))
        .map_err(err)
}
async fn mcp_servers() -> Json<Value> {
    let configured = dirs::home_dir()
        .map(|home| home.join(".codex").join("config.toml"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| toml::from_str::<toml::Value>(&text).ok())
        .and_then(|value| value.get("mcp_servers").cloned())
        .and_then(|value| serde_json::to_value(value).ok())
        .unwrap_or_else(|| json!({}));
    Json(json!({ "mcpServers": configured }))
}
async fn acp_agents() -> Json<Value> {
    Json(json!(opendots_acp::list_agents()))
}
#[derive(Deserialize)]
struct AgentId {
    agent_id: String,
}
async fn acp_install_agent(
    Json(p): Json<AgentId>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    tokio::task::spawn_blocking(move || opendots_acp::install_preset(&p.agent_id))
        .await
        .map_err(|e| err(e.to_string()))?
        .map(|v| Json(json!(v)))
        .map_err(err)
}
#[derive(Deserialize)]
struct AcpStart {
    agent_id: String,
    cwd: String,
    bot_id: Option<String>,
    custom: Option<AcpAgentDef>,
}
async fn acp_start(
    State(s): State<ApiState>,
    Json(p): Json<AcpStart>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    s.acp
        .start(&p.agent_id, &p.cwd, p.custom, p.bot_id)
        .await
        .map(|v| Json(json!(v)))
        .map_err(err)
}
#[derive(Deserialize)]
struct AcpPrompt {
    connection_id: String,
    session_id: Option<String>,
    text: String,
}
async fn acp_prompt(
    State(s): State<ApiState>,
    Json(p): Json<AcpPrompt>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    s.acp
        .prompt(&p.connection_id, p.session_id.as_deref(), &p.text)
        .await
        .map(Json)
        .map_err(err)
}
#[derive(Deserialize)]
struct AcpSession {
    connection_id: String,
    cwd: Option<String>,
    session_id: Option<String>,
}
async fn acp_cancel(
    State(s): State<ApiState>,
    Json(p): Json<AcpSession>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    s.acp
        .cancel(&p.connection_id, p.session_id.as_deref())
        .await
        .map(|_| Json(json!(null)))
        .map_err(err)
}
async fn acp_new_session(
    State(s): State<ApiState>,
    Json(p): Json<AcpSession>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    s.acp
        .new_session(&p.connection_id, p.cwd.as_deref().unwrap_or("."))
        .await
        .map(Json)
        .map_err(err)
}
async fn acp_load_session(
    State(s): State<ApiState>,
    Json(p): Json<AcpSession>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let id = p
        .session_id
        .ok_or_else(|| err("session_id required".into()))?;
    s.acp
        .load_session(&p.connection_id, &id, p.cwd.as_deref().unwrap_or("."))
        .await
        .map(Json)
        .map_err(err)
}
async fn acp_authenticate(
    State(s): State<ApiState>,
    Json(p): Json<Auth>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    s.acp
        .authenticate(&p.connection_id, &p.method_id)
        .await
        .map(|_| Json(json!(null)))
        .map_err(err)
}
#[derive(Deserialize)]
struct Auth {
    connection_id: String,
    method_id: String,
}
#[derive(Deserialize)]
struct SessionId {
    session_id: String,
}
async fn acp_get_session(
    Json(p): Json<SessionId>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_acp::get_updates(&p.session_id)
        .map(|v| Json(json!(v)))
        .map_err(err)
}
#[derive(Deserialize)]
struct SessionList {
    cwd: Option<String>,
    limit: Option<usize>,
}
async fn acp_list_sessions(
    Json(p): Json<SessionList>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_acp::list_sessions(p.cwd.as_deref(), p.limit.unwrap_or(100))
        .map(|v| Json(json!(v)))
        .map_err(err)
}
async fn acp_delete_session(
    Json(p): Json<SessionId>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_acp::delete_session(&p.session_id)
        .map(|_| Json(json!(null)))
        .map_err(err)
}
#[derive(Deserialize)]
struct Permission {
    connection_id: String,
    request_id: String,
    option_id: Option<String>,
}
async fn acp_permission(
    State(s): State<ApiState>,
    Json(p): Json<Permission>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    s.acp
        .respond_permission(&p.connection_id, &p.request_id, p.option_id)
        .map(|_| Json(json!(null)))
        .map_err(err)
}
#[derive(Deserialize)]
struct SetMode {
    connection_id: String,
    session_id: Option<String>,
    mode_id: String,
}
async fn acp_set_mode(
    State(s): State<ApiState>,
    Json(p): Json<SetMode>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    s.acp
        .set_mode(&p.connection_id, p.session_id.as_deref(), &p.mode_id)
        .await
        .map(Json)
        .map_err(err)
}
#[derive(Deserialize)]
struct SetModel {
    connection_id: String,
    session_id: Option<String>,
    model_id: String,
    reasoning_effort: Option<String>,
}
async fn acp_set_model(
    State(s): State<ApiState>,
    Json(p): Json<SetModel>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    s.acp
        .set_model(
            &p.connection_id,
            p.session_id.as_deref(),
            &p.model_id,
            p.reasoning_effort.as_deref(),
        )
        .await
        .map(Json)
        .map_err(err)
}
#[derive(Deserialize)]
struct SetConfig {
    connection_id: String,
    session_id: Option<String>,
    config_id: String,
    value: Value,
}
async fn acp_set_config(
    State(s): State<ApiState>,
    Json(p): Json<SetConfig>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    s.acp
        .set_config_option(
            &p.connection_id,
            p.session_id.as_deref(),
            &p.config_id,
            &p.value,
        )
        .await
        .map(Json)
        .map_err(err)
}
async fn acp_stop(
    State(s): State<ApiState>,
    Json(p): Json<Connection>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    s.acp
        .stop(&p.connection_id)
        .await
        .map(|_| Json(json!(null)))
        .map_err(err)
}
#[derive(Deserialize)]
struct Connection {
    connection_id: String,
}
#[derive(Deserialize)]
struct AutomationListBody {
    #[serde(default)]
    _unused: bool,
}

#[derive(Deserialize)]
struct RoutineInput {
    name: String,
    projects: Vec<String>,
    prompt: String,
    schedule: AutomationSchedule,
    agent: Option<String>,
    model_provider: Option<String>,
    model: Option<String>,
    cwd_mode: Option<CwdMode>,
    bot_id: Option<String>,
}
impl From<RoutineInput> for AutomationInput {
    fn from(p: RoutineInput) -> Self {
        Self {
            name: p.name,
            projects: p.projects,
            prompt: p.prompt,
            schedule: p.schedule,
            agent: p.agent,
            model_provider: p.model_provider,
            model: p.model,
            cwd_mode: p.cwd_mode,
            bot_id: p.bot_id,
        }
    }
}
async fn automation_list(
    State(s): State<ApiState>,
    Json(_p): Json<AutomationListBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_automation::list_automations(&s.automation)
        .await
        .map(|v| Json(json!(v)))
        .map_err(err)
}
async fn automation_create(
    State(s): State<ApiState>,
    Json(p): Json<RoutineInput>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_automation::create_automation(&s.automation, p.into())
        .await
        .map(|v| Json(json!(v)))
        .map_err(err)
}
#[derive(Deserialize)]
struct RoutineUpdate {
    id: String,
    #[serde(flatten)]
    input: RoutineInput,
}
async fn automation_update(
    State(s): State<ApiState>,
    Json(p): Json<RoutineUpdate>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_automation::update_automation(&s.automation, p.id, p.input.into())
        .await
        .map(|v| Json(json!(v)))
        .map_err(err)
}
#[derive(Deserialize)]
struct Pause {
    id: String,
    paused: bool,
}
async fn automation_pause(
    State(s): State<ApiState>,
    Json(p): Json<Pause>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_automation::set_automation_paused(&s.automation, p.id, p.paused)
        .await
        .map(|v| Json(json!(v)))
        .map_err(err)
}
async fn automation_delete(
    State(s): State<ApiState>,
    Json(p): Json<Id>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_automation::delete_automation(&s.automation, p.id)
        .await
        .map(|_| Json(json!(null)))
        .map_err(err)
}
async fn automation_run(
    State(s): State<ApiState>,
    Json(p): Json<Id>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_automation::run_automation_now(&s.automation, p.id)
        .await
        .map(|_| Json(json!(null)))
        .map_err(err)
}
#[derive(Deserialize)]
struct RunList {
    task_id: Option<String>,
    limit: Option<u32>,
}
async fn automation_runs(Json(p): Json<RunList>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    opendots_automation::list_automation_runs(p.task_id, p.limit)
        .await
        .map(|v| Json(json!(v)))
        .map_err(err)
}

async fn events_sse(State(s): State<ApiState>) -> impl IntoResponse {
    let stream = tokio_stream::wrappers::BroadcastStream::new(s.events.subscribe()).filter_map(
        |item| async move {
            let (event, payload) = item.ok()?;
            let text = serde_json::to_string(&json!({"event":event,"payload":payload})).ok()?;
            Some(Ok::<_, std::convert::Infallible>(
                axum::response::sse::Event::default().data(text),
            ))
        },
    );
    axum::response::sse::Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

#[derive(Deserialize)]
struct FromBot {
    from: Option<String>,
}
async fn bot_mcp(
    State(s): State<ApiState>,
    Query(q): Query<FromBot>,
    Json(request): Json<Value>,
) -> impl IntoResponse {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => {
            json!({"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"opendots-bots","version":env!("CARGO_PKG_VERSION")}})
        }
        "ping" => json!({}),
        "tools/list" => json!({"tools":[
            {"name":"list_bots","description":"List other active bots by id, name and role.","inputSchema":{"type":"object","properties":{}}},
            {"name":"ask_bot","description":"Ask another bot to perform a self-contained task.","inputSchema":{"type":"object","properties":{"bot":{"type":"string"},"message":{"type":"string"}},"required":["bot","message"]}}
        ]}),
        "tools/call" => {
            let tool = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            if tool == "list_bots" {
                match opendots_db::bots::list_bots(false) {
                    Ok(bots) => {
                        json!({"content":[{"type":"text","text":json!(bots.into_iter().filter(|b|Some(b.id.as_str())!=q.from.as_deref()).map(|b|json!({"id":b.id,"name":b.name,"title":b.title})).collect::<Vec<_>>()).to_string()}]})
                    }
                    Err(e) => json!({"isError":true,"content":[{"type":"text","text":e}]}),
                }
            } else if tool == "ask_bot" {
                let wanted = args.get("bot").and_then(Value::as_str).unwrap_or("");
                let prompt = args.get("message").and_then(Value::as_str).unwrap_or("");
                let target = opendots_db::bots::list_bots(false).ok().and_then(|bots| {
                    bots.into_iter().find(|b| {
                        Some(b.id.as_str()) != q.from.as_deref()
                            && (b.id == wanted || b.name.eq_ignore_ascii_case(wanted))
                    })
                });
                match target {
                    Some(bot) => match s
                        .acp
                        .run_bot_unattended(&bot.id, prompt, None, false, |_| {})
                        .await
                    {
                        Ok(report) => {
                            json!({"content":[{"type":"text","text":report.reply}],"isError":false})
                        }
                        Err(e) => json!({"isError":true,"content":[{"type":"text","text":e}]}),
                    },
                    None => {
                        json!({"isError":true,"content":[{"type":"text","text":"Bot not found"}]})
                    }
                }
            } else {
                json!({"isError":true,"content":[{"type":"text","text":"Unknown tool"}]})
            }
        }
        _ => json!({"error":{"code":-32601,"message":"Method not found"}}),
    };
    (
        StatusCode::OK,
        Json(json!({"jsonrpc":"2.0","id":id,"result":result})),
    )
}
