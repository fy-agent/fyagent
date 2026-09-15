use serde::{Deserialize, Serialize};

use super::{get_settings, mutate_settings, AppSettings};
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstUseGuideState {
    Pending,
    Dismissed,
}

fn initial_state(settings: &AppSettings, fresh_install: bool) -> FirstUseGuideState {
    if settings.first_run_notice_confirmed == Some(true) {
        return FirstUseGuideState::Dismissed;
    }
    settings.first_use_guide_state.unwrap_or(if fresh_install {
        FirstUseGuideState::Pending
    } else {
        FirstUseGuideState::Dismissed
    })
}

/// Persist eligibility before database creation/seeding so an unfinished guide
/// survives a restart. Missing markers on existing or unreadable data are not
/// evidence of a new install; those users keep the ordinary directory.
pub(crate) fn initialize_first_use_guide(new_data_dir: bool) -> Result<(), AppError> {
    let settings = get_settings();
    let no_settings_file =
        AppSettings::settings_path().is_some_and(|path| matches!(path.try_exists(), Ok(false)));
    if settings.first_use_guide_state.is_none()
        && initial_state(&settings, new_data_dir && no_settings_file) == FirstUseGuideState::Pending
    {
        mutate_settings(|current| {
            current.first_use_guide_state = Some(FirstUseGuideState::Pending);
        })?;
    }
    Ok(())
}

pub(crate) fn get_first_use_guide_state() -> FirstUseGuideState {
    initial_state(&get_settings(), false)
}

pub(crate) fn dismiss_first_use_guide() -> Result<FirstUseGuideState, AppError> {
    mutate_settings(|settings| {
        settings.first_use_guide_state = Some(FirstUseGuideState::Dismissed);
        settings.first_run_notice_confirmed = Some(true);
    })?;
    Ok(FirstUseGuideState::Dismissed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_confirmed_fresh_install_enters_the_guide() {
        let settings = AppSettings::default();
        assert_eq!(initial_state(&settings, true), FirstUseGuideState::Pending);
        assert_eq!(
            initial_state(&settings, false),
            FirstUseGuideState::Dismissed
        );
    }

    #[test]
    fn pending_survives_database_creation_and_settings_roundtrip() {
        let settings = AppSettings {
            first_use_guide_state: Some(FirstUseGuideState::Pending),
            ..AppSettings::default()
        };
        let json = serde_json::to_string(&settings).expect("serialize settings");
        let restored: AppSettings = serde_json::from_str(&json).expect("restore settings");
        assert_eq!(initial_state(&restored, false), FirstUseGuideState::Pending);
        assert!(json.contains("\"firstUseGuideState\":\"pending\""));
    }

    #[test]
    fn completed_and_legacy_confirmed_users_never_reenter() {
        for fresh_install in [false, true] {
            let dismissed = AppSettings {
                first_use_guide_state: Some(FirstUseGuideState::Dismissed),
                ..AppSettings::default()
            };
            assert_eq!(
                initial_state(&dismissed, fresh_install),
                FirstUseGuideState::Dismissed
            );
            for state in [None, Some(FirstUseGuideState::Pending)] {
                let legacy = AppSettings {
                    first_run_notice_confirmed: Some(true),
                    first_use_guide_state: state,
                    ..AppSettings::default()
                };
                assert_eq!(
                    initial_state(&legacy, fresh_install),
                    FirstUseGuideState::Dismissed
                );
            }
        }
    }

    #[test]
    fn guide_state_is_closed_and_old_settings_remain_compatible() {
        let json = serde_json::to_value(AppSettings::default()).expect("settings");
        assert!(json.get("firstUseGuideState").is_none());
        assert!(serde_json::from_str::<FirstUseGuideState>("\"unknown\"").is_err());
        assert!(serde_json::from_str::<FirstUseGuideState>("true").is_err());
        assert_eq!(
            serde_json::to_string(&FirstUseGuideState::Dismissed).expect("state"),
            "\"dismissed\""
        );
    }
}
