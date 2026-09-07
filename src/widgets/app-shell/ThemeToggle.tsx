import { MoonIcon } from "@phosphor-icons/react/dist/csr/Moon";
import { SunIcon } from "@phosphor-icons/react/dist/csr/Sun";
import { useAppearance } from "../../shared/features/useAppearance";
import { Button } from "../../shared/ui/Button";

export function ThemeToggle() {
  const { theme, toggle } = useAppearance();
  const label = theme === "light" ? "切换为雾蓝暗色" : "切换为清亮蓝色";
  return (
    <Button
      className="fy-theme-toggle"
      data-testid="theme-toggle"
      aria-label={label}
      title={label}
      onClick={(event) =>
        toggle(
          event.currentTarget,
          event.detail > 0 ? { x: event.clientX, y: event.clientY } : undefined,
        )
      }
    >
      {theme === "light" ? (
        <MoonIcon size={20} aria-hidden="true" />
      ) : (
        <SunIcon size={20} aria-hidden="true" />
      )}
    </Button>
  );
}
