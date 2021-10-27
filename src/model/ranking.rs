use super::{
    beatmap::Beatmapset,
    user::{
        deserialize_country, AccountHistory, Badge, Group, MedalCompact, MonthlyCount, UserCompact,
        UserCover, UserPage, UserStatistics,
    },
    GameMode,
};
use crate::{prelude::Username, Osu, OsuResult};

use chrono::{DateTime, Utc};
use serde::{
    de::{Deserializer, Error, IgnoredAny, MapAccess, SeqAccess, Visitor},
    ser::{SerializeSeq, SerializeStruct, Serializer},
    Deserialize, Serialize,
};
use std::fmt;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChartRankings {
    /// The list of beatmaps in the requested spotlight for the given mode
    #[serde(rename = "beatmapsets")]
    pub mapsets: Vec<Beatmapset>,
    #[serde(
        deserialize_with = "deserialize_user_stats_vec",
        serialize_with = "serialize_user_stats_vec"
    )]
    /// Score details ordered by score in descending order.
    pub ranking: Vec<UserCompact>,
    /// Spotlight details
    pub spotlight: Spotlight,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CountryRanking {
    /// Active user count
    pub active_users: u32,
    /// Country name
    #[serde(deserialize_with = "deserialize_country")]
    pub country: String,
    #[serde(rename = "code")]
    pub country_code: String,
    /// Summed playcount for all users
    #[serde(rename = "play_count")]
    pub playcount: u64,
    /// Summed performance points for all users
    #[serde(rename = "performance")]
    pub pp: f32,
    /// Summed ranked score for all users
    pub ranked_score: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CountryRankings {
    /// The next page of the ranking
    #[serde(
        default,
        rename = "cursor",
        deserialize_with = "deserialize_rankings_cursor",
        skip_serializing_if = "Option::is_none"
    )]
    pub next_page: Option<u32>,
    /// Country details ordered by pp in descending order.
    pub ranking: Vec<CountryRanking>,
    /// Total amount of countries
    pub total: u32,
}

impl CountryRankings {
    /// If `next_page` is `Some`, the API can provide the next set of countries and this method will request them.
    /// Otherwise, this method returns `None`.
    #[inline]
    pub async fn get_next(&self, osu: &Osu, mode: GameMode) -> Option<OsuResult<CountryRankings>> {
        Some(osu.country_rankings(mode).page(self.next_page?).await)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Rankings {
    #[serde(default)]
    pub(crate) mode: Option<GameMode>,
    #[serde(
        default,
        rename = "cursor",
        deserialize_with = "deserialize_rankings_cursor",
        skip_serializing_if = "Option::is_none"
    )]
    pub next_page: Option<u32>,
    #[serde(
        deserialize_with = "deserialize_user_stats_vec",
        serialize_with = "serialize_user_stats_vec"
    )]
    pub ranking: Vec<UserCompact>,
    #[serde(default)]
    pub(crate) ranking_type: Option<RankingType>,
    pub total: u32,
}

struct UserStatsVecVisitor;

impl<'de> Visitor<'de> for UserStatsVecVisitor {
    type Value = Vec<UserCompact>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a vec of UserStatistics structs")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut users = Vec::with_capacity(seq.size_hint().unwrap_or_default());

        while let Some(UserCompactWrapper(user)) = seq.next_element()? {
            users.push(user);
        }

        Ok(users)
    }
}

struct UserCompactWrapper(UserCompact);

impl<'de> Deserialize<'de> for UserCompactWrapper {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_map(UserStatsVisitor).map(UserCompactWrapper)
    }
}

struct UserStatsVisitor;

impl<'de> Visitor<'de> for UserStatsVisitor {
    type Value = UserCompact;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a UserStatistics struct")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut accuracy = None;
        let mut country_rank = None;
        let mut global_rank = None;
        let mut grade_counts = None;
        let mut is_ranked = None;
        let mut level = None;
        let mut max_combo = None;
        let mut playcount = None;
        let mut playtime = None;
        let mut pp = None;
        let mut ranked_score = None;
        let mut replays_watched = None;
        let mut total_hits = None;
        let mut total_score = None;

        let mut user = None;

        while let Some(key) = map.next_key()? {
            match key {
                "hit_accuracy" => {
                    accuracy.replace(map.next_value()?);
                }
                "country_rank" => country_rank = map.next_value()?,
                "global_rank" => global_rank = map.next_value()?,
                "grade_counts" => {
                    grade_counts.replace(map.next_value()?);
                }
                "is_ranked" => {
                    is_ranked.replace(map.next_value()?);
                }
                "level" => {
                    level.replace(map.next_value()?);
                }
                "maximum_combo" => {
                    max_combo.replace(map.next_value()?);
                }
                "play_count" => {
                    playcount.replace(map.next_value()?);
                }
                "play_time" => {
                    playtime.replace(map.next_value::<Option<u32>>()?.unwrap_or_default());
                }
                "pp" => {
                    pp.replace(map.next_value::<Option<f32>>()?.unwrap_or_default());
                }
                "ranked_score" => {
                    ranked_score.replace(map.next_value()?);
                }
                "replays_watched_by_others" => {
                    replays_watched.replace(map.next_value()?);
                }
                "total_hits" => {
                    total_hits.replace(map.next_value()?);
                }
                "total_score" => {
                    total_score.replace(map.next_value()?);
                }
                "user" => user = map.next_value()?,
                _ => {
                    let _: IgnoredAny = map.next_value()?;
                }
            }
        }

        let accuracy = accuracy.ok_or_else(|| Error::missing_field("hit_accuracy"))?;
        let grade_counts = grade_counts.ok_or_else(|| Error::missing_field("grade_counts"))?;
        let is_ranked = is_ranked.ok_or_else(|| Error::missing_field("is_ranked"))?;
        let level = level.ok_or_else(|| Error::missing_field("level"))?;
        let max_combo = max_combo.ok_or_else(|| Error::missing_field("maximum_combo"))?;
        let playcount = playcount.ok_or_else(|| Error::missing_field("play_count"))?;
        let playtime = playtime.ok_or_else(|| Error::missing_field("play_time"))?;
        let pp = pp.ok_or_else(|| Error::missing_field("pp"))?;
        let ranked_score = ranked_score.ok_or_else(|| Error::missing_field("ranked_score"))?;
        let replays_watched =
            replays_watched.ok_or_else(|| Error::missing_field("replays_watched_by_others"))?;
        let total_hits = total_hits.ok_or_else(|| Error::missing_field("total_hits"))?;
        let total_score = total_score.ok_or_else(|| Error::missing_field("total_score"))?;
        let mut user: UserCompact = user.ok_or_else(|| Error::missing_field("user"))?;

        let stats = UserStatistics {
            accuracy,
            country_rank,
            global_rank,
            grade_counts,
            is_ranked,
            level,
            max_combo,
            playcount,
            playtime,
            pp,
            ranked_score,
            replays_watched,
            total_hits,
            total_score,
        };

        user.statistics.replace(stats);

        Ok(user)
    }
}

fn deserialize_user_stats_vec<'de, D>(d: D) -> Result<Vec<UserCompact>, D::Error>
where
    D: Deserializer<'de>,
{
    d.deserialize_seq(UserStatsVecVisitor)
}

struct UserCompactBorrowed<'u>(&'u UserCompact);

impl<'u> Serialize for UserCompactBorrowed<'u> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let user = self.0;
        let stats = user.statistics.as_ref().unwrap();

        let len = 13 + stats.country_rank.is_some() as usize + stats.global_rank.is_some() as usize;

        let mut s = s.serialize_struct("UserStatistics", len)?;
        s.serialize_field("hit_accuracy", &stats.accuracy)?;

        if let Some(ref rank) = stats.country_rank {
            s.serialize_field("country_rank", rank)?;
        }

        if let Some(ref rank) = stats.global_rank {
            s.serialize_field("global_rank", rank)?;
        }

        s.serialize_field("grade_counts", &stats.grade_counts)?;
        s.serialize_field("is_ranked", &stats.is_ranked)?;
        s.serialize_field("level", &stats.level)?;
        s.serialize_field("maximum_combo", &stats.max_combo)?;
        s.serialize_field("play_count", &stats.playcount)?;
        s.serialize_field("play_time", &stats.playtime)?;
        s.serialize_field("pp", &stats.pp)?;
        s.serialize_field("ranked_score", &stats.ranked_score)?;
        s.serialize_field("replays_watched_by_others", &stats.replays_watched)?;
        s.serialize_field("total_hits", &stats.total_hits)?;
        s.serialize_field("total_score", &stats.total_score)?;
        s.serialize_field("user", &UserCompactWithoutStats::new(user))?;

        s.end()
    }
}

// Serializing a UserCompact reference without statistics
#[derive(Serialize)]
struct UserCompactWithoutStats<'u> {
    pub avatar_url: &'u String,
    pub country_code: &'u String,
    pub default_group: &'u String,
    pub is_active: &'u bool,
    pub is_bot: &'u bool,
    pub is_deleted: &'u bool,
    pub is_online: &'u bool,
    pub is_supporter: &'u bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_visit: &'u Option<DateTime<Utc>>,
    pub pm_friends_only: &'u bool,
    #[serde(rename = "profile_colour", skip_serializing_if = "Option::is_none")]
    pub profile_color: &'u Option<String>,
    #[serde(rename = "id")]
    pub user_id: &'u u32,
    pub username: &'u Username,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_history: &'u Option<Vec<AccountHistory>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badges: &'u Option<Vec<Badge>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beatmap_playcounts_count: &'u Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: &'u Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: &'u Option<UserCover>,
    #[serde(
        rename = "favourite_beatmapset_count",
        skip_serializing_if = "Option::is_none"
    )]
    pub favourite_mapset_count: &'u Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follower_count: &'u Option<u32>,
    #[serde(
        rename = "graveyard_beatmapset_count",
        skip_serializing_if = "Option::is_none"
    )]
    pub graveyard_mapset_count: &'u Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: &'u Option<Vec<Group>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_admin: &'u Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_bng: &'u Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_full_bn: &'u Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_gmt: &'u Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_limited_bn: &'u Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_moderator: &'u Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_nat: &'u Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_silenced: &'u Option<bool>,
    #[serde(
        rename = "loved_beatmapset_count",
        skip_serializing_if = "Option::is_none"
    )]
    pub loved_mapset_count: &'u Option<u32>,
    #[serde(rename = "user_achievements", skip_serializing_if = "Option::is_none")]
    pub medals: &'u Option<Vec<MedalCompact>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly_playcounts: &'u Option<Vec<MonthlyCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: &'u Option<UserPage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_usernames: &'u Option<Vec<Username>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank_history: &'u Option<Vec<u32>>,
    #[serde(
        rename = "ranked_beatmapset_count",
        skip_serializing_if = "Option::is_none"
    )]
    pub ranked_mapset_count: &'u Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replays_watched_counts: &'u Option<Vec<MonthlyCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scores_best_count: &'u Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scores_first_count: &'u Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scores_recent_count: &'u Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_level: &'u Option<u8>,
    #[serde(
        rename = "pending_beatmapset_count",
        skip_serializing_if = "Option::is_none"
    )]
    pub pending_mapset_count: &'u Option<u32>,
}

impl<'u> UserCompactWithoutStats<'u> {
    fn new(user: &'u UserCompact) -> Self {
        let UserCompact {
            avatar_url,
            country_code,
            default_group,
            is_active,
            is_bot,
            is_deleted,
            is_online,
            is_supporter,
            last_visit,
            pm_friends_only,
            profile_color,
            user_id,
            username,
            account_history,
            badges,
            beatmap_playcounts_count,
            country,
            cover,
            favourite_mapset_count,
            follower_count,
            graveyard_mapset_count,
            groups,
            is_admin,
            is_bng,
            is_full_bn,
            is_gmt,
            is_limited_bn,
            is_moderator,
            is_nat,
            is_silenced,
            loved_mapset_count,
            medals,
            monthly_playcounts,
            page,
            previous_usernames,
            rank_history,
            ranked_mapset_count,
            replays_watched_counts,
            scores_best_count,
            scores_first_count,
            scores_recent_count,
            statistics: _,
            support_level,
            pending_mapset_count,
        } = user;

        Self {
            avatar_url,
            country_code,
            default_group,
            is_active,
            is_bot,
            is_deleted,
            is_online,
            is_supporter,
            last_visit,
            pm_friends_only,
            profile_color,
            user_id,
            username,
            account_history,
            badges,
            beatmap_playcounts_count,
            country,
            cover,
            favourite_mapset_count,
            follower_count,
            graveyard_mapset_count,
            groups,
            is_admin,
            is_bng,
            is_full_bn,
            is_gmt,
            is_limited_bn,
            is_moderator,
            is_nat,
            is_silenced,
            loved_mapset_count,
            medals,
            monthly_playcounts,
            page,
            previous_usernames,
            rank_history,
            ranked_mapset_count,
            replays_watched_counts,
            scores_best_count,
            scores_first_count,
            scores_recent_count,
            support_level,
            pending_mapset_count,
        }
    }
}

fn serialize_user_stats_vec<S: Serializer>(users: &[UserCompact], s: S) -> Result<S::Ok, S::Error> {
    let mut seq = s.serialize_seq(Some(users.len()))?;

    for user in users {
        seq.serialize_element(&UserCompactBorrowed(user))?;
    }

    seq.end()
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RankingType {
    Charts,
    Country,
    Performance,
    Score,
}

impl fmt::Display for RankingType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let kind = match self {
            Self::Charts => "charts",
            Self::Country => "country",
            Self::Performance => "performance",
            Self::Score => "score",
        };

        f.write_str(kind)
    }
}

impl Rankings {
    /// If `next_page` is `Some`, the API can provide the next set of users and this method will request them.
    /// Otherwise, this method returns `None`.
    #[inline]
    pub async fn get_next(&self, osu: &Osu) -> Option<OsuResult<Rankings>> {
        let page = self.next_page?;
        let mode = self.mode?;
        let kind = self.ranking_type?;

        let rankings = match kind {
            RankingType::Performance => osu.performance_rankings(mode).page(page).await,
            RankingType::Score => osu.score_rankings(mode).page(page).await,
            RankingType::Charts | RankingType::Country => unreachable!(),
        };

        Some(rankings)
    }
}

struct RankingsCursorVisitor;

impl<'de> Visitor<'de> for RankingsCursorVisitor {
    type Value = Option<u32>;

    fn expecting(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        f.write_str("a u32, a map containing a `page` field, or null")
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Some(v as u32))
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_any(Self)
    }

    fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut page = None;

        while let Some(key) = map.next_key()? {
            match key {
                "page" => {
                    page.replace(map.next_value()?);
                }
                _ => {
                    let _: IgnoredAny = map.next_value()?;
                }
            }
        }

        page.ok_or_else(|| Error::missing_field("page")).map(Some)
    }
}

fn deserialize_rankings_cursor<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    d.deserialize_option(RankingsCursorVisitor)
}

/// The details of a spotlight.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Spotlight {
    /// The end date of the spotlight.
    pub end_date: DateTime<Utc>,
    /// If the spotlight has different mades specific to each [`GameMode`](crate::model::GameMode).
    pub mode_specific: bool,
    /// The name of the spotlight.
    pub name: String,
    /// The number of users participating in this spotlight. This is only shown when viewing a single spotlight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participant_count: Option<u32>,
    /// The ID of this spotlight.
    #[serde(rename = "id")]
    pub spotlight_id: u32,
    /// The type of spotlight.
    #[serde(rename = "type")]
    pub spotlight_type: String,
    /// The starting date of the spotlight.
    pub start_date: DateTime<Utc>,
}

impl PartialEq for Spotlight {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.spotlight_id == other.spotlight_id
            && self.start_date == other.start_date
            && self.end_date == other.end_date
    }
}

impl Eq for Spotlight {}
