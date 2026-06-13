# VibePilot API Reference

## Overview

This document describes the public API and architecture of VibePilot, the Visual AI Automation Orchestrator.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                        VibePilot                        │
├─────────────┬──────────────┬──────────────┬────────────┤
│  UI (egui)  │ Orchestrator │ LLM Client   │ Config     │
│             │              │              │            │
│ - Tab routing│ - Decision  │ - HTTP req   │ - DPAPI    │
│ - Timeline   │   loop      │ - Base64 img │ - TOML     │
│ - Console    │ - Action    │ - Vision     │ - Profiles │
│              │   execution │   models     │            │
└─────────────┴──────────────┴──────────────┴────────────┘
         │               │              │
         ▼               ▼              ▼
    Windows Desktop   LLM Server    Encrypted
    (screen capture)  (vision API)  Storage
```

## Core Modules

### `orchestrator` — Decision Loop

The core automation engine. Manages the observation-decision-action loop:

1. **Capture** — Takes a screenshot of the target window
2. **Analyze** — Sends screenshot + prompts to LLM
3. **Decide** — Parses LLM response into structured actions
4. **Execute** — Performs the action (click, type, scroll, wait)
5. **Verify** — Takes another screenshot to verify the action

```rust
// Key types (simplified)
pub struct Orchestrator {
    pub config: GlobalConfig,
    pub profiles: Vec<PromptProfile>,
    pub action_log: Vec<ActionEntry>,
    pub is_running: bool,
}

pub enum Action {
    Click { x: i32, y: i32, button: MouseButton },
    Type { text: String },
    Scroll { direction: ScrollDirection, amount: i32 },
    Wait { duration_ms: u64 },
    KeyPress { key: VirtualKeyCode },
    Custom { command: String },
}
```

### `llm_client` — LLM Communication

Handles HTTP requests to vision-capable LLM endpoints:

```rust
pub enum LlmEngine {
    LmStudio,
    Ollama,
    OpenAI,
    Custom { url: String },
}

pub struct LlmClient {
    pub engine: LlmEngine,
    pub api_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

// Request format sent to LLM
pub struct LlmRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub screenshots: Vec<Base64Image>,
    pub previous_actions: Vec<ActionEntry>,
}

// Expected response from LLM
pub struct LlmResponse {
    pub action: Action,
    pub confidence: f32,
    pub reasoning: String,
    pub should_stop: bool,
}
```

### `screen_capture` — Screenshot Capture

Windows GDI-based screen capture:

```rust
pub struct ScreenCapture;

impl ScreenCapture {
    // Capture entire screen
    pub fn capture_screen() -> Result<ImageBuffer>;
    
    // Capture specific window
    pub fn capture_window(hwnd: isize) -> Result<ImageBuffer>;
    
    // Capture specific region
    pub fn capture_region(x: i32, y: i32, w: i32, h: i32) -> Result<ImageBuffer>;
}
```

### `peripheral_controller` — Input Simulation

Uses `rdev` library for mouse/keyboard simulation:

```rust
pub struct PeripheralController;

impl PeripheralController {
    pub fn click(x: i32, y: i32, button: MouseButton) -> Result<()>;
    pub fn type_text(text: &str) -> Result<()>;
    pub fn scroll(direction: ScrollDirection, amount: i32) -> Result<()>;
    pub fn wait(duration_ms: u64);
    pub fn move_mouse(x: i32, y: i32) -> Result<()>;
}
```

### `config` — Configuration Management

Handles encrypted configuration storage using Windows DPAPI:

```rust
pub struct GlobalConfig {
    pub llm_engine: LlmEngine,
    pub api_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub target_window: Option<String>,
    pub auto_validate: bool,
    pub language: Language,
    pub theme: Theme,
}

impl GlobalConfig {
    pub fn save(&self) -> Result<()>;    // Encrypt and save
    pub fn load() -> Result<Self>;        // Decrypt and load
}
```

### `event_bus` — Event System

Thread-safe event bus using flume channels for inter-module communication:

```rust
pub enum Event {
    ActionExecuted(ActionEntry),
    LlmResponseReceived(LlmResponse),
    ScreenshotTaken(ImageBuffer),
    Error(String),
    StatusChanged(String),
    ProfileChanged(PromptProfile),
}

impl EventBus {
    pub fn subscribe(&self) -> Receiver<Event>;
    pub fn publish(&self, event: Event);
}
```

## Prompt Profiles

The core configuration unit. Defines what the AI should do:

```toml
# Profile format (stored as .enc files)
[prompt_profile]
name = "My Automation"
context = "You are automating a web form..."
task = "Fill the form and submit"
objective = "Complete the registration"
directives = [
    "Wait for elements to load before interacting",
    "Verify each action with a screenshot",
]
```

## Action Timeline

VibePilot maintains a visual timeline of all actions:

```rust
pub struct ActionEntry {
    pub step: u64,
    pub action: Action,
    pub screenshot_before: Option<Base64Image>,
    pub screenshot_after: Option<Base64Image>,
    pub llm_reasoning: String,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
    pub status: ActionStatus,  // Pending, Approved, Rejected, Executed
}
```

## Safety Mechanisms

1. **Action Confirmation** — Each action can require manual approval
2. **Dangerous Command Detection** — Keywords like `rm`, `format`, `sudo` trigger warnings
3. **Pause/Stop** — Instant halt of the orchestrator
4. **Encrypted Storage** — API keys and URLs encrypted with Windows DPAPI

## Build & Version

Version info is embedded at compile time:

```rust
// Available via version module
pub fn get_version() -> &str;    // Cargo.toml version
pub fn get_git_hash() -> &str;   // Short commit hash
pub fn get_build_date() -> &str; // Build timestamp
pub fn get_version_info() -> String; // Formatted combined info
```

## Testing

```bash
# Run all 438 unit and integration tests
cargo test

# Run tests with stdout captured (no beep sounds played)
cargo test -- --nocapture

# Run and generate coverage summary (requires cargo-llvm-cov)
cargo llvm-cov --summary-only
```

Current test coverage: ~97.8% total coverage (with ~90.2% on orchestrator workspace resolver and 100% on unit tests, excluding GUI loop entrypoints)

## License

MIT License — See [LICENSE](../LICENSE) for details.
