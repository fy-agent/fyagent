//! WorkBuddy subscription binding stored in the existing settings table.
//!
//! The blob is a routing record. It must not contain OAuth material.

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;

pub(crate) const WORKBUDDY_SUBSCRIPTION_BINDING_KEY: &str = "workbuddy_subscription_binding_v1";

impl Database {
    pub(crate) fn get_workbuddy_subscription_binding(&self) -> Result<Option<String>, AppError> {
        self.get_setting(WORKBUDDY_SUBSCRIPTION_BINDING_KEY)
    }

    pub(crate) fn set_workbuddy_subscription_binding(&self, value: &str) -> Result<(), AppError> {
        self.set_setting(WORKBUDDY_SUBSCRIPTION_BINDING_KEY, value)
    }

    pub(crate) fn delete_workbuddy_subscription_binding(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM settings WHERE key = ?1",
            params![WORKBUDDY_SUBSCRIPTION_BINDING_KEY],
        )
        .map_err(|error| AppError::Database(error.to_string()))?;
        Ok(())
    }
}
