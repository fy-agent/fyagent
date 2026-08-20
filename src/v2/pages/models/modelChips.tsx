import { CaretDownIcon } from "@phosphor-icons/react/dist/csr/CaretDown";
import { XIcon } from "@phosphor-icons/react/dist/csr/X";
import { useMemo, useState } from "react";

import { resolveModelVendorIcon } from "../../shared/assets/models";
import { classNames } from "../../shared/design-system/classNames";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { groupModelIds } from "./workBuddyModels";

export function ModelVendorIcon({
  modelId,
  ownedBy,
}: {
  modelId: string;
  ownedBy?: string | null;
}) {
  return (
    <img
      className="fy-models-chip-icon"
      src={resolveModelVendorIcon(modelId, ownedBy)}
      alt=""
      aria-hidden
    />
  );
}

export function GroupedModelChips({
  ids,
  removable = false,
  removeDisabled = false,
  onRemove,
  selectedId,
  onSelect,
  ownedByById,
  emptyLabel,
}: {
  ids: readonly string[];
  removable?: boolean;
  removeDisabled?: boolean;
  onRemove?: (modelId: string) => void;
  selectedId?: string;
  onSelect?: (modelId: string) => void;
  ownedByById?: Readonly<Record<string, string>>;
  emptyLabel: string;
}) {
  const groups = useMemo(() => groupModelIds(ids), [ids]);
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set());

  if (ids.length === 0) {
    return <p className="fy-models-muted">{emptyLabel}</p>;
  }

  return (
    <div className="fy-models-groups">
      {groups.map((group) => {
        const isCollapsed = collapsed.has(group.type);
        return (
          <section key={group.type} className="fy-models-group">
            <button
              type="button"
              className="fy-models-group-toggle"
              aria-expanded={!isCollapsed}
              aria-label={`${group.type} 分组`}
              onClick={() =>
                setCollapsed((current) => {
                  const next = new Set(current);
                  if (next.has(group.type)) next.delete(group.type);
                  else next.add(group.type);
                  return next;
                })
              }
            >
              <span>{group.type}</span>
              <span className="fy-models-group-count">{group.ids.length}</span>
              <CaretDownIcon
                className={classNames(
                  "fy-models-caret",
                  isCollapsed && "fy-models-caret-collapsed",
                )}
                size={14}
                aria-hidden
              />
            </button>
            {isCollapsed ? null : (
              <ul className="fy-models-chips">
                {group.ids.map((modelId) => {
                  const selected = selectedId === modelId;
                  const content = (
                    <>
                      <ModelVendorIcon
                        modelId={modelId}
                        ownedBy={ownedByById?.[modelId]}
                      />
                      <code>{modelId}</code>
                    </>
                  );
                  return (
                    <li
                      key={modelId}
                      className={classNames(
                        "fy-models-chip",
                        selected && "fy-models-chip-selected",
                      )}
                    >
                      {onSelect ? (
                        <button
                          type="button"
                          className="fy-models-chip-select"
                          aria-pressed={selected}
                          onClick={() => onSelect(modelId)}
                        >
                          {content}
                        </button>
                      ) : (
                        content
                      )}
                      {removable ? (
                        <button
                          type="button"
                          className="fy-models-chip-remove"
                          aria-label={`移除模型 ${modelId}`}
                          disabled={removeDisabled}
                          onClick={() => onRemove?.(modelId)}
                        >
                          <XIcon size={12} aria-hidden />
                        </button>
                      ) : null}
                    </li>
                  );
                })}
              </ul>
            )}
          </section>
        );
      })}
    </div>
  );
}

export function ModelSearchField({
  id,
  label,
  value,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="fy-control-field fy-models-search" htmlFor={id}>
      {label}
      <FeatureSearch
        id={id}
        ariaLabel={label}
        value={value}
        onValueChange={onChange}
        placeholder="按模型 ID 筛选"
      />
    </label>
  );
}
