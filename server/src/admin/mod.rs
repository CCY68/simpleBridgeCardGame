//! Admin Module - 遠端管理介面
//!
//! 提供獨立 TCP Port (8890) 的管理功能：
//! - 伺服器狀態監控
//! - 遊戲訊息記錄查看
//! - 遊戲重設
//! - 玩家踢除

pub mod commands;
pub mod logger;
pub mod server;

pub use commands::{AdminEvent, AdminResponse, PlayerInfo, RoomInfo};
#[allow(unused_imports)]
pub use logger::{EventType, GameLogger};
pub use server::{spawn_admin_server, AdminConfig};
