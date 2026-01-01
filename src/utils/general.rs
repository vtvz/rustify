use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{NaiveDateTime, SubsecRound, Utc};
use tokio::sync::{RwLock, broadcast};
use tokio::time::Instant;

pub static TICK_STATUS: LazyLock<RwLock<HashMap<(&'static str, Duration), Instant>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Debug)]
pub struct TickHealthStatus {
    pub lagging: Vec<&'static str>,
    pub unhealthy: Vec<&'static str>,
    pub total: usize,
}

pub async fn tick_health() -> TickHealthStatus {
    let hash = TICK_STATUS.read().await;

    let now = Instant::now();

    let mut lagging = vec![];
    let mut unhealthy = vec![];
    for ((module, period), last) in hash.iter() {
        let diff = now.duration_since(*last);

        if diff >= (*period * 3) {
            unhealthy.push(*module);
        } else if diff >= (*period * 2) {
            lagging.push(*module);
        }
    }

    TickHealthStatus {
        unhealthy,
        lagging,
        total: hash.len(),
    }
}

macro_rules! tick {
    ($period:expr, $code:block) => {
        let period = $period;
        let instance_key = concat!(module_path!(), ":", line!());
        let health_check_key = (instance_key, period);
        let mut interval = ::tokio::time::interval(period);
        let mut iteration: u64 = 0;
        loop {
            use ::tracing::Instrument;

            ::tokio::select! {
                _ = interval.tick() => {},
                _ = $crate::utils::ctrl_c() => {
                    ::tracing::debug!(tick_iteration = iteration, instance = instance_key, "Received terminate signal. Stop processing");
                    $crate::utils::TICK_STATUS
                        .write()
                        .await
                        .remove(&health_check_key);
                    break;
                },
            }

            // interval.tick() can lag behind
            let start = ::tokio::time::Instant::now();

            {
                $crate::utils::TICK_STATUS
                    .write()
                    .await
                    .insert(health_check_key, start);
            }

            async { $code }
                .instrument(tracing::info_span!("tick", tick_iteration = iteration, instance = instance_key))
                .await;

            let diff = start.elapsed();

            if (diff > period) {
                ::tracing::warn!(
                    tick_iteration = iteration,
                    diff = (diff - period).as_secs_f64(),
                    unit = "s",
                    instance = instance_key,
                    "Task took a bit more time than allowed"
                );
            }

            iteration += 1;
        }
    };
}

pub(crate) use tick;

static KILL: LazyLock<(broadcast::Sender<()>, broadcast::Receiver<()>)> =
    LazyLock::new(|| broadcast::channel(1));

static KILLED: AtomicBool = AtomicBool::new(false);

pub async fn listen_for_ctrl_c() {
    tokio::signal::ctrl_c().await.ok();

    KILL.0.send(()).ok();

    KILLED.store(true, Ordering::Relaxed);
}

pub async fn ctrl_c() {
    if KILLED.load(Ordering::Relaxed) {
        return;
    }

    KILL.0.subscribe().recv().await.ok();
}

pub struct Clock;

impl Clock {
    #[must_use]
    pub fn now() -> NaiveDateTime {
        Utc::now().naive_local().round_subsecs(0)
    }
}

pub trait StringUtils {
    fn chars_len(&self) -> usize;

    fn chars_crop(&self, len: usize) -> String;
}

impl<T> StringUtils for T
where
    T: AsRef<str>,
{
    fn chars_len(&self) -> usize {
        self.as_ref().chars().count()
    }

    fn chars_crop(&self, len: usize) -> String {
        self.as_ref().chars().take(len).collect()
    }
}

pub trait DurationPrettyFormat {
    fn pretty_format(&self) -> String;
}

impl DurationPrettyFormat for chrono::Duration {
    fn pretty_format(&self) -> String {
        let total_seconds = self.num_seconds();
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let centiseconds = (self.num_milliseconds() % 1000) / 10;

        let subsec_suffix = if centiseconds > 0 {
            format!(".{centiseconds:02}")
        } else {
            String::new()
        };

        if days > 0 {
            format!("{days:02}:{hours:02}:{minutes:02}:{seconds:02}{subsec_suffix}")
        } else if hours > 0 {
            format!("{hours:02}:{minutes:02}:{seconds:02}{subsec_suffix}")
        } else {
            format!("{minutes:02}:{seconds:02}{subsec_suffix}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for StringUtils::chars_len()
    #[test]
    fn test_chars_len_ascii() {
        assert_eq!("hello".chars_len(), 5);
    }

    #[test]
    fn test_chars_len_empty() {
        assert_eq!("".chars_len(), 0);
    }

    #[test]
    fn test_chars_len_unicode_emoji() {
        // Emoji are single characters in Unicode
        assert_eq!("😀".chars_len(), 1);
        assert_eq!("😀😁😂".chars_len(), 3);
    }

    #[test]
    fn test_chars_len_unicode_chinese() {
        assert_eq!("你好世界".chars_len(), 4);
    }

    #[test]
    fn test_chars_len_unicode_mixed() {
        assert_eq!("Hello 世界 😀".chars_len(), 10);
    }

    #[test]
    fn test_chars_len_unicode_arabic() {
        assert_eq!("مرحبا".chars_len(), 5);
    }

    // Tests for StringUtils::chars_crop()
    #[test]
    fn test_chars_crop_ascii() {
        assert_eq!("hello world".chars_crop(5), "hello");
    }

    #[test]
    fn test_chars_crop_empty() {
        assert_eq!("".chars_crop(5), "");
    }

    #[test]
    fn test_chars_crop_zero() {
        assert_eq!("hello".chars_crop(0), "");
    }

    #[test]
    fn test_chars_crop_longer_than_string() {
        assert_eq!("hi".chars_crop(10), "hi");
    }

    #[test]
    fn test_chars_crop_unicode_emoji() {
        // Should crop by characters, not bytes
        assert_eq!("😀😁😂😃".chars_crop(2), "😀😁");
    }

    #[test]
    fn test_chars_crop_unicode_chinese() {
        assert_eq!("你好世界".chars_crop(2), "你好");
    }

    #[test]
    fn test_chars_crop_unicode_mixed() {
        assert_eq!("Hello 世界 😀".chars_crop(8), "Hello 世界");
    }

    #[test]
    fn test_chars_crop_exact_length() {
        let s = "hello";
        assert_eq!(s.chars_crop(5), "hello");
    }

    #[test]
    fn test_chars_crop_preserves_multibyte() {
        // Ensure we don't break multibyte characters
        let text = "café";
        let cropped = text.chars_crop(3);
        assert_eq!(cropped, "caf");
        // Verify it's valid UTF-8
        assert!(cropped.is_char_boundary(cropped.len()));
    }

    // Tests for Clock::now()
    #[test]
    fn test_clock_now_returns_valid_datetime() {
        let now = Clock::now();
        // Should return a valid datetime (doesn't panic)
        // Year should be reasonable (between 2020 and 2100)
        assert!(now.and_utc().timestamp() > 1_600_000_000);
    }

    #[test]
    fn test_clock_now_subsecond_precision() {
        let now = Clock::now();
        // Nanoseconds should be rounded to 0 (round_subsecs(0))
        assert_eq!(now.and_utc().timestamp_subsec_nanos(), 0);
    }

    #[test]
    fn test_clock_now_multiple_calls() {
        let time1 = Clock::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let time2 = Clock::now();
        // time2 should be >= time1
        assert!(time2 >= time1);
    }

    // Tests for DurationFormat::pretty_format()
    #[test]
    fn test_duration_format_less_than_minute() {
        // Test duration < 1 minute (should show ss.nn)
        let duration = chrono::Duration::seconds(45) + chrono::Duration::milliseconds(500);
        assert_eq!(duration.pretty_format(), "00:45.50");
    }

    #[test]
    fn test_duration_format_less_than_minute_zero() {
        let duration = chrono::Duration::milliseconds(100);
        assert_eq!(duration.pretty_format(), "00:00.10");
    }

    #[test]
    fn test_duration_format_exactly_one_minute() {
        // Test duration = 1 minute (should show mm:ss without centiseconds)
        let duration = chrono::Duration::minutes(1);
        assert_eq!(duration.pretty_format(), "01:00");
    }

    #[test]
    fn test_duration_format_minutes_range() {
        // Test duration >= 1 minute but < 1 hour (should show mm:ss.nn)
        let duration = chrono::Duration::minutes(15)
            + chrono::Duration::seconds(30)
            + chrono::Duration::milliseconds(750);
        assert_eq!(duration.pretty_format(), "15:30.75");
    }

    #[test]
    fn test_duration_format_exactly_one_hour() {
        // Test duration = 1 hour (should show hh:mm:ss without centiseconds)
        let duration = chrono::Duration::hours(1);
        assert_eq!(duration.pretty_format(), "01:00:00");
    }

    #[test]
    fn test_duration_format_hours_range() {
        // Test duration >= 1 hour but < 1 day (should show hh:mm:ss.nn)
        let duration = chrono::Duration::hours(5)
            + chrono::Duration::minutes(23)
            + chrono::Duration::seconds(45)
            + chrono::Duration::milliseconds(120);
        assert_eq!(duration.pretty_format(), "05:23:45.12");
    }

    #[test]
    fn test_duration_format_exactly_one_day() {
        // Test duration = 1 day (should show dd:hh:mm:ss without centiseconds)
        let duration = chrono::Duration::days(1);
        assert_eq!(duration.pretty_format(), "01:00:00:00");
    }

    #[test]
    fn test_duration_format_days_range() {
        // Test duration >= 1 day (should show dd:hh:mm:ss.nn)
        let duration = chrono::Duration::days(3)
            + chrono::Duration::hours(12)
            + chrono::Duration::minutes(45)
            + chrono::Duration::seconds(30)
            + chrono::Duration::milliseconds(990);
        assert_eq!(duration.pretty_format(), "03:12:45:30.99");
    }

    #[test]
    fn test_duration_format_large_duration() {
        // Test very large duration
        let duration = chrono::Duration::days(99)
            + chrono::Duration::hours(23)
            + chrono::Duration::minutes(59)
            + chrono::Duration::seconds(59)
            + chrono::Duration::milliseconds(999);
        assert_eq!(duration.pretty_format(), "99:23:59:59.99");
    }

    #[test]
    fn test_duration_format_centiseconds_rounding() {
        // Test centisecond precision (milliseconds / 10)
        let duration = chrono::Duration::milliseconds(456);
        assert_eq!(duration.pretty_format(), "00:00.45"); // 456ms = 45 centiseconds (truncated)
    }

    #[test]
    fn test_duration_format_zero_duration() {
        let duration = chrono::Duration::zero();
        assert_eq!(duration.pretty_format(), "00:00");
    }

    #[test]
    fn test_duration_format_padding() {
        // Test that single-digit values are zero-padded
        let duration = chrono::Duration::hours(1)
            + chrono::Duration::minutes(2)
            + chrono::Duration::seconds(3)
            + chrono::Duration::milliseconds(40);
        assert_eq!(duration.pretty_format(), "01:02:03.04");
    }
}
