use sea_orm::entity::prelude::*;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &str {
        "user_word_whitelist"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq)]
pub struct Model {
    pub id: i32,
    pub user_id: String,
    pub word: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    UserId,
    Word,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = i32;

    fn auto_increment() -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    User,
}

impl ColumnTrait for Column {
    type EntityName = Entity;

    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::Integer.def(),
            Self::UserId => ColumnType::Text.def(),
            Self::Word => ColumnType::Text.def().null(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::User => Entity::belongs_to(super::user::Entity)
                .from(Column::UserId)
                .to(super::user::Column::Id)
                .into(),
        }
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
