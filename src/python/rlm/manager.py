"""
RLM Manager - Main entry point for Recursive Language Model integration

Manages the full RLM pipeline including context building,
summarization, and querying.
"""

import logging
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional

from .context import RecursiveContext, ContextLayer, ContextItem
from .query import QueryEngine, QueryResult

logger = logging.getLogger(__name__)


class RLMManager:
    """
    Main manager for Recursive Language Model functionality

    # Responsibilities
    - Build and maintain hierarchical context
    - Coordinate summarization across layers
    - Execute queries with context expansion

    # Usage
    ```python
    manager = RLMManager(api_client=api_client)

    # Add events from timeline
    await manager.build_context_from_timeline(user_id, days=30)

    # Query with context
    result = await manager.query(
        user_id="user123",
        question="What did I eat last week?"
    )
    ```
    """

    def __init__(self, api_client: Any):
        """
        Initialize RLM manager

        Args:
            api_client: DirSoul API client (or compatible HTTP client)
        """
        self.api_client = api_client
        self.contexts: Dict[str, RecursiveContext] = {}
        self.query_engines: Dict[str, QueryEngine] = {}

    def get_context(self, user_id: str) -> RecursiveContext:
        """Get or create context for user"""
        if user_id not in self.contexts:
            self.contexts[user_id] = RecursiveContext(user_id)
            self.query_engines[user_id] = QueryEngine(self.contexts[user_id])
        return self.contexts[user_id]

    async def build_context_from_timeline(
        self,
        user_id: str,
        days: int = 30,
        force_refresh: bool = False
    ) -> int:
        """
        Build hierarchical context from user's timeline

        Args:
            user_id: User identifier
            days: Number of days to fetch
            force_refresh: Rebuild even if already built

        Returns:
            Number of events processed
        """
        context = self.get_context(user_id)

        # Check if context is already built
        if not force_refresh and context.last_updated.get(ContextLayer.RAW):
            logger.info(f"Context already exists for user {user_id}, skipping build")
            return 0

        logger.info(f"Building context for user {user_id} from last {days} days")

        # Calculate date range
        end_date = datetime.now()
        start_date = end_date - timedelta(days=days)

        # Fetch timeline from API
        try:
            timeline_data = await self.api_client.get_timeline(
                user_id=user_id,
                start_date=start_date.isoformat(),
                end_date=end_date.isoformat()
            )
        except Exception as e:
            logger.error(f"Failed to fetch timeline: {e}")
            return 0

        # Process events and add to context
        event_count = 0
        for date, events in timeline_data.get("events_by_date", {}).items():
            for event in events:
                # Convert timeline event to ContextItem
                item = ContextItem(
                    timestamp=datetime.fromisoformat(event["timestamp"]),
                    content=self._format_event_content(event),
                    event_type=event.get("action", "unknown"),
                    metadata={
                        "event_id": event.get("event_id"),
                        "actor": event.get("actor"),
                        "target": event.get("target"),
                        "quantity": event.get("quantity"),
                        "confidence": event.get("confidence")
                    }
                )

                # Estimate token count
                item.token_count = context.estimate_tokens(item.content)

                # Add to context
                context.add_raw_event(item)
                event_count += 1

        logger.info(f"Built context with {event_count} events for user {user_id}")

        # Generate summaries for higher layers
        await self._generate_summaries(user_id)

        return event_count

    async def query(
        self,
        user_id: str,
        question: str,
        max_tokens: int = 4000,
        start_date: Optional[datetime] = None,
        end_date: Optional[datetime] = None,
        llm_client: Optional[Any] = None
    ) -> QueryResult:
        """
        Execute a query with context expansion

        Args:
            user_id: User identifier
            question: User's question
            max_tokens: Maximum context tokens
            start_date: Optional start date filter
            end_date: Optional end date filter
            llm_client: Optional LLM client for answering

        Returns:
            QueryResult
        """
        # Ensure context exists
        context = self.get_context(user_id)

        # Auto-build context if needed
        if not context.last_updated.get(ContextLayer.RAW):
            logger.info(f"Auto-building context for user {user_id}")
            await self.build_context_from_timeline(user_id)

        # Get query engine
        engine = self.query_engines.get(user_id)
        if not engine:
            engine = QueryEngine(context)
            self.query_engines[user_id] = engine

        # Execute query
        result = await engine.query(
            question=question,
            max_tokens=max_tokens,
            start_date=start_date,
            end_date=end_date,
            llm_client=llm_client
        )

        return result

    async def _generate_summaries(self, user_id: str) -> None:
        """
        Generate summaries for all context layers

        Args:
            user_id: User identifier
        """
        context = self.get_context(user_id)

        # For V1, we'll skip actual summarization
        # In V2, this would:
        # 1. Summarize RAW events into daily summaries
        # 2. Summarize daily summaries into weekly summaries
        # 3. And so on...

        logger.info(f"Summary generation for user {user_id} - V1 (placeholder)")

        # Placeholder: In V2, call context.summarize_layer() for each layer
        # await context.summarize_layer(ContextLayer.DAY, llm_client)
        # await context.summarize_layer(ContextLayer.WEEK, llm_client)
        # ...

    def _format_event_content(self, event: Dict[str, Any]) -> str:
        """Format event as human-readable string"""
        actor = event.get("actor") or "You"
        action = event.get("action", "did something")
        target = event.get("target", "")
        quantity = event.get("quantity")
        unit = event.get("unit")

        parts = [actor, action]
        if target:
            parts.append(target)
        if quantity is not None:
            parts.append(str(quantity))
            if unit:
                parts.append(unit)

        return " ".join(parts)

    def get_context_info(self, user_id: str) -> Dict[str, Any]:
        """
        Get information about user's context

        Args:
            user_id: User identifier

        Returns:
            Context information
        """
        context = self.get_context(user_id)
        engine = self.query_engines.get(user_id)

        info = {
            "user_id": user_id,
            "layers": {},
            "last_updated": context.last_updated,
        }

        for layer in ContextLayer:
            items = context.layers.get(layer, [])
            info["layers"][layer.value] = {
                "count": len(items),
                "capacity": layer.capacity
            }

        if engine:
            info["query_stats"] = engine.get_query_stats()

        return info

    def clear_context(self, user_id: str) -> None:
        """
        Clear all context data for a user

        Args:
            user_id: User identifier
        """
        if user_id in self.contexts:
            del self.contexts[user_id]
        if user_id in self.query_engines:
            del self.query_engines[user_id]

        logger.info(f"Cleared context for user {user_id}")
