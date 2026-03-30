use thiserror::Error;

#[derive(Error, Debug)]
pub enum LiteError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("加密错误: {0}")]
    Crypto(String),

    #[error("Keychain错误: {0}")]
    Keychain(String),

    #[error("终端唤起错误: {0}")]
    Terminal(String),

    #[error("SSH错误: {0}")]
    Ssh(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON序列化错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("服务器未找到: {0}")]
    ServerNotFound(String),

    #[error("分组未找到: {0}")]
    GroupNotFound(String),

    #[error("认证失败")]
    AuthFailed,

    #[error("无效的主密码")]
    InvalidMasterPassword,
}

impl serde::Serialize for LiteError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}
