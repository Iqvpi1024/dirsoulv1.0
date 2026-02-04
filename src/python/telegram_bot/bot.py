"""
DirSoul Telegram Bot - Mobile Input Interface

A simple Telegram bot that allows users to record memories on the go.
Critical for reducing input friction (chat88.md priority).

# Design Principles
- **V1策略**: Simple text input first
- **低摩擦**: No account setup needed - uses Telegram user_id
- **异步处理**: Non-blocking message handling

# Example
```bash
python bot.py
```
"""

import logging
import os
import sys
import asyncio
from datetime import datetime
from typing import Optional

from telegram import Update, InlineKeyboardButton, InlineKeyboardMarkup
from telegram.ext import (
    Application,
    CommandHandler,
    MessageHandler,
    filters,
    ContextTypes,
)

# Add parent directory to path for API client
sys.path.insert(0, str(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from api_client import DirSoulAPI

# Configure logging
logging.basicConfig(
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    level=logging.INFO,
)
logger = logging.getLogger(__name__)


# ============================================================================
# Configuration
# ============================================================================

# Get environment variables
DIRSOUL_API_URL = os.getenv("DIRSOUL_API_URL", "http://127.0.0.1:8080")
TELEGRAM_BOT_TOKEN = os.getenv("TELEGRAM_BOT_TOKEN")

if not TELEGRAM_BOT_TOKEN:
    logger.error("TELEGRAM_BOT_TOKEN environment variable not set!")
    logger.error("Please set it with: export TELEGRAM_BOT_TOKEN='your_token_here'")
    sys.exit(1)


# ============================================================================
# API Client
# ============================================================================

api_client = DirSoulAPI(base_url=DIRSOUL_API_URL)


# ============================================================================
# Command Handlers
# ============================================================================

async def start_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle /start command - bot initialization"""
    user = update.effective_user

    welcome_message = f"""
🧠 *Welcome to DirSoul - Your Digital Brain*

Hi {user.first_name}! I'm your personal memory assistant.

*Quick Start:*
• Just send me any message to record a memory
• Use /help to see all commands
• Use /stats to view your statistics

*What can I remember for you?*
✓ Daily events ("I ate 2 apples for breakfast")
✓ Thoughts and ideas
✓ Meetings and tasks
✓ Anything you want to track over time

Your data is stored locally and encrypted. 🔒
    """.strip()

    await update.message.reply_text(
        welcome_message,
        parse_mode="Markdown"
    )


async def help_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle /help command - show available commands"""
    help_text = """
🧠 *DirSoul Commands*

*Memory Recording:*
• Any text message → Record as memory
• /record <text> → Explicit record command

*Viewing Data:*
• /stats [7d|30d|90d|all] → View statistics
• /timeline <days> → View recent timeline (e.g., /timeline 7)

*Settings:*
• /help → Show this help message
• /start → Welcome message

*Tips:*
• Use natural language: "I drank 3 cups of coffee"
• Include quantities when possible: "Read for 2 hours"
• Be specific: "Meeting with John about Project X"

_V1版本 - 更多功能开发中..._
    """.strip()

    await update.message.reply_text(
        help_text,
        parse_mode="Markdown"
    )


async def stats_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle /stats command - show user statistics"""
    user_id = str(update.effective_user.id)

    # Get time range from args (default: 30d)
    time_range = "30d"
    if context.args and context.args[0] in ["7d", "30d", "90d", "all"]:
        time_range = context.args[0]

    try:
        stats = await api_client.get_stats(user_id, time_range)

        stats_message = f"""
📊 *Your DirSoul Statistics* ({time_range})

*Overview:*
• Total Events: {stats['total_events']}
• Total Entities: {stats['total_entities']}

*Top Event Types:*
"""

        # Add top 5 event types
        event_types = sorted(
            stats['event_types'].items(),
            key=lambda x: x[1],
            reverse=True
        )[:5]

        for action, count in event_types:
            stats_message += f"• {action}: {count}\n"

        if stats['events_per_day']:
            most_active = max(stats['events_per_day'].items(), key=lambda x: x[1])
            stats_message += f"\n*Most Active Day:* {most_active[0]} ({most_active[1]} events)\n"

        await update.message.reply_text(stats_message, parse_mode="Markdown")

    except Exception as e:
        logger.error(f"Error getting stats: {e}")
        await update.message.reply_text(
            f"❌ Sorry, couldn't fetch statistics. Please try again later.\n\n(Error: {str(e)})"
        )


async def timeline_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle /timeline command - show recent timeline"""
    user_id = str(update.effective_user.id)

    # Get days from args (default: 7)
    days = 7
    if context.args and context.args[0].isdigit():
        days = int(context.args[0])

    # Calculate date range
    end_date = datetime.now().strftime("%Y-%m-%dT%H:%M:%S")
    start_date = (datetime.now() - __import__("datetime").timedelta(days=days)).strftime("%Y-%m-%dT%H:%M:%S")

    try:
        timeline = await api_client.get_timeline(user_id, start_date, end_date)

        if timeline['total_events'] == 0:
            await update.message.reply_text(
                f"📅 No events found in the last {days} days.\n\n"
                f"Start recording memories by sending me a message!"
            )
            return

        timeline_message = f"📅 *Your Timeline* (Last {days} days)\n\n"

        # Show up to 20 most recent events
        events_shown = 0
        for date, events in sorted(timeline['events_by_date'].items(), reverse=True):
            if events_shown >= 20:
                break

            timeline_message += f"*{date}*\n"
            for event in events[:5]:  # Max 5 events per day
                actor = event['actor'] or "You"
                quantity = f" {event['quantity']}{event['unit']}" if event.get('quantity') else ""
                timeline_message += f"  • {actor} {event['action']}{quantity}\n"
                events_shown += 1

            if events_shown >= 20:
                timeline_message += f"\n_... and more_"
                break
            timeline_message += "\n"

        await update.message.reply_text(timeline_message, parse_mode="Markdown")

    except Exception as e:
        logger.error(f"Error getting timeline: {e}")
        await update.message.reply_text(
            f"❌ Sorry, couldn't fetch timeline. Please try again later.\n\n(Error: {str(e)})"
        )


async def record_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle /record command - explicit record command"""
    user_id = str(update.effective_user.id)

    # Get text from args
    if not context.args:
        await update.message.reply_text(
            "Usage: /record <your memory text>\n\n"
            "Example: /record I ate 2 apples for breakfast"
        )
        return

    text = " ".join(context.args)

    # Show typing indicator
    await update.message.chat.send_action("typing")

    try:
        response = await api_client.send_chat(user_id, text)

        await update.message.reply_text(
            f"✅ *Memory Recorded!* 🧠\n\n{response['response']}\n\n"
            f"_Processed in {response['processing_time_ms']}ms_",
            parse_mode="Markdown"
        )

    except Exception as e:
        logger.error(f"Error recording memory: {e}")
        await update.message.reply_text(
            f"❌ Sorry, couldn't record memory. Please try again later.\n\n(Error: {str(e)})"
        )


async def handle_message(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle regular text messages - record as memory"""
    user_id = str(update.effective_user.id)
    text = update.message.text

    if not text:
        return

    # Show typing indicator
    await update.message.chat.send_action("typing")

    try:
        response = await api_client.send_chat(user_id, text)

        # Format response
        response_text = response['response']

        # Add success indicator
        await update.message.reply_text(
            f"{response_text}\n\n_✓ Recorded_",
            parse_mode="Markdown"
        )

    except Exception as e:
        logger.error(f"Error handling message: {e}")
        await update.message.reply_text(
            f"❌ Sorry, couldn't process your message. Please try again later.\n\n(Error: {str(e)})"
        )


# ============================================================================
# Error Handler
# ============================================================================

async def error_handler(update: object, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Log errors caused by updates"""
    logger.error(f"Exception while handling an update: {context.error}", exc_info=context.error)


# ============================================================================
# Main
# ============================================================================

def main() -> None:
    """Start the bot"""
    logger.info("🧠 DirSoul Telegram Bot starting...")
    logger.info(f"API URL: {DIRSOUL_API_URL}")

    # Create the Application
    application = Application.builder().token(TELEGRAM_BOT_TOKEN).build()

    # Register command handlers
    application.add_handler(CommandHandler("start", start_command))
    application.add_handler(CommandHandler("help", help_command))
    application.add_handler(CommandHandler("stats", stats_command))
    application.add_handler(CommandHandler("timeline", timeline_command))
    application.add_handler(CommandHandler("record", record_command))

    # Register message handler (for non-command text messages)
    application.add_handler(MessageHandler(filters.TEXT & ~filters.COMMAND, handle_message))

    # Register error handler
    application.add_error_handler(error_handler)

    # Start the bot
    logger.info("Bot is running! Press Ctrl+C to stop.")
    application.run_polling(allowed_updates=Update.ALL_TYPES)


if __name__ == "__main__":
    main()
