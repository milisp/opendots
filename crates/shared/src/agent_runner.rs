use async_trait::async_trait;
use std::sync::Arc;

pub type RunStartedHook = Arc<dyn Fn(&str) + Send + Sync>;

pub struct AgentRunSpec {
    pub task_id: String,
    pub task_name: String,
    pub prompt: String,
    pub model: String,
    pub model_provider: String,
    pub cwd: Option<String>,
    pub bot_id: Option<String>,
    pub on_started: RunStartedHook,
}

pub enum AgentRunOutcome {
    Finished,
    Detached,
}

#[async_trait]
pub trait AgentRunner: Send + Sync {
    fn agent(&self) -> &'static str;
    async fn start_run(&self, spec: AgentRunSpec) -> Result<AgentRunOutcome, String>;
}
