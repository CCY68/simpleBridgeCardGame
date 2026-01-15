//! AI 模組 - Server 內建 AI 玩家
//!
//! 此模組提供 Bridge Mode 所需的 AI 功能：
//! - AiPlayer: 虛擬 AI 玩家 (不佔用 TCP 連線)
//! - Strategy: 可插拔的出牌策略
//! - TurnHandler: AI 出牌處理

mod player;
mod strategy;

pub use player::AiPlayer;
pub use strategy::{AiStrategy, SmartStrategy};
