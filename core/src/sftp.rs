//! SFTP文件系统浏览器
//! 使用ssh2 crate的SFTP支持

use crate::error::LiteError;
use ssh2::Sftp;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// SFTP会话管理器
pub struct SftpSessionManager {
    sessions: HashMap<String, Arc<TokioMutex<Sftp>>>,
}

impl SftpSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// 创建SFTP会话
    pub async fn create_session(&mut self, session_id: &str, sftp: Sftp) -> Result<(), LiteError> {
        self.sessions
            .insert(session_id.to_string(), Arc::new(TokioMutex::new(sftp)));
        Ok(())
    }

    /// 列出目录
    pub async fn list_dir(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<Vec<SftpEntry>, LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or(LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        let dir = sftp
            .readdir(Path::new(path))
            .map_err(|e| LiteError::Ssh(format!("读取目录失败: {}", e)))?;

        let mut entries = Vec::new();
        for (p, stat) in dir {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let file_type = if stat.is_dir() { "directory" } else { "file" };

            entries.push(SftpEntry {
                name,
                path: p.to_string_lossy().to_string(),
                file_type: file_type.to_string(),
                size: stat.size.unwrap_or(0) as i64,
                mtime: stat.mtime.unwrap_or(0) as i64,
            });
        }

        // 按类型和名称排序
        entries.sort_by(|a, b| match (a.file_type.as_str(), b.file_type.as_str()) {
            ("directory", "file") => std::cmp::Ordering::Less,
            ("file", "directory") => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(entries)
    }

    /// 创建目录
    pub async fn mkdir(&self, session_id: &str, path: &str) -> Result<(), LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or(LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        sftp.mkdir(Path::new(path), 0o755)
            .map_err(|e| LiteError::Ssh(format!("创建目录失败: {}", e)))?;
        Ok(())
    }

    /// 删除文件
    pub async fn remove_file(&self, session_id: &str, path: &str) -> Result<(), LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or(LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        sftp.unlink(Path::new(path))
            .map_err(|e| LiteError::Ssh(format!("删除文件失败: {}", e)))?;
        Ok(())
    }

    /// 删除目录
    pub async fn rmdir(&self, session_id: &str, path: &str) -> Result<(), LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or(LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        sftp.rmdir(Path::new(path))
            .map_err(|e| LiteError::Ssh(format!("删除目录失败: {}", e)))?;
        Ok(())
    }

    /// 重命名
    pub async fn rename(
        &self,
        session_id: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<(), LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or(LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        sftp.rename(Path::new(old_path), Path::new(new_path), None)
            .map_err(|e| LiteError::Ssh(format!("重命名失败: {}", e)))?;
        Ok(())
    }

    /// 下载文件
    pub async fn download(
        &self,
        session_id: &str,
        remote_path: &str,
    ) -> Result<Vec<u8>, LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or(LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        let mut file = sftp
            .open(Path::new(remote_path))
            .map_err(|e| LiteError::Ssh(format!("打开文件失败: {}", e)))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| LiteError::Ssh(format!("读取文件失败: {}", e)))?;

        Ok(contents)
    }

    /// 上传文件
    pub async fn upload(
        &self,
        session_id: &str,
        remote_path: &str,
        contents: &[u8],
    ) -> Result<(), LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or(LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        let mut file = sftp
            .create(Path::new(remote_path))
            .map_err(|e| LiteError::Ssh(format!("创建文件失败: {}", e)))?;

        file.write_all(contents)
            .map_err(|e| LiteError::Ssh(format!("写入文件失败: {}", e)))?;

        Ok(())
    }

    /// 获取统计信息
    pub async fn stat(&self, session_id: &str, path: &str) -> Result<SftpEntry, LiteError> {
        let sftp_mutex = self
            .sessions
            .get(session_id)
            .ok_or(LiteError::Ssh("SFTP会话不存在".to_string()))?;

        let sftp = sftp_mutex.lock().await;
        let stat = sftp
            .stat(Path::new(path))
            .map_err(|e| LiteError::Ssh(format!("获取文件信息失败: {}", e)))?;

        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let file_type = if stat.is_dir() { "directory" } else { "file" };

        Ok(SftpEntry {
            name,
            path: path.to_string(),
            file_type: file_type.to_string(),
            size: stat.size.unwrap_or(0) as i64,
            mtime: stat.mtime.unwrap_or(0) as i64,
        })
    }

    /// 关闭会话
    pub async fn close_session(&mut self, session_id: &str) -> Result<(), LiteError> {
        self.sessions.remove(session_id);
        Ok(())
    }

    /// 列出所有会话
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }
}

impl Default for SftpSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// SFTP条目
#[derive(Debug, Clone, serde::Serialize)]
pub struct SftpEntry {
    pub name: String,
    pub path: String,
    pub file_type: String, // "file", "directory", "symlink"
    pub size: i64,
    pub mtime: i64,
}

impl SftpEntry {
    /// 获取文件大小格式化字符串
    pub fn size_display(&self) -> String {
        if self.file_type == "directory" {
            "-".to_string()
        } else {
            format_size(self.size as u64)
        }
    }

    /// 获取修改时间格式化字符串
    pub fn mtime_display(&self) -> String {
        if self.mtime == 0 {
            "-".to_string()
        } else {
            let dt =
                chrono::DateTime::from_timestamp(self.mtime, 0).unwrap_or_else(chrono::Utc::now);
            dt.format("%Y-%m-%d %H:%M").to_string()
        }
    }
}

/// 格式化文件大小
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1}GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}KB", size as f64 / KB as f64)
    } else {
        format!("{}B", size)
    }
}
