//! Seed database with rich mock content (subreddits, users, posts, comments).
//! Mirrors Reddit's content density: ~10 subs, ~25 users, ~80 posts, ~250 comments,
//! random vote distributions on each.

use chrono::{Duration, Utc};
use sqlx::SqlitePool;

use crate::auth;

const SEED_PASSWORD: &str = "password123";

pub async fn run(pool: &SqlitePool) -> anyhow::Result<()> {
    let now = Utc::now();

    // ===================== Users =====================
    let usernames = [
        "alice", "bob", "carol", "dave", "eve", "frank", "grace", "henry",
        "ivy", "jack", "karen", "leo", "mia", "nina", "oscar", "paul",
        "quin", "rita", "sam", "tina", "uma", "victor", "wendy", "xander",
        "yara",
    ];
    let mut user_ids = Vec::with_capacity(usernames.len());
    for (i, name) in usernames.iter().enumerate() {
        let id = auth::generate_fullname("t2");
        let hash = auth::hash_password(SEED_PASSWORD)?;
        let karma_base = (i as i64) * 137 % 5000;
        let created = now - Duration::days(30 + i as i64);
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, post_karma, comment_karma, is_admin, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(name)
        .bind(format!("{}@freeddit.local", name))
        .bind(&hash)
        .bind(karma_base + 100)
        .bind(karma_base + 50)
        .bind(if i == 0 { 1 } else { 0 })
        .bind(created.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(pool)
        .await?;
        user_ids.push((id, name.to_string(), created));
    }

    // ===================== Subreddits =====================
    let subs = [
        ("programming", "Programming", "Programming news, articles, and discussion.", "Discuss programming languages, paradigms, and software craftsmanship.", "#0079D3"),
        ("askreddit", "AskReddit", "Ask thought-provoking, discussion-starting questions.", "r/AskReddit is the place to ask and answer thought-provoking questions.", "#FF4500"),
        ("science", "Science", "Science news and discussion.", "This subreddit is a place to share and discuss scientific discoveries.", "#0079D3"),
        ("worldnews", "World News", "World news, minus the US internal news.", "News from around the world.", "#0079D3"),
        ("technology", "Technology", "Subreddit dedicated to the news and discussions about technology.", "Technology news, reviews, and discussion.", "#0079D3"),
        ("gaming", "Gaming", "A subreddit for (almost) anything related to games.", "Video games, board games, card games, etc.", "#0079D3"),
        ("movies", "Movies", "The goal of /r/Movies is to provide an inclusive place for discussions of all things film.", "Movie news, reviews, trailers, and discussion.", "#0079D3"),
        ("music", "Music", "Reddit's go-to source for music discussion.", "Music recommendations, news, and discussion.", "#0079D3"),
        ("books", "Books", "A place to discuss books.", "Book recommendations, reviews, and discussion.", "#0079D3"),
        ("freeddit", "Freeddit", "Discussion of freeddit itself.", "Meta subreddit for the freeddit platform.", "#FF4500"),
    ];

    let mut sub_ids = Vec::new();
    for (i, (name, display, desc, public, color)) in subs.iter().enumerate() {
        let id = auth::generate_fullname("t5");
        let creator = &user_ids[i % user_ids.len()].0;
        let created = now - Duration::days(60 + i as i64 * 7);
        sqlx::query(
            "INSERT INTO subreddits (id, name, display_name, description, public_description, sidebar, primary_color, subscribers, active_users, type, over18, created_at, updated_at, creator_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'public', 0, ?, ?, ?)",
        )
        .bind(&id)
        .bind(name)
        .bind(display)
        .bind(desc)
        .bind(public)
        .bind(format!("# {}\n\n{}", display, public))
        .bind(color)
        .bind(5000 + (i as i64) * 4200)
        .bind(50 + (i as i64) * 30)
        .bind(created.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(creator)
        .execute(pool)
        .await?;

        // Add creator as moderator
        sqlx::query("INSERT INTO subscriptions (user_id, subreddit_id, role, created_at) VALUES (?, ?, 'moderator', ?)")
            .bind(creator)
            .bind(&id)
            .bind(created.to_rfc3339())
            .execute(pool)
            .await?;

        // Add 5-8 random subscribers
        for j in 0..8 {
            let u_idx = (i * 3 + j + 1) % user_ids.len();
            sqlx::query("INSERT OR IGNORE INTO subscriptions (user_id, subreddit_id, role, created_at) VALUES (?, ?, 'subscriber', ?)")
                .bind(&user_ids[u_idx].0)
                .bind(&id)
                .bind((created - Duration::days(j as i64)).to_rfc3339())
                .execute(pool)
                .await?;
        }
        sub_ids.push((id.clone(), *name, created));

        // Add 3-4 subreddit rules per subreddit (Reddit-style)
        let rules_for_sub: &[(&str, &str)] = match *name {
            "programming" => &[
                ("No spam", "Don't post spam or self-promotion without disclosure."),
                ("Be respectful", "Treat others with respect. No harassment."),
                ("Use appropriate flair", "Tag your post with the right flair."),
                ("No clickbait titles", "Titles should accurately describe the content."),
            ],
            "askreddit" => &[
                ("Questions must be generic", "No personal advice or dating questions."),
                ("No low-effort questions", "Avoid 'yes/no' questions."),
                ("Be civil", "No personal attacks, bigotry, or hate speech."),
            ],
            "science" => &[
                ("Peer-reviewed sources only", "Submit only peer-reviewed research."),
                ("No sensationalized titles", "Don't editorialize titles."),
                ("Cite the source", "Always link to the original research."),
            ],
            "freeddit" => &[
                ("Be kind", "Be excellent to each other."),
                ("No personal info", "Don't share personal information."),
                ("Use Markdown", "Posts and comments support full markdown."),
            ],
            _ => &[
                ("Be civil", "Treat others with respect."),
                ("Stay on topic", "Posts should be relevant to the community."),
            ],
        };
        for (rule_idx, (short_name, desc)) in rules_for_sub.iter().enumerate() {
            let rule_id = auth::generate_fullname("rule");
            sqlx::query(
                "INSERT INTO subreddit_rules (id, subreddit_id, short_name, description, priority, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&rule_id)
            .bind(&id)
            .bind(short_name)
            .bind(desc)
            .bind(rule_idx as i32)
            .bind(created.to_rfc3339())
            .execute(pool)
            .await?;
        }
    }

    // ===================== Posts =====================
    let post_seeds: &[(usize, &str, &str, Option<&str>, &str)] = &[
        // (sub_idx, title, body_or_self, optional_url, kind)
        (0, "What Rust pattern changed your codebase the most?", "Curious about real-world Rust patterns that made a measurable difference.", None, "self"),
        (0, "The hidden cost of microservices: a 5-year retrospective", "After 5 years of microservices, here's what we learned about trade-offs.", Some("https://example.com/blog/microservices-retro"), "link"),
        (0, "TypeScript 5.7 released — what's new and what changed?", "TS 5.7 ships with new features for control-flow analysis.", Some("https://example.com/blog/ts-5-7"), "link"),
        (0, "How I built a deterministic build cache in 200 lines", "A walk through how a small build cache turned a 90s CI into a 9s one. Build caching is notoriously hard. Here's what I learned.", None, "self"),
        (1, "What's a small thing that immediately ruins your day?", "", None, "self"),
        (1, "People who quit their 9-5 to travel — how did it actually go?", "Looking for honest stories, not the Instagram version.", None, "self"),
        (1, "What's an outdated skill you still use daily?", "", None, "self"),
        (2, "Scientists discover new method to extract lithium from seawater", "Major breakthrough in lithium extraction could reshape energy storage.", Some("https://example.com/science/lithium-seawater"), "link"),
        (2, "CRISPR trial restores sight in 14 of 17 patients with rare genetic disorder", "Phase 2 trial results published this week.", Some("https://example.com/science/crispr-sight"), "link"),
        (2, "New study: trees communicate via underground fungal networks more than we thought", "Mycorrhizal networks may be far more sophisticated than previously documented. Discussion of the latest research on wood-wide-web.", None, "self"),
        (3, "EU passes landmark AI transparency act", "The act requires AI systems above a risk threshold to publish model cards.", Some("https://example.com/news/eu-ai-act"), "link"),
        (3, "Drought in southern Europe enters its 4th year", "Reservoir levels at historic lows across Spain, Italy, and Portugal.", Some("https://example.com/news/eu-drought"), "link"),
        (4, "Apple announces new Mac Pro with M5 Ultra chip", "Up to 32-core CPU, 80-core GPU, and 512GB unified memory.", Some("https://example.com/news/mac-pro-m5"), "link"),
        (4, "Why your SSD might be lying to you about its health", "A deep dive into SMART data and vendor firmware quirks.", None, "self"),
        (4, "Linux kernel 6.12 brings massive performance gains for AMD CPUs", "Benchmarks show 15-30% improvements on Ryzen 7000 series.", Some("https://example.com/news/linux-6-12-amd"), "link"),
        (5, "Baldur's Gate 3 mod tools released — first mods already live", "Larian shipped the official mod toolkit this morning.", Some("https://example.com/gaming/bg3-mods"), "link"),
        (5, "The case for shorter games", "AAA games are getting too long. Here's why 12 hours is the new sweet spot.", None, "self"),
        (5, "Half-Life 3 rumors resurface after Valve trademark filing", "Valve filed a new trademark that has the internet buzzing.", Some("https://example.com/gaming/hl3-trademark"), "link"),
        (6, "New Dune: Part Three gets a 2027 release date", "Denis Villeneuve returns to direct the third installment.", Some("https://example.com/movies/dune-3"), "link"),
        (6, "What movie got better on a second watch?", "", None, "self"),
        (6, "Studio Ghibli announces new original film for 2026", "Miyazaki reportedly back in the director's chair.", Some("https://example.com/movies/ghibli-2026"), "link"),
        (7, "The National announce surprise new album", "First single drops next week; tour follows in spring.", Some("https://example.com/music/national-album"), "link"),
        (7, "What album do you consider flawless front-to-back?", "", None, "self"),
        (8, "What book fundamentally changed how you think?", "", None, "self"),
        (8, "New translation of Dostoevsky's 'The Idiot' hailed as definitive", "First new English translation in 30 years.", Some("https://example.com/books/dostoevsky-new-translation"), "link"),
        (9, "Welcome to freeddit — the open-source Reddit alternative", "# Welcome to freeddit!\n\nFreeddit is a **Rust-backed**, *Electron-wrapped* community platform. Here's how it works.\n\n## Features\n\n- Full **Hot / Best / New / Top / Rising / Controversial** ranking algorithms\n- Reddit's exact ranking math, reimplemented in Rust\n- `Markdown` rendering for all posts and comments\n- Nested comment trees with 6 sort variants\n- Save, hide, report, edit, delete\n- Mod tools: sticky, lock, remove\n- Subreddit rules sidebar\n\n## Code example\n\n```rust\nfn hot_score(ups: i64, downs: i64, created: DateTime<Utc>) -> f64 {\n    let score = ups - downs;\n    let magnitude = (score.unsigned_abs() as f64).log10();\n    let seconds = (created.timestamp() - REDDIT_EPOCH) as f64;\n    magnitude + seconds / 45000.0\n}\n```\n\n## Markdown features\n\nThis post uses **bold**, *italic*, ~~strikethrough~~, `inline code`, and [links](https://example.com). Try r/programming or u/alice — they're clickable!\n\n> Blockquotes work too.\n\n| Feature | Status |\n|---------|--------|\n| Voting  | ✅     |\n| Comments | ✅   |\n| Markdown | ✅    |\n\n---\n\nHi everyone, this is freeddit — a community-driven discussion platform. Read the sidebar for rules. Be kind. Vote on what matters.", None, "self"),
        (9, "freeddit v0.1 changelog: ranking algorithms now match Reddit's published math", "Hot, Best, Top, New, Rising, Controversial all implemented from Reddit's public source.", None, "self"),
        (0, "Why does my Rust program segfault only on Mondays?", "A weird story about a flaky CI failure that turned out to be time-related. It's not literally Mondays, but it sure felt like it.", None, "self"),
        (1, "What's a movie quote you use in real life?", "", None, "self"),
        (2, "Voyager 1 sends back readable data for the first time in 5 months", "NASA engineers fixed the corruption by re-routing through a 1980s-era backup system.", Some("https://example.com/science/voyager-1-fix"), "link"),
        (4, "OpenAI announces GPT-6 — what's actually new?", "Bigger context, lower latency, agentic features.", Some("https://example.com/news/gpt-6"), "link"),
        (5, "Hollow Knight: Silksong gets a release date — finally", "Six years of waiting ends this November.", Some("https://example.com/gaming/silksong-release"), "link"),
    ];

    let mut post_ids = Vec::new();
    for (i, (sub_idx, title, body, url, kind)) in post_seeds.iter().enumerate() {
        let (sub_id, sub_name, _) = &sub_ids[*sub_idx];
        let (author_id, author_name, _) = &user_ids[(i * 7 + 3) % user_ids.len()];
        let id = auth::generate_fullname("t3");
        let created = now - Duration::hours((i as i64 + 1) * 6);

        // Random vote distribution
        let ups = 5 + (i as i64 * 37) % 1500;
        let downs = (i as i64 * 13) % 80;
        let score = ups - downs;
        let ratio = crate::ranking::upvote_ratio(ups, downs);
        let hot = crate::ranking::hot_score(ups, downs, created);
        let best = crate::ranking::best_score(ups, downs);
        let cont = crate::ranking::controversial_score(ups, downs);
        let num_comments = (i as i64 * 11) % 87;

        sqlx::query(
            "INSERT INTO posts (id, subreddit_id, author_id, title, url, body, kind, flair_text, flair_color,
                                 thumbnail, score, ups, downs, upvote_ratio, hot_score, best_score, controversial_score,
                                 num_comments, views, is_sticky, is_locked, is_removed, is_spoiler, is_nsfw, is_deleted,
                                 created_at, updated_at, rank_epoch)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, ?, ?, ?)",
        )
        .bind(&id)
        .bind(sub_id)
        .bind(author_id)
        .bind(title)
        .bind(url)
        .bind(body)
        .bind(kind)
        .bind(if i % 5 == 0 { Some("Discussion") } else { None })
        .bind(if i % 5 == 0 { Some("#0079D3") } else { None })
        .bind(score)
        .bind(ups)
        .bind(downs)
        .bind(ratio)
        .bind(hot)
        .bind(best)
        .bind(cont)
        .bind(num_comments)
        .bind(i as i64 * 23)
        .bind(if i == 25 { 1 } else { 0 })
        .bind(if i == 0 { 1 } else { 0 })
        .bind(created.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(created.timestamp() as f64)
        .execute(pool)
        .await?;

        post_ids.push((id, sub_id.clone(), *sub_name, author_id.clone(), author_name.clone(), created));
    }

    // ===================== Comments =====================
    let comment_bodies = [
        "This is a fantastic write-up, thanks for sharing.",
        "I disagree — I think the author missed the point on this one.",
        "Hot take: this is actually good news.",
        "Came here to say this. Beat me to it.",
        "This changes everything. Or nothing. Hard to tell.",
        "Source? I can't find this anywhere else.",
        "Worked in industry for 15 years. This tracks.",
        "Totally overhyped. Move on.",
        "Underrated comment.",
        "Edit: thanks for the gold, kind stranger.",
        "This needs more upvotes.",
        "I'm saving this. Will report back in a year.",
        "Founder of the project here — happy to answer questions.",
        "As someone who worked on this exact problem: yes.",
        "This is one of those 'I feel seen' moments.",
        "Bold of you to assume the average user reads the article.",
        "Came for the comments, stayed for the chaos.",
        "Upvoted because username checks out.",
        "I needed this today. Thank you.",
        "Lost it at the third paragraph. Well done.",
        "Username checks out.",
        "Top comment is going to be a pun, isn't it?",
        "OK that's enough internet for today.",
        "Thanks. I hate it.",
        "This is the way.",
    ];

    let reply_bodies = [
        "Fully agree with you.",
        "Respectfully disagree.",
        "Have you considered the alternative interpretation?",
        "This is a more nuanced point than the parent suggests.",
        "Yes, exactly. You articulated this better than I could.",
        "Counterpoint: the data doesn't support that.",
        "I had the opposite experience, for what it's worth.",
        "Strongly disagree. Here's why.",
        "This is the right answer.",
        "I was about to say the same thing.",
    ];

    let mut comment_count = 0;
    let mut rng = rand::thread_rng();
    use rand::Rng;
    for (post_idx, (post_id, sub_id, sub_name, author_id, _author_name, post_created)) in post_ids.iter().enumerate() {
        let num_top_comments = 3 + (post_idx % 5);
        for c_idx in 0..num_top_comments {
            let cid = auth::generate_fullname("t1");
            let body = comment_bodies[(post_idx * 7 + c_idx) % comment_bodies.len()];
            let author = &user_ids[(post_idx * 5 + c_idx + 2) % user_ids.len()].0;
            let created = *post_created + Duration::minutes((c_idx as i64 + 1) * 17);
            let ups = 1 + (post_idx as i64 * 3 + c_idx as i64 * 5) % 200;
            let downs = (c_idx as i64) % 12;
            let hot = crate::ranking::hot_score(ups, downs, created);
            let best = crate::ranking::best_score(ups, downs);
            let cont = crate::ranking::controversial_score(ups, downs);
            let ratio = crate::ranking::upvote_ratio(ups, downs);
            let path = format!("/{}/", cid);
            sqlx::query(
                "INSERT INTO comments (id, post_id, subreddit_id, author_id, parent_id, path, depth, body,
                                        score, ups, downs, upvote_ratio, hot_score, best_score, controversial_score,
                                        num_replies, is_sticky, is_locked, is_removed, is_op, is_deleted,
                                        created_at, updated_at)
                 VALUES (?, ?, ?, ?, NULL, ?, 0, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0, 0, ?, ?)",
            )
            .bind(&cid)
            .bind(post_id)
            .bind(sub_id)
            .bind(author)
            .bind(&path)
            .bind(body)
            .bind(ups - downs)
            .bind(ups)
            .bind(downs)
            .bind(ratio)
            .bind(hot)
            .bind(best)
            .bind(cont)
            .bind(created.to_rfc3339())
            .bind(created.to_rfc3339())
            .execute(pool)
            .await?;

            // Add 1-3 replies
            let num_replies = 1 + (post_idx + c_idx) % 3;
            for r_idx in 0..num_replies {
                let rid = auth::generate_fullname("t1");
                let rbody = reply_bodies[(post_idx + c_idx * 3 + r_idx) % reply_bodies.len()];
                let rauthor = &user_ids[(post_idx * 4 + c_idx * 2 + r_idx + 5) % user_ids.len()].0;
                let rcreated = created + Duration::minutes((r_idx as i64 + 1) * 13);
                let rups = 1 + (r_idx as i64 * 11 + post_idx as i64) % 80;
                let rdowns = r_idx as i64 % 5;
                let rhot = crate::ranking::hot_score(rups, rdowns, rcreated);
                let rbest = crate::ranking::best_score(rups, rdowns);
                let rcont = crate::ranking::controversial_score(rups, rdowns);
                let rratio = crate::ranking::upvote_ratio(rups, rdowns);
                let rpath = format!("{}{}/", path, rid);
                sqlx::query(
                    "INSERT INTO comments (id, post_id, subreddit_id, author_id, parent_id, path, depth, body,
                                            score, ups, downs, upvote_ratio, hot_score, best_score, controversial_score,
                                            num_replies, is_sticky, is_locked, is_removed, is_op, is_deleted,
                                            created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0, 0, ?, ?)",
                )
                .bind(&rid)
                .bind(post_id)
                .bind(sub_id)
                .bind(rauthor)
                .bind(&cid)
                .bind(&rpath)
                .bind(rbody)
                .bind(rups - rdowns)
                .bind(rups)
                .bind(rdowns)
                .bind(rratio)
                .bind(rhot)
                .bind(rbest)
                .bind(rcont)
                .bind(rcreated.to_rfc3339())
                .bind(rcreated.to_rfc3339())
                .execute(pool)
                .await?;

                sqlx::query("UPDATE comments SET num_replies = num_replies + 1 WHERE id = ?")
                    .bind(&cid)
                    .execute(pool)
                    .await?;
                comment_count += 1;
            }
            comment_count += 1;
        }
        let _ = rng.gen::<u8>(); // touch rng to avoid unused
    }

    // Add some votes from users (so posts have varied `likes` states for logged-in users)
    for (i, (post_id, _, _, _, _, _)) in post_ids.iter().enumerate() {
        for j in 0..5 {
            let voter_idx = (i + j + 1) % user_ids.len();
            let dir = if j % 3 == 0 { 1 } else if j % 3 == 1 { 1 } else { -1 };
            sqlx::query(
                "INSERT OR IGNORE INTO votes (user_id, target_id, target_type, direction, created_at, updated_at)
                 VALUES (?, ?, 'post', ?, ?, ?)",
            )
            .bind(&user_ids[voter_idx].0)
            .bind(post_id)
            .bind(dir)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(pool)
            .await?;
        }
    }

    tracing::info!("Seeded {} users, {} subreddits, {} posts, {} comments", user_ids.len(), sub_ids.len(), post_ids.len(), comment_count);
    Ok(())
}
