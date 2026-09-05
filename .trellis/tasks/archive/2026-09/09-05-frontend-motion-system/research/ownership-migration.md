# Ownership and name review

| Previous location / candidate                                      | Current owner                                        | Reason and preserved boundary                                                                                                                           |
| ------------------------------------------------------------------ | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Button/GlassButton/IconButton in `shared/ui/primitives.tsx`        | `shared/ui/Button.tsx`                               | Native button/ref props plus one reusable press controller; no second business click or native API.                                                     |
| Dialog/ConfirmDialog in `shared/ui/primitives.tsx`                 | `shared/ui/Dialog.tsx`                               | Modal geometry, presence and focus are a cohesive owner, not a growing miscellaneous primitive file.                                                    |
| Inline toast rendering in `shared/features/provider.tsx`           | `shared/ui/ToastViewport.tsx`                        | Pure animation/announcement presentation; timing, messages and effects remain with FeatureProvider.                                                     |
| Candidate session hook alongside geometry in `dialogOrigin.ts`     | `shared/ui/useDialogState.ts`                        | Session state/key lifetime is distinct from source measurement. All new callers use the hook's named module.                                            |
| Feature-only media subscription in `shared/features/responsive.ts` | `shared/ui/useMediaQuery.ts`                         | One existing React external-store pattern serves responsive and live motion preference consumers. No new polling store.                                 |
| Broad visual-language contract receiving more motion details       | `frontend/motion-system.md` + focused existing specs | New motion contract has executable signatures and failure matrix; original visual-language name still correctly describes typography, sizing and focus. |

The shipped `Page.tsx` route convention is not itself a naming defect and was
not mechanically renamed. Unrelated business code, native commands, source
schemas, installer behavior and Git history are unchanged. Button/Dialog are
not re-exported through the old miscellaneous file; source, test and SPEC
references move to their actual owner. Root CSS class contracts remain stable
while the material/foreground layers are explicit children.

Review commands include TypeScript's actual import graph, source/type checks,
search for stale owner imports, and Trellis context validation. Historical
narrative paths in old task reviews are not rewritten as though old commits
had used new files; executable context links are repaired on final archival.

The TypeScript JSX audit classifies WorkBuddy's post-write trust notices as
automatic no-source dialogs. Ordinary Auth, model, MCP, Skills, prompt/memory
and guarded-sidebar confirmations carry explicit source refs. A source hidden
or removed before presentation correctly takes the neutral path rather than
guessing a replacement element.
