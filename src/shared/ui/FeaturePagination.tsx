import { CaretLeftIcon } from "@phosphor-icons/react/dist/csr/CaretLeft";
import { CaretRightIcon } from "@phosphor-icons/react/dist/csr/CaretRight";

import { Button } from "./Button";

export type FeaturePaginationItem =
  | { type: "page"; page: number }
  | { type: "ellipsis"; id: "start" | "end" };

export function buildFeaturePaginationItems(
  page: number,
  totalPages: number,
): FeaturePaginationItem[] {
  if (totalPages < 1) return [];
  const current = Math.min(Math.max(page, 1), totalPages);
  if (totalPages <= 7) {
    return Array.from({ length: totalPages }, (_, index) => ({
      type: "page" as const,
      page: index + 1,
    }));
  }
  const sibling = 1;
  const left = Math.max(2, current - sibling);
  const right = Math.min(totalPages - 1, current + sibling);
  const items: FeaturePaginationItem[] = [{ type: "page", page: 1 }];
  if (left > 2) items.push({ type: "ellipsis", id: "start" });
  for (let number = left; number <= right; number += 1) {
    items.push({ type: "page", page: number });
  }
  if (right < totalPages - 1) items.push({ type: "ellipsis", id: "end" });
  items.push({ type: "page", page: totalPages });
  return items;
}

export function FeaturePagination({
  page,
  totalPages,
  onPageChange,
  ariaLabel,
}: {
  page: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  ariaLabel: string;
}) {
  if (totalPages <= 1) return null;
  const current = Math.min(Math.max(page, 1), totalPages);
  return (
    <nav className="fy-feature-pagination" aria-label={ariaLabel}>
      <p className="fy-feature-pagination-status" aria-live="polite">
        第 {current} / {totalPages} 页
      </p>
      <div className="fy-feature-pagination-controls">
        <Button
          aria-label="上一页"
          disabled={current <= 1}
          onClick={() => onPageChange(current - 1)}
        >
          <CaretLeftIcon size={14} weight="bold" aria-hidden />
          上一页
        </Button>
        {buildFeaturePaginationItems(current, totalPages).map((item) =>
          item.type === "ellipsis" ? (
            <span
              key={item.id}
              className="fy-feature-pagination-ellipsis"
              aria-hidden
            >
              …
            </span>
          ) : (
            <Button
              key={item.page}
              className={
                item.page === current
                  ? "fy-feature-pagination-page fy-control-button-primary"
                  : "fy-feature-pagination-page"
              }
              aria-current={item.page === current ? "page" : undefined}
              onClick={() => onPageChange(item.page)}
            >
              {item.page}
            </Button>
          ),
        )}
        <Button
          aria-label="下一页"
          disabled={current >= totalPages}
          onClick={() => onPageChange(current + 1)}
        >
          下一页
          <CaretRightIcon size={14} weight="bold" aria-hidden />
        </Button>
      </div>
    </nav>
  );
}
