//! Data models — match Reddit's public JSON API shape exactly so the
//! frontend can render responses the same way r.pan.do / RDX / Apollo do.
//!
//! Reddit uses "Thing" wrappers like:
//!   { "kind": "t3", "data": { ...post... } }
//! We replicate this shape for fidelity (and so any Reddit client library
//! can consume our responses unchanged).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThingKind {
    T1, // comment
    T2, // account
    T3, // link / post
    T4, // message
    T5, // subreddit
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thing<T> {
    pub kind: String,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing<T> {
    pub kind: String,
    pub data: ListingData<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingData<T> {
    pub after: Option<String>,
    pub before: Option<String>,
    pub children: Vec<Thing<T>>,
    pub dist: i64,
}

// ============ User ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,           // username
    pub display_name: String,
    pub avatar: Option<String>,
    pub banner_color: Option<String>,
    pub bio: Option<String>,
    pub post_karma: i64,
    pub comment_karma: i64,
    pub total_karma: i64,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
}

// ============ Subreddit ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subreddit {
    pub id: String,
    pub display_name: String,         // r/name
    pub display_name_prefixed: String,// "r/name"
    pub title: String,                // human title
    pub description: Option<String>,
    pub public_description: Option<String>,
    pub sidebar: Option<String>,
    pub icon: Option<String>,
    pub banner: Option<String>,
    pub primary_color: Option<String>,
    pub subscribers: i64,
    pub active_user_count: i64,
    pub sub_type: String,
    pub over18: bool,
    pub created_at: DateTime<Utc>,
}

// ============ Post ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub name: String,                 // fullname t3_xxxx
    pub subreddit_id: String,
    pub subreddit: String,             // r/name
    pub subreddit_name_prefixed: String,
    pub author: String,
    pub author_id: String,
    pub title: String,
    pub url: Option<String>,
    pub body: Option<String>,
    pub kind: String,                  // link|self|image
    pub flair_text: Option<String>,
    pub flair_color: Option<String>,
    pub thumbnail: Option<String>,
    pub score: i64,
    pub ups: i64,
    pub downs: i64,
    pub upvote_ratio: f64,
    pub num_comments: i64,
    pub views: i64,
    pub is_sticky: bool,
    pub is_locked: bool,
    pub is_removed: bool,
    pub is_spoiler: bool,
    pub over18: bool,
    pub is_self: bool,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub likes: Option<i32>,            // current user's vote (-1|0|1) — null if not logged in
}

// ============ Comment ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub name: String,
    pub post_id: String,
    pub subreddit_id: String,
    pub subreddit: String,
    pub author: String,
    pub author_id: String,
    pub parent_id: Option<String>,
    pub depth: i32,
    pub body: String,
    pub score: i64,
    pub ups: i64,
    pub downs: i64,
    pub upvote_ratio: f64,
    pub num_replies: i64,
    pub is_sticky: bool,
    pub is_locked: bool,
    pub is_op: bool,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub likes: Option<i32>,
    pub replies: Vec<Comment>,         // nested children — Reddit builds these client-side from listing
}

// ============ Auth ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub is_admin: bool,
    pub post_karma: i64,
    pub comment_karma: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub user: AuthUser,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VoteRequest {
    pub target_id: String,    // fullname
    pub direction: i32,        // +1, 0, -1
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSubredditRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub sidebar: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePostRequest {
    pub subreddit: String,
    pub title: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,        // defaults to "self" if body else "link"
    #[serde(default)]
    pub is_nsfw: bool,
    #[serde(default)]
    pub is_spoiler: bool,
    #[serde(default)]
    pub flair_text: Option<String>,
    #[serde(default)]
    pub flair_color: Option<String>,
    #[serde(default)]
    pub media_url: Option<String>,    // for image/video posts
    #[serde(default)]
    pub media_kind: Option<String>,   // 'image' | 'video' | 'gif'
    #[serde(default)]
    pub crosspost_parent_id: Option<String>,  // for crossposts
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCommentRequest {
    pub post_id: String,
    pub body: String,
    #[serde(default)]
    pub parent_id: Option<String>,
}

// ============ Sorting ============

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PostSort {
    Hot,
    Best,
    New,
    Top,
    Rising,
    Controversial,
}

impl Default for PostSort {
    fn default() -> Self { PostSort::Hot }
}

impl PostSort {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "best" => Self::Best,
            "new" => Self::New,
            "top" => Self::Top,
            "rising" => Self::Rising,
            "controversial" => Self::Controversial,
            _ => Self::Hot,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hot => "hot",
            Self::Best => "best",
            Self::New => "new",
            Self::Top => "top",
            Self::Rising => "rising",
            Self::Controversial => "controversial",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommentSort {
    Best,
    Top,
    New,
    Controversial,
    Old,
    Qa,
}

impl Default for CommentSort {
    fn default() -> Self { CommentSort::Best }
}

impl CommentSort {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "top" => Self::Top,
            "new" => Self::New,
            "controversial" => Self::Controversial,
            "old" => Self::Old,
            "qa" => Self::Qa,
            _ => Self::Best,
        }
    }
}
