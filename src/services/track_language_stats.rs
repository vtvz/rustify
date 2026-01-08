use isolang::Language;
use itertools::Itertools;
use sea_orm::ActiveValue::Set;
use sea_orm::prelude::Expr;
use sea_orm::sea_query::{Alias, OnConflict};
use sea_orm::{
    ColumnTrait as _,
    ConnectionTrait,
    EntityTrait as _,
    QueryFilter,
    QueryOrder,
    QuerySelect,
};

use crate::entity::prelude::{
    TrackLanguageStatsActiveModel,
    TrackLanguageStatsColumn,
    TrackLanguageStatsEntity,
};
use crate::utils::Clock;

pub struct TrackLanguageStatsService {}

impl TrackLanguageStatsService {
    #[tracing::instrument(skip_all, fields(
        user_id,
        language = language.map_or("none", |l| l.to_639_3()))
    )]
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

    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn stats_for_user(
        db: &impl ConnectionTrait,
        user_id: &str,
    ) -> anyhow::Result<Vec<(Option<Language>, i32)>> {
        let res: Vec<(Option<String>, i32)> = TrackLanguageStatsEntity::find()
            .filter(TrackLanguageStatsColumn::UserId.eq(user_id))
            .select_only()
            .columns([
                TrackLanguageStatsColumn::Language,
                TrackLanguageStatsColumn::Count,
            ])
            .order_by_desc(TrackLanguageStatsColumn::Count)
            .into_tuple()
            .all(db)
            .await?;

        let res = res
            .into_iter()
            .map(|(lang, stat)| (lang.and_then(|code| Language::from_639_3(&code)), stat))
            .collect_vec();

        Ok(res)
    }

    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn stats_all_users(
        db: &impl ConnectionTrait,
    ) -> anyhow::Result<Vec<(Option<Language>, i64)>> {
        let res: Vec<(Option<String>, i64)> = TrackLanguageStatsEntity::find()
            .select_only()
            .column(TrackLanguageStatsColumn::Language)
            .expr_as(TrackLanguageStatsColumn::Count.sum(), "sum")
            .order_by_desc(TrackLanguageStatsColumn::Count.sum())
            .group_by(TrackLanguageStatsColumn::Language)
            .into_tuple()
            .all(db)
            .await?;

        let res = res
            .into_iter()
            .map(|(lang, stat)| (lang.and_then(|code| Language::from_639_3(&code)), stat))
            .collect_vec();

        Ok(res)
    }
}
