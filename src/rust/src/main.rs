use dirsoul::Result;
use dirsoul::http_api::HttpServer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string())
        )
        .init();

    info!("🧠 DirSoul - 本地优先的永久记忆框架");
    info!("版本: {}", env!("CARGO_PKG_VERSION"));
    info!("构建你的数字大脑...");

    // 获取数据库 URL
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://user443319201@/dirsoul_db".to_string());

    // 获取绑定地址（默认 0.0.0.0:8080 允许公网访问）
    let bind_address = std::env::var("DIRSOUL_BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    // 创建并启动 HTTP 服务器
    info!("📡 启动 API 服务器: {}", bind_address);
    let server = HttpServer::new(bind_address, database_url)?;

    // 启动服务器（阻塞运行）
    server.start().await?;

    Ok(())
}
