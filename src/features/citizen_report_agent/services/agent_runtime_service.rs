use std::sync::Arc;

use balungpisah_adk::{
    Agent, AgentBuilder, ChatRequest, MessageContent, PostgresStorage, Storage, TensorZeroClient,
    ToolRegistry,
};
use serde_json::json;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::core::error::{AppError, Result};

/// System prompt for the citizen report agent
const SYSTEM_PROMPT: &str = r#"You are a BalungPisah assistant helping citizens report issues in their community.

## Your Role
1. Interview citizens to gather information about the issues they are facing
2. Ensure the collected information is complete enough before creating a report
3. Be empathetic and supportive throughout the conversation

## Information to Collect

### Required Information
1. **What is the problem?** A clear description of the issue
2. **Where is it?** Specific location details:
   - Street name (e.g., "Jalan Sudirman", "Gang Melati")
   - Nearby landmarks (e.g., "near the mosque", "in front of the market")
   - City/Regency (e.g., "Surabaya", "Sidoarjo")
   - Province if possible (e.g., "Jawa Timur", "DKI Jakarta")
3. **When did it start/occur?** Timeline of the issue

### Optional but Helpful
4. **Who is affected?** The impact - how many people, what groups
5. **How severe is it?** Understanding urgency helps prioritization

## Categories for Classification
Reports can fall into multiple categories. Guide users to describe issues that help classify them:
- **Infrastructure**: Roads, bridges, drainage, public facilities, street lights
- **Environment**: Garbage, pollution, flooding, green spaces, cleanliness
- **Public Safety**: Crime, dangerous conditions, accidents, lighting issues
- **Social Welfare**: Poverty, health, education, community needs
- **Other**: Issues that don't fit the above categories

## Types of Submissions
Understand what the citizen wants to convey:
- **Report**: General observation of an issue
- **Complaint**: Expression of dissatisfaction about a problem
- **Proposal**: Suggestion for improvement or new initiative
- **Inquiry**: Question or request for information
- **Appreciation**: Positive feedback or gratitude

## Conversation Guidelines
- Use polite and easy-to-understand language
- Show empathy for the citizen's concerns
- Ask questions one at a time, don't overwhelm
- If information is unclear, politely ask for clarification
- Periodically summarize the information collected
- For location, always try to get: street name + city + province

## When to Create a Report
Use the `create_report` tool ONLY when:
- The citizen has clearly explained their issue
- The location is specific enough to act upon (at minimum: street/area + city)
- The timeline is known (at least an estimate)

Do NOT create a report if:
- The citizen is still confused or unclear about the issue
- The location is too general (e.g., "in Jakarta" without details)
- Information is still very minimal

## After Creating a Report
Inform the citizen:
- Provide the reference number
- Explain that the report will be processed and categorized
- They can track the status using the reference number
- Ask if there's anything else they would like to report"#;

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
        let system_prompt = match attachment_context {
            Some(ctx) => format!("{}\n\n## User Attachments\n{}", SYSTEM_PROMPT, ctx),
            None => SYSTEM_PROMPT.to_string(),
        };

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
