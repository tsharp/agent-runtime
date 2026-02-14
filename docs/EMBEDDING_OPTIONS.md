# Embedding Models for Tool Search in Rust

## Overview

For semantic tool filtering, you need to convert tool descriptions into vectors (embeddings) that can be compared for similarity. Here are your options in Rust:

---

## Option 1: **Local Embedding Models** (Recommended)

### **A. ONNX Runtime + Sentence Transformers**

Use pre-trained models via ONNX (Open Neural Network Exchange):

**Rust Crate:** `ort` (ONNX Runtime)

```toml
[dependencies]
ort = "2.0"  # ONNX Runtime
ndarray = "0.15"  # For tensor operations
```

**Popular Models:**
- `all-MiniLM-L6-v2` (384 dims, 80 MB, **fast**)
- `all-mpnet-base-v2` (768 dims, 420 MB, **accurate**)
- `bge-small-en-v1.5` (384 dims, 133 MB, **balanced**)

**Performance:**
```
Model: all-MiniLM-L6-v2
Embedding time: 1-2 ms per tool description (CPU)
Embedding time: 0.1-0.3 ms (GPU)
Memory: 80 MB (model) + 10 KB per 1000 tools
```

**Example Code:**
```rust
use ort::{Environment, Session, Value};
use ndarray::Array2;

pub struct OnnxEmbedder {
    session: Session,
}

impl OnnxEmbedder {
    pub fn new(model_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let environment = Environment::builder()
            .with_name("embedder")
            .build()?;
        
        let session = Session::builder()?
            .with_model_from_file(model_path)?;
        
        Ok(Self { session })
    }
    
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        // Tokenize input (simplified)
        let tokens = self.tokenize(text);
        
        // Create input tensor
        let input = Array2::from_shape_vec(
            (1, tokens.len()),
            tokens
        )?;
        
        // Run inference
        let outputs = self.session.run(vec![Value::from_array(input)?])?;
        
        // Extract embedding
        let embedding = outputs[0].try_extract::<f32>()?;
        Ok(embedding.to_vec())
    }
}
```

**Pros:**
- ✅ No API calls (offline, fast, free)
- ✅ Privacy (data never leaves your server)
- ✅ Low latency (1-2 ms)
- ✅ No rate limits
- ✅ One-time download (~80-400 MB)

**Cons:**
- ⚠️ Initial setup (download model, convert to ONNX)
- ⚠️ Memory footprint (80-400 MB)
- ⚠️ Need to handle tokenization

---

### **B. Candle (Hugging Face's Rust ML Framework)**

Native Rust ML framework from Hugging Face:

**Rust Crate:** `candle-core`, `candle-nn`, `candle-transformers`

```toml
[dependencies]
candle-core = "0.3"
candle-nn = "0.3"
candle-transformers = "0.3"
tokenizers = "0.15"  # For tokenization
```

**Example:**
```rust
use candle_core::{Device, Tensor};
use candle_transformers::models::bert::BertModel;
use tokenizers::Tokenizer;

pub struct CandleEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl CandleEmbedder {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let device = Device::cuda_if_available(0)?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_pretrained(
            "sentence-transformers/all-MiniLM-L6-v2", 
            None
        )?;
        
        // Load model
        let model = BertModel::load(&device, "model.safetensors")?;
        
        Ok(Self { model, tokenizer, device })
    }
    
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        // Tokenize
        let encoding = self.tokenizer.encode(text, false)?;
        let ids = Tensor::new(encoding.get_ids(), &self.device)?;
        
        // Run model
        let output = self.model.forward(&ids)?;
        
        // Mean pooling
        let embedding = output.mean(1)?;
        
        Ok(embedding.to_vec1()?)
    }
}
```

**Pros:**
- ✅ Pure Rust (no C++ dependencies)
- ✅ GPU support (CUDA/Metal)
- ✅ Growing ecosystem
- ✅ Same models as Python

**Cons:**
- ⚠️ Newer library (less mature than ONNX)
- ⚠️ Fewer pre-built models
- ⚠️ Compilation can be slow

---

### **C. llama.cpp Embeddings**

If you're already using llama.cpp for LLM:

```rust
// Use your existing LlamaClient
impl LlamaClient {
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError> {
        // POST to /embedding endpoint
        let response = self.http_client
            .post(&format!("{}/embedding", self.base_url))
            .json(&json!({ "content": text }))
            .send()
            .await?;
        
        let data: EmbeddingResponse = response.json().await?;
        Ok(data.embedding)
    }
}
```

**Pros:**
- ✅ No new dependencies
- ✅ Same server as your LLM
- ✅ Simple integration

**Cons:**
- ⚠️ Requires llama.cpp embedding model
- ⚠️ Not specialized for semantic similarity
- ⚠️ Network call overhead

---

## Option 2: **API-Based Embeddings**

### **A. OpenAI Embeddings API**

```rust
use reqwest::Client;

pub struct OpenAIEmbedder {
    client: Client,
    api_key: String,
}

impl OpenAIEmbedder {
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": "text-embedding-3-small",
                "input": text
            }))
            .send()
            .await?
            .json::<OpenAIEmbeddingResponse>()
            .await?;
        
        Ok(response.data[0].embedding.clone())
    }
}
```

**Cost:**
- `text-embedding-3-small`: $0.02 per 1M tokens (512 dims)
- `text-embedding-3-large`: $0.13 per 1M tokens (3072 dims)

**For 20 tools:**
- One-time embedding: ~500 tokens = $0.00001
- Cost is negligible

**Pros:**
- ✅ Zero setup
- ✅ High quality embeddings
- ✅ No local compute needed
- ✅ Latest models

**Cons:**
- ⚠️ API dependency
- ⚠️ Network latency (50-100 ms)
- ⚠️ Costs money (tiny for tools)
- ⚠️ Privacy concerns

---

### **B. Voyage AI / Cohere Embeddings**

Similar to OpenAI but often cheaper/better for specific tasks.

---

## Option 3: **Simple Keyword/TF-IDF** (No ML)

For basic filtering without embeddings:

```rust
use std::collections::HashMap;

pub struct KeywordMatcher {
    tool_keywords: HashMap<String, Vec<String>>,
}

impl KeywordMatcher {
    pub fn new(registry: &ToolRegistry) -> Self {
        let mut tool_keywords = HashMap::new();
        
        for (name, tool) in registry.tools.iter() {
            // Extract keywords from name + description
            let keywords = extract_keywords(
                &format!("{} {}", tool.name(), tool.description())
            );
            tool_keywords.insert(name.clone(), keywords);
        }
        
        Self { tool_keywords }
    }
    
    pub fn find_relevant(&self, query: &str, max: usize) -> Vec<String> {
        let query_keywords = extract_keywords(query);
        
        let mut scores: Vec<_> = self.tool_keywords
            .iter()
            .map(|(name, keywords)| {
                let score = keywords.iter()
                    .filter(|k| query_keywords.contains(k))
                    .count();
                (name.clone(), score)
            })
            .filter(|(_, score)| *score > 0)
            .collect();
        
        scores.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
        scores.into_iter().take(max).map(|(name, _)| name).collect()
    }
}

fn extract_keywords(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 3) // Skip short words
        .map(|w| w.to_string())
        .collect()
}
```

**Pros:**
- ✅ Zero dependencies
- ✅ Ultra fast (< 1 μs)
- ✅ No model needed
- ✅ Works for simple cases

**Cons:**
- ⚠️ Not semantic (misses synonyms)
- ⚠️ Requires exact keyword matches
- ⚠️ Poor for complex queries

---

## Recommendation Matrix

### For Your Use Case:

| Scenario | Best Choice | Why |
|----------|-------------|-----|
| **< 20 tools** | Keyword matching | Simple, fast, good enough |
| **20-50 tools** | ONNX + MiniLM | Good accuracy, low overhead |
| **50-100 tools** | Candle or ONNX | Need semantic understanding |
| **100+ tools** | Candle + GPU | Best performance at scale |
| **Cloud-first** | OpenAI API | Easiest setup, high quality |
| **Edge/Embedded** | Keyword only | Minimal footprint |

---

## Recommended Implementation

### **Phase 1: Keyword Filtering (Day 1)**
Start simple:
```rust
pub struct ToolFilterConfig {
    pub strategy: ToolFilterStrategy,
}

pub enum ToolFilterStrategy {
    All,
    Keyword { max_tools: usize },
    // Add later: Semantic, Custom
}
```

**Impact:**
- 70% cost reduction with minimal effort
- No new dependencies
- Works well for most cases

### **Phase 2: Add ONNX Embeddings (Week 2)**
When you need better accuracy:
```rust
pub enum ToolFilterStrategy {
    All,
    Keyword { max_tools: usize },
    Semantic {
        embedder: Arc<dyn Embedder>,
        max_tools: usize,
    },
}
```

**Setup:**
1. Download `all-MiniLM-L6-v2.onnx` (80 MB)
2. Embed all tools at startup
3. At runtime: embed query, find nearest neighbors

**Impact:**
- 90% accuracy (vs 70% for keywords)
- Still fast (1-2 ms)
- Handles synonyms, semantic similarity

---

## Sample Performance

### Startup (One-Time):
```
Load ONNX model:        50-100 ms
Embed 20 tools:         20-40 ms
Build search index:     < 1 ms
-------------------------------------
Total startup time:     70-140 ms
```

### Runtime (Per Request):
```
Embed user query:       1-2 ms
Search index (20 tools): 10-50 μs
Filter to top 5:        < 1 μs
-------------------------------------
Total overhead:         1-2 ms
```

**This is still negligible compared to 200ms LLM calls!**

---

## Code Integration Example

### Minimal Interface:
```rust
pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>>;
    fn dimension(&self) -> usize;
}

pub struct ToolIndex {
    embedder: Arc<dyn Embedder>,
    tool_embeddings: HashMap<String, Vec<f32>>,
}

impl ToolIndex {
    pub fn new(embedder: Arc<dyn Embedder>, registry: &ToolRegistry) -> Self {
        let mut tool_embeddings = HashMap::new();
        
        for (name, tool) in registry.tools.iter() {
            let text = format!("{}: {}", tool.name(), tool.description());
            let embedding = embedder.embed(&text).unwrap();
            tool_embeddings.insert(name.clone(), embedding);
        }
        
        Self { embedder, tool_embeddings }
    }
    
    pub fn search(&self, query: &str, k: usize) -> Vec<String> {
        let query_emb = self.embedder.embed(query).unwrap();
        
        let mut scores: Vec<_> = self.tool_embeddings
            .iter()
            .map(|(name, emb)| {
                let similarity = cosine_similarity(&query_emb, emb);
                (name.clone(), similarity)
            })
            .collect();
        
        scores.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        scores.into_iter().take(k).map(|(name, _)| name).collect()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}
```

---

## My Recommendation

**Start with keywords, add embeddings later:**

1. **Week 1:** Implement keyword filtering
   - Zero dependencies
   - 70% cost reduction
   - Good enough for most cases

2. **Week 2-3:** Add ONNX embeddings (optional)
   - Use `all-MiniLM-L6-v2` (80 MB, fast)
   - Improves accuracy to 90%
   - Still very fast (1-2 ms)

3. **Later:** Consider API embeddings for edge cases
   - OpenAI for highest quality
   - Only for tools with complex descriptions

**For your 20 tools:** Keyword matching is probably sufficient. Semantic search becomes valuable at 50+ tools or when descriptions are complex/similar.

Want me to implement keyword filtering first? It's a 2-hour task with big impact.
