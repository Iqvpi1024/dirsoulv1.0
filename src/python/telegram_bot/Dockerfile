# DirSoul Telegram Bot Dockerfile

FROM python:3.12-slim-bookworm

# Set working directory
WORKDIR /app

# Copy requirements file
COPY requirements.txt .

# Install dependencies
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY . .

# Create logs directory
RUN mkdir -p /app/logs

# Set environment variables
ENV PYTHONUNBUFFERED=1
ENV DIRSOUL_API_URL=http://app:8080

# Create non-root user
RUN useradd -m -u 1000 telegrambot && \
    chown -R telegrambot:telegrambot /app

USER telegrambot

# Health check
HEALTHCHECK --interval=60s --timeout=10s --start-period=10s --retries=3 \
    CMD python -c "import aiohttp; import asyncio; asyncio.run(aiohttp.ClientSession().get('${DIRSOUL_API_URL}/health'))" || exit 1

# Run the bot
CMD ["python", "bot.py"]
