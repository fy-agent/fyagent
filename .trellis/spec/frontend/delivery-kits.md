# Project Delivery Kits Panel

`shared/features/delivery-kits-ui/ProjectDeliveryKitsPanel.tsx` exports one panel for Projects,
not a top-level route. Props include projectId/projectRevision, active, a real
DeliveryKitsPort, optional project/evidence adapters and onProjectChanged.
Root owns Projects navigation/composition and final shared-facade registration.

`domain/delivery-kits/index.ts` owns strict runtime wire parsers and closed
business-result labels; no native access or React. The independent production
factory is `shared/platform/tauri/feature-ports/delivery-kits.ts`. It invokes
literal registered commands and parses `unknown` before returning. Arbitrary
native errors never become user-facing text. Import apply requests carry only
previewId/manifestDigest; file selection/save are native, not raw path IPC.

Query uses `featureKeys.deliveryKits`; hidden panels do not mount queries/actions.
Project/revision changes recreate the session, revoke dialogs and discard late
responses. A synchronous action guard prevents duplicate click mutations.
Shared CatalogMasterDetail, FeatureList/Search, Button/Dialog own layout and
interaction. Resource text is rendered as inert text, no HTML or remote images.

Import preview shows content, origin, permissions, connection uncertainty and
conflict/compatibility. Only confirmation applies, then a catalogue reread is
required before success. Built-in or already imported immutable versions can be
shared. Export preview shows the actual content and origin; imported content
requires confirmation that it contains no customer information or credentials.
It never acquires built-in provenance or machine-check eligibility by being
shared. Actual save can be cancelled, existing files are not overwritten.
Import never assigns Agent targets or enables tools.

Weekly report UI shows native-computed cents as currency and basis points as
percentages, source row IDs and each actual failure. It never shows template
existence as online authentication/tool readiness. Starter kits have no machine
run action. Missing project/evidence adapters visibly disable those actions;
the independent native demo remains usable. No production mock-success exists.

Project adapter receives bindingIntentId and expectedRevision plus kit identity.
Evidence adapter receives only project/kit identity; root must bridge it to a
native dependency reader and validator, not persist renderer-provided results.
Until then a demo is explicitly temporary page state. Fixtures and customer
acceptance remain separate under the evidence owner's contract.

Required tests: strict/excess/malformed/identity DTOs, error canary, literal IPC
payloads and ACL, zero-write preview/cancel, confirmed import/reread, conflict,
late result after project switch, hidden surface, native-result presentation,
imported share preview/confirmation/cancellation/retry, and disabled unavailable
adapters. Fixture IPC does not prove native UI or
real connection operation.
