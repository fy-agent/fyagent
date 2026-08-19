import { Button } from "./primitives";

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
  const numbers = Array.from(
    { length: totalPages },
    (_, index) => index + 1,
  ).slice(Math.max(0, page - 3), Math.min(totalPages, page + 2));
  return (
    <nav className="fy-feature-pagination" aria-label={ariaLabel}>
      {numbers.map((number) => (
        <Button
          key={number}
          aria-current={number === page ? "page" : undefined}
          onClick={() => onPageChange(number)}
        >
          {number}
        </Button>
      ))}
    </nav>
  );
}
