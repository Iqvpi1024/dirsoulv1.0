use dirsoul::Result;
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

    // TODO: 加载配置
    // TODO: 初始化数据库连接
    // TODO: 启动 API 服务器

    info!("✅ DirSoul 核心启动成功");

    Ok(())
}
