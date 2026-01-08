use isolang::Language;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::{Alias, OnConflict};
use sea_orm::{ConnectionTrait, EntityTrait as _};

use crate::entity::prelude::{
    TrackLanguageStatsActiveModel,
    TrackLanguageStatsColumn,
    TrackLanguageStatsEntity,
};
use crate::utils::Clock;

pub struct TrackLanguageStatsService {}

impl TrackLanguageStatsService {
    #[tracing::instrument(skip_all)]
    pub async fn increase_count(
        db: &impl ConnectionTrait,
        language: Option<Language>,
        user_id: &str,
    ) -> anyhow::Result<()> {
        let model = TrackLanguageStatsActiveModel {
            user_id: Set(user_id.into()),
            language: Set(language.map(|language| language.to_639_3().into())),
            count: Set(1),
            updated_at: Set(Clock::now()),
            ..Default::default()
        };

        TrackLanguageStatsEntity::insert_many([model])
            .on_conflict(
                OnConflict::columns([
                    TrackLanguageStatsColumn::UserId,
                    TrackLanguageStatsColumn::Language,
                ])
                .value(
                    TrackLanguageStatsColumn::Count,
                    Expr::col((TrackLanguageStatsEntity, TrackLanguageStatsColumn::Count)).add(
                        Expr::col((Alias::new("excluded"), TrackLanguageStatsColumn::Count)),
                    ),
                )
                .to_owned(),
            )
            .exec(db)
            .await?;

        Ok(())
    }
}
