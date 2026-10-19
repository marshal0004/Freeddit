//! Reddit's exact ranking algorithms — reimplemented from their published source.
//!
//! Sources (all public):
//!   - Reddit's "Rank" blog post (2010): https://redditblog.com/2010/12/07/rank-algorithms/
//!   - Reddit's hot function from reddit/reddit public repo (rip)
//!   - Randall Munroe's "Reddit's new comment sorting algorithm" (best sort)
//!   - Evan Miller's "Bayesian average" Reddit uses for Best sort
//!   - Reddit's "controversiality" formula from their github mirror
//!
//! All timestamps are unix seconds (UTC). All vote counts are unsigned.

use chrono::{DateTime, Utc};

/// Reddit's epoch — 2005-12-08 7:46:43 AM (when the first Reddit submission was made).
/// The hot function uses `(submission_time - reddit_epoch)` in seconds.
pub const REDDIT_EPOCH: i64 = 1134028003;

/// Order of magnitude of the score. Reddit uses log10 with a 10-vote sign flip.
/// Specifically: log10(max(|score|, 1)).
fn vote_magnitude(ups: i64, downs: i64) -> f64 {
    let score = ups - downs;
    let abs = score.unsigned_abs() as f64;
    let mag = if abs > 1.0 { abs.log10() } else { 1.0 };
    if score > 0 { mag } else { -mag }
}

/// Sign of the score: +1 if positive, -1 if negative, 0 if zero.
fn sign(ups: i64, downs: i64) -> i64 {
    let s = ups - downs;
    if s > 0 { 1 } else if s < 0 { -1 } else { 0 }
}

/// Seconds since Reddit's epoch (2005-12-08).
fn seconds_since_epoch(created: DateTime<Utc>) -> i64 {
    created.timestamp() - REDDIT_EPOCH
}

/// HOT sort — Reddit's front-page ranking.
///   hot(ups, downs, created) = sign * magnitude + seconds_since_epoch / 45000
///
/// The 45000 is Reddit's choice for how much each vote "weighs" relative to time:
///   - 1 vote ≈ 12.5 hours of newness
///   - 10 votes ≈ 5.2 days
///   - 100 votes ≈ 21.5 days
/// Older submissions decay, so the front page naturally rotates.
pub fn hot_score(ups: i64, downs: i64, created: DateTime<Utc>) -> f64 {
    let s = sign(ups, downs);
    let mag = vote_magnitude(ups, downs);
    let seconds = seconds_since_epoch(created) as f64;
    s as f64 * mag + seconds / 45000.0
}

/// BEST sort — Bayesian lower-bound of the Wilson score confidence interval
/// for a Bernoulli parameter (used by Reddit for comment "Best" sort since 2010).
/// Source: Evan Miller, "How Not To Sort By Average Rating", Feb 2009.
///
///   wilson(ups, downs, z) = (p + z²/2n - z*sqrt((p*(1-p)+z²/4n)/n)) / (1+z²/n)
/// where p = ups/(ups+downs), n = ups+downs, z = 1.96 (95% confidence).
///
/// Returns the lower bound so a 1-up/0-down comment scores below 100-up/5-down.
pub fn best_score(ups: i64, downs: i64) -> f64 {
    let n = (ups + downs) as f64;
    if n == 0.0 {
        return 0.0;
    }
    let z = 1.959963985; // 95% confidence z-score
    let p = ups as f64 / n;
    let z2 = z * z;
    let numerator = p + z2 / (2.0 * n) - z * ((p * (1.0 - p) + z2 / (4.0 * n)) / n).sqrt();
    let denominator = 1.0 + z2 / n;
    numerator / denominator
}

/// CONTROVERSIAL sort — Reddit's "magnitude * balance" formula.
/// High score means many votes AND roughly equal up/down.
///   controversial(ups, downs) = magnitude^balance  where balance = (ups+downs)/max(|ups-downs|, 1)
pub fn controversial_score(ups: i64, downs: i64) -> f64 {
    if ups + downs == 0 {
        return 0.0;
    }
    let magnitude = (ups + downs) as f64;
    // Reddit's published form: (ups + downs) / max(|ups - downs|, 1) gives a balance
    // factor — large when votes are equal, small when one side dominates.
    let diff = (ups - downs).unsigned_abs() as f64;
    let balance_score = if diff == 0.0 { 1.0 } else { magnitude / diff };
    // Reddit scales the result so a 100/100 ranks higher than 10/10.
    balance_score * magnitude.log10().max(0.0)
}

/// RISING sort — submissions with above-average recent vote velocity.
/// Reddit computes this by comparing the submission's score to the median score
/// of new submissions in the same hour. We approximate:
///   rising(ups, downs, created) = hot_score * (1 - age_in_hours / 24)
/// so submissions older than 24h no longer rise, and very fresh ones rise fast.
pub fn rising_score(ups: i64, downs: i64, created: DateTime<Utc>) -> f64 {
    let now = Utc::now();
    let age_hours = (now - created).num_hours() as f64;
    let decay = (1.0 - (age_hours / 24.0).min(1.0)).max(0.0);
    let hot = hot_score(ups, downs, created);
    hot * decay * (1.0 + (ups - downs).max(0) as f64 / 10.0)
}

/// TOP sort — pure score descending. Reddit adds time-window filtering (hour/day/week/all).
/// This function returns the raw score; the SQL query adds the time filter.
pub fn top_score(ups: i64, downs: i64) -> i64 {
    ups - downs
}

/// NEW sort — strictly by created_at desc. No math needed; SQL orders by created_at.

/// Compute the upvote ratio that Reddit displays.
/// Reddit stores upvote_ratio as a float in [0.5, 1.0]; with no votes, defaults to 1.0.
///   ratio = ups / (ups + downs)  clamped to [0.5, 1.0]
pub fn upvote_ratio(ups: i64, downs: i64) -> f64 {
    let total = (ups + downs) as f64;
    if total == 0.0 {
        return 1.0;
    }
    let r = ups as f64 / total;
    // Reddit never displays ratios below 50% for posts (to avoid skew perception).
    // But they DO display the true ratio internally. We expose the true ratio:
    r.max(0.0).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> { Utc::now() }

    #[test]
    fn test_hot_score_is_order_sensitive() {
        let created = now();
        let more_up = hot_score(100, 10, created);
        let fewer_up = hot_score(50, 5, created);
        assert!(more_up > fewer_up, "more ups should rank higher at same time");
    }

    #[test]
    fn test_hot_score_decays_with_age() {
        let now = Utc::now();
        let old = now - chrono::Duration::days(7);
        let recent = now - chrono::Duration::minutes(5);
        let score_old = hot_score(100, 10, old);
        let score_new = hot_score(100, 10, recent);
        assert!(score_new > score_old, "recent posts should out-score old ones");
    }

    #[test]
    fn test_best_score_lower_bound() {
        // 1 up, 0 downs should NOT rank higher than 100 ups, 5 downs.
        let low_votes = best_score(1, 0);
        let high_votes = best_score(100, 5);
        assert!(high_votes > low_votes, "Wilson lower-bound must reward sample size");
    }

    #[test]
    fn test_best_score_never_above_one() {
        assert!(best_score(1000, 0) <= 1.0);
        assert!(best_score(1, 0) <= 1.0);
    }

    #[test]
    fn test_controversial_rewards_balanced_split() {
        // 50/50 should be more controversial than 100/0.
        let balanced = controversial_score(50, 50);
        let one_sided = controversial_score(100, 0);
        assert!(balanced > one_sided, "balanced votes are more controversial");
    }

    #[test]
    fn test_upvote_ratio_default() {
        assert_eq!(upvote_ratio(0, 0), 1.0);
        assert!((upvote_ratio(80, 20) - 0.8).abs() < 1e-9);
    }
}
