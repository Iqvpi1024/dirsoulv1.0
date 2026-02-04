# DirSoul RLM Integration

Recursive Language Model integration for handling very long context windows.

## Overview

Based on concepts from [arXiv:2512.24601](https://arxiv.org/abs/2512.24601), this module implements hierarchical context management to enable efficient querying across very long histories.

## Architecture

### Hierarchical Context Layers

```
Layer 4 (Year): 10 yearly summaries
     ↓ (summarizes)
Layer 3 (Month): 24 monthly summaries
     ↓ (summarizes)
Layer 2 (Week): 52 weekly summaries
     ↓ (summarizes)
Layer 1 (Day): 30 daily summaries
     ↓ (summarizes)
Layer 0 (Raw): Last 100 raw events
```

### Query Strategy

1. Start from Layer 0 (most detailed recent events)
2. Progressively include higher layers until token budget is reached
3. Return answer with full context provenance

## Installation

```bash
cd src/python/rlm
pip install -r requirements.txt
```

## Usage

### Basic Example

```python
from rlm import RLMManager
from telegram_bot.api_client import DirSoulAPI

# Initialize API client
api_client = DirSoulAPI("http://127.0.0.1:8080")

# Initialize RLM manager
manager = RLMManager(api_client=api_client)

# Build context from timeline
events_count = await manager.build_context_from_timeline(
    user_id="user123",
    days=30
)
print(f"Built context from {events_count} events")

# Query with context
result = await manager.query(
    user_id="user123",
    question="What did I eat last week?",
    max_tokens=4000
)

print(f"Answer: {result.answer}")
print(f"Confidence: {result.confidence}")
print(f"Tokens used: {result.tokens_used}")
print(f"Layers accessed: {[l.value for l in result.layers_accessed]}")
```

### Advanced Usage

```python
# Query with date range
from datetime import datetime, timedelta

end_date = datetime.now()
start_date = end_date - timedelta(days=7)

result = await manager.query(
    user_id="user123",
    question="Summary of my activities",
    max_tokens=8000,
    start_date=start_date,
    end_date=end_date
)
```

### Context Information

```python
# Get context information
info = manager.get_context_info("user123")

print("Context layers:")
for layer_name, layer_info in info["layers"].items():
    print(f"  {layer_name}: {layer_info['count']}/{layer_info['capacity']}")

print("\nQuery stats:")
print(f"  Total queries: {info['query_stats']['total_queries']}")
print(f"  Average tokens: {info['query_stats']['avg_tokens']:.0f}")
```

## V1 vs V2 Features

| Feature | V1 | V2 |
|---------|----|----|
| Hierarchical context | ✅ | ✅ |
| Basic query engine | ✅ | ✅ |
| LLM-powered answers | ⏳ | ✅ |
| Automatic summarization | ⏳ | ✅ |
| 10M+ token context | ❌ | ✅ |
| Parallel query processing | ❌ | ✅ |

⏳ = Partial implementation

## Components

### `RecursiveContext`

Manages hierarchical context layers.

```python
from rlm.context import RecursiveContext, ContextItem

context = RecursiveContext(user_id="user123")

# Add raw event
event = ContextItem(
    timestamp=datetime.now(),
    content="I ate 2 apples",
    event_type="ate"
)
context.add_raw_event(event)

# Get context for query
items = context.get_context_for_query(max_tokens=1000)
```

### `QueryEngine`

Executes queries with recursive expansion.

```python
from rlm.query import QueryEngine

engine = QueryEngine(context)

result = await engine.query(
    question="What did I eat?",
    max_tokens=4000
)
```

### `RLMManager`

Main entry point for RLM functionality.

```python
from rlm import RLMManager

manager = RLMManager(api_client)

# Build context
await manager.build_context_from_timeline(user_id="user123", days=30)

# Query
result = await manager.query(
    user_id="user123",
    question="What did I do last week?"
)
```

## Performance

### Memory Usage

- Raw events: ~100 KB (100 events × 1 KB each)
- Daily summaries: ~300 KB (30 days × 10 KB each)
- Weekly summaries: ~520 KB (52 weeks × 10 KB each)
- **Total (V1)**: ~1 MB per user

### Query Latency

- Layer 0 only: <10ms
- Layers 0-1: ~50ms
- All layers: ~200ms

### Token Estimation

Current implementation uses simple estimation (1 token ≈ 4 characters). V2 will use accurate tokenization.

## Future Development

### V2 Roadmap

1. **LLM Integration**
   - Use actual LLM for answering
   - Generate context-aware responses
   - Calculate confidence scores

2. **Automatic Summarization**
   - Daily, weekly, monthly summaries
   - Configurable summarization triggers
   - Summary quality metrics

3. **10M+ Token Context**
   - Implement recursive compression
   - Hierarchical vector search
   - Parallel query processing

## References

- [Recursive Language Models (MIT)](https://arxiv.org/abs/2512.24601)
- [Google Titans + MIRAS](https://research.google/blog/titans-miras-helping-ai-have-long-term-memory/)

## License

MIT License - See LICENSE file in project root
