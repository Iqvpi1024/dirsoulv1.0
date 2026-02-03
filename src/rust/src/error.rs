use thiserror::Error;

/// DirSoul 统一错误类型
#[derive(Error, Debug)]
pub enum DirSoulError {
    #[error("数据库错误: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("数据库连接错误: {0}")]
    DatabaseConnection(#[from] diesel::result::ConnectionError),

    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("加密错误: {0}")]
    Encryption(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("未找到: {0}")]
    NotFound(String),
}

/// DirSoul 统一 Result 类型
pub type Result<T> = std::result::Result<T, DirSoulError>;
