import type {
  BindDeliveryKitRequest,
  CredentialOption,
  Customer,
  Project,
  ProjectContext,
  ProjectDependencySnapshot,
  ProjectMutation,
  ResourceKind,
  ResourceOption,
} from "../../domain/projects";

export interface ProjectsPort {
  listCustomers(): Promise<Customer[]>;
  createCustomer(name: string): Promise<Customer>;
  updateCustomer(
    customerId: string,
    expectedRevision: number,
    name: string,
    archived: boolean,
  ): Promise<Customer>;
  list(): Promise<Project[]>;
  get(projectId: string): Promise<Project>;
  create(customerId: string, name: string): Promise<Project>;
  update(
    request: ProjectMutation,
    name: string,
    archived: boolean,
  ): Promise<Project>;
  resourceOptions(): Promise<ResourceOption[]>;
  credentialOptions(): Promise<CredentialOption[]>;
  bindResource(
    request: ProjectMutation,
    resource: Pick<ResourceOption, "kind" | "agentId" | "rawId">,
    model: string | null,
  ): Promise<Project>;
  removeResource(
    request: ProjectMutation,
    resource: { kind: ResourceKind; agentId: string; rawId: string },
  ): Promise<Project>;
  bindCredential(
    request: ProjectMutation,
    credential: Pick<CredentialOption, "credentialId" | "purpose" | "consumer">,
  ): Promise<Project>;
  removeCredential(
    request: ProjectMutation,
    credentialId: string,
  ): Promise<Project>;
  getContext(projectId: string): Promise<ProjectContext>;
  writeContext(
    request: ProjectMutation,
    content: string,
    recover?: boolean,
  ): Promise<ProjectContext>;
  prepareCodex(request: ProjectMutation): Promise<ProjectContext>;
  bindDeliveryKit(request: BindDeliveryKitRequest): Promise<Project>;
  dependencySnapshot(projectId: string): Promise<ProjectDependencySnapshot>;
}

export function projectError(error: unknown): string {
  const code =
    error instanceof Error
      ? error.message
      : typeof error === "string"
        ? error
        : "";
  const messages: Record<string, string> = {
    projects_revision_conflict:
      "项目已发生变化，请刷新后重新操作。当前草稿仍保留。",
    projects_archived: "项目已归档，不能再修改。",
    projects_active_projects: "请先归档这个客户的进行中项目。",
    projects_path_rejected: "项目目录或文件已被替换，无法安全读写。",
    projects_credential_unavailable: "凭据已失效或用途不匹配，请重新选择。",
    projects_resource_missing: "资源已不存在，请刷新后重新选择。",
    projects_kit_owner_unavailable: "交付包库尚未接入，当前不能绑定包。",
    projects_platform_unavailable: "此平台尚不支持受管项目文件产出。",
    projects_context_changed:
      "项目文件已在其他位置修改，请保留外部内容并重新核对。",
    projects_context_required: "请先保存项目工作说明。",
    projects_context_unavailable: "项目文件无法回读，请检查目录后重试。",
  };
  return messages[code] ?? "无法完成操作，请刷新后重试。";
}
