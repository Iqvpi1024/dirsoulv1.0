"""
RLM Query Engine

Handles querying across hierarchical context layers with recursive expansion.

# Query Strategy
1. Start with recent context (Layer 0)
2. If answer not found, expand to Layer 1 (daily summaries)
3. Continue expanding until answer found or context exhausted
"""

import logging
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass

from .context import RecursiveContext, ContextLayer, ContextItem, ContextSummary

logger = logging.getLogger(__name__)


@dataclass
class QueryResult:
    """Result of a query"""
    answer: str
    context_used: List[Any]  # ContextItem or ContextSummary
    tokens_used: int
    layers_accessed: List[ContextLayer]
    confidence: float
    metadata: Dict[str, Any]


class QueryEngine:
    """
    Engine for querying hierarchical context with recursive expansion

    # Query Process
    1. Parse query to identify intent and entities
    2. Search across context layers (from detailed to summary)
    3. Recursively expand search if answer not found
    4. Return result with context used
    """

    def __init__(self, context: RecursiveContext):
        """
        Initialize query engine

        Args:
            context: RecursiveContext to query
        """
        self.context = context
        self.query_stats: Dict[str, Any] = {
            "total_queries": 0,
            "layers_used": [],
            "avg_tokens": 0
        }

    async def query(
        self,
        question: str,
        max_tokens: int = 4000,
        start_date: Optional[datetime] = None,
        end_date: Optional[datetime] = None,
        llm_client: Optional[Any] = None
    ) -> QueryResult:
        """
        Execute a query with automatic context expansion

        Args:
            question: User's question
            max_tokens: Maximum context tokens to use
            start_date: Search events after this date
            end_date: Search events before this date
            llm_client: Optional LLM client for answering

        Returns:
            QueryResult with answer and context used
        """
        self.query_stats["total_queries"] += 1

        # Step 1: Get context within token budget
        context_items = self.context.get_context_for_query(
            max_tokens=max_tokens,
            start_date=start_date,
            end_date=end_date
        )

        if not context_items:
            return QueryResult(
                answer="I don't have any relevant memories to answer that question.",
                context_used=[],
                tokens_used=0,
                layers_accessed=[],
                confidence=0.0,
                metadata={"reason": "no_context"}
            )

        # Step 2: Estimate tokens used
        tokens_used = sum(
            getattr(item, 'token_count', len(str(item.content)) // 4)
            for item in context_items
        )

        # Step 3: Track which layers were accessed
        layers_accessed = list(set(
            getattr(item, 'layer', ContextLayer.RAW)
            for item in context_items
            if hasattr(item, 'layer')
        )) or [ContextLayer.RAW]

        # Step 4: Generate answer (V1: simple, V2: with LLM)
        if llm_client:
            answer = await self._answer_with_llm(
                question=question,
                context=context_items,
                llm_client=llm_client
            )
            confidence = 0.7  # V2: Calculate based on context relevance
        else:
            answer = self._answer_simple(question, context_items)
            confidence = 0.5  # Lower confidence without LLM

        # Update stats
        self.query_stats["layers_used"].extend(layers_accessed)
        self.query_stats["avg_tokens"] = (
            (self.query_stats["avg_tokens"] * (self.query_stats["total_queries"] - 1) + tokens_used) /
            self.query_stats["total_queries"]
        )

        return QueryResult(
            answer=answer,
            context_used=context_items,
            tokens_used=tokens_used,
            layers_accessed=layers_accessed,
            confidence=confidence,
            metadata={
                "items_count": len(context_items),
                "timestamp": datetime.now().isoformat()
            }
        )

    def _answer_simple(
        self,
        question: str,
        context_items: List[Any]
    ) -> str:
        """
        Generate simple answer without LLM (V1 fallback)

        Args:
            question: User's question
            context_items: Context to use

        Returns:
            Generated answer
        """
        # Simple keyword matching for V1
        question_lower = question.lower()

        # Extract relevant items
        relevant_items = []
        for item in context_items[:10]:  # Check top 10 items
            content = str(item.content).lower()
            if any(word in content for word in question_lower.split()):
                relevant_items.append(item)

        if not relevant_items:
            return "I found some memories but couldn't determine a specific answer. Try rephrasing your question."

        # Format relevant items as answer
        response_parts = []
        response_parts.append(f"Based on your memories, here's what I found:\n")

        for item in relevant_items[:5]:  # Max 5 items
            if isinstance(item, ContextItem):
                timestamp_str = item.timestamp.strftime("%Y-%m-%d")
                response_parts.append(f"• {timestamp_str}: {item.content}")
            elif isinstance(item, ContextSummary):
                timestamp_str = item.timestamp.strftime("%Y-%m-%d")
                response_parts.append(f"• {timestamp_str} (summary): {item.content}")

        return "\n".join(response_parts)

    async def _answer_with_llm(
        self,
        question: str,
        context_items: List[Any],
        llm_client: Any
    ) -> str:
        """
        Generate answer using LLM (V2)

        Args:
            question: User's question
            context_items: Context to use
            llm_client: LLM client

        Returns:
            Generated answer
        """
        # Format context for LLM
        context_str = self._format_context_for_llm(context_items)

        # This would call the LLM client in full implementation
        # For V1, return formatted context
        return f"Context:\n{context_str}\n\nQuestion: {question}\n\n[LLM response would be generated here]"

    def _format_context_for_llm(self, context_items: List[Any]) -> str:
        """Format context items for LLM input"""
        parts = []

        for item in context_items:
            if isinstance(item, ContextItem):
                timestamp = item.timestamp.strftime("%Y-%m-%d %H:%M")
                parts.append(f"[{timestamp}] {item.event_type}: {item.content}")
            elif isinstance(item, ContextSummary):
                timestamp = item.timestamp.strftime("%Y-%m-%d")
                parts.append(f"[{timestamp} Summary] {item.content}")

        return "\n".join(parts)

    def get_query_stats(self) -> Dict[str, Any]:
        """Get query statistics"""
        return self.query_stats.copy()

    def reset_stats(self) -> None:
        """Reset query statistics"""
        self.query_stats = {
            "total_queries": 0,
            "layers_used": [],
            "avg_tokens": 0
        }
