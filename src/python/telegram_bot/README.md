# DirSoul Telegram Bot

Mobile input interface for DirSoul - your personal digital brain.

## Features

- 📝 **Quick Memory Recording**: Just send a message to record a memory
- 📊 **Statistics**: View your memory statistics with `/stats`
- 📅 **Timeline**: Browse your memory timeline with `/timeline`
- 🔄 **Async Processing**: Non-blocking message handling
- 🔒 **Local Storage**: All data stored locally on your server

## Setup

### Prerequisites

1. **Rust API Server** running on `http://127.0.0.1:8080`
2. **Telegram Bot Token** from [@BotFather](https://t.me/botfather)

### Installation

```bash
cd src/python/telegram_bot

# Install dependencies
pip install -r requirements.txt

# Set your bot token
export TELEGRAM_BOT_TOKEN="your_bot_token_here"

# (Optional) Set API URL if different from default
export DIRSOUL_API_URL="http://127.0.0.1:8080"

# Run the bot
python bot.py
```

### Getting a Bot Token

1. Open Telegram and search for [@BotFather](https://t.me/botfather)
2. Send `/newbot`
3. Follow the instructions to name your bot
4. Copy the token provided

## Commands

| Command | Description | Example |
|---------|-------------|---------|
| `/start` | Welcome message | `/start` |
| `/help` | Show all commands | `/help` |
| `/stats [range]` | View statistics | `/stats 30d` |
| `/timeline [days]` | View timeline | `/timeline 7` |
| `/record <text>` | Record memory | `/record I ate 2 apples` |
| `<any text>` | Record memory (implicit) | `I went for a run` |

## Usage Examples

### Recording Memories

```
You: I ate 2 apples for breakfast
Bot: 🧠 DirSoul V1收到: I ate 2 apples for breakfast
     [这是一个演示响应，完整AI功能待实现]
     ✓ Recorded

You: /record Had a meeting with John about Project X
Bot: ✅ Memory Recorded! 🧠
```

### Viewing Statistics

```
You: /stats 7d
Bot: 📊 Your DirSoul Statistics (7d)

     Overview:
     • Total Events: 42
     • Total Entities: 15

     Top Event Types:
     • ate: 8
     • went: 6
     • bought: 4
```

### Viewing Timeline

```
You: /timeline 3
Bot: 📅 Your Timeline (Last 3 days)

     2024-02-04
     • You ate apples
     • You went running
     • You read a book

     2024-02-03
     • You bought groceries
     • You watched a movie
```

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `TELEGRAM_BOT_TOKEN` | *required* | Your Telegram bot token |
| `DIRSOUL_API_URL` | `http://127.0.0.1:8080` | DirSoul API server URL |

## Development

### Project Structure

```
telegram_bot/
├── bot.py           # Main bot application
├── api_client.py    # HTTP client for Rust API
├── requirements.txt # Python dependencies
└── README.md        # This file
```

### Testing

To test without a real bot:

```python
import asyncio
from api_client import DirSoulAPI

async def test():
    api = DirSoulAPI("http://127.0.0.1:8080")

    # Test health check
    health = await api.health_check()
    print(f"API Health: {health}")

    # Test chat
    response = await api.send_chat("test_user", "Hello!")
    print(f"Response: {response}")

    await api.close()

asyncio.run(test())
```

## Troubleshooting

### Bot not responding

1. Check if the bot token is correct
2. Check if the Rust API server is running
3. Check the bot logs for errors

### API connection errors

1. Verify `DIRSOUL_API_URL` is correct
2. Check if the Rust API server is accessible
3. Try `curl http://127.0.0.1:8080/health`

### Permission errors

1. Make sure you have permission to read/write log files
2. Check if port is available

## Future Features (V2)

- 🎤 Voice message support (whisper local transcription)
- 📸 Image input support
- 📎 File/document support
- 🌐 Multi-language support
- 📱 Rich inline keyboards

## License

MIT License - See LICENSE file in project root
