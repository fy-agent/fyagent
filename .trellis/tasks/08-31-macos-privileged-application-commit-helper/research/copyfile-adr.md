# Copy implementation ADR

Date: 2026-08-31
Decision: **fd-relative `openat` recursion + `fcopyfile` for regular files**. Do not use recursive Apple `copyfile` paths or `ditto`.

## Question

Can Apple `copyfile(3)` recursively copy a verified `.app` from an already-opened directory file descriptor with `O_NOFOLLOW` semantics, so a later replacement of the original path cannot change what root copies?

## What `copyfile` actually offers

- `copyfile(from, to, state, flags)` with `COPYFILE_RECURSIVE` takes **path strings**. The implementation walks the tree by path. That reintroduces TOCTOU once the caller already holds a directory FD.
- `COPYFILE_NOFOLLOW_SRC` / `COPYFILE_NOFOLLOW_DST` only affect the **root** path arguments. Nested names are still resolved as paths.
- `fcopyfile(from_fd, to_fd, state, flags)` copies **one** already-opened object. It does not recurse. Used with `COPYFILE_ALL` it can preserve data, mode, xattr, ACL, and resource forks **without a path**.
- There is no public `copyfile` API that accepts a source directory FD and recursively copies fd-relative children with `openat`/`O_NOFOLLOW`.

Converting the FD back to a path (`fcntl(F_GETPATH)`) and then calling recursive `copyfile` would copy whatever now occupies that path, which is the attack the source-capability design exists to prevent.

## Decision

1. Walk the source with `fdopendir` / `readdir` / `openat(..., O_RDONLY | O_NOFOLLOW | O_CLOEXEC)`.
2. Create destinations with `mkdirat` / `openat(..., O_CREAT | O_EXCL)` under the generated stage name.
3. Copy regular files with **`fcopyfile` + `COPYFILE_ALL`** between the two FDs (the one case where Apple's copier is FD-safe).
4. Recurse directories; reject sockets, devices, fifos, and absolute or `..`-escaping symlinks.
5. Allow only **relative, contained** symlinks (required for real `.app` framework `Versions/Current` layout). Copy them with `readlinkat` + `symlinkat` of the original relative target after a component-stack containment check.
6. Never call `ditto`, `Process`, `system`, `popen`, or a shell.

## Why not a fully custom byte copier for files

`fcopyfile` on two already-open regular-file FDs is the smallest reuse of a reviewed Apple primitive that still preserves bundle metadata. Recursion stays in FyAgent code because Apple does not expose an FD-relative recursive API.

## Test evidence required

- After `open(..., O_DIRECTORY | O_NOFOLLOW)`, replacing the original path must not change what is read or copied. Tests rename the opened directory aside and create an attacker bundle at the old path; they do not `removeItem` the tree, which would unlink children of the still-open inode.
- File / symlink / wrong-bundle FDs are rejected.
- Portable transaction tests use an injected parent directory; production still hard-codes `/Applications`.

This does not claim APFS clone/copy-on-write or Finder-equivalent metadata parity. Formal HIL remains the gate for real `/Applications` commits.
