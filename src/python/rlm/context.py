"""
RLM Context Management

Implements hierarchical context layers for recursive language model.

# Context Layers
- **Layer 0 (Raw)**: Recent events (last 100)
- **Layer 1 (Day)**: Daily summaries
- **Layer 2 (Week)**: Weekly summaries
- **Layer 3 (Month)**: Monthly summaries
- **Layer 4 (Year)**: Yearly summaries
"""

import logging
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional
from dataclasses import dataclass, field
from enum import Enum

logger = logging.getLogger(__name__)


class ContextLayer(Enum):
    """Context layer types"""
    RAW = "raw"           # Recent events (last ~100)
    DAY = "day"           # Daily summaries
    WEEK = "week"         # Weekly summaries
    MONTH = "month"       # Monthly summaries
    YEAR = "year"         # Yearly summaries

    @property
    def time_span_hours(self) -> int:
        """Get time span in hours for this layer"""
        spans = {
            ContextLayer.RAW: 24,      # Last 24 hours (raw events)
            ContextLayer.DAY: 24,      # 1 day
            ContextLayer.WEEK: 168,    # 7 days
            ContextLayer.MONTH: 720,   # 30 days
            ContextLayer.YEAR: 8760,   # 365 days
        }
        return spans[self]

    @property
    def capacity(self) -> int:
        """Maximum number of items in this layer"""
        capacities = {
            ContextLayer.RAW: 100,
            ContextLayer.DAY: 30,     # Last 30 days
            ContextLayer.WEEK: 52,    # Last 52 weeks
            ContextLayer.MONTH: 24,   # Last 24 months
            ContextLayer.YEAR: 10,    # Last 10 years
        }
        return capacities[self]


@dataclass
class ContextSummary:
    """A context summary at a specific layer"""
    layer: ContextLayer
    timestamp: datetime
    content: str
    metadata: Dict[str, Any] = field(default_factory=dict)
    token_count: int = 0


@dataclass
class ContextItem:
    """A single context item (event)"""
    timestamp: datetime
    content: str
    event_type: str
    metadata: Dict[str, Any] = field(default_factory=dict)
    token_count: int = 0


class RecursiveContext:
    """
    Manages hierarchical context layers for recursive processing

    # Architecture
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

    # Query Strategy
    When querying, we start from Layer 0 and progressively
    include higher layers until we reach the token budget.
    """

    def __init__(self, user_id: str):
        """
        Initialize recursive context for a user

        Args:
            user_id: User identifier
        """
        self.user_id = user_id
        self.layers: Dict[ContextLayer, List[Any]] = {
            layer: [] for layer in ContextLayer
        }
        self.last_updated: Dict[ContextLayer, datetime] = {}

    def add_raw_event(self, event: ContextItem) -> None:
        """
        Add a raw event to Layer 0

        Args:
            event: ContextItem to add
        """
        self.layers[ContextLayer.RAW].append(event)

        # Keep only the most recent events
        capacity = ContextLayer.RAW.capacity
        if len(self.layers[ContextLayer.RAW]) > capacity:
            self.layers[ContextLayer.RAW] = self.layers[ContextLayer.RAW][-capacity:]

        self.last_updated[ContextLayer.RAW] = datetime.now()

    def get_context_for_query(
        self,
        max_tokens: int = 4000,
        start_date: Optional[datetime] = None,
        end_date: Optional[datetime] = None
    ) -> List[Any]:
        """
        Get context items for a query, respecting token budget

        Strategy: Start from Layer 0 (most detailed), then progressively
        include higher layers until we reach max_tokens or exhaust all layers.

        Args:
            max_tokens: Maximum total tokens to return
            start_date: Filter events after this date
            end_date: Filter events before this date

        Returns:
            List of context items (ContextItem or ContextSummary)
        """
        context_items = []
        tokens_used = 0

        # Process layers from most detailed to least detailed
        for layer in [ContextLayer.RAW, ContextLayer.DAY, ContextLayer.WEEK,
                      ContextLayer.MONTH, ContextLayer.YEAR]:
            items = self.layers[layer]

            # Filter by date range if specified
            if start_date or end_date:
                items = self._filter_by_date(items, start_date, end_date)

            # Add items until we reach token limit
            for item in items:
                item_tokens = getattr(item, 'token_count', 100)  # Default estimate

                if tokens_used + item_tokens > max_tokens:
                    break

                context_items.append(item)
                tokens_used += item_tokens

            # Stop if we've used up our token budget
            if tokens_used >= max_tokens * 0.95:  # 95% threshold
                break

        logger.info(f"Retrieved {len(context_items)} context items using {tokens_used} tokens")
        return context_items

    def _filter_by_date(
        self,
        items: List[Any],
        start_date: Optional[datetime],
        end_date: Optional[datetime]
    ) -> List[Any]:
        """Filter items by date range"""
        if not start_date and not end_date:
            return items

        filtered = []
        for item in items:
            item_date = item.timestamp

            if start_date and item_date < start_date:
                continue
            if end_date and item_date > end_date:
                continue

            filtered.append(item)

        return filtered

    async def summarize_layer(
        self,
        target_layer: ContextLayer,
        llm_client: Any
    ) -> List[ContextSummary]:
        """
        Generate summaries for a specific layer

        Args:
            target_layer: Layer to generate summaries for
            llm_client: LLM client for generating summaries

        Returns:
            List of generated summaries
        """
        # Determine source layer (next more detailed layer)
        layer_order = [
            ContextLayer.RAW,
            ContextLayer.DAY,
            ContextLayer.WEEK,
            ContextLayer.MONTH,
        ]

        try:
            source_idx = layer_order.index(target_layer) - 1
        except ValueError:
            logger.error(f"Cannot summarize layer {target_layer}")
            return []

        if source_idx < 0:
            logger.error(f"No source layer for {target_layer}")
            return []

        source_layer = layer_order[source_idx]
        source_items = self.layers[source_layer]

        if not source_items:
            return []

        # Group source items by time period
        groups = self._group_by_time_period(source_items, target_layer)

        # Generate summary for each group
        summaries = []
        for period_start, items in groups.items():
            summary = await self._generate_summary(
                items=items,
                layer=target_layer,
                period_start=period_start,
                llm_client=llm_client
            )
            summaries.append(summary)

        # Store summaries
        self.layers[target_layer] = summaries
        self.last_updated[target_layer] = datetime.now()

        return summaries

    def _group_by_time_period(
        self,
        items: List[Any],
        target_layer: ContextLayer
    ) -> Dict[datetime, List[Any]]:
        """Group items by time period based on target layer"""
        groups = {}

        for item in items:
            item_date = item.timestamp

            # Determine period start based on layer
            if target_layer == ContextLayer.DAY:
                # Group by day
                period_start = item_date.replace(hour=0, minute=0, second=0, microsecond=0)
            elif target_layer == ContextLayer.WEEK:
                # Group by week (Monday)
                days_since_monday = item_date.weekday()
                period_start = (item_date - timedelta(days=days_since_monday)).replace(
                    hour=0, minute=0, second=0, microsecond=0
                )
            elif target_layer == ContextLayer.MONTH:
                # Group by month
                period_start = item_date.replace(day=1, hour=0, minute=0, second=0, microsecond=0)
            elif target_layer == ContextLayer.YEAR:
                # Group by year
                period_start = item_date.replace(month=1, day=1, hour=0, minute=0, second=0, microsecond=0)
            else:
                period_start = item_date

            if period_start not in groups:
                groups[period_start] = []
            groups[period_start].append(item)

        return groups

    async def _generate_summary(
        self,
        items: List[Any],
        layer: ContextLayer,
        period_start: datetime,
        llm_client: Any
    ) -> ContextSummary:
        """Generate a summary for a group of items"""
        # Concatenate item contents
        content_parts = []
        for item in items:
            if isinstance(item, ContextItem):
                content_parts.append(f"[{item.event_type}] {item.content}")
            elif isinstance(item, ContextSummary):
                content_parts.append(item.content)

        combined_content = "\n".join(content_parts)

        # For V1, use simple summarization (V2 will use LLM)
        # This is a placeholder that would call the LLM in full implementation
        summary_text = f"Summary of {len(items)} items from {period_start.strftime('%Y-%m-%d')}"

        return ContextSummary(
            layer=layer,
            timestamp=period_start,
            content=summary_text,
            metadata={
                "item_count": len(items),
                "source_layer": "RAW" if layer == ContextLayer.DAY else layer.value
            },
            token_count=len(summary_text.split())
        )

    def estimate_tokens(self, text: str) -> int:
        """
        Estimate token count for text

        Args:
            text: Text to estimate

        Returns:
            Estimated token count
        """
        # Simple estimation: ~1 token per 4 characters
        return len(text) // 4
