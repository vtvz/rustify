use chrono::Duration;
use redis::AsyncCommands;

pub enum RateLimitOutput {
    Allowed,
    NeedToWait(Duration),
}

pub enum RateLimitAction {
    Analyze,
    Details,
    Dislike,
    Like,
    Recommendasion,
    Magic,
}

impl RateLimitAction {
    /// Provides the rate-limit configuration for this action.
    ///
    /// The returned tuple is (action_name, allowed_count, window_duration).
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Duration;
    /// use crate::services::rate_limit::RateLimitAction;
    ///
    /// let (name, limit, window) = RateLimitAction::Analyze.config();
    /// assert_eq!(name, "analyze");
    /// assert_eq!(limit, 1);
    /// assert_eq!(window, Duration::minutes(5));
    /// ```
    fn config(&self) -> (&str, u32, Duration) {
        match self {
            RateLimitAction::Analyze => ("analyze", 1, Duration::minutes(5)),
            RateLimitAction::Details => ("details", 1, Duration::seconds(15)),
            RateLimitAction::Dislike => ("dislike", 2, Duration::seconds(20)),
            RateLimitAction::Like => ("like", 1, Duration::seconds(10)),
            RateLimitAction::Recommendasion => ("recommendasion", 1, Duration::hours(1)),
            RateLimitAction::Magic => ("magic", 1, Duration::hours(6)),
        }
    }
}

pub struct RateLimitService {}

impl RateLimitService {
    /// Enforces a per-user, per-action rate limit using Redis.
    ///
    /// If the action's counter for the user is within the configured limit, the function records the attempt and returns `RateLimitOutput::Allowed`. If the limit is exceeded, it returns `RateLimitOutput::NeedToWait(duration)` where `duration` is the remaining time until the rate window resets.
    ///
    /// # Returns
    ///
    /// `RateLimitOutput::Allowed` if the user's action count is within the configured limit, `RateLimitOutput::NeedToWait(duration)` with the remaining window duration otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use chrono::Duration;
    /// # use crate::services::rate_limit::{RateLimitService, RateLimitAction, RateLimitOutput};
    /// # async fn example(mut conn: deadpool_redis::Connection) -> anyhow::Result<()> {
    /// let result = RateLimitService::enforce_limit(&mut conn, "user:123", RateLimitAction::Analyze).await?;
    /// match result {
    ///     RateLimitOutput::Allowed => println!("Allowed"),
    ///     RateLimitOutput::NeedToWait(d) => println!("Wait for {}s", d.num_seconds()),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn enforce_limit(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
        action: RateLimitAction,
    ) -> anyhow::Result<RateLimitOutput> {
        let (action_name, limit, window_duration) = action.config();

        let key = format!("rustify:ratelimit:{user_id}:{action_name}");

        let ttl_seconds = window_duration.num_seconds();

        let count: u32 = redis_conn.incr(&key, 1).await?;

        if count == 1 {
            let _: () = redis_conn.expire(&key, ttl_seconds).await?;
        }

        if count > limit {
            let remaining_ttl: i64 = redis_conn.ttl(&key).await?;

            let wait_duration = if remaining_ttl > 0 {
                Duration::seconds(remaining_ttl)
            } else {
                Duration::zero()
            };

            return Ok(RateLimitOutput::NeedToWait(wait_duration));
        }

        Ok(RateLimitOutput::Allowed)
    }
}