# VibePilot 🚀

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange)](https://www.rust-lang.org/)
[![Tested on Windows 10/11](https://img.shields.io/badge/Tested%20on-Windows%2010%2F11-blue?logo=windows)](https://www.microsoft.com/windows)
[![Release](https://github.com/Hidr4c/VibePilot/actions/workflows/release.yml/badge.svg)](https://github.com/Hidr4c/VibePilot/actions/workflows/release.yml)
[![CI](https://github.com/Hidr4c/VibePilot/actions/workflows/ci.yml/badge.svg)](https://github.com/Hidr4c/VibePilot/actions/workflows/ci.yml)
[![GitHub release](https://img.shields.io/github/v/release/Hidr4c/VibePilot?display_name=tag&sort=semver)](https://github.com/Hidr4c/VibePilot/releases)
[![GitHub stars](https://img.shields.io/github/stars/Hidr4c/VibePilot?style=social)](https://github.com/Hidr4c/VibePilot/stargazers)

**Visual AI Automation Orchestrator** — A native Windows desktop application that uses vision-capable LLMs to observe your screen, make decisions, and autonomously operate your PC.

[VibePilot in Action](#demo) · [Getting Started](#getting-started) · [API Docs](vibe_pilot_rust/docs/API.md) · [Report a Bug](https://github.com/Hidr4c/VibePilot/issues/new?template=bug-report.md)

<!-- Demo GIF placeholder — record with LICEcap or similar -->
![VibePilot Demo](vibe_pilot_rust/docs/demo.gif)

> **Demo**: VibePilot automating a web form using vision-based AI decisions.

## Table of Contents

- [Demo](#demo)
- [Features](#features)
- [Getting Started](#getting-started)
- [Use Cases](#use-cases)
- [How It Works](#how-it-works)
- [Comparison](#comparison)
- [Architecture](#architecture)
- [Testing](#testing)
- [Safety](#safety)
- [License](#license)
- [Report Bug](#report-bug)

## Demo

Record your own demo with [LICEcap](https://www.cockos.com/licecap/) (GIF) or [OBS](https://obsproject.com/) (video):

```
1. Open LICEcap
2. Select a region showing VibePilot in action
3. Click record → run an automation profile
4. Save as demo.gif → place in docs/
```

## Features

- 🧠 **Vision-Based Decision Loop** — Screenshots → LLM analysis → Action execution → Repeat
- ⚙️ **Multiple LLM Engine Support** — LM Studio, Ollama, or any OpenAI-compatible API
- 🎯 **Window Targeting** — Monitor specific applications or the entire desktop
- 📝 **Prompt Profiles** — Save, load, rename, and manage multiple automation configurations
- ✨ **AI Prompt Generator** — Describe your goal in natural language, let the AI structure the prompts
- 🪄 **Field-Level Optimization** — AI-powered prompt refinement for each configuration field
- 🔒 **Safety Controls** — Manual confirmation for AI actions, with separate toggle for dangerous commands
- 🌐 **Bilingual UI** — Full French and English localization
- 🔐 **Encrypted Config** — Windows DPAPI-encrypted settings at rest
- 📊 **Visual Action Timeline** — Git commit-style visualization of all AI actions
- 🎨 **Dark/Light Theme** — Toggle between dark and light modes

## Getting Started

### Prerequisites

- **Windows 10/11** (uses Win32 APIs for screen capture and input simulation)
- **Rust 1.70+** with the `stable-x86_64-pc-windows-msvc` toolchain
- **A local LLM server** with vision support:
  - [LM Studio](https://lmstudio.ai/) (recommended)
  - [Ollama](https://ollama.ai/)
  - Any OpenAI-compatible API endpoint

### Build

```bash
# Clone the repository
git clone https://github.com/Hidr4c/VibePilot.git
cd vibe-pilot/vibe_pilot_rust

# Build in release mode (optimized, no console window)
cargo build --release

# The binary is at: target/release/vibepilot.exe
```

### Usage

1. **Start your LLM server** (e.g., LM Studio with a vision model loaded)
2. **Launch VibePilot** — `target/release/vibepilot.exe`
3. **Configure the AI engine** — Select your engine, set the API URL and model name
4. **Set your target** — Choose which application window to monitor
5. **Write your prompts** — Or use Quick Start to generate them from a natural language description
6. **Click Start** — VibePilot will begin the observation-decision-action loop

### Tabs

| Tab | Purpose |
|-----|---------|
| ⚙ Global Configuration | Engine selection, window monitoring, profile management |
| ✎ Prompt Editor | Edit and manage prompt profiles (context, task, objective, directives) |
| ❖ Quick Start | Describe your goal → AI generates the full prompt configuration |
| ⛭ Setup | Application settings (zoom, language, theme, auto-validation) |

## Use Cases

### 1. Automate Software Installation

Automate the process of installing software by recording and replaying click sequences:

```
Prompt Profile:
  Context: "You are automating a software installer wizard"
  Task: "Navigate through the installation wizard"
  Objective: "Install the application with default settings"
  Directives: [
    "Click 'Next' on each wizard page",
    "Accept the license agreement",
    "Click 'Install' when the button appears",
    "Click 'Finish' when installation completes"
  ]
```

### 2. Test a Legacy Web Interface

Automate regression testing on a web application that lacks automated testing tools:

```
Prompt Profile:
  Context: "You are testing a legacy web application"
  Task: "Verify the login flow and dashboard loading"
  Objective: "Confirm the application works after deployment"
  Directives: [
    "Wait for page elements to be visible before interacting",
    "Take screenshots at each step for the report",
    "Report any visual anomalies or errors"
  ]
```

### 3. Play a Casual Game Automatically

Automate repetitive tasks in casual or idle games:

```
Prompt Profile:
  Context: "You are playing a casual idle/clicker game"
  Task: "Collect resources and upgrade buildings"
  Objective: "Maximize resource collection efficiency"
  Directives: [
    "Click on resource icons when they appear",
    "Open the upgrade menu when resources are sufficient",
    "Purchase upgrades in priority order"
  ]
```

## How It Works

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

**The loop:** Capture → Send to LLM → Receive decision → Execute action → Verify → Repeat

## Comparison

| Feature | **VibePilot** | AutoHotkey | SikuliX | PyAutoGUI |
|---------|---------------|------------|---------|-----------|
| Vision-based AI decisions | ✅ | ❌ | ✅ (basic) | ❌ |
| Local LLM support | ✅ LM Studio, Ollama | ❌ | ❌ | ❌ |
| Natural language prompts | ✅ | ❌ | ❌ | ❌ |
| Windows native | ✅ | ✅ | ⚠️ Java-based | ⚠️ Cross-platform |
| Privacy (offline mode) | ✅ | ✅ | ✅ | ✅ |
| No coding required | ✅ | ❌ (scripting) | ⚠️ Partial | ❌ (Python) |
| Multi-window targeting | ✅ | ⚠️ Limited | ❌ | ❌ |
| Action verification | ✅ (screenshots) | ❌ | ✅ | ❌ |
| Encrypted config | ✅ (DPAPI) | ❌ | ❌ | ❌ |
| Desktop app (GUI) | ✅ | ❌ (script) | ❌ (script) | ❌ (script) |

**Why VibePilot?** Unlike traditional automation tools that rely on hardcoded coordinates or image matching, VibePilot uses vision-capable LLMs to understand the screen context and make intelligent decisions — similar to how a human would look at the screen and decide what to do next.

## Architecture

```
src/
├── main.rs                    # Entry point, window config
├── app.rs                     # Application state, event processing
├── config.rs                  # DPAPI encryption, models, persistence
├── content.rs                 # FR/EN localization, AI prompt templates
├── defaults.rs                # Default profile factory
├── event_bus.rs               # Thread-safe event system (flume channels)
├── llm_client.rs              # HTTP client for LLM APIs
├── orchestrator.rs            # Core decision loop
├── peripheral_controller.rs   # Mouse/keyboard simulation (rdev)
├── screen_capture.rs          # Win32 GDI screen capture
├── version.rs                 # Build-time version info
├── integration_tests.rs       # Comprehensive integration tests
└── ui/
    ├── mod.rs                 # UI entry point, tab routing, modals
    ├── components.rs          # Timeline, console, footer controls
    ├── global_config.rs       # Engine/window/profile config tab
    ├── prompt_editor.rs       # Prompt editing with profile management
    ├── quick_start.rs         # AI prompt generator
    └── setup.rs               # Settings (zoom, language, toggles)
```

## Testing

```bash
cargo test
# Expected: 87 tests, 0 failures
```

## Safety

VibePilot includes several safety mechanisms:

- **Action Confirmation** — When auto-validation is disabled, every AI action requires manual approval
- **Dangerous Command Detection** — Commands containing keywords like `rm`, `format`, `sudo` are flagged with a red warning dialog
- **Pause/Stop Controls** — Instantly pause or stop the orchestrator at any time
- **Encrypted Config** — API URLs and settings are encrypted with Windows DPAPI

> ⚠️ **Warning**: VibePilot controls your mouse and keyboard. Always supervise the automation and keep the Stop button accessible.

## License

[MIT](LICENSE)

## Report Bug

Found a bug? [Open an issue on GitHub](https://github.com/Hidr4c/VibePilot/issues/new?template=bug-report.md)
