"""
DirSoul RLM (Recursive Language Model) Integration

A simplified implementation of Recursive Language Models for handling
very long context windows. Based on concepts from arXiv:2512.24601.

# Design Principles (V1 Simplified)
- **分层摘要**: Maintain summaries at multiple time scales
- **递归压缩**: Recursively compress older context
- **高效检索**: Query across long histories efficiently

# V1 Scope
- Basic recursive query framework
- Hierarchical memory summarization
- Efficient context retrieval for chat

# V2 Enhancement
- 10M+ token context window
- Advanced compression algorithms
- Parallel query processing

# Example
```python
from rlm import RLMManager

manager = RLMManager(api_client=api)

# Query with automatic context expansion
response = await manager.query(
    user_id="user123",
    question="What did I eat last week?",
    max_context_tokens=10000
)
```
"""

__version__ = "1.0.0"

from .manager import RLMManager
from .context import RecursiveContext, ContextLayer
from .query import QueryEngine

__all__ = [
    "RLMManager",
    "RecursiveContext",
    "ContextLayer",
    "QueryEngine",
]
