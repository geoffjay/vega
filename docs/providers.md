# Providers and Models Documentation

This document describes the available LLM providers and models supported by the Ally AI agent system, along with recommendations for different use cases.

## Overview

Ally supports multiple LLM providers to give you flexibility in choosing between local and cloud-based models. Each provider has different strengths, costs, and requirements.

## Supported Providers

### 1. Ollama (Local Models)

**Purpose**: Run large language models locally on your machine for privacy, offline usage, and no API costs.

**Key Features**:

- Complete privacy - no data leaves your machine
- No API costs or rate limits
- Offline capability
- Multiple model sizes to fit your hardware
- Easy model management with `ollama pull` command

**Requirements**:

- [Ollama](https://ollama.ai/) installed and running
- Sufficient RAM and compute resources
- Models downloaded locally

**Configuration**:

```bash
# Default provider (no additional setup needed)
cargo run

# Explicit configuration
cargo run -- --provider ollama --model llama3.1
```

**Environment Variables**:

- No API keys required
- Ollama service should be running on default port (11434)

#### Recommended Ollama Models

**🔥 Highly Recommended**:

- **`llama3.1`** (8B, 70B, 405B variants)

  - Excellent tool support and reasoning
  - Good balance of performance and resource usage
  - Strong code understanding
  - Usage: `ollama pull llama3.1` then `--model llama3.1`

- **`llama3.2`** (1B, 3B variants)
  - Lighter weight for resource-constrained systems
  - Good for basic tasks and quick responses
  - Usage: `ollama pull llama3.2` then `--model llama3.2`

**Specialized Models**:

- **`codellama`** - Optimized for code generation and analysis
- **`mistral`** - Good general-purpose alternative to Llama
- **`phi3`** - Microsoft's efficient small model
- **`qwen2.5`** - Strong multilingual support

**Model Installation**:

```bash
# Install recommended models
ollama pull llama3.1        # ~4.7GB (8B model)
ollama pull llama3.2        # ~2.0GB (3B model)
ollama pull codellama       # ~3.8GB
ollama pull mistral         # ~4.1GB

# List available models
ollama list

# Check model details
ollama show llama3.1
```

**Hardware Requirements**:

- **8B models**: 8GB+ RAM recommended
- **70B models**: 64GB+ RAM required
- **GPU acceleration**: NVIDIA/AMD GPUs supported for faster inference

### 2. OpenRouter (Cloud Models)

**Purpose**: Access to a wide variety of state-of-the-art models from different providers through a single API.

**Key Features**:

- Access to latest models from OpenAI, Anthropic, Google, and others
- No local hardware requirements
- Consistent API across different model providers
- Pay-per-use pricing
- High availability and reliability

**Requirements**:

- OpenRouter API key from [openrouter.ai](https://openrouter.ai/)
- Internet connection
- API credits/billing setup

**Configuration**:

```bash
# Set API key via environment variable (recommended)
export OPENROUTER_API_KEY="your-api-key-here"
cargo run -- --provider openrouter --model "openai/gpt-4"

# Or pass API key directly
cargo run -- --provider openrouter --model "openai/gpt-4" --openrouter-api-key "your-key"
```

**Environment Variables**:

- `OPENROUTER_API_KEY`: Your OpenRouter API key

#### Recommended OpenRouter Models

**🔥 Top Tier (Best Performance)**:

- **`openai/gpt-4`** - Excellent tool support, reasoning, and reliability
- **`openai/gpt-4-turbo`** - Faster and more cost-effective than GPT-4
- **`openai/gpt-4o`** - Latest GPT-4 variant with improved capabilities
- **`anthropic/claude-3-opus`** - Exceptional reasoning and analysis
- **`anthropic/claude-3-sonnet`** - Good balance of performance and cost
- **`anthropic/claude-3-haiku`** - Fast and cost-effective for simpler tasks

**📋 Good Alternatives**:

- **`openai/gpt-3.5-turbo`** - Cost-effective for basic tasks
- **`mistralai/mistral-large`** - Strong European alternative
- **`google/gemini-pro`** - Google's flagship model
- **`meta-llama/llama-3.1-405b`** - Largest open-source model

**Cost Considerations**:

- GPT-4 models: Higher cost, best performance
- Claude models: Competitive pricing, excellent quality
- GPT-3.5-turbo: Most cost-effective for basic tasks
- Check current pricing at [openrouter.ai/docs/models](https://openrouter.ai/docs/models)

### 3. Embedding Providers

For context awareness and semantic search, Ally supports multiple embedding providers:

#### OpenAI Embeddings

- **Model**: `text-embedding-3-small` (default)
- **Features**: High-quality embeddings, good performance
- **Requirements**: OpenAI API key
- **Usage**: `--embedding-provider openai`

#### Ollama Embeddings

- **Model**: `nomic-embed-text` (default)
- **Features**: Local embeddings, privacy-focused
- **Requirements**: Ollama with embedding model installed
- **Usage**: `--embedding-provider ollama`

#### Simple Embeddings

- **Model**: Hash-based (development/testing)
- **Features**: No external dependencies, fast
- **Usage**: `--embedding-provider simple` (default)

## Provider Comparison

| Feature              | Ollama                 | OpenRouter            |
| -------------------- | ---------------------- | --------------------- |
| **Privacy**          | ✅ Complete            | ❌ Data sent to cloud |
| **Cost**             | ✅ Free after setup    | 💰 Pay per use        |
| **Performance**      | 🔄 Depends on hardware | ✅ Consistently high  |
| **Offline Usage**    | ✅ Yes                 | ❌ Requires internet  |
| **Model Variety**    | 📊 Growing selection   | ✅ Extensive catalog  |
| **Setup Complexity** | 🔧 Moderate            | ✅ Simple             |
| **Resource Usage**   | 💻 High local usage    | ✅ Minimal local      |

## Configuration Guide

### Command Line Configuration

```bash
# Ollama (local)
cargo run -- --provider ollama --model llama3.1

# OpenRouter (cloud)
cargo run -- --provider openrouter --model "openai/gpt-4" --openrouter-api-key "key"

# With additional options
cargo run -- --provider openrouter \
              --model "anthropic/claude-3-sonnet" \
              --max-tokens 4000 \
              --verbose
```

### Environment Variables

```bash
# Provider configuration
export ALLY_PROVIDER="openrouter"
export ALLY_MODEL="openai/gpt-4"

# API keys
export OPENROUTER_API_KEY="your-openrouter-key"
export OPENAI_API_KEY="your-openai-key"  # For embeddings

# Embedding configuration
export ALLY_EMBEDDING_PROVIDER="openai"
export ALLY_EMBEDDING_MODEL="text-embedding-3-small"

# Run with environment configuration
cargo run
```

### Configuration File (Future)

While not currently implemented, future versions may support configuration files:

```toml
# ally.toml (planned)
[provider]
name = "openrouter"
model = "openai/gpt-4"
api_key_env = "OPENROUTER_API_KEY"

[embeddings]
provider = "openai"
model = "text-embedding-3-small"

[limits]
max_tokens = 4000
timeout_seconds = 30
```

## Model Selection Guidelines

### For Code Development

1. **Best**: `openai/gpt-4` or `anthropic/claude-3-opus`
2. **Good**: `llama3.1` (70B) or `codellama`
3. **Budget**: `openai/gpt-3.5-turbo` or `llama3.2`

### For General Chat

1. **Best**: `anthropic/claude-3-sonnet` or `openai/gpt-4-turbo`
2. **Good**: `llama3.1` (8B) or `mistral`
3. **Budget**: `anthropic/claude-3-haiku` or `llama3.2`

### For Analysis and Research

1. **Best**: `anthropic/claude-3-opus` or `openai/gpt-4`
2. **Good**: `llama3.1` (405B) or `mistralai/mistral-large`
3. **Budget**: `openai/gpt-4-turbo` or `llama3.1` (8B)

### For Privacy-Sensitive Work

1. **Required**: Ollama models only
2. **Recommended**: `llama3.1`, `codellama`, or `mistral`
3. **Lightweight**: `llama3.2` or `phi3`

## Performance Optimization

### Ollama Optimization

```bash
# Enable GPU acceleration (if available)
export OLLAMA_GPU_LAYERS=35  # Adjust based on your GPU memory

# Optimize for CPU usage
export OLLAMA_NUM_THREADS=8  # Match your CPU cores

# Memory management
export OLLAMA_MAX_LOADED_MODELS=2
```

### OpenRouter Optimization

```bash
# Use faster models for development
export ALLY_MODEL="openai/gpt-3.5-turbo"

# Reduce token limits for faster responses
cargo run -- --max-tokens 1000

# Enable request caching (if supported)
export OPENROUTER_CACHE_ENABLED=true
```

## Troubleshooting

### Common Ollama Issues

1. **"Connection refused"**:

   ```bash
   # Start Ollama service
   ollama serve

   # Or check if running
   ps aux | grep ollama
   ```

2. **"Model not found"**:

   ```bash
   # List available models
   ollama list

   # Pull missing model
   ollama pull llama3.1
   ```

3. **Out of memory**:

   ```bash
   # Use smaller model
   ollama pull llama3.2:1b

   # Or increase system memory/swap
   ```

### Common OpenRouter Issues

1. **"Invalid API key"**:

   ```bash
   # Check API key format
   echo $OPENROUTER_API_KEY

   # Verify at openrouter.ai dashboard
   ```

2. **"Model not available"**:

   ```bash
   # Check model name format
   cargo run -- --provider openrouter --model "openai/gpt-4"

   # List available models at openrouter.ai/docs/models
   ```

3. **Rate limiting**:
   ```bash
   # Add delays between requests
   # Check your usage limits at openrouter.ai
   ```

### Debug Commands

```bash
# Test provider connection
cargo run -- --provider ollama --model llama3.1 --verbose

# Check model recommendations
cargo run -- --help-models

# Validate configuration
cargo run -- --dry-run
```

## Security Considerations

### API Key Security

1. **Use environment variables** instead of command-line arguments
2. **Rotate API keys regularly**
3. **Monitor API usage** for unexpected activity
4. **Use separate keys** for development and production

### Data Privacy

1. **Ollama**: Complete data privacy - nothing leaves your machine
2. **OpenRouter**: Data sent to cloud providers - review their privacy policies
3. **Embeddings**: Consider using local Ollama embeddings for sensitive data

### Best Practices

1. **Start with Ollama** for privacy-sensitive projects
2. **Use OpenRouter** for production applications requiring reliability
3. **Monitor costs** when using cloud providers
4. **Keep models updated** for security patches
5. **Use appropriate model sizes** for your hardware capabilities

This provider system gives you the flexibility to choose the right balance of performance, privacy, cost, and convenience for your specific use case.
