/// Knife 5: map opened-store rows through the four frozen constructors
/// onto the three command success DTOs. Lives in crate::secret via include!
/// so private contract fields stay in-module.

fn wire_err<T, E>(_: E) -> Result<T, SecretInternalError> {
    Err(SecretInternalError::input_invalid())
}

fn parse_wire<T>(result: Result<T, WireValidationError>) -> Result<T, SecretInternalError> {
    result.map_err(|_| SecretInternalError::input_invalid())
}

fn clear_legacy_source_coverage_receipt() -> Result<LegacySourceCoverageReceipt, SecretInternalError> {
    let absent = || {
        LegacySourceDomainCoverageIdentity::checked_from_structural_inventory(
            LegacySourceInventoryRevision::checked_from_structural_generation(1)?,
            LegacySourceDomainPresence::Absent,
            0,
        )
    };
    Ok(LegacySourceCoverageReceipt {
        inventory_revision: LegacySourceInventoryRevision::checked_from_structural_generation(1)?,
        coverage_identity: CompleteLegacySourceCoverageIdentity::checked_exact_eleven_domains(
            absent()?,
            absent()?,
            absent()?,
            absent()?,
            absent()?,
            absent()?,
            absent()?,
            absent()?,
            absent()?,
            absent()?,
            absent()?,
        )?,
        current_scrubbable: CurrentLegacySourceExpectations::checked_from_codex_inventory_bridge(
            Vec::new(),
        )
        .map_err(|_| SecretInternalError::input_invalid())?,
        adjacent_blocked: Vec::new(),
    })
}

fn parse_purpose(raw: &str) -> Result<SecretPurpose, SecretInternalError> {
    match raw {
        "codexApiKey" => Ok(SecretPurpose::CodexApiKey),
        _ => Err(SecretInternalError::input_invalid()),
    }
}

fn parse_owner(row: &device_store::schema::StoredOwner) -> Result<SecretOwner, SecretInternalError> {
    let kind = match row.kind.as_str() {
        "provider" => SecretOwnerKind::Provider,
        "agent" => SecretOwnerKind::Agent,
        _ => return Err(SecretInternalError::input_invalid()),
    };
    let slot = match row.slot.as_str() {
        "primaryApiKey" => SecretSlot::PrimaryApiKey,
        _ => return Err(SecretInternalError::input_invalid()),
    };
    Ok(SecretOwner {
        kind,
        namespace: parse_wire(SecretOwnerNamespace::parse(row.namespace.clone()))?,
        owner_id: parse_wire(OwnerId::parse(row.owner_id.clone()))?,
        slot,
    })
}

fn os_keyring_backend(
    instance_id: &str,
    generation: u64,
) -> Result<SecretBackendInstanceView, SecretInternalError> {
    SecretBackendInstanceView::try_registered(
        SecretBackendKind::OsKeyring,
        parse_wire(SecretBackendInstanceId::parse(instance_id.to_string()))?,
        parse_wire(SecretBackendGeneration::parse(generation))?,
        SecretBackendAvailability::Available,
        None,
    )
}

fn os_keyring_capabilities(
    backend: &SecretBackendInstanceView,
    capability_revision: u64,
    device_binding_generation: u64,
) -> Result<SecretRecordCapabilities, SecretInternalError> {
    SecretRecordCapabilities::try_new(
        backend,
        parse_wire(CapabilityRevision::parse(capability_revision))?,
        parse_wire(DeviceBindingGeneration::parse(device_binding_generation))?,
        DeviceBinding::HostUser,
        StorageResidency::OsProtectedStore,
        SecretOperationConfirmationCapabilities {
            capture_verify: PhysicalConfirmation::Never,
            validate: PhysicalConfirmation::Never,
            resolve_for_apply: PhysicalConfirmation::Never,
            delete: PhysicalConfirmation::Never,
            revoke: PhysicalConfirmation::Never,
        },
        vec![
            SecretRuntimeConsumer::ChangePlanApply,
            SecretRuntimeConsumer::ProxyRequest,
            SecretRuntimeConsumer::UsageProbe,
            SecretRuntimeConsumer::CodingPlanUsageProbe,
            SecretRuntimeConsumer::ModelFetch,
        ],
        vec![
            SecretRuntimeSink::ProcessMemory,
            SecretRuntimeSink::ExternalConfigFile,
        ],
        true,
        false,
        BackendRevocationObservationCapability::Unsupported,
    )
}

fn map_binding_set_cas(
    cas: &device_store::schema::StoredBindingSetCas,
) -> Result<SecretBindingSetCas, SecretInternalError> {
    Ok(SecretBindingSetCas {
        revision: parse_wire(SecretBindingSetRevision::parse(cas.revision))?,
        digest: parse_wire(BindingSetDigest::parse(cas.digest.clone()))?,
        count: u32::try_from(cas.count).map_err(|_| SecretInternalError::input_invalid())?,
    })
}

fn map_owner_binding_summaries_for_ref(
    payload: &device_store::schema::StatePayload,
    secret_ref: &SecretRef,
) -> Result<Vec<SecretOwnerBindingSummary>, SecretInternalError> {
    let mut bindings = Vec::new();
    for row in &payload.owner_bindings {
        if row.state != device_store::schema::StoredBindingState::Bound {
            continue;
        }
        if row.secret_ref.as_deref() != Some(secret_ref.as_str()) {
            continue;
        }
        let owner = parse_owner(&row.owner)?;
        let binding_revision = parse_wire(SecretBindingRevision::parse(
            row.binding_revision
                .ok_or_else(SecretInternalError::input_invalid)?,
        ))?;
        bindings.push(SecretOwnerBindingSummary {
            owner,
            purpose: parse_purpose(&row.purpose)?,
            binding_revision,
            created_at: parse_wire(UtcTimestamp::parse(row.created_at.clone()))?,
            updated_at: parse_wire(UtcTimestamp::parse(row.updated_at.clone()))?,
        });
    }
    bindings.sort_by(|a, b| secret_owner_sort_key(&a.owner).cmp(&secret_owner_sort_key(&b.owner)));
    Ok(bindings)
}

fn map_secret_ref_aggregate(
    payload: &device_store::schema::StatePayload,
    row: &device_store::schema::StoredSecretRecord,
) -> Result<SecretRefAggregate, SecretInternalError> {
    let secret_ref = parse_wire(SecretRef::parse(row.secret_ref.clone()))?;
    let backend = os_keyring_backend(&row.backend_instance_id, row.backend_generation)?;
    let capabilities = os_keyring_capabilities(
        &backend,
        row.capability_revision,
        row.device_binding_generation,
    )?;
    let binding_set_cas = map_binding_set_cas(&row.binding_set_cas)?;
    // CAS count is the authority for aggregate.bindings. Seeded rows keep
    // count=0 even when an owner row exists; that owner is listed separately.
    let bindings = if binding_set_cas.count == 0 {
        Vec::new()
    } else {
        let bindings = map_owner_binding_summaries_for_ref(payload, &secret_ref)?;
        if u32::try_from(bindings.len()).ok() != Some(binding_set_cas.count) {
            return Err(SecretInternalError::input_invalid());
        }
        bindings
    };
    let (presence, availability, lock) = match (row.retirement_state, row.policy_state) {
        (
            device_store::schema::StoredRetirementState::Live,
            device_store::schema::StoredPolicyState::Active,
        ) => (SecretPresence::Present, SecretStableAvailability::Ready, None),
        (
            device_store::schema::StoredRetirementState::Stale,
            device_store::schema::StoredPolicyState::Active,
        ) => (SecretPresence::Present, SecretStableAvailability::Stale, None),
        (
            device_store::schema::StoredRetirementState::Live,
            device_store::schema::StoredPolicyState::Locked,
        ) => (
            SecretPresence::Present,
            SecretStableAvailability::Locked,
            Some(SecretLockView {
                source: SecretLockSource::FyAgentPolicy,
                locked_at: parse_wire(UtcTimestamp::parse(row.updated_at.clone()))?,
            }),
        ),
        _ => return Err(SecretInternalError::input_invalid()),
    };
    SecretRefAggregate::checked_from_authority(SecretRefAggregate {
        schema_version: SchemaVersionV1,
        secret_ref: secret_ref.clone(),
        secret_ref_display: SecretRefDisplay::derive_from(&secret_ref),
        purpose: parse_purpose(&row.purpose)?,
        record_revision: parse_wire(SecretRecordRevision::parse(row.record_revision))?,
        binding_set_cas,
        backend,
        capabilities,
        bindings,
        presence,
        availability,
        lock,
        revocation: None,
        issue: None,
        created_at: parse_wire(UtcTimestamp::parse(row.created_at.clone()))?,
        rotated_at: None,
        last_validated_at: None,
    })
}

fn map_owner_summary(
    row: &device_store::schema::StoredOwnerBindingRecord,
) -> Result<SecretOwnerCredentialSummary, SecretInternalError> {
    let owner = parse_owner(&row.owner)?;
    let binding_state = match row.state {
        device_store::schema::StoredBindingState::Unbound => {
            OwnerBindingState(OwnerBindingStateRepr::Unbound)
        }
        device_store::schema::StoredBindingState::Bound => {
            let secret_ref = parse_wire(SecretRef::parse(
                row.secret_ref
                    .clone()
                    .ok_or_else(SecretInternalError::input_invalid)?,
            ))?;
            OwnerBindingState(OwnerBindingStateRepr::Bound {
                secret_ref: secret_ref.clone(),
                secret_ref_display: SecretRefDisplay::derive_from(&secret_ref),
                binding_revision: parse_wire(SecretBindingRevision::parse(
                    row.binding_revision
                        .ok_or_else(SecretInternalError::input_invalid)?,
                ))?,
            })
        }
    };
    let coverage = clear_legacy_source_coverage_receipt()?;
    Ok(SecretOwnerCredentialSummary {
        schema_version: SchemaVersionV1,
        owner,
        purpose: parse_purpose(&row.purpose)?,
        owner_binding_revision: parse_wire(SecretOwnerBindingRevision::parse(
            row.owner_binding_revision,
        ))?,
        binding_state,
        legacy_source_coverage: LegacySourceCoverageView::checked_from_coverage_receipt(&coverage)?,
    })
}

pub(crate) fn list_secret_summaries_result_from_store(
    store: &device_store::DeviceLocalSecretStore,
    request: &ListSecretSummariesRequest,
) -> Result<ListSecretSummariesResult, SecretInternalError> {
    if request.cursor.is_some() {
        return Err(SecretInternalError::input_invalid());
    }
    let payload = store.load()?.payload;
    let mut refs = Vec::new();
    for row in &payload.secrets {
        if let Some(want) = request.secret_ref.as_ref() {
            if row.secret_ref != want.as_str() {
                continue;
            }
        }
        let aggregate = map_secret_ref_aggregate(&payload, row)?;
        if let Some(allowed) = request.availability.as_ref() {
            if !allowed.contains(&aggregate.availability) {
                continue;
            }
        }
        refs.push(aggregate);
    }
    refs.sort_by(|a, b| a.secret_ref.as_str().cmp(b.secret_ref.as_str()));

    let mut owners = Vec::new();
    for row in &payload.owner_bindings {
        match row.state {
            device_store::schema::StoredBindingState::Unbound if !request.include_unbound_owners => {
                continue;
            }
            _ => {}
        }
        if let Some(want) = request.secret_ref.as_ref() {
            if row.secret_ref.as_deref() != Some(want.as_str()) {
                continue;
            }
        }
        let summary = map_owner_summary(row)?;
        if let Some(want) = request.owner.as_ref() {
            if &summary.owner != want {
                continue;
            }
        }
        owners.push(summary);
    }
    owners.sort_by(|a, b| secret_owner_sort_key(&a.owner).cmp(&secret_owner_sort_key(&b.owner)));

    let limit = usize::from(request.limit.0);
    if refs.len() > limit {
        refs.truncate(limit);
    }
    if owners.len() > limit {
        owners.truncate(limit);
    }
    Ok(ListSecretSummariesResult {
        owners,
        refs,
        next_cursor: None,
    })
}

/// Mint a validated activation projection from D2 store fields already on
/// the candidate/record/owner rows (`comparison_policy`, `candidate_read`,
/// paired `target_owners` / `expected_bindings`, `projection_digest`).
/// Constructor/`validate_repr` failure fail-closes. An empty filtered
/// list may return Ok with zero rows.
pub(crate) fn list_secret_candidates_result_from_store(
    store: &device_store::DeviceLocalSecretStore,
    request: &ListSecretCandidatesRequest,
) -> Result<ListSecretCandidatesResult, SecretInternalError> {
    let payload = store.load()?.payload;
    let mut candidates = Vec::new();
    for row in &payload.candidates {
        if !request.include_terminal && row.state.is_terminal() {
            continue;
        }
        let record = payload
            .secrets
            .iter()
            .find(|secret| secret.secret_ref == row.secret_ref)
            .ok_or_else(SecretInternalError::input_invalid)?;
        let minted = mint_candidate_with_projection(&payload, row, record, None)?;
        if let Some(want) = request.owner.as_ref() {
            if !minted.candidate.target_owners.iter().any(|owner| owner == want) {
                continue;
            }
        }
        candidates.push(SecretCandidateWithProjection {
            candidate: minted.candidate,
            activation_projection: minted.activation_projection,
        });
    }
    candidates.sort_by(|left, right| {
        left.candidate
            .candidate_id
            .as_str()
            .cmp(right.candidate.candidate_id.as_str())
    });
    Ok(ListSecretCandidatesResult { candidates })
}

pub(crate) fn stage_secret_candidate_result_from_store(
    store: &device_store::DeviceLocalSecretStore,
    candidate_id: &SecretCandidateId,
    unbound_owner: SecretOwner,
) -> Result<StageSecretCandidateResult, SecretInternalError> {
    let payload = store.load()?.payload;
    let row = payload
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_id == candidate_id.as_str())
        .ok_or_else(SecretInternalError::input_invalid)?;
    let record = payload
        .secrets
        .iter()
        .find(|secret| secret.secret_ref == row.secret_ref)
        .ok_or_else(SecretInternalError::input_invalid)?;
    let minted = mint_candidate_with_projection(&payload, row, record, Some(unbound_owner))?;
    StageSecretCandidateResult::checked_from_candidate_snapshot(
        StageSecretCandidateResult {
            status: SecretCandidateStageStatus::Staged,
            candidate: minted.candidate,
            activation_projection: minted.activation_projection,
            impact: NullableSecretMutationImpact(None),
            audit_event_id: SecretAuditEventId::generate(),
        },
        &minted.snapshot,
    )
}

struct MintedCandidateRow {
    candidate: SecretCandidateSummary,
    activation_projection: SecretCandidateActivationProjection,
    snapshot: SecretCandidateAuthoritySnapshot,
}

fn mint_candidate_with_projection(
    payload: &device_store::schema::StatePayload,
    row: &device_store::schema::StoredCandidateRecord,
    record: &device_store::schema::StoredSecretRecord,
    unbound_owner: Option<SecretOwner>,
) -> Result<MintedCandidateRow, SecretInternalError> {
    let (target_owners, expected_bindings) =
        activation_owners_and_bindings(payload, &row.secret_ref, unbound_owner)?;
    let kind = map_candidate_kind(row.kind);
    let (comparison_policy, comparison_impact) = comparison_for_kind(kind);
    let candidate_id = parse_wire(SecretCandidateId::parse(row.candidate_id.clone()))?;
    let candidate_revision = parse_wire(SecretCandidateRevision::parse(row.candidate_revision))?;
    let secret_ref = parse_wire(SecretRef::parse(row.secret_ref.clone()))?;
    let record_revision = parse_wire(SecretRecordRevision::parse(row.record_revision))?;
    let backend_instance_id = parse_wire(SecretBackendInstanceId::parse(
        row.backend_instance_id.clone(),
    ))?;
    let backend_generation = parse_wire(SecretBackendGeneration::parse(row.backend_generation))?;
    let device_binding_generation =
        parse_wire(DeviceBindingGeneration::parse(row.device_binding_generation))?;
    let capability_revision = parse_wire(CapabilityRevision::parse(row.capability_revision))?;
    let mut projection_body = SecretCandidateActivationProjectionRepr {
            contract_version: SecretContractVersionV1::V1,
            operation: SecretCandidateActivationOperation::SecretCandidateActivation,
            candidate_id: candidate_id.clone(),
            candidate_revision,
            kind,
            comparison_policy,
            comparison_impact: comparison_impact.clone(),
            secret_ref: secret_ref.clone(),
            purpose: parse_purpose(&record.purpose)?,
            record_revision,
            backend_instance_id: backend_instance_id.clone(),
            backend_generation,
            device_binding_generation,
            capability_revision,
            target_owners: target_owners.clone(),
            expected_bindings: expected_bindings.clone(),
            legacy_sources_to_scrub:
                CurrentLegacySourceExpectations::checked_from_codex_inventory_bridge(Vec::new())
                    .map_err(|_| SecretInternalError::input_invalid())?,
            candidate_read: SecretActivationCandidateReadExpectation {
                operation: ActivationCandidateReadOperation::ResolveForApply,
                scope: ActivationCandidateReadScope::ActivationCandidateCompare,
                backend_instance_id: backend_instance_id.clone(),
                backend_generation,
                device_binding_generation,
                capability_revision,
                confirmation: PhysicalConfirmation::Never,
            },
            old_record_delete: SecretActivationOldRecordDeleteExpectation::NotApplicable,
            projection_digest: parse_wire(SecretProjectionDigest::parse("cd".repeat(32)))?,
        };
    projection_body.projection_digest = hash_candidate_activation_projection(&projection_body)
        .map_err(|_| SecretInternalError::input_invalid())?;
    let projection = SecretCandidateActivationProjection::validate_repr(projection_body)
        .map_err(|_| SecretInternalError::input_invalid())?;
    let backend = os_keyring_backend(&row.backend_instance_id, row.backend_generation)?;
    let capabilities = os_keyring_capabilities(
        &backend,
        row.capability_revision,
        row.device_binding_generation,
    )?;
    let summary = SecretCandidateSummary {
        schema_version: SchemaVersionV1,
        candidate_id: candidate_id.clone(),
        candidate_revision,
        kind,
        comparison_policy,
        comparison_impact,
        state: map_candidate_state(row.state),
        secret_ref: secret_ref.clone(),
        secret_ref_display: SecretRefDisplay::derive_from(&secret_ref),
        purpose: parse_purpose(&record.purpose)?,
        record_revision,
        backend,
        capabilities,
        target_owners: target_owners.clone(),
        expected_bindings: expected_bindings,
        legacy_sources_to_scrub: projection.0.legacy_sources_to_scrub.clone(),
        created_at: parse_wire(UtcTimestamp::parse(row.created_at.clone()))?,
        expires_at: parse_wire(UtcTimestamp::parse(row.expires_at.clone()))?,
        pending_terminal_disposition: row
            .pending_terminal_disposition
            .map(map_terminal_disposition),
        issue: None,
    };
    let affected_owners = map_owner_binding_summaries_for_ref(payload, &secret_ref)?;
    let snapshot = SecretCandidateAuthoritySnapshot::from_staged(
        candidate_id,
        candidate_revision,
        kind,
        comparison_policy,
        secret_ref,
        record_revision,
        projection.clone(),
        map_binding_set_cas(&record.binding_set_cas)?,
        affected_owners,
    )?;
    let minted = SecretCandidateWithProjection::checked_from_candidate_snapshot(
        SecretCandidateWithProjection {
            candidate: summary,
            activation_projection: projection,
        },
        &snapshot,
    )?;
    Ok(MintedCandidateRow {
        candidate: minted.candidate,
        activation_projection: minted.activation_projection,
        snapshot,
    })
}

fn activation_owners_and_bindings(
    payload: &device_store::schema::StatePayload,
    secret_ref: &str,
    unbound_owner: Option<SecretOwner>,
) -> Result<(Vec<SecretOwner>, Vec<OwnerBindingExpectation>), SecretInternalError> {
    match journal_target_owners_and_bindings(payload, secret_ref) {
        Ok((owners, bindings)) => Ok((owners.0, bindings.0)),
        Err(error) => {
            let has_row = payload.owner_bindings.iter().any(|row| {
                row.secret_ref.as_deref() == Some(secret_ref)
            });
            let Some(owner) = unbound_owner else {
                return Err(error);
            };
            if has_row {
                return Err(error);
            }
            Ok((
                vec![owner.clone()],
                vec![OwnerBindingExpectation::Unbound {
                    owner,
                    owner_binding_revision: parse_wire(SecretOwnerBindingRevision::parse(1))?,
                }],
            ))
        }
    }
}

fn map_candidate_state(
    state: device_store::schema::StoredCandidateState,
) -> SecretCandidateState {
    match state {
        device_store::schema::StoredCandidateState::VerifiedPendingPlan => {
            SecretCandidateState::VerifiedPendingPlan
        }
        device_store::schema::StoredCandidateState::Activated => SecretCandidateState::Activated,
        device_store::schema::StoredCandidateState::Discarded => SecretCandidateState::Discarded,
        device_store::schema::StoredCandidateState::CleanupRequired => {
            SecretCandidateState::CleanupRequired
        }
        device_store::schema::StoredCandidateState::Expired => SecretCandidateState::Expired,
    }
}

fn map_terminal_disposition(
    disposition: device_store::schema::TerminalDisposition,
) -> CandidateTerminalState {
    match disposition {
        device_store::schema::TerminalDisposition::Discarded => CandidateTerminalState::Discarded,
        device_store::schema::TerminalDisposition::Expired => CandidateTerminalState::Expired,
    }
}

fn map_candidate_kind(
    kind: device_store::schema::StoredCandidateKind,
) -> SecretCandidateKind {
    match kind {
        device_store::schema::StoredCandidateKind::NewBinding => SecretCandidateKind::NewBinding,
        device_store::schema::StoredCandidateKind::ReplaceBinding => {
            SecretCandidateKind::ReplaceBinding
        }
        device_store::schema::StoredCandidateKind::RotateBindingSet => {
            SecretCandidateKind::RotateBindingSet
        }
        device_store::schema::StoredCandidateKind::LegacyReconcile => {
            SecretCandidateKind::LegacyReconcile
        }
        device_store::schema::StoredCandidateKind::LegacyScrubExistingBinding => {
            SecretCandidateKind::LegacyScrubExistingBinding
        }
    }
}

fn comparison_for_kind(
    kind: SecretCandidateKind,
) -> (LegacyActivationComparisonPolicy, LegacyActivationComparisonImpact) {
    match kind {
        SecretCandidateKind::LegacyScrubExistingBinding
        | SecretCandidateKind::LegacyReconcile => (
            LegacyActivationComparisonPolicy::CandidateEquality,
            LegacyActivationComparisonImpact::CandidateEquality {
                user_meaning: VerifySameValueMigrationMeaning::VerifySameValueMigration,
            },
        ),
        _ => (
            LegacyActivationComparisonPolicy::ExplicitReplacement,
            LegacyActivationComparisonImpact::ExplicitReplacement {
                user_meaning: ReplaceExistingCredentialMeaning::ReplaceExistingCredential,
                affected_source_count: 0,
                replaces_bound_binding: false,
            },
        ),
    }
}

fn journal_target_owners_and_bindings(
    payload: &device_store::schema::StatePayload,
    secret_ref: &str,
) -> Result<
    (
        NonEmptySortedJournalTargetOwners,
        NonEmptySortedJournalBindingExpectations,
    ),
    SecretInternalError,
> {
    let mut owners = Vec::new();
    let mut bindings = Vec::new();
    for row in &payload.owner_bindings {
        if row.secret_ref.as_deref() != Some(secret_ref)
            && row.state != device_store::schema::StoredBindingState::Unbound
        {
            continue;
        }
        if row.secret_ref.as_deref() != Some(secret_ref)
            && row.state == device_store::schema::StoredBindingState::Unbound
        {
            continue;
        }
        let owner = parse_owner(&row.owner)?;
        let expectation = match row.state {
            device_store::schema::StoredBindingState::Unbound => {
                OwnerBindingExpectation::Unbound {
                    owner: owner.clone(),
                    owner_binding_revision: parse_wire(SecretOwnerBindingRevision::parse(
                        row.owner_binding_revision,
                    ))?,
                }
            }
            device_store::schema::StoredBindingState::Bound => OwnerBindingExpectation::Bound {
                owner: owner.clone(),
                secret_ref: parse_wire(SecretRef::parse(
                    row.secret_ref
                        .clone()
                        .ok_or_else(SecretInternalError::input_invalid)?,
                ))?,
                owner_binding_revision: parse_wire(SecretOwnerBindingRevision::parse(
                    row.owner_binding_revision,
                ))?,
                binding_revision: parse_wire(SecretBindingRevision::parse(
                    row.binding_revision
                        .ok_or_else(SecretInternalError::input_invalid)?,
                ))?,
                source_binding_set: map_binding_set_cas(
                    &payload
                        .secrets
                        .iter()
                        .find(|secret| secret.secret_ref == secret_ref)
                        .ok_or_else(SecretInternalError::input_invalid)?
                        .binding_set_cas,
                )?,
            },
        };
        owners.push(owner);
        bindings.push(expectation);
    }
    if owners.is_empty() {
        return Err(SecretInternalError::input_invalid());
    }
    let mut paired: Vec<(SecretOwner, OwnerBindingExpectation)> =
        owners.into_iter().zip(bindings).collect();
    paired.sort_by(|a, b| {
        secret_owner_sort_key(&a.0).cmp(&secret_owner_sort_key(&b.0))
    });
    let (owners, bindings): (Vec<_>, Vec<_>) = paired.into_iter().unzip();
    Ok((
        NonEmptySortedJournalTargetOwners(owners),
        NonEmptySortedJournalBindingExpectations(bindings),
    ))
}

fn candidate_delete_journal_from_store_and_envelope(
    payload: &device_store::schema::StatePayload,
    candidate: &device_store::schema::StoredCandidateRecord,
    record: &device_store::schema::StoredSecretRecord,
    envelope: &device_store::schema::JournalEnvelope,
) -> Result<CandidateDeleteJournalRow, SecretInternalError> {
    let device_store::schema::JournalEnvelope::DiscardCandidate {
        attempt,
        terminal_disposition,
        phase,
        device_instance_id,
        ..
    } = envelope
    else {
        return Err(SecretInternalError::input_invalid());
    };
    let device_store::schema::DiscardCandidatePhase::Terminal {
        terminal_disposition: phase_disposition,
    } = phase
    else {
        return Err(SecretInternalError::input_invalid());
    };
    if phase_disposition != terminal_disposition {
        return Err(SecretInternalError::input_invalid());
    }
    let terminal = match terminal_disposition {
        device_store::schema::TerminalDisposition::Discarded => CandidateTerminalState::Discarded,
        device_store::schema::TerminalDisposition::Expired => CandidateTerminalState::Expired,
    };
    let candidate_id = parse_wire(SecretCandidateId::parse(candidate.candidate_id.clone()))?;
    let kind = map_candidate_kind(candidate.kind);
    let (comparison_policy, comparison_impact) = comparison_for_kind(kind);
    let (target_owners, expected_bindings) =
        journal_target_owners_and_bindings(payload, &candidate.secret_ref)?;
    Ok(CandidateDeleteJournalRow {
        attempt: JournalAttempt::checked(*attempt)?,
        expected_store_revision: SecretStoreRevision::parse(payload.store_revision)?,
        terminal_disposition: terminal,
        candidate: JournalCandidateIdentity {
            candidate_id,
            candidate_revision: parse_wire(SecretCandidateRevision::parse(
                candidate.candidate_revision,
            ))?,
            candidate_kind: kind,
            comparison_policy,
            comparison_impact,
        },
        target_owners,
        expected_bindings,
        record: JournalBackendIdentity {
            device_instance_id: parse_wire(DeviceInstanceId::parse(device_instance_id.clone()))?,
            secret_ref: parse_wire(SecretRef::parse(record.secret_ref.clone()))?,
            record_revision: parse_wire(SecretRecordRevision::parse(record.record_revision))?,
            binding_set_cas: map_binding_set_cas(&record.binding_set_cas)?,
            backend_instance_id: parse_wire(SecretBackendInstanceId::parse(
                record.backend_instance_id.clone(),
            ))?,
            backend_generation: parse_wire(SecretBackendGeneration::parse(
                record.backend_generation,
            ))?,
            device_binding_generation: parse_wire(DeviceBindingGeneration::parse(
                record.device_binding_generation,
            ))?,
            capability_revision: parse_wire(CapabilityRevision::parse(record.capability_revision))?,
            confirmation: PhysicalConfirmation::Never,
        },
        delete_slot: CandidateDiscardConfirmationSlot::RecordDelete,
        missing_readback_slot: CandidateDiscardConfirmationSlot::RecordMissingReadback,
        delete_confirmation: PhysicalConfirmation::Never,
        missing_readback_confirmation: PhysicalConfirmation::Never,
        phase: DiscardCandidateJournalPhase::Terminal {
            terminal_disposition: terminal,
        },
    })
}

pub(crate) fn discard_secret_candidate_result_from_store(
    store: &device_store::DeviceLocalSecretStore,
    request: &DiscardSecretCandidateRequest,
    backend: Option<&testing::InMemorySecretBackend>,
) -> Result<DiscardSecretCandidateResult, SecretInternalError> {
    let backend = backend.ok_or_else(SecretInternalError::input_invalid)?;
    let payload = store.load()?.payload;
    let candidate = payload
        .candidates
        .iter()
        .find(|row| row.candidate_id == request.candidate_id.as_str())
        .cloned()
        .ok_or_else(SecretInternalError::input_invalid)?;
    let record = payload
        .secrets
        .iter()
        .find(|row| row.secret_ref == candidate.secret_ref)
        .cloned()
        .ok_or_else(SecretInternalError::input_invalid)?;
    let outcome = service::discard_secret_candidate_in_store(
        store,
        request.candidate_id.as_str(),
        request.expected_candidate_revision.get(),
        backend,
    )?;
    let service::LocalDiscardOutcome::Discarded { .. } = outcome else {
        return Err(SecretInternalError::input_invalid());
    };
    let journals = device_store::journal::list_journals(store.root())
        .map_err(|_| SecretInternalError::input_invalid())?;
    let envelope = journals
        .iter()
        .rev()
        .find(|row| {
            matches!(
                row,
                device_store::schema::JournalEnvelope::DiscardCandidate {
                    phase: device_store::schema::DiscardCandidatePhase::Terminal { .. },
                    ..
                }
            )
        })
        .ok_or_else(SecretInternalError::input_invalid)?;
    let journal = candidate_delete_journal_from_store_and_envelope(
        &payload,
        &candidate,
        &record,
        envelope,
    )?;
    DiscardSecretCandidateResult::checked_from_candidate_journal(
        DiscardSecretCandidateResultRepr::Discarded {
            terminal_state: DiscardedCandidateTerminalState::Discarded,
            candidate_id: request.candidate_id.clone(),
            audit_event_id: SecretAuditEventId::generate(),
        },
        &journal,
    )
}

#[cfg(test)]
pub(crate) fn secret_discard_result_from_mismatched_journal_is_err() -> bool {
    let candidate_id = SecretCandidateId::generate();
    let other = SecretCandidateId::generate();
    let owner = SecretOwner {
        kind: SecretOwnerKind::Provider,
        namespace: SecretOwnerNamespace::parse("codex".to_string()).expect("namespace"),
        owner_id: OwnerId::parse("owner-mismatch".to_string()).expect("owner"),
        slot: SecretSlot::PrimaryApiKey,
    };
    let journal = CandidateDeleteJournalRow {
        attempt: JournalAttempt::checked(1).expect("attempt"),
        expected_store_revision: SecretStoreRevision::parse(1).expect("store"),
        terminal_disposition: CandidateTerminalState::Discarded,
        candidate: JournalCandidateIdentity {
            candidate_id: other,
            candidate_revision: SecretCandidateRevision::parse(1).expect("candidate"),
            candidate_kind: SecretCandidateKind::NewBinding,
            comparison_policy: LegacyActivationComparisonPolicy::ExplicitReplacement,
            comparison_impact: LegacyActivationComparisonImpact::ExplicitReplacement {
                user_meaning: ReplaceExistingCredentialMeaning::ReplaceExistingCredential,
                affected_source_count: 0,
                replaces_bound_binding: false,
            },
        },
        target_owners: NonEmptySortedJournalTargetOwners(vec![owner.clone()]),
        expected_bindings: NonEmptySortedJournalBindingExpectations(vec![
            OwnerBindingExpectation::Unbound {
                owner,
                owner_binding_revision: SecretOwnerBindingRevision::parse(1).expect("rev"),
            },
        ]),
        record: JournalBackendIdentity {
            device_instance_id: DeviceInstanceId::generate(),
            secret_ref: SecretRef::generate(),
            record_revision: SecretRecordRevision::parse(1).expect("record"),
            binding_set_cas: SecretBindingSetCas {
                revision: SecretBindingSetRevision::parse(1).expect("binding set"),
                digest: BindingSetDigest::parse("ab".repeat(32)).expect("digest"),
                count: 0,
            },
            backend_instance_id: SecretBackendInstanceId::generate(),
            backend_generation: SecretBackendGeneration::parse(1).expect("generation"),
            device_binding_generation: DeviceBindingGeneration::parse(1).expect("device"),
            capability_revision: CapabilityRevision::parse(1).expect("capability"),
            confirmation: PhysicalConfirmation::Never,
        },
        delete_slot: CandidateDiscardConfirmationSlot::RecordDelete,
        missing_readback_slot: CandidateDiscardConfirmationSlot::RecordMissingReadback,
        delete_confirmation: PhysicalConfirmation::Never,
        missing_readback_confirmation: PhysicalConfirmation::Never,
        phase: DiscardCandidateJournalPhase::Terminal {
            terminal_disposition: CandidateTerminalState::Discarded,
        },
    };
    DiscardSecretCandidateResult::checked_from_candidate_journal(
        DiscardSecretCandidateResultRepr::Discarded {
            terminal_state: DiscardedCandidateTerminalState::Discarded,
            candidate_id,
            audit_event_id: SecretAuditEventId::generate(),
        },
        &journal,
    )
    .is_err()
}

pub(crate) fn check_secret_apply_readiness_from_store(
    store: &device_store::DeviceLocalSecretStore,
    request: &CheckSecretApplyReadinessRequest,
) -> Result<SecretApplyReadiness, SecretInternalError> {
    let (owner, consumer, target_sink, live_sink_id, rollback) = match request {
        CheckSecretApplyReadinessRequest::Target {
            owner,
            consumer,
            target_sink,
            live_sink_id,
            ..
        } => (owner, consumer, target_sink, live_sink_id, false),
        CheckSecretApplyReadinessRequest::Rollback {
            owner,
            consumer,
            target_sink,
            live_sink_id,
            ..
        } => (owner, consumer, target_sink, live_sink_id, true),
    };
    let consumer = match consumer {
        SecretConsumer::ChangePlanApply => SecretChangePlanApplyConsumer::ChangePlanApply,
        _ => return Err(SecretInternalError::input_invalid()),
    };
    let target_sink = match target_sink {
        ApplyTargetSink::ExternalConfigFile => SecretChangePlanApplySink::ExternalConfigFile,
        _ => return Err(SecretInternalError::input_invalid()),
    };
    let payload = store.load()?.payload;
    let binding = payload
        .owner_bindings
        .iter()
        .find(|row| parse_owner(&row.owner).ok().as_ref() == Some(owner))
        .ok_or_else(SecretInternalError::input_invalid)?;
    if binding.state != device_store::schema::StoredBindingState::Bound {
        return Err(SecretInternalError::input_invalid());
    }
    let secret_ref_raw = binding
        .secret_ref
        .clone()
        .ok_or_else(SecretInternalError::input_invalid)?;
    let record = payload
        .secrets
        .iter()
        .find(|row| row.secret_ref == secret_ref_raw)
        .ok_or_else(SecretInternalError::input_invalid)?;
    let secret_ref = parse_wire(SecretRef::parse(secret_ref_raw))?;
    let binding_set_cas = map_binding_set_cas(&record.binding_set_cas)?;
    let owner_binding_revision = parse_wire(SecretOwnerBindingRevision::parse(
        binding.owner_binding_revision,
    ))?;
    let binding_revision = parse_wire(SecretBindingRevision::parse(
        binding
            .binding_revision
            .ok_or_else(SecretInternalError::input_invalid)?,
    ))?;
    let record_revision = parse_wire(SecretRecordRevision::parse(record.record_revision))?;
    let backend_instance_id = parse_wire(SecretBackendInstanceId::parse(
        record.backend_instance_id.clone(),
    ))?;
    let backend_generation = parse_wire(SecretBackendGeneration::parse(record.backend_generation))?;
    let device_binding_generation = parse_wire(DeviceBindingGeneration::parse(
        record.device_binding_generation,
    ))?;
    let capability_revision = parse_wire(CapabilityRevision::parse(record.capability_revision))?;
    let target = SecretApplyTargetProjection::validate_repr(SecretApplyTargetProjectionRepr {
        role: SecretApplyTargetRole::Target,
        consumer,
        target_sink,
        live_sink_id: *live_sink_id,
        owner: owner.clone(),
        secret_ref: secret_ref.clone(),
        owner_binding_revision,
        binding_revision,
        record_revision,
        binding_set_cas: binding_set_cas.clone(),
        backend_instance_id: backend_instance_id.clone(),
        backend_generation,
        device_binding_generation,
        capability_revision,
    })
    .map_err(|_| SecretInternalError::input_invalid())?;
    let (credential, plan) = if rollback {
        let rollback_proj = SecretApplyRollbackProjection::validate_repr(
            SecretApplyRollbackProjectionRepr {
                role: SecretApplyRollbackRole::Rollback,
                consumer,
                target_sink,
                live_sink_id: *live_sink_id,
                owner: owner.clone(),
                secret_ref: secret_ref.clone(),
                owner_binding_revision,
                binding_revision,
                record_revision,
                binding_set_cas: binding_set_cas.clone(),
                backend_instance_id: backend_instance_id.clone(),
                backend_generation,
                device_binding_generation,
                capability_revision,
            },
        )
        .map_err(|_| SecretInternalError::input_invalid())?;
        let plan = mint_apply_plan_projection(target.clone(), Some(rollback_proj.clone()))
            .map_err(|_| SecretInternalError::input_invalid())?;
        (SecretApplyCredentialProjection::Rollback(rollback_proj), plan)
    } else {
        let plan = mint_apply_plan_projection(target.clone(), None)
            .map_err(|_| SecretInternalError::input_invalid())?;
        (SecretApplyCredentialProjection::Target(target), plan)
    };
    let checked_at = parse_wire(UtcTimestamp::parse(device_store::utc_now()))?;
    let expires_at = parse_wire(UtcTimestamp::parse(
        "2099-01-01T00:00:00.000Z".to_string(),
    ))?;
    let readiness = SecretApplyReadiness::checked_from_authority(SecretApplyReadinessRepr::Ready {
        context: SecretApplyReadinessContext {
            schema_version: SchemaVersionV1,
            operation_id: SecretOperationId::generate(),
            projection: credential,
            checked_at,
            expires_at,
        },
    })?;
    let _plan = plan;
    Ok(readiness)
}


pub(crate) fn set_secret_locked_from_store(
    store: &device_store::DeviceLocalSecretStore,
    request: &SetSecretLockedRequest,
) -> Result<SecretMutationResult, SecretInternalError> {
    let mut payload = store.load()?.payload;
    let idx = payload
        .secrets
        .iter()
        .position(|row| row.secret_ref == request.secret_ref.as_str())
        .ok_or_else(SecretInternalError::input_invalid)?;
    let current_cas = map_binding_set_cas(&payload.secrets[idx].binding_set_cas)?;
    let current_rev = parse_wire(SecretRecordRevision::parse(
        payload.secrets[idx].record_revision,
    ))?;
    if current_rev != request.expected_record_revision
        || current_cas != request.expected_binding_set
    {
        return Err(SecretInternalError::input_invalid());
    }
    let now = device_store::utc_now();
    {
        let row = &mut payload.secrets[idx];
        row.policy_state = if request.locked {
            device_store::schema::StoredPolicyState::Locked
        } else {
            device_store::schema::StoredPolicyState::Active
        };
        row.record_revision = row.record_revision.saturating_add(1);
        row.updated_at = now;
    }
    payload.store_revision = payload.store_revision.saturating_add(1);
    let stored = payload.clone();
    store.store(payload)?;
    let row = stored
        .secrets
        .iter()
        .find(|row| row.secret_ref == request.secret_ref.as_str())
        .ok_or_else(SecretInternalError::input_invalid)?;
    let aggregate = map_secret_ref_aggregate(&stored, row)?;
    SecretMutationResult::checked_from_authority(SecretMutationResult {
        aggregate,
        audit_event_id: SecretAuditEventId::generate(),
    })
}


pub(crate) fn get_secret_delete_impact_from_store(
    store: &device_store::DeviceLocalSecretStore,
    request: &GetSecretDeleteImpactRequest,
) -> Result<SecretDeleteImpact, SecretInternalError> {
    let payload = store.load()?.payload;
    let row = payload
        .secrets
        .iter()
        .find(|row| row.secret_ref == request.secret_ref.as_str())
        .ok_or_else(SecretInternalError::input_invalid)?;
    let binding_set_cas = map_binding_set_cas(&row.binding_set_cas)?;
    let record_revision = parse_wire(SecretRecordRevision::parse(row.record_revision))?;
    let affected_owners = if binding_set_cas.count == 0 {
        Vec::new()
    } else {
        let owners = map_owner_binding_summaries_for_ref(&payload, &request.secret_ref)?;
        if u32::try_from(owners.len()).ok() != Some(binding_set_cas.count) {
            return Err(SecretInternalError::input_invalid());
        }
        owners
    };
    let effect = if binding_set_cas.count <= 1 {
        SecretImpactEffect::OneBindingAffected
    } else {
        SecretImpactEffect::AllBindingsAffected
    };
    let checked_at = parse_wire(UtcTimestamp::parse(device_store::utc_now()))?;
    let expires_at = parse_wire(UtcTimestamp::parse(
        "2099-01-01T00:00:00.000Z".to_string(),
    ))?;
    Ok(SecretDeleteImpact {
        impact: SecretMutationImpact {
            schema_version: SchemaVersionV1,
            secret_ref: request.secret_ref.clone(),
            secret_ref_display: SecretRefDisplay::derive_from(&request.secret_ref),
            record_revision,
            binding_set_cas: binding_set_cas.clone(),
            affected_owners,
            effect,
            no_fallback: AlwaysTrue,
        },
        readiness: SecretDeleteReadiness::Ready {
            context: SecretDeleteReadinessContext {
                schema_version: SchemaVersionV1,
                operation_id: SecretOperationId::generate(),
                operation: SecretDeleteOperation::Delete,
                secret_ref: request.secret_ref.clone(),
                record_revision,
                binding_set_cas,
                checked_at,
                expires_at,
            },
        },
    })
}


pub(crate) fn validate_secret_from_store(
    store: &device_store::DeviceLocalSecretStore,
    request: &ValidateSecretRequest,
) -> Result<SecretValidationResult, SecretInternalError> {
    let payload = store.load()?.payload;
    let row = payload
        .secrets
        .iter()
        .find(|row| row.secret_ref == request.secret_ref.as_str())
        .ok_or_else(SecretInternalError::input_invalid)?;
    let current_rev = parse_wire(SecretRecordRevision::parse(row.record_revision))?;
    if current_rev != request.expected_record_revision {
        return Err(SecretInternalError::input_invalid());
    }
    let aggregate = map_secret_ref_aggregate(&payload, row)?;
    SecretValidationResult::checked_from_authority(SecretValidationResult {
        outcome: SecretValidationOutcome::Valid,
        aggregate,
        audit_event_id: SecretAuditEventId::generate(),
    })
}

#[allow(dead_code)]
fn _keep_wire_err<T, E>(err: E) -> Result<T, SecretInternalError> {
    wire_err(err)
}
