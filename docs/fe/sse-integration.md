# SSE Integration Guide - Citizen Report Agent Chat

This document describes the Server-Sent Events (SSE) integration for the citizen report agent chat feature. Frontend developers can use this guide to implement real-time streaming chat UI.

## Endpoint

```
POST /api/citizen-report-agent/chat
Content-Type: application/json
Authorization: Bearer <token>
Accept: text/event-stream
```

## Request Body

```typescript
interface ChatRequest {
  // Optional thread ID
  // - undefined: Creates a new thread
  // - UUID not found: Creates thread with this ID (optimistic UI)
  // - UUID found: Uses existing thread
  thread_id?: string;

  // Optional user message ID for optimistic UI or edit mode
  // - undefined: Auto-generates message ID
  // - UUID not found: Creates message with this ID (optimistic UI)
  // - UUID found: Edit mode - updates message and deletes all subsequent messages
  user_message_id?: string;

  // Message content - can be string or multimodal blocks
  content: string | ContentBlock[];
}

// Content block types for multimodal messages
type ContentBlock =
  | { type: "text"; text: string }
  | { type: "file"; url: string }
  | { type: "file_data"; mime_type: string; data: string }; // base64 encoded
```

### Example Requests

**Simple text message (new thread):**
```json
{
  "content": "Saya ingin melaporkan jalan rusak"
}
```

**Continue existing thread:**
```json
{
  "thread_id": "550e8400-e29b-41d4-a716-446655440000",
  "content": "Lokasi di Jl. Sudirman No. 123"
}
```

**Optimistic UI (FE generates IDs):**
```json
{
  "thread_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_message_id": "660e8400-e29b-41d4-a716-446655440001",
  "content": "Lampiran foto kerusakan"
}
```

**Multimodal with image:**
```json
{
  "content": [
    { "type": "text", "text": "Ini foto kerusakan jalan" },
    { "type": "file", "url": "https://storage.example.com/image.jpg" }
  ]
}
```

---

## SSE Event Types

All events follow the format:
```
event: <event_type>
data: <json_payload>

```

### Event Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                       Message Lifecycle                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  message.started                                                 │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Block Lifecycle (repeats)                   │    │
│  │                                                          │    │
│  │  block.created ──► block.delta (many) ──► block.completed│    │
│  │       │                                                  │    │
│  │       ├── text block                                     │    │
│  │       ├── thought block (thinking/reasoning)             │    │
│  │       └── tool_call block ──► tool.execution_started     │    │
│  │                               tool.execution_completed   │    │
│  │                               block.created (tool_result)│    │
│  │                               block.completed            │    │
│  └─────────────────────────────────────────────────────────┘    │
│       │                                                          │
│       ▼                                                          │
│  message.usage (optional)                                        │
│       │                                                          │
│       ▼                                                          │
│  message.completed                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Event Reference

### 1. `message.started`

Marks the beginning of an assistant message.

```typescript
interface MessageStartedEvent {
  message_id: string;    // "msg_<uuid>"
  thread_id: string;     // Thread UUID
  role: "assistant";
  model: string;         // Model name used
  timestamp: string;     // ISO 8601 timestamp
}
```

**Example:**
```json
event: message.started
data: {"message_id":"msg_550e8400-e29b-41d4-a716-446655440000","thread_id":"660e8400-e29b-41d4-a716-446655440001","role":"assistant","model":"gpt-4o","timestamp":"2024-01-15T10:30:00.000Z"}
```

---

### 2. `block.created`

A new content block has started. Block types: `text`, `thought`, `tool_call`, `tool_result`.

```typescript
interface BlockCreatedEvent {
  message_id: string;
  block_id: string;      // "block_<uuid>"
  block_type: "text" | "thought" | "tool_call" | "tool_result";
  index: number;         // Block order in message

  // Only for tool_call blocks:
  tool_name?: string;
  tool_call_id?: string;

  // Only for tool_result blocks:
  tool_call_id?: string;
  tool_name?: string;
}
```

**Example (text block):**
```json
event: block.created
data: {"message_id":"msg_...","block_id":"block_abc123","block_type":"text","index":0}
```

**Example (tool_call block):**
```json
event: block.created
data: {"message_id":"msg_...","block_id":"block_def456","block_type":"tool_call","index":1,"tool_name":"search_reports","tool_call_id":"call_xyz789"}
```

---

### 3. `block.delta`

Streaming content update for a block. This is the main event for real-time text display.

#### Text Delta

```typescript
interface BlockDeltaTextEvent {
  message_id: string;
  block_id: string;
  block_type: "text";
  delta: {
    text: string;        // Text chunk to append
  };
}
```

**Example:**
```json
event: block.delta
data: {"message_id":"msg_...","block_id":"block_abc123","block_type":"text","delta":{"text":"Terima kasih atas"}}
```

#### Thought Delta (Extended Thinking)

```typescript
interface BlockDeltaThoughtEvent {
  message_id: string;
  block_id: string;
  block_type: "thought";
  delta: {
    text?: string;       // Thought text chunk
    signature?: string;  // Optional signature for verification
  };
}
```

**Example:**
```json
event: block.delta
data: {"message_id":"msg_...","block_id":"block_thought1","block_type":"thought","delta":{"text":"Analyzing the user's report..."}}
```

#### Tool Call Delta

```typescript
interface BlockDeltaToolCallEvent {
  message_id: string;
  block_id: string;
  block_type: "tool_call";
  tool_name: string;
  tool_call_id: string;
  delta: {
    arguments: string;   // JSON arguments chunk
  };
  partial_arguments: string;  // Accumulated arguments so far
}
```

**Example:**
```json
event: block.delta
data: {"message_id":"msg_...","block_id":"block_def456","block_type":"tool_call","tool_name":"search_reports","tool_call_id":"call_xyz789","delta":{"arguments":"{\"query\":"},"partial_arguments":"{\"query\":"}
```

#### Tool Result Delta

```typescript
interface BlockDeltaToolResultEvent {
  message_id: string;
  block_id: string;
  block_type: "tool_result";
  tool_call_id: string;
  tool_name: string;
  delta: {
    result?: any;        // Tool result (if success)
    error?: string;      // Error message (if failed)
    success: boolean;
  };
}
```

---

### 4. `block.completed`

Block has finished streaming.

#### Text Block Completed

```typescript
interface BlockCompletedTextEvent {
  message_id: string;
  block_id: string;
  block_type: "text";
  final_content: string;  // Complete text content
}
```

#### Thought Block Completed

```typescript
interface BlockCompletedThoughtEvent {
  message_id: string;
  block_id: string;
  block_type: "thought";
  final_content: string;
  signature?: string;
}
```

#### Tool Call Block Completed

```typescript
interface BlockCompletedToolCallEvent {
  message_id: string;
  block_id: string;
  block_type: "tool_call";
  tool_name: string;
  tool_call_id: string;
  final_arguments: string;      // Raw JSON string
  parsed_arguments: object;     // Parsed JSON object
}
```

#### Tool Result Block Completed

```typescript
interface BlockCompletedToolResultEvent {
  message_id: string;
  block_id: string;
  block_type: "tool_result";
  tool_call_id: string;
  tool_name: string;
  success: boolean;
  result?: any;                 // Tool result (if success)
  error?: string;               // Error message (if failed)
  execution_time_ms: number;
}
```

---

### 5. `tool.execution_started`

Tool execution has begun. Use this to show loading state.

```typescript
interface ToolExecutionStartedEvent {
  message_id: string;
  block_id: string;
  tool_call_id: string;
  tool_name: string;
  arguments: object;
  started_at: string;    // ISO 8601 timestamp
}
```

**Example:**
```json
event: tool.execution_started
data: {"message_id":"msg_...","block_id":"block_...","tool_call_id":"call_xyz789","tool_name":"search_reports","arguments":{"query":"jalan rusak"},"started_at":"2024-01-15T10:30:05.000Z"}
```

---

### 6. `tool.execution_completed`

Tool execution finished successfully.

```typescript
interface ToolExecutionCompletedEvent {
  message_id: string;
  block_id: string;
  tool_call_id: string;
  tool_name: string;
  success: true;
  result: any;
  execution_time_ms: number;
  completed_at: string;
}
```

---

### 7. `tool.execution_failed`

Tool execution failed.

```typescript
interface ToolExecutionFailedEvent {
  message_id: string;
  block_id: string;
  tool_call_id: string;
  tool_name: string;
  success: false;
  error: {
    code: string;
    message: string;
    details?: string;
  };
  execution_time_ms: number;
  failed_at: string;
}
```

---

### 8. `message.usage`

Token usage statistics.

```typescript
interface MessageUsageEvent {
  message_id: string;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
}
```

---

### 9. `message.completed`

Message is complete. This is a **terminal event**.

```typescript
interface MessageCompletedEvent {
  message_id: string;
  thread_id: string;
  total_blocks: number;
  finish_reason: "stop" | "tool_calls" | "max_tokens" | "error";
  timestamp: string;
}
```

---

### 10. `error`

An error occurred. This is a **terminal event**.

```typescript
interface ErrorEvent {
  type: string;          // Error type identifier
  message: string;       // Human-readable error message
}
```

**Example:**
```json
event: error
data: {"type":"agent_error","message":"Failed to process request: rate limit exceeded"}
```

---

## Frontend Implementation Example

### TypeScript/React Example

```typescript
interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  blocks: Block[];
  isStreaming: boolean;
}

interface Block {
  id: string;
  type: 'text' | 'thought' | 'tool_call' | 'tool_result';
  content: string;
  isComplete: boolean;
  // For tool blocks
  toolName?: string;
  toolCallId?: string;
  isExecuting?: boolean;
}

function useChatStream() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const currentMessageRef = useRef<ChatMessage | null>(null);
  const blocksRef = useRef<Map<string, Block>>(new Map());

  const sendMessage = async (content: string, threadId?: string) => {
    const response = await fetch('/api/citizen-report-agent/chat', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify({ content, thread_id: threadId }),
    });

    const reader = response.body?.getReader();
    const decoder = new TextDecoder();
    let buffer = '';

    while (reader) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      let eventType = '';
      let eventData = '';

      for (const line of lines) {
        if (line.startsWith('event: ')) {
          eventType = line.slice(7);
        } else if (line.startsWith('data: ')) {
          eventData = line.slice(6);

          if (eventType && eventData) {
            handleEvent(eventType, JSON.parse(eventData));
            eventType = '';
            eventData = '';
          }
        }
      }
    }
  };

  const handleEvent = (type: string, data: any) => {
    switch (type) {
      case 'message.started':
        currentMessageRef.current = {
          id: data.message_id,
          role: 'assistant',
          blocks: [],
          isStreaming: true,
        };
        blocksRef.current.clear();
        setMessages(prev => [...prev, currentMessageRef.current!]);
        break;

      case 'block.created':
        const newBlock: Block = {
          id: data.block_id,
          type: data.block_type,
          content: '',
          isComplete: false,
          toolName: data.tool_name,
          toolCallId: data.tool_call_id,
        };
        blocksRef.current.set(data.block_id, newBlock);
        updateCurrentMessage();
        break;

      case 'block.delta':
        const block = blocksRef.current.get(data.block_id);
        if (block && data.delta?.text) {
          block.content += data.delta.text;
          updateCurrentMessage();
        }
        break;

      case 'tool.execution_started':
        const toolBlock = blocksRef.current.get(data.block_id);
        if (toolBlock) {
          toolBlock.isExecuting = true;
          updateCurrentMessage();
        }
        break;

      case 'tool.execution_completed':
      case 'tool.execution_failed':
        const execBlock = blocksRef.current.get(data.block_id);
        if (execBlock) {
          execBlock.isExecuting = false;
          updateCurrentMessage();
        }
        break;

      case 'block.completed':
        const completedBlock = blocksRef.current.get(data.block_id);
        if (completedBlock) {
          completedBlock.isComplete = true;
          if (data.final_content) {
            completedBlock.content = data.final_content;
          }
          updateCurrentMessage();
        }
        break;

      case 'message.completed':
        if (currentMessageRef.current) {
          currentMessageRef.current.isStreaming = false;
          updateCurrentMessage();
        }
        break;

      case 'error':
        console.error('SSE Error:', data.message);
        // Handle error state
        break;
    }
  };

  const updateCurrentMessage = () => {
    if (currentMessageRef.current) {
      currentMessageRef.current.blocks = Array.from(blocksRef.current.values());
      setMessages(prev => {
        const updated = [...prev];
        const idx = updated.findIndex(m => m.id === currentMessageRef.current!.id);
        if (idx >= 0) {
          updated[idx] = { ...currentMessageRef.current! };
        }
        return updated;
      });
    }
  };

  return { messages, sendMessage };
}
```

---

## Keepalive

The server sends keepalive pings every **15 seconds**:

```
: ping
```

This is a comment line (starts with `:`) and should be ignored by the client. Most EventSource implementations handle this automatically.

---

## Error Handling

### HTTP Errors

| Status | Description |
|--------|-------------|
| 400 | Validation error (invalid request body) |
| 401 | Unauthorized (missing or invalid token) |
| 403 | Forbidden (thread belongs to another user) |
| 404 | Thread not found |
| 502 | AI service error |

### SSE Errors

If an error occurs during streaming, an `error` event will be sent followed by stream termination.

---

## Thread Lifecycle & Optimistic UI

### Creating New Threads

1. Send request without `thread_id`
2. Backend creates thread and returns events
3. Get `thread_id` from `message.started` event
4. Store for subsequent messages

### Optimistic UI Pattern

Frontend can generate UUIDs for immediate UI updates:

1. Generate `thread_id` and `user_message_id` client-side
2. Show message immediately in UI
3. Send request with both IDs
4. Backend accepts the IDs (creates if not exists)
5. No need to wait for server response to show user message

### Edit Message (Regenerate)

To edit a previous message and regenerate response:

1. Send request with existing `thread_id` and `user_message_id`
2. Backend detects existing message
3. Updates message content
4. Deletes all messages after this one
5. Generates new response

---

## Best Practices

1. **Always handle `error` events** - Display user-friendly error messages
2. **Show streaming indicator** - Use `isStreaming` state for visual feedback
3. **Handle tool execution states** - Show loading spinners during tool calls
4. **Buffer partial events** - SSE data may arrive in chunks
5. **Implement reconnection** - Handle network disconnections gracefully
6. **Store thread_id** - Persist for conversation continuity
7. **Use optimistic UI** - Generate client-side IDs for snappy UX
