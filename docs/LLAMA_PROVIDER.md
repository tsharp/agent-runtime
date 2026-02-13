# LLM Provider Reorganization Summary

## Changes Made

### Structure Reorganization

**Before:**
```
src/llm/
├── mod.rs
├── types.rs
└── openai.rs
```

**After:**
```
src/llm/
├── mod.rs           (Updated imports)
├── types.rs         (No change)
├── provider/
│   ├── mod.rs       (New - exports providers)
│   ├── openai.rs    (Moved + updated imports)
│   └── llama.rs     (New - llama.cpp support)
└── README.md        (Updated for both providers)
```

## New Provider: Llama.cpp

### What is it?
**llama.cpp** is a C++ implementation of Meta's LLaMA models that:
- Runs locally (no API key needed)
- Supports quantized models (4-bit, 5-bit, 8-bit)
- Provides OpenAI-compatible API server
- Works with consumer hardware (CPU/GPU)

### Why add it?
1. **Privacy** - No data leaves your machine
2. **Cost** - No per-token charges
3. **Offline** - Works without internet
4. **Control** - Full control over model behavior
5. **Development** - Test without burning OpenAI credits

### How to use

**1. Download and build llama.cpp:**
```bash
git clone https://github.com/ggerganov/llama.cpp
cd llama.cpp
make
```

**2. Download a model:**
```bash
# Example: Llama 2 7B Chat (4-bit quantized)
wget https://huggingface.co/TheBloke/Llama-2-7B-Chat-GGUF/resolve/main/llama-2-7b-chat.Q4_K_M.gguf \
  -O models/llama-2-7b-chat.gguf
```

**3. Start the server:**
```bash
./server -m models/llama-2-7b-chat.gguf --port 8080 --ctx-size 2048
```

**4. Use in Rust:**
```rust
use agent_runtime::llm::{LlamaClient, ChatClient, ChatMessage, ChatRequest};

let client = LlamaClient::localhost();
let response = client.chat(request).await?;
```

## API Compatibility

Both providers implement the same `ChatClient` trait, so they're **drop-in replacements**:

```rust
// Trait-based - works with any provider
async fn chat_with_llm(client: &dyn ChatClient, prompt: &str) {
    let request = ChatRequest::new(vec![
        ChatMessage::user(prompt),
    ]);
    let response = client.chat(request).await.unwrap();
    println!("{}", response.content);
}

// Use with OpenAI
let openai = OpenAIClient::new(api_key);
chat_with_llm(&openai, "Hello!").await;

// Use with Llama.cpp
let llama = LlamaClient::localhost();
chat_with_llm(&llama, "Hello!").await;
```

## Llama.cpp Client API

### Constructors

```rust
// Default: http://localhost:8080
let client = LlamaClient::localhost();

// Custom port
let client = LlamaClient::localhost_with_port(8081);

// Custom URL (e.g., remote server)
let client = LlamaClient::new("http://192.168.1.100:8080", "llama");
```

### Features

- ✅ OpenAI-compatible API
- ✅ Same request/response format
- ✅ Token usage tracking
- ✅ Temperature/top_p/max_tokens support
- ✅ Error handling
- ✅ No authentication required (local server)

### Limitations

- Model parameter is optional (llama.cpp ignores it)
- No rate limiting (local server)
- No API key needed
- Server must be running separately

## Demo Applications

### OpenAI Demo (`llm_demo.rs`)
```bash
export OPENAI_API_KEY="sk-..."
cargo run --bin llm_demo
```

Output:
```
=== LLM Client Demo ===

Provider: openai
Model: gpt-3.5-turbo

Sending request...

✅ Success!
Response: Hello, how are you today?
Model: gpt-3.5-turbo-0125

Usage:
  Prompt tokens: 15
  Completion tokens: 8
  Total tokens: 23
```

### Llama.cpp Demo (`llama_demo.rs`)
```bash
# Terminal 1: Start server
./server -m models/llama-2-7b-chat.gguf --port 8080

# Terminal 2: Run demo
cargo run --bin llama_demo
```

Output:
```
=== Llama.cpp Client Demo ===

Provider: llama.cpp
Model: llama
Connecting to localhost:8080...

Sending request...

✅ Success!
Response: Hello there, how are you?
Model: llama-2-7b-chat

Usage:
  Prompt tokens: 12
  Completion tokens: 7
  Total tokens: 19
```

## Import Changes

**Old:**
```rust
use agent_runtime::llm::openai::OpenAIClient;
```

**New:**
```rust
use agent_runtime::llm::OpenAIClient;  // Re-exported from mod.rs
```

Both are now top-level exports for convenience.

## Files Modified

- `src/llm/mod.rs` - Updated to use `provider` module
- `src/llm/provider/openai.rs` - Fixed `super::super::` imports
- `src/bin/llm_demo.rs` - Updated import path
- `src/llm/README.md` - Added llama.cpp documentation
- `Cargo.toml` - Added llama_demo binary

## Files Created

- `src/llm/provider/mod.rs` - Provider module exports
- `src/llm/provider/llama.rs` - Llama.cpp implementation
- `src/bin/llama_demo.rs` - Demo application

## Testing

```bash
# Build everything
cargo build --all-targets

# Test OpenAI (requires API key)
OPENAI_API_KEY="sk-..." cargo run --bin llm_demo

# Test Llama.cpp (requires running server)
cargo run --bin llama_demo
```

## Popular Models for Llama.cpp

### Llama 2 (Meta)
- **7B Chat** - Good for chatbots, ~4GB RAM
- **13B Chat** - Better quality, ~8GB RAM
- **70B Chat** - Best quality, ~40GB RAM (needs GPU)

### Mistral (Mistral AI)
- **7B Instruct** - Fast and capable, ~4GB RAM
- **7B OpenOrca** - Fine-tuned for instructions

### Code Models
- **CodeLlama 7B** - Code generation
- **WizardCoder 15B** - Advanced coding

### Download from:
- https://huggingface.co/TheBloke (Quantized models)
- https://huggingface.co/meta-llama (Original models)

## Next Steps

1. **Wire to agents** - Make `Agent::execute()` use LLM clients
2. **Add Anthropic** - Claude provider
3. **Add Ollama** - Similar to llama.cpp but easier setup
4. **Streaming** - Support streaming responses
5. **Function calling** - Tool use support
6. **Caching** - Response caching layer
7. **Retry logic** - Exponential backoff
8. **Load balancing** - Multiple providers with fallback

## Conclusion

The LLM module now supports both **cloud** (OpenAI) and **local** (llama.cpp) providers with a unified interface. This gives users:

- **Flexibility** - Choose based on needs
- **Privacy** - Keep data local
- **Cost control** - Use local for development
- **Reliability** - Fallback options

All while maintaining the same simple API! 🎉
