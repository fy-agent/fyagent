> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# 身份与回执独立复核（Grok 4.7）

范围：`session_manager/migrate/{identity.rs,identity_platform.rs,receipt.rs,export.rs}`，`database/dao/session_restore.rs`，以及 `database/schema.rs` 里的 `session_restore_attempts`。对照 `.trellis/spec/backend/session-migration.md`。未改代码。restore、reconcile、native writer、extract 仍在改，只记跨接口依赖，不把它们的现状写成终审。本轮没有重跑 Rust 测试，避免和并行编译抢构建目录。

## P0

没有发现。

## P1

预分配的 native id 写不进去时仍然返回成功。`set_restore_native_id` 只在 `target_native_id IS NULL` 时更新，执行完不看行数（`session_restore.rs:124-131`）。行不存在，或已经有另一个 id，都是 `Ok(())`。单测把第二次写成 `thread-2` 也期待成功，库里仍是 `thread-1`（`644-654`）。触发：调用方生成了新的 native id，再调用这个函数。影响：发布前的映射可以和即将写入的 id 不一致；崩溃后的回执指向旧 id，或者根本没有这行。规格要求能支持时先记下预分配 id 再发布。最小修法：更新到 1 行，或库里的 id 与本次相同，才返回成功；0 行或 id 不同返回错误。调用方不得在错误之后继续发布。

## P2

1. 请求指纹绑的是传入的选择列表，不检查行上的快照集合。`record_for_request` 把排序去重后的 `snapshot_ids` 和目标、工作区、`requestKind` 哈希进 `request_fingerprint`（`receipt.rs:75-88`）。`claim()` 只放入当前这一条快照（`59-62`）。`claim_batch` 把同一份列表写进每一行，但不要求这些 attempt 的 `snapshot_id` 集合与列表相等（`69-73`）。反例：列表是 `[A]`，批里还有快照 B。B 的指纹与 A 相同，唯一索引 `(device_binding, request_id, snapshot_id)` 挡不住 B，B 会被插入。之后用 `[A,B]` 重试，指纹不同，`claim_on_connection` 返回 `identityFieldInvalid`（`session_restore.rs:257-266`），这个 requestId 不能再代表真实的整批。最小修法：进事务前要求两边的快照集合相等，否则拒绝，不要插入。已确认这条接口；当前 restore 还没走 `claim_batch`，见下方依赖。
2. 有回执但算不出本轮认可的 store id 时，导出仍成功，并带上文件上的新 origin。`preview_session` 在 `for_native` 非空之后，只对 codex 调用 `store_identity_at`，对 opencode/hermes 调用 `store_instance_id`，其它 provider 直接 `return Ok(session)`（`export.rs:33-42`）。触发：该 native id 已有本机回执，provider 不在这个 match 里。影响：精确映射被丢掉，包里的 origin 是提取时新算的，恢复会话会被当成另一个来源。store id 对不上而走到 `inherit_origin` 后不继承，是替换 store 的既定行为（`export.rs:147-155`），不是这条。最小修法：已经查到回执却不能计算 store id 时返回错误，不要返回未继承的 session。哪些 provider 今天真能写出回执，随 writer 尚未冻结，不把它们写成已经导出错。
3. 安装哈希里的路径摘要会把同一目录 inode 看成新安装。`install_identity_locked` 把种子、机器用户、目录路径摘要和 `file_instance` 哈希在一起（`identity.rs:483-488`）。复制到新目录时 inode 已经变了，路径摘要对「复制不继承」不是必要的；`copied_configuration_directory_*` 与 `directory_replacement_*` 都换了 inode（`668-687`、`757-769`）。反例：同一目录被改名或挂到另一条路径，inode 与种子不变，路径摘要变了，`install_identity()` 变了。旧行的 `device_binding` 对不上 `belongs_here`（`receipt.rs:44-48`），新的 default slot 也不再撞上旧槽，同一 target store 可以再写一次。最小修法：安装哈希去掉路径摘要，保留种子、机器用户和目录 `file_instance`。store 侧的路径摘要不要跟着删：CLI 是按路径打开 store 的。
4. 身份锁锁的是 lock 文件描述符，随后的读写仍用路径。`with_locked_state` 打开 `identity.lock` 并 `flock`/`LockFileEx`（`identity.rs:329-358`，`identity_platform.rs:28-96`），临界区里的 `install_identity_locked` 再用这个路径做 `file_instance` 和 `atomic_write`（`identity.rs:451-475`）。反例：进程 A 已 canonicalize 并持有旧目录上的锁，期间该目录被换成新 inode、同一路径；A 的路径现在指向新目录，写的却不是自己锁住的那份。顺序替换测试覆盖的是锁外替换，没有覆盖这个窗口。最小修法：用目录 fd 做后续 open，锁住之后再核对本目录 inode；对不上就放弃这次读写。损坏 JSON 不会被重铸，这点有测试（`690-722`），不是这条。
5. 同一 native id 的来源冲突按整个 `OriginIdentity` 比较。`inherit_origin` 在 provider、store、native id 都精确命中且阶段不是 `failed` 时，用 `!=` 比较整个 origin（`export.rs:95-106`）。`OriginIdentity` 的 `PartialEq` 包含可选的 `cli_version` 和 `store_fingerprint`（`model.rs:80-94`）。反例：两条映射的 `origin_id` 相同，一条有 CLI 版本，一条没有。导出变成 `receiptStoreFailed`，不会选错来源，但本来该继承的来源被挡住。最小修法：冲突只比较 `origin_id`。

## 两个新字段

`installation_id` 和 `request_fingerprint` 都去不掉，也不是已有列的重复。

- 没有 `installation_id` 时，同一 `request_id` 可以跨 store 再占一行。唯一索引是 `(device_binding, request_id, snapshot_id)`（`schema.rs:1910-1911`）。`device_binding` 含 target store（`identity.rs:540-545`），换 store 后索引不撞。反例：请求 R 已在 store S1 落行，再用 R 去 S2。DAO 测试把 `installation_id` 保持不变、改指纹后拒绝（`session_restore.rs:823-832`）。这列是跨 store 的动作键。
- 没有 `request_fingerprint` 时，单行列里没有「整批选择」。事务里先插入快照 A 再插入 B。若用已插入的快照集合和本次请求比较，B 会把 `{A}` 看成选择被改了，整批无法提交。指纹在第一行就写上完整选择。反例：已提交 `{A,B}` 后只重试 `{A}`。只按 A 那一行相等会返回 `ExistingRequest`，丢了「B 也属于这次动作」。指纹不同则拒绝（`receipt.rs:339-351`）。不要为了去掉这列再加一张动作头表。

`content_digest` 列是摘要，不是正文。`idx_sra_digest`（`schema.rs:1917-1918`）没有查询使用，索引可以删，列不能删。

## 跨接口依赖

不作为 restore 终审结论。`restore_one` 现在调用 `ReceiptStore::claim`（`restore.rs:313`），而 `claim` 的指纹只有当前快照。同一 `requestId` 的第二张快照会在回执层被拒，第一张已经单独提交。整批要先 `claim_batch` 成功，再对返回的 claim 做 native write。`claim_restore_attempts` 本身是一个 `Immediate` 事务，提交前没有 native write（`session_restore.rs:72-83`）。

导出继承要求两边的 store id 是同一种算法：codex 必须是 `store_identity_at`（`native/codex.rs:40-45`），opencode/hermes 必须是 `store_instance_id`。算法不一致时 `inherit_origin` 会安静地不继承。没有 native id 时，`stable_random_origin_id` 只稳定到 `store_key|source_path` 这个文件（`identity.rs:136-154`，调用在 `extract/mod.rs:332-335`）。同一路径上的两条无 id 会话会共用一个 origin。extract 仍在改，这里只要求它保证这条路径一会话一文件，或改用带会话区分的键。

`set_restore_stage` 可以写成任意阶段，不看当前阶段（`session_restore.rs:146-152`）。未知结果会不会被标成 `nativeWritten`，取决于 reconcile/restore，不在本轮终审。

## 没有发现

- 复制配置不继承回执。安装哈希含机器用户和目录 inode；种子文件照抄到新目录或新用户，`install_identity()` 仍不同，且不改写种子（`identity.rs:647-687`）。`belongs_here` 对不上的行不参与列表、幂等和阶段更新（`receipt.rs:234-295`）。
- 同机 store 换成新 inode 后，store id 和这条 origin 都变（`identity.rs:726-753`）。slot 含 snapshot、provider、store、device binding（`102-119`）。新 store 不会占旧槽，旧槽也不会挡住新 store。原地覆盖同一 inode 仍视为同一 store；现有测试把「替换」定义成新文件对象。
- 同一 origin、不同 snapshot 的 default import 返回 `sourceSnapshotConflict` 并回滚已插入的同批行；`saveAsNewCopy` 不占 slot，可以再插（`session_restore.rs:280-291`、`798-820`）。同一 request 重试走 `ExistingRequest`，不会再插一行。
- 导出继承只接受 provider、store、native id 都相同的映射，正文相同但 store 或 native id 不同不继承（`export.rs:147-155`）。多条精确映射的 origin 冲突则失败，不按正文挑来源。继续对话会重算 snapshot，不改 origin（`137-144`）。
- 表只有身份、映射、阶段和错误。没有消息正文字段。`user_attestation` 不改 `stage`。
- 损坏或重复的安装/namespace JSON 拒绝并保持原字节，不重铸身份（`identity.rs:690-722`）。已有 namespace 文件却没有安装种子时也不新造种子（`461-468`）。
- 生产机器用户：macOS 用固定路径 `ioreg` 的单个非 nil `IOPlatformUUID` 加 `geteuid`；对不上就拒绝（`identity_platform.rs:109-139`）。Windows 用 MachineGuid 加交互用户 SID，复核失败则拒绝，不退回进程 SID（`142-167`）。这段生产函数包在 `not(test)` 里，本轮测试注入的是合成指纹，没有读本机标识。

## 证据限制

结论来自上述源码和其中的合成测试。没有打开用户会话，没有读密钥，没有跑真实推理，也没有在 Windows 上执行文件锁或 MachineGuid 路径。同进程第二把 `flock` 在 macOS 上是否真能挡住其它线程，本轮没有再跑 `concurrent_threads_*`；进程间测试是分开的进程。并行编译的失败不记为缺陷。
