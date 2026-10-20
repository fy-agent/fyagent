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

    /// Compare and publish under the settings connection lock. A concurrent
    /// lifecycle/account update must never be overwritten by a stale plan or
    /// by its compensation.
    pub(crate) fn compare_exchange_workbuddy_subscription_binding(
        &self,
        expected: Option<&str>,
        replacement: Option<&str>,
    ) -> Result<bool, AppError> {
        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|error| AppError::Database(error.to_string()))?;
        use rusqlite::OptionalExtension;
        let current: Option<String> = tx
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![WORKBUDDY_SUBSCRIPTION_BINDING_KEY],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| AppError::Database(error.to_string()))?;
        if current.as_deref() != expected {
            return Ok(false);
        }
        if let Some(value) = replacement {
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![WORKBUDDY_SUBSCRIPTION_BINDING_KEY, value],
            )
        } else {
            tx.execute(
                "DELETE FROM settings WHERE key = ?1",
                params![WORKBUDDY_SUBSCRIPTION_BINDING_KEY],
            )
        }
        .map_err(|error| AppError::Database(error.to_string()))?;
        tx.commit()
            .map_err(|error| AppError::Database(error.to_string()))?;
        Ok(true)
    }

    // 第一刀仅测试覆盖，第二刀再挂命令。
    #[cfg_attr(not(test), allow(dead_code))]
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
