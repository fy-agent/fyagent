//! Process-local Agent-auth session coordination.
//!
//! Auth observation and lifecycle are deliberately separate from installer
//! jobs. A successful terminal/application handoff is never promoted to a
//! verified login state without an authoritative adapter reread.

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use uuid::Uuid;

use super::{
    auth_actions::{
        account_state, launch_auth_action, observe_agent_auth, provider_ids, AuthLaunchDisposition,
    },
    desktop::launch_desktop_installation,
    inventory::validate_action_target,
    types::{
        validate_auth_session_id, validate_opaque_auth_provider_id, AgentActionId,
        AgentAuthAccountState, AgentAuthIntent, AgentAuthObservationDto, AgentAuthReasonCode,
        AgentAuthSessionOutcome, AgentAuthSessionSnapshot, AgentAuthSessionStage, AgentReasonCode,
        StartAgentActionRequest, StartAgentAuthSessionRequest, AGENT_AUTH_CONTRACT_VERSION,
    },
};
use crate::{services::external_agents::AgentCatalogId, store::AppState};

const AUTH_SESSION_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const AUTH_SESSION_POLL_INTERVAL: Duration = Duration::from_secs(1);
const MAX_AUTH_SESSIONS: usize = 64;

#[derive(Clone)]
struct AuthSessionRecord {
    snapshot: AgentAuthSessionSnapshot,
    stop: Arc<AtomicBool>,
}

#[derive(Default)]
struct AuthSessionCache {
    records: HashMap<String, AuthSessionRecord>,
    order: VecDeque<String>,
    active_by_agent: HashMap<AgentCatalogId, String>,
}

pub struct AgentAuthSessionStore {
    inner: Mutex<AuthSessionCache>,
}

impl AgentAuthSessionStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AuthSessionCache::default()),
        }
    }

    fn start(
        &self,
        agent_id: AgentCatalogId,
        intent: AgentAuthIntent,
        observation: AgentAuthObservationDto,
    ) -> Result<(AgentAuthSessionSnapshot, Arc<AtomicBool>), AgentAuthReasonCode> {
        let mut cache = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(session_id) = cache.active_by_agent.get(&agent_id) {
            if cache
                .records
                .get(session_id)
                .is_some_and(|record| !is_terminal(record.snapshot.stage))
            {
                return Err(AgentAuthReasonCode::OperationConflict);
            }
        }

        let session_id = Uuid::new_v4().hyphenated().to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let snapshot = AgentAuthSessionSnapshot {
            contract_version: AGENT_AUTH_CONTRACT_VERSION,
            session_id: session_id.clone(),
            agent_id,
            intent,
            stage: AgentAuthSessionStage::Preparing,
            can_stop_waiting: false,
            outcome: None,
            observation,
            reason_code: None,
        };
        cache.active_by_agent.insert(agent_id, session_id.clone());
        cache.order.push_back(session_id.clone());
        cache.records.insert(
            session_id,
            AuthSessionRecord {
                snapshot: snapshot.clone(),
                stop: Arc::clone(&stop),
            },
        );
        prune_sessions(&mut cache);
        Ok((snapshot, stop))
    }

    pub fn get(&self, session_id: &str) -> Result<AgentAuthSessionSnapshot, AgentAuthReasonCode> {
        if !validate_auth_session_id(session_id) {
            return Err(AgentAuthReasonCode::OperationConflict);
        }
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .records
            .get(session_id)
            .map(|record| record.snapshot.clone())
            .ok_or(AgentAuthReasonCode::OperationConflict)
    }

    pub fn active_for_agent(&self, agent_id: AgentCatalogId) -> Option<AgentAuthSessionSnapshot> {
        let cache = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let session_id = cache.active_by_agent.get(&agent_id)?;
        cache
            .records
            .get(session_id)
            .filter(|record| !is_terminal(record.snapshot.stage))
            .map(|record| record.snapshot.clone())
    }

    pub fn stop_waiting(
        &self,
        session_id: &str,
    ) -> Result<AgentAuthSessionSnapshot, AgentAuthReasonCode> {
        if !validate_auth_session_id(session_id) {
            return Err(AgentAuthReasonCode::OperationConflict);
        }
        let mut cache = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (agent_id, snapshot) = {
            let record = cache
                .records
                .get_mut(session_id)
                .ok_or(AgentAuthReasonCode::OperationConflict)?;
            if !record.snapshot.can_stop_waiting || is_terminal(record.snapshot.stage) {
                return Err(AgentAuthReasonCode::OperationConflict);
            }
            record.stop.store(true, Ordering::Release);
            record.snapshot.stage = AgentAuthSessionStage::Cancelled;
            record.snapshot.can_stop_waiting = false;
            record.snapshot.outcome = Some(AgentAuthSessionOutcome::Cancelled);
            record.snapshot.reason_code = Some(AgentAuthReasonCode::MonitoringStopped);
            (record.snapshot.agent_id, record.snapshot.clone())
        };
        clear_active_if_matches(&mut cache, agent_id, session_id);
        Ok(snapshot)
    }

    fn transition(
        &self,
        session_id: &str,
        stage: AgentAuthSessionStage,
        observation: AgentAuthObservationDto,
        outcome: Option<AgentAuthSessionOutcome>,
        reason_code: Option<AgentAuthReasonCode>,
    ) -> Result<AgentAuthSessionSnapshot, AgentAuthReasonCode> {
        let mut cache = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (agent_id, terminal, snapshot) = {
            let record = cache
                .records
                .get_mut(session_id)
                .ok_or(AgentAuthReasonCode::OperationConflict)?;
            if is_terminal(record.snapshot.stage)
                || !is_allowed_transition(record.snapshot.stage, stage)
                || !terminal_shape_is_valid(stage, outcome)
            {
                return Err(AgentAuthReasonCode::OperationConflict);
            }
            record.snapshot.stage = stage;
            record.snapshot.can_stop_waiting = matches!(
                stage,
                AgentAuthSessionStage::AwaitingUser | AgentAuthSessionStage::Verifying
            );
            record.snapshot.outcome = outcome;
            record.snapshot.observation = observation;
            record.snapshot.reason_code = reason_code;
            (
                record.snapshot.agent_id,
                is_terminal(stage),
                record.snapshot.clone(),
            )
        };
        if terminal {
            clear_active_if_matches(&mut cache, agent_id, session_id);
        }
        Ok(snapshot)
    }

    fn stopped(&self, stop: &AtomicBool) -> bool {
        stop.load(Ordering::Acquire)
    }
}

impl Default for AgentAuthSessionStore {
    fn default() -> Self {
        Self::new()
    }
}

fn clear_active_if_matches(
    cache: &mut AuthSessionCache,
    agent_id: AgentCatalogId,
    session_id: &str,
) {
    if cache
        .active_by_agent
        .get(&agent_id)
        .is_some_and(|current| current == session_id)
    {
        cache.active_by_agent.remove(&agent_id);
    }
}

fn prune_sessions(cache: &mut AuthSessionCache) {
    if cache.records.len() <= MAX_AUTH_SESSIONS {
        return;
    }

    // A long-running oldest session must not block eviction of later terminal
    // snapshots. Retain every active record, but continue scanning the whole
    // queue until the bounded history limit is restored.
    let mut retained = VecDeque::with_capacity(cache.order.len());
    while let Some(session_id) = cache.order.pop_front() {
        let removable = cache.records.len() > MAX_AUTH_SESSIONS
            && cache
                .records
                .get(&session_id)
                .is_none_or(|record| is_terminal(record.snapshot.stage));
        if removable {
            cache.records.remove(&session_id);
        } else {
            retained.push_back(session_id);
        }
    }
    cache.order = retained;
}

fn is_terminal(stage: AgentAuthSessionStage) -> bool {
    matches!(
        stage,
        AgentAuthSessionStage::Verified
            | AgentAuthSessionStage::HandoffComplete
            | AgentAuthSessionStage::Failed
            | AgentAuthSessionStage::Cancelled
            | AgentAuthSessionStage::TimedOut
    )
}

fn is_allowed_transition(from: AgentAuthSessionStage, to: AgentAuthSessionStage) -> bool {
    matches!(
        (from, to),
        (
            AgentAuthSessionStage::Preparing,
            AgentAuthSessionStage::Launching
                | AgentAuthSessionStage::Failed
                | AgentAuthSessionStage::Cancelled
        ) | (
            AgentAuthSessionStage::Launching,
            AgentAuthSessionStage::AwaitingUser
                | AgentAuthSessionStage::Verifying
                | AgentAuthSessionStage::HandoffComplete
                | AgentAuthSessionStage::Failed
                | AgentAuthSessionStage::Cancelled
        ) | (
            AgentAuthSessionStage::AwaitingUser,
            AgentAuthSessionStage::Verifying
                | AgentAuthSessionStage::HandoffComplete
                | AgentAuthSessionStage::Failed
                | AgentAuthSessionStage::Cancelled
                | AgentAuthSessionStage::TimedOut
        ) | (
            AgentAuthSessionStage::Verifying,
            AgentAuthSessionStage::AwaitingUser
                | AgentAuthSessionStage::Verified
                | AgentAuthSessionStage::Failed
                | AgentAuthSessionStage::Cancelled
                | AgentAuthSessionStage::TimedOut
        )
    )
}

fn terminal_shape_is_valid(
    stage: AgentAuthSessionStage,
    outcome: Option<AgentAuthSessionOutcome>,
) -> bool {
    match stage {
        AgentAuthSessionStage::Verified => matches!(
            outcome,
            Some(
                AgentAuthSessionOutcome::VerifiedLoggedIn
                    | AgentAuthSessionOutcome::VerifiedLoggedOut
                    | AgentAuthSessionOutcome::VerifiedProviderChange
            )
        ),
        AgentAuthSessionStage::HandoffComplete => {
            outcome == Some(AgentAuthSessionOutcome::HandoffOnly)
        }
        AgentAuthSessionStage::Failed => outcome == Some(AgentAuthSessionOutcome::Failed),
        AgentAuthSessionStage::Cancelled => outcome == Some(AgentAuthSessionOutcome::Cancelled),
        AgentAuthSessionStage::TimedOut => outcome == Some(AgentAuthSessionOutcome::TimedOut),
        AgentAuthSessionStage::Preparing
        | AgentAuthSessionStage::Launching
        | AgentAuthSessionStage::AwaitingUser
        | AgentAuthSessionStage::Verifying => outcome.is_none(),
    }
}

#[derive(Debug, Clone)]
enum AuthLaunchTarget {
    None,
    Desktop(PathBuf),
}

pub async fn auth_observation_for(agent_id: AgentCatalogId) -> AgentAuthObservationDto {
    observe_agent_auth(agent_id).await
}

pub async fn start_agent_auth_session(
    request: StartAgentAuthSessionRequest,
    state: &AppState,
) -> Result<AgentAuthSessionSnapshot, AgentAuthReasonCode> {
    validate_provider_request(&request)?;
    let before = observe_agent_auth(request.agent_id).await;
    validate_intent(&request, &before)?;
    let target = validate_auth_target(&request, state).await?;
    let (snapshot, stop) =
        state
            .agent_auth_sessions
            .start(request.agent_id, request.intent, before.clone())?;
    let store = Arc::clone(&state.agent_auth_sessions);
    tokio::spawn(run_auth_session(
        store,
        snapshot.session_id.clone(),
        request,
        before,
        target,
        stop,
    ));
    Ok(snapshot)
}

pub fn get_agent_auth_session(
    session_id: &str,
    state: &AppState,
) -> Result<AgentAuthSessionSnapshot, AgentAuthReasonCode> {
    state.agent_auth_sessions.get(session_id)
}

pub fn get_active_agent_auth_session(
    agent_id: AgentCatalogId,
    state: &AppState,
) -> Option<AgentAuthSessionSnapshot> {
    state.agent_auth_sessions.active_for_agent(agent_id)
}

pub fn stop_waiting_for_agent_auth(
    session_id: &str,
    state: &AppState,
) -> Result<AgentAuthSessionSnapshot, AgentAuthReasonCode> {
    state.agent_auth_sessions.stop_waiting(session_id)
}

fn validate_provider_request(
    request: &StartAgentAuthSessionRequest,
) -> Result<(), AgentAuthReasonCode> {
    if request
        .provider_id
        .as_deref()
        .is_some_and(|provider_id| !validate_opaque_auth_provider_id(provider_id))
    {
        return Err(AgentAuthReasonCode::ProviderChanged);
    }
    match (
        request.agent_id,
        request.intent,
        request.provider_id.as_deref(),
    ) {
        (AgentCatalogId::OpenCode, AgentAuthIntent::ConnectProvider, None) => Ok(()),
        (AgentCatalogId::OpenCode, AgentAuthIntent::Logout, Some(_)) => Ok(()),
        (AgentCatalogId::OpenCode, _, _) => Err(AgentAuthReasonCode::ProviderSelectionRequired),
        (_, _, None) => Ok(()),
        _ => Err(AgentAuthReasonCode::ProviderSelectionRequired),
    }
}

fn validate_intent(
    request: &StartAgentAuthSessionRequest,
    observation: &AgentAuthObservationDto,
) -> Result<(), AgentAuthReasonCode> {
    let allowed = observation_allowed_intents(observation);
    if !allowed.contains(&request.intent) {
        return Err(match observation {
            AgentAuthObservationDto::FyagentManaged { .. } => {
                AgentAuthReasonCode::ManagedByAuthCenter
            }
            AgentAuthObservationDto::Unavailable { .. } => {
                AgentAuthReasonCode::AuthObserverUnavailable
            }
            _ => AgentAuthReasonCode::ExecutorNotImplemented,
        });
    }
    if request.agent_id == AgentCatalogId::OpenCode && request.intent == AgentAuthIntent::Logout {
        let selected = request
            .provider_id
            .as_deref()
            .ok_or(AgentAuthReasonCode::ProviderSelectionRequired)?;
        let providers =
            provider_ids(observation).ok_or(AgentAuthReasonCode::AuthObserverUnavailable)?;
        if !providers.contains(selected) {
            return Err(AgentAuthReasonCode::ProviderChanged);
        }
    }
    Ok(())
}

fn observation_allowed_intents(observation: &AgentAuthObservationDto) -> &[AgentAuthIntent] {
    match observation {
        AgentAuthObservationDto::Account {
            allowed_intents, ..
        }
        | AgentAuthObservationDto::ProviderConnections {
            allowed_intents, ..
        }
        | AgentAuthObservationDto::HandoffOnly {
            allowed_intents, ..
        }
        | AgentAuthObservationDto::FyagentManaged {
            allowed_intents, ..
        }
        | AgentAuthObservationDto::Unavailable {
            allowed_intents, ..
        } => allowed_intents,
    }
}

async fn validate_auth_target(
    request: &StartAgentAuthSessionRequest,
    state: &AppState,
) -> Result<AuthLaunchTarget, AgentAuthReasonCode> {
    let has_binding = request.inventory_id.is_some()
        || request.target_id.is_some()
        || request.expected_target_revision.is_some();
    if !matches!(
        request.agent_id,
        AgentCatalogId::QoderWork | AgentCatalogId::TraeWork | AgentCatalogId::WorkBuddy
    ) {
        return if has_binding {
            Err(AgentAuthReasonCode::TargetChanged)
        } else {
            Ok(AuthLaunchTarget::None)
        };
    }
    if request.intent != AgentAuthIntent::Login {
        return Err(AgentAuthReasonCode::ExecutorNotImplemented);
    }
    let compatibility = StartAgentActionRequest {
        agent_id: request.agent_id,
        action: AgentActionId::AuthLogin,
        expected_release_id: None,
        inventory_id: request.inventory_id.clone(),
        target_id: request.target_id.clone(),
        expected_target_revision: request.expected_target_revision.clone(),
    };
    let target = validate_action_target(&compatibility, state)
        .await
        .map_err(map_target_reason)?;
    target
        .desktop_path()
        .cloned()
        .map(AuthLaunchTarget::Desktop)
        .ok_or(AgentAuthReasonCode::TargetNotExecutable)
}

fn map_target_reason(reason: AgentReasonCode) -> AgentAuthReasonCode {
    match reason {
        AgentReasonCode::TargetSelectionRequired => AgentAuthReasonCode::TargetSelectionRequired,
        AgentReasonCode::TargetChanged | AgentReasonCode::RefreshRequired => {
            AgentAuthReasonCode::TargetChanged
        }
        AgentReasonCode::TargetNotExecutable => AgentAuthReasonCode::TargetNotExecutable,
        AgentReasonCode::InventoryExpired => AgentAuthReasonCode::InventoryExpired,
        AgentReasonCode::InteractiveUserUnavailable => {
            AgentAuthReasonCode::InteractiveUserUnavailable
        }
        AgentReasonCode::OperationConflict => AgentAuthReasonCode::OperationConflict,
        _ => AgentAuthReasonCode::TargetChanged,
    }
}

async fn run_auth_session(
    store: Arc<AgentAuthSessionStore>,
    session_id: String,
    request: StartAgentAuthSessionRequest,
    before: AgentAuthObservationDto,
    target: AuthLaunchTarget,
    stop: Arc<AtomicBool>,
) {
    if store
        .transition(
            &session_id,
            AgentAuthSessionStage::Launching,
            before.clone(),
            None,
            None,
        )
        .is_err()
    {
        return;
    }
    let disposition = match launch_session_action(request.agent_id, request.intent, target).await {
        Ok(disposition) => disposition,
        Err(reason) => {
            let _ = store.transition(
                &session_id,
                AgentAuthSessionStage::Failed,
                before,
                Some(AgentAuthSessionOutcome::Failed),
                Some(reason),
            );
            return;
        }
    };
    if disposition == AuthLaunchDisposition::HandoffComplete {
        let current = observe_agent_auth(request.agent_id).await;
        let _ = store.transition(
            &session_id,
            AgentAuthSessionStage::HandoffComplete,
            current,
            Some(AgentAuthSessionOutcome::HandoffOnly),
            Some(AgentAuthReasonCode::HandoffOnly),
        );
        return;
    }
    if store
        .transition(
            &session_id,
            AgentAuthSessionStage::AwaitingUser,
            before.clone(),
            None,
            None,
        )
        .is_err()
    {
        return;
    }

    let deadline = Instant::now() + AUTH_SESSION_TIMEOUT;
    let mut latest = before.clone();
    loop {
        if store.stopped(&stop) {
            return;
        }
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            let _ = store.transition(
                &session_id,
                AgentAuthSessionStage::TimedOut,
                latest,
                Some(AgentAuthSessionOutcome::TimedOut),
                Some(AgentAuthReasonCode::TimedOut),
            );
            return;
        };
        tokio::time::sleep(remaining.min(AUTH_SESSION_POLL_INTERVAL)).await;
        if store.stopped(&stop) {
            return;
        }
        if store
            .transition(
                &session_id,
                AgentAuthSessionStage::Verifying,
                latest.clone(),
                None,
                None,
            )
            .is_err()
        {
            return;
        }
        latest = observe_agent_auth(request.agent_id).await;
        if let Some(outcome) = verified_outcome(&request, &before, &latest) {
            let _ = store.transition(
                &session_id,
                AgentAuthSessionStage::Verified,
                latest,
                Some(outcome),
                None,
            );
            return;
        }
        if store
            .transition(
                &session_id,
                AgentAuthSessionStage::AwaitingUser,
                latest.clone(),
                None,
                None,
            )
            .is_err()
        {
            return;
        }
    }
}

async fn launch_session_action(
    agent_id: AgentCatalogId,
    intent: AgentAuthIntent,
    target: AuthLaunchTarget,
) -> Result<AuthLaunchDisposition, AgentAuthReasonCode> {
    match target {
        AuthLaunchTarget::Desktop(path) => tokio::task::spawn_blocking(move || {
            launch_desktop_installation(agent_id, &path)
                .map(|_| AuthLaunchDisposition::HandoffComplete)
                .map_err(map_target_reason)
        })
        .await
        .unwrap_or(Err(AgentAuthReasonCode::InteractiveUserUnavailable)),
        AuthLaunchTarget::None => {
            tokio::task::spawn_blocking(move || launch_auth_action(agent_id, intent))
                .await
                .unwrap_or(Err(AgentAuthReasonCode::InteractiveUserUnavailable))
        }
    }
}

fn verified_outcome(
    request: &StartAgentAuthSessionRequest,
    before: &AgentAuthObservationDto,
    after: &AgentAuthObservationDto,
) -> Option<AgentAuthSessionOutcome> {
    match (request.agent_id, request.intent) {
        (AgentCatalogId::ClaudeCode, AgentAuthIntent::Login)
            if account_state(after) == Some(AgentAuthAccountState::LoggedIn) =>
        {
            Some(AgentAuthSessionOutcome::VerifiedLoggedIn)
        }
        (AgentCatalogId::ClaudeCode, AgentAuthIntent::Logout)
            if account_state(after) == Some(AgentAuthAccountState::LoggedOut) =>
        {
            Some(AgentAuthSessionOutcome::VerifiedLoggedOut)
        }
        (AgentCatalogId::OpenCode, AgentAuthIntent::ConnectProvider) => {
            let before = provider_ids(before)?;
            let after = provider_ids(after)?;
            (before != after && after.difference(&before).next().is_some())
                .then_some(AgentAuthSessionOutcome::VerifiedProviderChange)
        }
        (AgentCatalogId::OpenCode, AgentAuthIntent::Logout) => {
            let provider_id = request.provider_id.as_deref()?;
            let before = provider_ids(before)?;
            let after = provider_ids(after)?;
            (before.contains(provider_id) && !after.contains(provider_id))
                .then_some(AgentAuthSessionOutcome::VerifiedProviderChange)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{
        AgentAuthAuthority, AgentAuthOwnership, AgentAuthProviderConnectionState,
        AgentAuthProviderSummaryDto,
    };
    use super::*;

    fn account(state: AgentAuthAccountState) -> AgentAuthObservationDto {
        AgentAuthObservationDto::Account {
            contract_version: AGENT_AUTH_CONTRACT_VERSION,
            agent_id: AgentCatalogId::ClaudeCode,
            ownership: AgentAuthOwnership::AgentOwned,
            authority: AgentAuthAuthority::Verified,
            state,
            allowed_intents: vec![AgentAuthIntent::Login, AgentAuthIntent::Logout],
            checked_at: "2026-08-30T00:00:00Z".into(),
            reason_codes: Vec::new(),
        }
    }

    fn providers(ids: &[&str]) -> AgentAuthObservationDto {
        AgentAuthObservationDto::ProviderConnections {
            contract_version: AGENT_AUTH_CONTRACT_VERSION,
            agent_id: AgentCatalogId::OpenCode,
            ownership: AgentAuthOwnership::ProviderOwned,
            authority: AgentAuthAuthority::Verified,
            state: if ids.is_empty() {
                AgentAuthProviderConnectionState::Empty
            } else {
                AgentAuthProviderConnectionState::Configured
            },
            providers: ids
                .iter()
                .map(|id| AgentAuthProviderSummaryDto {
                    provider_id: (*id).into(),
                    label: "Provider".into(),
                })
                .collect(),
            allowed_intents: vec![AgentAuthIntent::ConnectProvider, AgentAuthIntent::Logout],
            checked_at: "2026-08-30T00:00:00Z".into(),
            reason_codes: Vec::new(),
        }
    }

    fn request(agent_id: AgentCatalogId, intent: AgentAuthIntent) -> StartAgentAuthSessionRequest {
        StartAgentAuthSessionRequest {
            agent_id,
            intent,
            provider_id: None,
            inventory_id: None,
            target_id: None,
            expected_target_revision: None,
        }
    }

    #[test]
    fn store_enforces_per_agent_single_flight_and_terminal_immutability() {
        let store = AgentAuthSessionStore::new();
        let (first, _) = store
            .start(
                AgentCatalogId::ClaudeCode,
                AgentAuthIntent::Login,
                account(AgentAuthAccountState::LoggedOut),
            )
            .unwrap();
        assert_eq!(
            store
                .start(
                    AgentCatalogId::ClaudeCode,
                    AgentAuthIntent::Logout,
                    account(AgentAuthAccountState::LoggedOut),
                )
                .err(),
            Some(AgentAuthReasonCode::OperationConflict)
        );
        store
            .transition(
                &first.session_id,
                AgentAuthSessionStage::Failed,
                account(AgentAuthAccountState::LoggedOut),
                Some(AgentAuthSessionOutcome::Failed),
                Some(AgentAuthReasonCode::CommandFailed),
            )
            .unwrap();
        assert_eq!(
            store.transition(
                &first.session_id,
                AgentAuthSessionStage::Verified,
                account(AgentAuthAccountState::LoggedIn),
                Some(AgentAuthSessionOutcome::VerifiedLoggedIn),
                None,
            ),
            Err(AgentAuthReasonCode::OperationConflict)
        );
        assert!(store
            .start(
                AgentCatalogId::ClaudeCode,
                AgentAuthIntent::Login,
                account(AgentAuthAccountState::LoggedOut),
            )
            .is_ok());
    }

    #[test]
    fn active_session_can_be_recovered_by_agent_until_it_becomes_terminal() {
        let store = AgentAuthSessionStore::new();
        let (snapshot, _) = store
            .start(
                AgentCatalogId::ClaudeCode,
                AgentAuthIntent::Login,
                account(AgentAuthAccountState::LoggedOut),
            )
            .unwrap();
        assert_eq!(
            store
                .active_for_agent(AgentCatalogId::ClaudeCode)
                .map(|current| current.session_id),
            Some(snapshot.session_id.clone())
        );
        assert!(store.active_for_agent(AgentCatalogId::OpenCode).is_none());

        store
            .transition(
                &snapshot.session_id,
                AgentAuthSessionStage::Failed,
                account(AgentAuthAccountState::LoggedOut),
                Some(AgentAuthSessionOutcome::Failed),
                Some(AgentAuthReasonCode::CommandFailed),
            )
            .unwrap();
        assert!(store.active_for_agent(AgentCatalogId::ClaudeCode).is_none());
    }

    #[test]
    fn stop_waiting_is_terminal_without_claiming_external_cancellation() {
        let store = AgentAuthSessionStore::new();
        let (session, stop) = store
            .start(
                AgentCatalogId::ClaudeCode,
                AgentAuthIntent::Login,
                account(AgentAuthAccountState::LoggedOut),
            )
            .unwrap();
        store
            .transition(
                &session.session_id,
                AgentAuthSessionStage::Launching,
                account(AgentAuthAccountState::LoggedOut),
                None,
                None,
            )
            .unwrap();
        store
            .transition(
                &session.session_id,
                AgentAuthSessionStage::AwaitingUser,
                account(AgentAuthAccountState::LoggedOut),
                None,
                None,
            )
            .unwrap();
        let stopped = store.stop_waiting(&session.session_id).unwrap();
        assert_eq!(stopped.stage, AgentAuthSessionStage::Cancelled);
        assert_eq!(
            stopped.reason_code,
            Some(AgentAuthReasonCode::MonitoringStopped)
        );
        assert!(stop.load(Ordering::Acquire));
    }

    #[test]
    fn verification_requires_authoritative_target_change() {
        assert_eq!(
            verified_outcome(
                &request(AgentCatalogId::ClaudeCode, AgentAuthIntent::Login),
                &account(AgentAuthAccountState::LoggedOut),
                &account(AgentAuthAccountState::LoggedIn),
            ),
            Some(AgentAuthSessionOutcome::VerifiedLoggedIn)
        );
        let mut connect = request(AgentCatalogId::OpenCode, AgentAuthIntent::ConnectProvider);
        assert_eq!(
            verified_outcome(
                &connect,
                &providers(&[]),
                &providers(&["p1:00000000000000000000000000000000"]),
            ),
            Some(AgentAuthSessionOutcome::VerifiedProviderChange)
        );
        connect.provider_id = None;
        assert_eq!(
            verified_outcome(&connect, &providers(&[]), &providers(&[])),
            None
        );
    }

    #[test]
    fn bounded_history_evicts_terminal_sessions_behind_an_active_oldest_record() {
        let store = AgentAuthSessionStore::new();
        let (active, _) = store
            .start(
                AgentCatalogId::ClaudeCode,
                AgentAuthIntent::Login,
                account(AgentAuthAccountState::LoggedOut),
            )
            .unwrap();

        for _ in 0..(MAX_AUTH_SESSIONS + 16) {
            let (session, _) = store
                .start(
                    AgentCatalogId::GrokBuild,
                    AgentAuthIntent::Login,
                    handoff_observation(),
                )
                .unwrap();
            store
                .transition(
                    &session.session_id,
                    AgentAuthSessionStage::Launching,
                    handoff_observation(),
                    None,
                    None,
                )
                .unwrap();
            store
                .transition(
                    &session.session_id,
                    AgentAuthSessionStage::HandoffComplete,
                    handoff_observation(),
                    Some(AgentAuthSessionOutcome::HandoffOnly),
                    Some(AgentAuthReasonCode::HandoffOnly),
                )
                .unwrap();
        }

        let cache = store
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert!(cache.records.len() <= MAX_AUTH_SESSIONS);
        assert!(cache.records.contains_key(&active.session_id));
        assert_eq!(
            cache.active_by_agent.get(&AgentCatalogId::ClaudeCode),
            Some(&active.session_id)
        );
    }

    fn handoff_observation() -> AgentAuthObservationDto {
        AgentAuthObservationDto::HandoffOnly {
            contract_version: AGENT_AUTH_CONTRACT_VERSION,
            agent_id: AgentCatalogId::GrokBuild,
            ownership: AgentAuthOwnership::AgentOwned,
            authority: AgentAuthAuthority::Unverified,
            allowed_intents: vec![AgentAuthIntent::Login, AgentAuthIntent::Logout],
            checked_at: "2026-08-30T00:00:00Z".into(),
            reason_codes: vec![AgentAuthReasonCode::HandoffOnly],
        }
    }
}
