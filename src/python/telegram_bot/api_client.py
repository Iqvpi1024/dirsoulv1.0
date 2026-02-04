"""
DirSoul API Client - Python HTTP Client for Rust Backend

Communicates with the DirSoul Rust HTTP API.

# Example
```python
client = DirSoulAPI("http://127.0.0.1:8080")

# Send chat message
response = await client.send_chat("user123", "I ate 2 apples")

# Get statistics
stats = await client.get_stats("user123", "30d")

# Get timeline
timeline = await client.get_timeline("user123", start_date, end_date)
```
"""

import logging
import aiohttp
from typing import Dict, List, Any, Optional
from datetime import datetime, timedelta

logger = logging.getLogger(__name__)


class DirSoulAPI:
    """Async HTTP client for DirSoul Rust API"""

    def __init__(self, base_url: str = "http://127.0.0.1:8080"):
        """
        Initialize API client

        Args:
            base_url: Base URL of the DirSoul API server
        """
        self.base_url = base_url.rstrip("/")
        self.session: Optional[aiohttp.ClientSession] = None

    async def _get_session(self) -> aiohttp.ClientSession:
        """Get or create HTTP session"""
        if self.session is None or self.session.closed:
            timeout = aiohttp.ClientTimeout(total=30)
            self.session = aiohttp.ClientSession(timeout=timeout)
        return self.session

    async def close(self):
        """Close the HTTP session"""
        if self.session and not self.session.closed:
            await self.session.close()

    # ========================================================================
    # Health Check
    # ========================================================================

    async def health_check(self) -> Dict[str, Any]:
        """
        Check API health status

        Returns:
            Dict with health status information
        """
        session = await self._get_session()
        url = f"{self.base_url}/health"

        try:
            async with session.get(url) as response:
                response.raise_for_status()
                return await response.json()
        except Exception as e:
            logger.error(f"Health check failed: {e}")
            raise

    # ========================================================================
    # Chat API
    # ========================================================================

    async def send_chat(
        self,
        user_id: str,
        message: str,
        history: Optional[List[Dict[str, str]]] = None,
        context: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Send a chat message and get AI response

        Args:
            user_id: User identifier
            message: User message text
            history: Conversation history (optional)
            context: Additional context (optional)

        Returns:
            Dict with response, updated history, and metadata
        """
        session = await self._get_session()
        url = f"{self.base_url}/api/chat"

        payload = {
            "message": message,
            "user_id": user_id,
            "history": history or [],
            "context": context
        }

        try:
            async with session.post(url, json=payload) as response:
                response.raise_for_status()
                return await response.json()
        except aiohttp.ClientError as e:
            logger.error(f"Chat request failed: {e}")
            raise

    # ========================================================================
    # Timeline API
    # ========================================================================

    async def get_timeline(
        self,
        user_id: str,
        start_date: str,
        end_date: Optional[str] = None,
        filters: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Get timeline events for a user

        Args:
            user_id: User identifier
            start_date: Start date in ISO format (e.g., "2024-01-01T00:00:00")
            end_date: End date in ISO format (defaults to now)
            filters: Optional filters for entities, event types, etc.

        Returns:
            Dict with events_by_date, total_events, and summary
        """
        session = await self._get_session()
        url = f"{self.base_url}/api/timeline"

        if end_date is None:
            end_date = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")

        payload = {
            "user_id": user_id,
            "start_date": start_date,
            "end_date": end_date,
            "filters": filters
        }

        try:
            async with session.post(url, json=payload) as response:
                response.raise_for_status()
                return await response.json()
        except aiohttp.ClientError as e:
            logger.error(f"Timeline request failed: {e}")
            raise

    # ========================================================================
    # Statistics API
    # ========================================================================

    async def get_stats(
        self,
        user_id: str,
        time_range: str = "30d"
    ) -> Dict[str, Any]:
        """
        Get user statistics

        Args:
            user_id: User identifier
            time_range: Time range - "7d", "30d", "90d", or "all"

        Returns:
            Dict with total_memories, total_events, entities, etc.
        """
        session = await self._get_session()
        url = f"{self.base_url}/api/stats"

        payload = {
            "user_id": user_id,
            "time_range": time_range
        }

        try:
            async with session.post(url, json=payload) as response:
                response.raise_for_status()
                return await response.json()
        except aiohttp.ClientError as e:
            logger.error(f"Stats request failed: {e}")
            raise

    # ========================================================================
    # Helper Methods
    # ========================================================================

    @staticmethod
    def calculate_date_range(days: int) -> tuple[str, str]:
        """
        Calculate ISO date range for last N days

        Args:
            days: Number of days back from now

        Returns:
            Tuple of (start_date, end_date) in ISO format
        """
        end = datetime.now()
        start = end - timedelta(days=days)

        return (
            start.strftime("%Y-%m-%dT%H:%M:%S"),
            end.strftime("%Y-%m-%dT%H:%M:%S")
        )

    @staticmethod
    def format_event_summary(event: Dict[str, Any]) -> str:
        """
        Format event as human-readable string

        Args:
            event: Event dict with actor, action, target, quantity, unit

        Returns:
            Formatted event string
        """
        actor = event.get("actor") or "You"
        action = event.get("action", "did something")
        target = event.get("target", "")

        text = f"{actor} {action}"
        if target:
            text += f" {target}"

        quantity = event.get("quantity")
        unit = event.get("unit")
        if quantity is not None:
            text += f" {quantity}"
            if unit:
                text += f"{unit}"

        return text


# ============================================================================
# Convenience Functions
# ============================================================================

async def record_memory(
    api: DirSoulAPI,
    user_id: str,
    text: str
) -> Dict[str, Any]:
    """
    Quick helper to record a memory

    Args:
        api: DirSoulAPI instance
        user_id: User identifier
        text: Memory text

    Returns:
        Chat response from API
    """
    return await api.send_chat(user_id, text)


async def get_recent_timeline(
    api: DirSoulAPI,
    user_id: str,
    days: int = 7
) -> Dict[str, Any]:
    """
    Quick helper to get recent timeline

    Args:
        api: DirSoulAPI instance
        user_id: User identifier
        days: Number of days back

    Returns:
        Timeline response from API
    """
    start_date, end_date = DirSoulAPI.calculate_date_range(days)
    return await api.get_timeline(user_id, start_date, end_date)
