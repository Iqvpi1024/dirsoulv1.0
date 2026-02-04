# DirSoul Multi-Stage Dockerfile
#
# Builds the Rust core and creates a runtime image with Python
# for the Streamlit interface and Telegram bot.

# ============================================================================
# Stage 1: Rust Builder
# ============================================================================
FROM rust:1.75-bookworm AS rust-builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy Cargo files
COPY src/rust/Cargo.toml src/rust/Cargo.lock ./

# Create src directory with placeholder main
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src/rust/src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# ============================================================================
# Stage 2: Python Runtime
# ============================================================================
FROM python:3.12-slim-bookworm AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    postgresql-client \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Ollama (for local models)
RUN curl -fsSL https://ollama.com/install.sh | sh

# Set working directory
WORKDIR /app

# Copy compiled Rust binary
COPY --from=rust-builder /build/target/release/dirsoul /usr/local/bin/dirsoul

# Copy Python application files
COPY src/python /app/src/python

# Install Python dependencies
WORKDIR /app/src/python
RUN pip install --no-cache-dir -r requirements.txt
RUN pip install --no-cache-dir -r telegram_bot/requirements.txt
RUN pip install --no-cache-dir -r rlm/requirements.txt

# Install Streamlit for the UI
RUN pip install --no-cache-dir streamlit

WORKDIR /app

# Create directories
RUN mkdir -p /app/data /app/config /app/prompts /app/logs

# Copy configuration files
COPY config /app/config
COPY prompts /app/prompts

# Set environment variables
ENV DIRSOUL_DATABASE_URL=postgresql://dirsoul:password@db:5432/dirsoul_db
ENV DIRSOUL_API_HOST=0.0.0.0
ENV DIRSOUL_API_PORT=8080
ENV OLLAMA_HOST=http://localhost:11434

# Expose ports
EXPOSE 8080 8501

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command: Start both API and Streamlit
CMD ["bash", "-c", "dirsoul & streamlit run streamlit/app.py --server.port=8501 --server.address=0.0.0.0"]
