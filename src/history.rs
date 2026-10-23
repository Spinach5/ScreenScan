//! 截图记录存储接口（当前仅预留清空入口）。

use anyhow::Result;

use crate::util::path;

pub fn clear() -> Result<()> {
    let dir = path::home_dir().join(".screenscan").join("history");
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir)?;
    }
    log::info!("截图记录已清空（存储接口预留）");
    Ok(())
}
