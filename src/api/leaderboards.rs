use napi_derive::napi;

#[napi]
pub mod leaderboards {
    use napi::bindgen_prelude::{BigInt, Buffer};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use steamworks::{
        LeaderboardDisplayType, LeaderboardSortMethod, 
        SteamId, LeaderboardEntry, LeaderboardId
    };

    #[napi(object)]
    pub struct LeaderboardEntry {
        pub global_rank: i32,
        pub score: i32,
        pub steam_id: BigInt,
        pub details: Vec<i32>,
    }

    #[napi]
    pub enum SortMethod {
        None,
        Ascending,
        Descending,
    }

    #[napi]
    pub enum DisplayType {
        None,
        Numeric,
        TimeSeconds,
        TimeMilliseconds,
    }

    #[napi]
    pub enum DataRequest {
        Global,
        GlobalAroundUser,
        Friends,
        Users,
    }

    #[napi]
    pub enum UploadScoreMethod {
        None,
        KeepBest,
        ForceUpdate,
    }

    // Static storage for leaderboard handles
    lazy_static::lazy_static! {
        static ref LEADERBOARD_HANDLES: Arc<Mutex<HashMap<String, LeaderboardId>>> = 
            Arc::new(Mutex::new(HashMap::new()));
    }

    impl From<SortMethod> for LeaderboardSortMethod {
        fn from(method: SortMethod) -> Self {
            match method {
                SortMethod::None => LeaderboardSortMethod::None,
                SortMethod::Ascending => LeaderboardSortMethod::Ascending,
                SortMethod::Descending => LeaderboardSortMethod::Descending,
            }
        }
    }

    impl From<DisplayType> for LeaderboardDisplayType {
        fn from(display_type: DisplayType) -> Self {
            match display_type {
                DisplayType::None => LeaderboardDisplayType::None,
                DisplayType::Numeric => LeaderboardDisplayType::Numeric,
                DisplayType::TimeSeconds => LeaderboardDisplayType::TimeSeconds,
                DisplayType::TimeMilliseconds => LeaderboardDisplayType::TimeMilliseconds,
            }
        }
    }

    impl From<steamworks::LeaderboardEntry> for LeaderboardEntry {
        fn from(entry: steamworks::LeaderboardEntry) -> Self {
            LeaderboardEntry {
                global_rank: entry.global_rank,
                score: entry.score,
                steam_id: BigInt::from(entry.steam_id.raw()),
                details: entry.details,
            }
        }
    }

    #[napi]
    pub async fn find_leaderboard(name: String) -> Option<String> {
        let client = crate::client::get_client();
        
        match client.user_stats().find_leaderboard(&name).await {
            Ok(leaderboard) => {
                let mut handles = LEADERBOARD_HANDLES.lock().unwrap();
                handles.insert(name.clone(), leaderboard);
                Some(name)
            }
            Err(_) => None,
        }
    }

    #[napi]
    pub async fn find_or_create_leaderboard(
        name: String,
        sort_method: SortMethod,
        display_type: DisplayType,
    ) -> Option<String> {
        let client = crate::client::get_client();
        
        match client.user_stats().find_or_create_leaderboard(
            &name,
            sort_method.into(),
            display_type.into(),
        ).await {
            Ok(leaderboard) => {
                let mut handles = LEADERBOARD_HANDLES.lock().unwrap();
                handles.insert(name.clone(), leaderboard);
                Some(name)
            }
            Err(_) => None,
        }
    }

    #[napi]
    pub async fn upload_score(
        leaderboard_name: String,
        score: i32,
        upload_method: UploadScoreMethod,
        details: Option<Vec<i32>>,
    ) -> Option<LeaderboardEntry> {
        let client = crate::client::get_client();
        let handles = LEADERBOARD_HANDLES.lock().unwrap();
        
        if let Some(leaderboard) = handles.get(&leaderboard_name) {
            let method = match upload_method {
                UploadScoreMethod::None => steamworks::UploadScoreMethod::None,
                UploadScoreMethod::KeepBest => steamworks::UploadScoreMethod::KeepBest,
                UploadScoreMethod::ForceUpdate => steamworks::UploadScoreMethod::ForceUpdate,
            };

            let score_details = details.unwrap_or_default();
            
            match client.user_stats().upload_leaderboard_score(
                *leaderboard,
                method,
                score,
                &score_details,
            ).await {
                Ok(result) => {
                    // Create a LeaderboardEntry from the result
                    Some(LeaderboardEntry {
                        global_rank: result.global_rank_new,
                        score: result.score,
                        steam_id: BigInt::from(client.user().steam_id().raw()),
                        details: score_details,
                    })
                }
                Err(_) => None,
            }
        } else {
            None
        }
    }

    #[napi]
    pub async fn download_scores(
        leaderboard_name: String,
        data_request: DataRequest,
        range_start: i32,
        range_end: i32,
    ) -> Vec<LeaderboardEntry> {
        let client = crate::client::get_client();
        let handles = LEADERBOARD_HANDLES.lock().unwrap();
        
        if let Some(leaderboard) = handles.get(&leaderboard_name) {
            let request_type = match data_request {
                DataRequest::Global => steamworks::LeaderboardDataRequest::Global,
                DataRequest::GlobalAroundUser => steamworks::LeaderboardDataRequest::GlobalAroundUser,
                DataRequest::Friends => steamworks::LeaderboardDataRequest::Friends,
                DataRequest::Users => steamworks::LeaderboardDataRequest::Users,
            };

            match client.user_stats().download_leaderboard_entries(
                *leaderboard,
                request_type,
                range_start,
                range_end,
            ).await {
                Ok(entries) => {
                    entries.into_iter().map(LeaderboardEntry::from).collect()
                }
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }

    #[napi]
    pub async fn download_scores_for_users(
        leaderboard_name: String,
        steam_ids: Vec<BigInt>,
    ) -> Vec<LeaderboardEntry> {
        let client = crate::client::get_client();
        let handles = LEADERBOARD_HANDLES.lock().unwrap();
        
        if let Some(leaderboard) = handles.get(&leaderboard_name) {
            let user_ids: Vec<SteamId> = steam_ids
                .iter()
                .map(|id| SteamId::from_raw(id.get_u64().1))
                .collect();

            match client.user_stats().download_leaderboard_entries_for_users(
                *leaderboard,
                &user_ids,
            ).await {
                Ok(entries) => {
                    entries.into_iter().map(LeaderboardEntry::from).collect()
                }
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        }
    }

    #[napi]
    pub fn get_leaderboard_name(leaderboard_name: String) -> Option<String> {
        let client = crate::client::get_client();
        let handles = LEADERBOARD_HANDLES.lock().unwrap();
        
        if let Some(leaderboard) = handles.get(&leaderboard_name) {
            client.user_stats().leaderboard_name(*leaderboard)
        } else {
            None
        }
    }

    #[napi]
    pub fn get_leaderboard_entry_count(leaderboard_name: String) -> Option<i32> {
        let client = crate::client::get_client();
        let handles = LEADERBOARD_HANDLES.lock().unwrap();
        
        if let Some(leaderboard) = handles.get(&leaderboard_name) {
            Some(client.user_stats().leaderboard_entry_count(*leaderboard))
        } else {
            None
        }
    }

    #[napi]
    pub fn get_leaderboard_sort_method(leaderboard_name: String) -> Option<SortMethod> {
        let client = crate::client::get_client();
        let handles = LEADERBOARD_HANDLES.lock().unwrap();
        
        if let Some(leaderboard) = handles.get(&leaderboard_name) {
            let sort_method = client.user_stats().leaderboard_sort_method(*leaderboard);
            Some(match sort_method {
                LeaderboardSortMethod::None => SortMethod::None,
                LeaderboardSortMethod::Ascending => SortMethod::Ascending,
                LeaderboardSortMethod::Descending => SortMethod::Descending,
            })
        } else {
            None
        }
    }

    #[napi]
    pub fn get_leaderboard_display_type(leaderboard_name: String) -> Option<DisplayType> {
        let client = crate::client::get_client();
        let handles = LEADERBOARD_HANDLES.lock().unwrap();
        
        if let Some(leaderboard) = handles.get(&leaderboard_name) {
            let display_type = client.user_stats().leaderboard_display_type(*leaderboard);
            Some(match display_type {
                LeaderboardDisplayType::None => DisplayType::None,
                LeaderboardDisplayType::Numeric => DisplayType::Numeric,
                LeaderboardDisplayType::TimeSeconds => DisplayType::TimeSeconds,
                LeaderboardDisplayType::TimeMilliseconds => DisplayType::TimeMilliseconds,
            })
        } else {
            None
        }
    }

    #[napi]
    pub async fn attach_leaderboard_ugc(
        leaderboard_name: String,
        ugc_handle: BigInt,
    ) -> bool {
        let client = crate::client::get_client();
        let handles = LEADERBOARD_HANDLES.lock().unwrap();
        
        if let Some(leaderboard) = handles.get(&leaderboard_name) {
            let ugc = steamworks::UGCHandle::from_raw(ugc_handle.get_u64().1);
            client.user_stats().attach_leaderboard_ugc(*leaderboard, ugc).await.is_ok()
        } else {
            false
        }
    }

    #[napi]
    pub fn clear_leaderboard_handle(leaderboard_name: String) -> bool {
        let mut handles = LEADERBOARD_HANDLES.lock().unwrap();
        handles.remove(&leaderboard_name).is_some()
    }

    #[napi]
    pub fn get_cached_leaderboard_names() -> Vec<String> {
        let handles = LEADERBOARD_HANDLES.lock().unwrap();
        handles.keys().cloned().collect()
    }
}