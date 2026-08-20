import { NavLink } from "react-router-dom";

import { navigationItems } from "../../shared/config/navigation";
import { classNames } from "../../shared/design-system/classNames";
import { LiquidGlassLens } from "../../shared/ui/LiquidGlassLens";
import {
  SelectionLens,
  SelectionLensGroup,
} from "../../shared/ui/SelectionLens";

export function PrimaryNav() {
  return (
    <nav
      className="fy-primary-nav"
      aria-label="主导航"
      data-testid="primary-navigation"
    >
      <SelectionLensGroup
        id="primary-nav"
        className="fy-primary-nav-track"
        inset={1}
      >
        {navigationItems.map((item) => (
          <NavLink
            key={item.id}
            to={item.path}
            className={({ isActive, isPending }) =>
              classNames(
                "fy-primary-nav-item",
                isActive && "fy-primary-nav-item-selected",
                isPending && "fy-primary-nav-item-pending",
              )
            }
          >
            {({ isActive }) => {
              const label = (
                <span className="fy-primary-nav-label">{item.label}</span>
              );

              return (
                <>
                  <SelectionLens active={isActive} />
                  {isActive ? (
                    <LiquidGlassLens>{label}</LiquidGlassLens>
                  ) : (
                    label
                  )}
                </>
              );
            }}
          </NavLink>
        ))}
      </SelectionLensGroup>
    </nav>
  );
}
