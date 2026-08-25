import { CaretDownIcon } from "@phosphor-icons/react/dist/csr/CaretDown";
import { useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { NavLink, useLocation } from "react-router-dom";

import {
  navigationGroups,
  type NavigationItem,
} from "../../shared/config/navigation";
import { classNames } from "../../shared/design-system/classNames";
import { LiquidGlassLens } from "../../shared/ui/LiquidGlassLens";
import {
  SelectionLens,
  SelectionLensGroup,
} from "../../shared/ui/SelectionLens";

const configurationItemsId = "fy-side-navigation-configuration-items";

function NavigationLink({
  item,
  visuallyAvailable = true,
}: {
  item: NavigationItem;
  visuallyAvailable?: boolean;
}) {
  return (
    <NavLink
      to={item.path}
      end
      className={({ isActive, isPending }) =>
        classNames(
          "fy-side-navigation-item",
          isActive && "fy-side-navigation-item-selected",
          isPending && "fy-side-navigation-item-pending",
        )
      }
    >
      {({ isActive }) => {
        const visuallyActive = isActive && visuallyAvailable;
        const label = (
          <span className="fy-side-navigation-item-label">{item.label}</span>
        );

        return (
          <>
            <SelectionLens active={visuallyActive} />
            {visuallyActive ? (
              <LiquidGlassLens className="fy-side-navigation-liquid-lens">
                {label}
              </LiquidGlassLens>
            ) : (
              label
            )}
          </>
        );
      }}
    </NavLink>
  );
}

function visibleNavigationControls(navigation: HTMLElement): HTMLElement[] {
  return Array.from(
    navigation.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  ).filter((control) => control.closest("[hidden]") === null);
}

export function SideNavigation() {
  const { pathname } = useLocation();
  const navigationRef = useRef<HTMLElement>(null);
  const configurationToggleRef = useRef<HTMLButtonElement>(null);
  const configurationItemsRef = useRef<HTMLUListElement>(null);
  const [configurationExpanded, setConfigurationExpanded] = useState(true);

  const handleKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;

    if (target === configurationToggleRef.current) {
      if (event.key === "ArrowRight") {
        event.preventDefault();
        if (configurationExpanded) {
          configurationItemsRef.current
            ?.querySelector<HTMLElement>("a[href]")
            ?.focus();
        } else {
          setConfigurationExpanded(true);
        }
        return;
      }

      if (event.key === "ArrowLeft" || event.key === "Escape") {
        event.preventDefault();
        setConfigurationExpanded(false);
        return;
      }
    }

    if (
      (event.key === "ArrowLeft" || event.key === "Escape") &&
      target.closest(`#${configurationItemsId}`)
    ) {
      event.preventDefault();
      setConfigurationExpanded(false);
      configurationToggleRef.current?.focus();
      return;
    }

    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
      return;
    }

    const navigation = navigationRef.current;
    if (!navigation) return;
    const controls = visibleNavigationControls(navigation);
    const currentIndex = controls.indexOf(target);
    if (currentIndex < 0 || controls.length === 0) return;

    event.preventDefault();
    if (event.key === "Home") {
      controls[0]?.focus();
      return;
    }
    if (event.key === "End") {
      controls.at(-1)?.focus();
      return;
    }

    const offset = event.key === "ArrowDown" ? 1 : -1;
    const nextIndex =
      (currentIndex + offset + controls.length) % controls.length;
    controls[nextIndex]?.focus();
  };

  return (
    <nav
      ref={navigationRef}
      className="fy-side-navigation"
      aria-label="主导航"
      data-testid="side-navigation"
      onKeyDown={handleKeyDown}
    >
      <SelectionLensGroup
        id="side-navigation"
        className="fy-side-navigation-track"
        inset={1}
      >
        {navigationGroups.map((group) => {
          const groupLabelId = `fy-side-navigation-${group.id}-label`;
          const { collapsible } = group;
          const groupActive = group.items.some(
            (item) => item.path === pathname,
          );
          let groupLabel: ReactNode;

          if (collapsible) {
            const content = (
              <span className="fy-side-navigation-toggle-content">
                <span>{group.label}</span>
                <CaretDownIcon
                  className="fy-side-navigation-caret"
                  size={16}
                  weight="bold"
                  aria-hidden
                  data-testid="configuration-management-caret"
                />
              </span>
            );
            const visuallyActive = groupActive && !configurationExpanded;
            groupLabel = (
              <button
                ref={configurationToggleRef}
                className={classNames(
                  "fy-side-navigation-toggle",
                  groupActive && "fy-side-navigation-toggle-active",
                )}
                id={groupLabelId}
                type="button"
                aria-expanded={configurationExpanded}
                aria-controls={configurationItemsId}
                data-active={groupActive || undefined}
                data-testid="configuration-management-toggle"
                onClick={() =>
                  setConfigurationExpanded((expanded) => !expanded)
                }
              >
                <SelectionLens active={visuallyActive} />
                {visuallyActive ? (
                  <LiquidGlassLens className="fy-side-navigation-liquid-lens">
                    {content}
                  </LiquidGlassLens>
                ) : (
                  content
                )}
              </button>
            );
          } else {
            groupLabel = (
              <h2 className="fy-side-navigation-group-title" id={groupLabelId}>
                {group.label}
              </h2>
            );
          }

          return (
            <section
              className={classNames(
                "fy-side-navigation-group",
                collapsible && "fy-side-navigation-group-collapsible",
              )}
              aria-labelledby={groupLabelId}
              data-navigation-group={group.id}
              key={group.id}
            >
              {groupLabel}
              <ul
                ref={collapsible ? configurationItemsRef : undefined}
                className="fy-side-navigation-items"
                id={collapsible ? configurationItemsId : undefined}
                hidden={collapsible && !configurationExpanded}
                data-testid={
                  collapsible ? "configuration-management-items" : undefined
                }
              >
                {group.items.map((item) => (
                  <li key={item.id}>
                    <NavigationLink
                      item={item}
                      visuallyAvailable={!collapsible || configurationExpanded}
                    />
                  </li>
                ))}
              </ul>
            </section>
          );
        })}
      </SelectionLensGroup>
    </nav>
  );
}
