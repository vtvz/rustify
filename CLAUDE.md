# Developer Guidelines

## Code Quality Checks

Run these at the end of every session:

- `cargo fmt --all` - Format all code
- `cargo test` - Run tests and fix until they pass
- `cargo clippy --all-targets --all-features --no-deps -- -D warnings` - Fix all warnings
- Generate tests for new features you implement

**Note**: Don't run `cargo check` separately - `cargo clippy` is sufficient as it includes all checks.

## Tracing Guidelines

Add `#[tracing::instrument(skip_all)]` to all public async functions:

```rust
// Always include user_id when state is available
#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(app: &App, state: &UserState) -> anyhow::Result<()> {

// Multiple fields - only include what's immediately available as parameters
#[tracing::instrument(skip_all, fields(%user_id, %track_id, ?status))]
pub async fn set_status(user_id: &str, track_id: &str, status: TrackStatus) -> anyhow::Result<()> {

// Service methods with string parameters
#[tracing::instrument(skip_all, fields(%user_id, %track_id))]
pub async fn get_track(db: &impl ConnectionTrait, user_id: &str, track_id: &str) -> anyhow::Result<Track> {
```

**Keep it simple**: Only add fields available as function parameters, don't overcomplicate with computed values.

## Code Style

- Use `anyhow::Result` for all error handling
- Use `?` operator for error propagation
- State access: `state.user_id()`, `state.locale()`, `state.spotify().await`
- Localization: `t!("translation-key", locale = state.locale())` - translates keys from `locales/` directory
- Service pattern: `ServiceName::method_name(db, params)`

## Naming Conventions

**Project-Specific Feature Names:**

- `recommendasion` - Intentional feature name for music recommendation functionality
- `skippage` - Intentional feature name (not a compound word or typo)

## Testing

- Add tests in `#[cfg(test)]` module at end of file
- Test business logic and edge cases
- Run `cargo test` to verify

## Database Migrations

To create a new migration:

```bash
sqlx migrate add migration_name
```

This creates a new migration file in the `migrations/` directory. Write SQL queries for PostgreSQL using **only lowercase** (including keywords):

```sql
-- Good
create table users (
    id bigint primary key,
    name text not null
);

-- Bad (don't use uppercase)
CREATE TABLE users (
    id BIGINT PRIMARY KEY,
    name TEXT NOT NULL
);
```

## Repository Information

- **GitHub Owner:** `vtvz`
- **GitHub Repo:** `rustify`
- **Main Branch:** `master`

## Common Patterns

Quick reference for common operations:

- Database: `app.db()`
- Redis: `app.redis_conn().await?`
- Spotify API: `state.spotify().await`
- Telegram Bot: `app.bot().send_message(...)`
- Current user: `state.user()`, `state.user_id()`
- Localization: `t!("key", locale = state.locale(), param = value)`
- GitHub Workflow: `gh workflow run <workflow> --ref <branch>` - when only one workflow exists, run it automatically for current branch
