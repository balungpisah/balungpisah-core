use std::sync::Arc;

use balungpisah_adk::{
    Agent, AgentBuilder, ChatRequest, MessageContent, PostgresStorage, Storage, TensorZeroClient,
    ToolRegistry,
};
use serde_json::json;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::shared::prompts::render_citizen_report_agent_prompt;

/// Service for managing agent runtime and chat operations
pub struct AgentRuntimeService {
    tensorzero_client: TensorZeroClient,
    storage: Arc<PostgresStorage>,
    openai_api_key: String,
    model_name: String,
    tools: ToolRegistry,
}

impl AgentRuntimeService {
    /// Create a new AgentRuntimeService
    #[allow(dead_code)]
    pub fn new(
        tensorzero_client: TensorZeroClient,
        storage: Arc<PostgresStorage>,
        openai_api_key: String,
        model_name: String,
    ) -> Self {
        Self {
            tensorzero_client,
            storage,
            openai_api_key,
            model_name,
            tools: ToolRegistry::new(),
        }
    }

    /// Create a new AgentRuntimeService with tools
    pub fn with_tools(
        tensorzero_client: TensorZeroClient,
        storage: Arc<PostgresStorage>,
        openai_api_key: String,
        model_name: String,
        tools: ToolRegistry,
    ) -> Self {
        Self {
            tensorzero_client,
            storage,
            openai_api_key,
            model_name,
            tools,
        }
    }

    /// Run database migrations for ADK tables
    #[allow(dead_code)]
    pub async fn migrate(&self) -> Result<()> {
        self.storage
            .migrate()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to run ADK migrations: {}", e)))
    }

    /// Build an agent instance for a chat session
    fn build_agent(&self) -> Result<Agent<PostgresStorage>> {
        self.build_agent_with_context(None)
    }

    /// Build an agent instance with optional attachment context
    fn build_agent_with_context(
        &self,
        attachment_context: Option<&str>,
    ) -> Result<Agent<PostgresStorage>> {
        // Render system prompt from template with dynamic context
        let system_prompt = render_citizen_report_agent_prompt(attachment_context)
            .map_err(|e| AppError::Internal(format!("Failed to render prompt template: {}", e)))?;

        AgentBuilder::new()
            .tensorzero_client(self.tensorzero_client.clone())
            .storage(Arc::clone(&self.storage))
            .model_name(&self.model_name)
            .credentials(json!({
                "system_api_key": self.openai_api_key
            }))
            .system_prompt(&system_prompt)
            .tools(self.tools.clone())
            .max_iterations(10)
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to build agent: {}", e)))
    }

    /// Get or create a thread for a user
    pub async fn get_or_create_thread(
        &self,
        external_id: &str,
        thread_id: Option<Uuid>,
    ) -> Result<balungpisah_adk::Thread> {
        let agent = self.build_agent()?;

        if let Some(tid) = thread_id {
            // Verify thread exists and belongs to this user
            let thread = agent
                .get_thread(tid)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to get thread: {}", e)))?;

            match thread {
                Some(t) if t.external_id == external_id => Ok(t),
                Some(_) => Err(AppError::Forbidden(
                    "Thread does not belong to this user".to_string(),
                )),
                None => Err(AppError::NotFound(format!("Thread {} not found", tid))),
            }
        } else {
            // Create new thread
            agent
                .get_or_create_thread(external_id, None)
                .await
                .map_err(|e| AppError::Internal(format!("Failed to create thread: {}", e)))
        }
    }

    /// Send a chat message and get a synchronous response
    pub async fn chat_sync(
        &self,
        external_id: &str,
        thread_id: Option<Uuid>,
        content: MessageContent,
        attachment_context: Option<&str>,
    ) -> Result<(Uuid, String, Uuid)> {
        let thread = self.get_or_create_thread(external_id, thread_id).await?;
        let agent = self.build_agent_with_context(attachment_context)?;

        let response = agent
            .chat(thread.id, content)
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("Chat failed: {}", e)))?;

        Ok((thread.id, response.text, response.episode_id))
    }

    /// Send a chat message and get a streaming response with full lifecycle support.
    ///
    /// Returns raw SSE-formatted strings that can be directly forwarded to HTTP responses.
    /// Each string is a complete SSE event like:
    /// ```text
    /// event: block.delta
    /// data: {"message_id":"msg_...","block_id":"block_...","delta":{"text":"Hello"}}
    ///
    /// ```
    ///
    /// ## Thread Lifecycle
    /// - `thread_id = None`: Create new thread with auto-generated ID
    /// - `thread_id = Some(id)` not found: Create thread with provided ID (optimistic UI)
    /// - `thread_id = Some(id)` found: Use existing thread (verifies ownership)
    ///
    /// ## Message Lifecycle
    /// - `user_message_id = None`: Create new message with auto-generated ID
    /// - `user_message_id = Some(id)` not found: Create message with provided ID (optimistic UI)
    /// - `user_message_id = Some(id)` found: Edit mode - update and delete subsequent messages
    pub async fn chat_stream(
        &self,
        external_id: &str,
        thread_id: Option<Uuid>,
        user_message_id: Option<Uuid>,
        content: MessageContent,
        attachment_context: Option<&str>,
    ) -> Result<(Uuid, mpsc::Receiver<String>)> {
        let agent = self.build_agent_with_context(attachment_context)?;

        // Build the chat request with full lifecycle support
        let mut request = ChatRequest::new(content);

        if let Some(tid) = thread_id {
            request = request.thread_id(tid);
        }

        if let Some(mid) = user_message_id {
            request = request.user_message_id(mid);
        }

        // Use chat_with_request for full lifecycle support
        let response = agent
            .chat_with_request(external_id, request)
            .await
            .map_err(|e| match e {
                balungpisah_adk::AgentError::ThreadAccessDenied { thread_id, reason } => {
                    AppError::Forbidden(format!("Thread {} access denied: {}", thread_id, reason))
                }
                balungpisah_adk::AgentError::MessageAccessDenied { message_id, reason } => {
                    AppError::Forbidden(format!("Message {} access denied: {}", message_id, reason))
                }
                _ => AppError::ExternalServiceError(format!("Chat stream failed: {}", e)),
            })?;

        Ok((response.thread_id, response.stream))
    }

    /// Get access to the storage for conversation queries
    #[allow(dead_code)]
    pub fn storage(&self) -> &Arc<PostgresStorage> {
        &self.storage
    }
}
