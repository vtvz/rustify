use clap::Subcommand;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set};

use crate::app::App;
use crate::entity::prelude::*;

#[derive(Subcommand)]
pub enum UsersCommands {
    /// Promote user to admin
    Promote {
        /// Telegram user ID to promote
        user_id: String,
    },
    /// Demote admin to regular user
    Demote {
        /// Telegram user ID to demote
        user_id: String,
    },
    /// List all admin users
    ListAdmins,
}

pub async fn promote_admin(app: &App, user_id: &str) -> anyhow::Result<()> {
    set_role(app, user_id, UserRole::Admin).await
}

pub async fn demote_admin(app: &App, user_id: &str) -> anyhow::Result<()> {
    set_role(app, user_id, UserRole::User).await
}

pub async fn list_admins(app: &App) -> anyhow::Result<()> {
    let admins = UserEntity::find()
        .filter(UserColumn::Role.eq(UserRole::Admin))
        .all(app.db())
        .await?;

    if admins.is_empty() {
        println!("No admins found");
    } else {
        println!("Admins:");
        for admin in admins {
            println!("  - {} ({})", admin.id, admin.name);
        }
    }

    Ok(())
}

async fn set_role(app: &App, user_id: &str, role: UserRole) -> anyhow::Result<()> {
    let user = UserEntity::find_by_id(user_id)
        .one(app.db())
        .await?
        .ok_or_else(|| anyhow::anyhow!("User not found: {}", user_id))?;

    let mut user_active = user.into_active_model();
    user_active.role = Set(role.clone());
    user_active.save(app.db()).await?;

    println!("User {} role set to {:?}", user_id, role);

    Ok(())
}

pub async fn run(command: UsersCommands) {
    crate::infrastructure::logger::init()
        .await
        .expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("GIT_SHA"),
        "Starting Rustify bot..."
    );

    let app = App::init().await.expect("State to be built");

    let result = match command {
        UsersCommands::Promote { user_id } => promote_admin(app, &user_id).await,
        UsersCommands::Demote { user_id } => demote_admin(app, &user_id).await,
        UsersCommands::ListAdmins => list_admins(app).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
