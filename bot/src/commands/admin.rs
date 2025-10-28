
use std::sync::Arc;
use std::time::Instant;
use teloxide::prelude::*;

use crate::state::AppState;

/// Handler for the /version command to check the current git version of the bot.
/// This function tries to get the git commit hash and optionally the branch/tag.
/// This function is only available for super admins.
pub async fn handle_version(
    bot: Bot,
    msg: Message,
    state: Arc<AppState>,
) -> Result<(), anyhow::Error> {
    let start_time = Instant::now();
    // These environment variables can be set at build time using build.rs or cargo build scripts.
    // Fallback to "unknown" if not set.
    let user_id = msg.from.as_ref().unwrap().id.0 as i64;
    let username = state.user_service.get_username_from_user(user_id).await.unwrap();

    tracing::info!(
        "Handling /version command with parameters: admin_user_id={}",
        user_id
    );
    let user = state.user_service.get_current_user(user_id)
        .await
        .ok()
        .flatten();

    // To get the git hash at runtime, you need to embed it at compile time using build scripts.
    // This code assumes you have a build.rs that sets GIT_HASH, GIT_BRANCH, and GIT_TAG as environment variables.
    // If not set, it will still show "unknown".
    // See example build.rs below for how to set these variables.

    let git_hash = option_env!("GIT_HASH").unwrap_or("unknown");
    let git_branch = option_env!("GIT_BRANCH").unwrap_or("unknown");
    let git_tag = option_env!("GIT_TAG").unwrap_or("unknown");


    // Format build time as human-readable string if possible
    let build_time_raw = option_env!("BUILD_TIME").unwrap_or("unknown");
    let build_time_human = if let Ok(epoch) = build_time_raw.parse::<u64>() {
        use chrono::{TimeZone, Utc};
        let dt = Utc.timestamp_opt(epoch as i64, 0).single();
        if let Some(dt) = dt {
            dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
        } else {
            build_time_raw.to_string()
        }
    } else {
        build_time_raw.to_string()
    };

    let version_info = format!(
        "✅ 🤖 <b>Bot Version</b> \n\
        <b>Branch:</b> <code>{}</code>\n\
        <b>Tag:</b> <code>{}</code>\n\
        <b>Commit:</b> <code>{}</code>\n\
        <b>Build Time:</b> <code>{}</code>\n\
        <b>OS:</b> <code>{}</code>",
        git_branch,
        git_tag,
        git_hash,
        build_time_human,
        option_env!("CARGO_CFG_TARGET_OS").unwrap_or("unknown")
    );

    bot.send_message(msg.chat.id, version_info)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}