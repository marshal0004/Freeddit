//! Routes registry.

use axum::{
    routing::{get, post, delete, patch},
    Router,
};
use tower_http::services::ServeDir;

use crate::AppState;

pub mod auth;
pub mod users;
pub mod subreddits;
pub mod posts;
pub mod comments;
pub mod votes;
pub mod search;
pub mod seed;
pub mod saves;
pub mod reports;
pub mod modtools;
pub mod media;
pub mod multireddits;
pub mod notifications;
pub mod messages;
pub mod wiki;
pub mod security;

pub fn routes() -> Router<AppState> {
    Router::new()
        // health
        .route("/api/health", get(health))
        // auth
        .route("/api/register", post(auth::register))
        .route("/api/login", post(auth::login))
        .route("/api/me", get(auth::me))
        // subreddits
        .route("/api/subreddits", get(subreddits::list).post(subreddits::create))
        .route("/api/subreddits/{name}", get(subreddits::get_by_name))
        .route("/api/subreddits/{name}/subscribe", post(subreddits::subscribe))
        .route("/api/subreddits/{name}/posts", get(posts::list_by_subreddit))
        .route("/api/subreddits/popular", get(subreddits::popular))
        // subreddit rules + moderators (mod tools)
        .route("/api/subreddits/{name}/rules", get(modtools::list_rules).post(modtools::create_rule))
        .route("/api/subreddits/{name}/rules/{rule_id}", delete(modtools::delete_rule))
        .route("/api/subreddits/{name}/moderators", get(modtools::list_moderators))
        // subreddit wiki
        .route("/api/subreddits/{name}/wiki", get(wiki::list_pages))
        .route("/api/subreddits/{name}/wiki/{slug}", get(wiki::get_page).post(wiki::upsert_page).delete(wiki::delete_page))
        // posts
        .route("/api/posts", get(posts::list_home).post(posts::create))
        .route("/api/posts/{id}", get(posts::get_by_id))
        .route("/api/posts/{id}/edit", patch(posts::edit_post))
        .route("/api/posts/{id}/delete", delete(posts::delete_post))
        .route("/api/posts/{id}/comments", get(comments::list_for_post).post(comments::create))
        .route("/api/posts/{id}/sort/{sort}", get(comments::list_for_post_sorted))
        .route("/api/posts/{id}/mod", post(modtools::mod_action))
        // comments
        .route("/api/comments/{id}", get(comments::get_by_id))
        .route("/api/comments/{id}/edit", patch(comments::edit_comment))
        .route("/api/comments/{id}/delete", delete(comments::delete_comment))
        .route("/api/comments/{id}/sticky", post(modtools::sticky_comment))
        // votes
        .route("/api/vote", post(votes::cast_vote))
        // saves + hides
        .route("/api/save/{post_id}", post(saves::save_post).delete(saves::unsave_post))
        .route("/api/hide/{post_id}", post(saves::hide_post).delete(saves::unhide_post))
        .route("/api/saved", get(saves::list_saved))
        .route("/api/hidden", get(saves::list_hidden))
        // reports
        .route("/api/reports", post(reports::create_report))
        .route("/api/reports/subreddit/{name}", get(reports::list_reports_for_subreddit))
        .route("/api/reports/{report_id}/dismiss", post(reports::dismiss_report))
        // users
        .route("/api/users/{name}", get(users::get_by_name))
        .route("/api/users/{name}/posts", get(users::posts_by_user))
        .route("/api/users/{name}/comments", get(users::comments_by_user))
        // search
        .route("/api/search", get(search::search))
        // media uploads
        .route("/api/media/upload", post(media::upload_media))
        .route("/api/media/mine", get(media::list_my_media))
        // multireddits
        .route("/api/multireddits", get(multireddits::list_mine).post(multireddits::create))
        .route("/api/multireddits/{name}", get(multireddits::get_by_name).delete(multireddits::delete_multireddit))
        .route("/api/multireddits/{name}/posts", get(multireddits::list_posts))
        // notifications
        .route("/api/notifications", get(notifications::list_notifications))
        .route("/api/notifications/{notif_id}/read", post(notifications::mark_read))
        .route("/api/notifications/read-all", post(notifications::mark_all_read))
        // messages (DMs)
        .route("/api/messages", post(messages::start_conversation))
        .route("/api/messages/conversations", get(messages::list_conversations))
        .route("/api/messages/conversations/{conv_id}", get(messages::get_conversation).delete(messages::delete_conversation))
        .route("/api/messages/conversations/{conv_id}/send", post(messages::send_message))
        // account security
        .route("/api/account/settings", get(security::get_settings).patch(security::update_settings))
        .route("/api/account/profile", patch(security::update_profile))
        .route("/api/account/delete", post(security::delete_account))
        .route("/api/account/2fa/enable", post(security::enable_2fa))
        .route("/api/account/2fa/disable", post(security::disable_2fa))
        .route("/api/account/2fa/status", get(security::get_2fa_status))
        .route("/api/account/send-verification", post(security::send_email_verification))
        .route("/api/verify-email", get(security::verify_email))
        .route("/api/account/request-password-reset", post(security::request_password_reset))
        .route("/api/account/confirm-password-reset", post(security::confirm_password_reset))
        // static file serving for media uploads (mounted at /uploads)
        .nest_service("/uploads", ServeDir::new("/home/z/my-project/uploads"))
}

async fn health() -> &'static str {
    "freeddit-backend ok"
}
