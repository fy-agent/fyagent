import { CaretDownIcon } from "@phosphor-icons/react/dist/csr/CaretDown";
import { useRef, useState, type KeyboardEvent } from "react";
import { NavLink, useLocation } from "react-router-dom";

import {
  navigationGroups,
  type NavigationItem,
} from "../../shared/config/navigation";
import { classNames } from "../../shared/design-system/classNames";
import {
  appendAgentReturnToPath,
  agentReturnDescriptorFromManagementSearch,
  agentReturnDescriptorFromSearch,
  agentReturnPath,
} from "../../shared/features/agent-navigation";
import {
  Collapsible,
  CollapsibleCaret,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../../shared/ui/Collapsible";
import {
  SelectionLens,
  SelectionLensGroup,
} from "../../shared/ui/SelectionLens";

const configurationItemsId = "fy-side-navigation-configuration-items";

function NavigationLink({
  item,
  destination = item.path,
  visuallyAvailable = true,
}: {
  item: NavigationItem;
  destination?: string;
  visuallyAvailable?: boolean;
}) {
  return (
    <NavLink
      to={destination}
      end
      data-selection-material="text-only"
      className={({ isActive, isPending }) =>
        classNames(
          "fy-side-navigation-item",
          isActive && "fy-side-navigation-item-selected",
          isPending && !isActive && "fy-side-navigation-item-pending",
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
            {label}
          </>
        );
      }}
    </NavLink>
  );
}

function isExcludedFromKeyboard(control: HTMLElement): boolean {
  return (
    control.closest("[hidden]") !== null ||
    control.closest("[inert]") !== null ||
    control.closest('[aria-hidden="true"]') !== null
  );
}

function visibleNavigationControls(navigation: HTMLElement): HTMLElement[] {
  return Array.from(
    navigation.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ),
  ).filter((control) => !isExcludedFromKeyboard(control));
}

export function SideNavigation() {
  const { pathname, search } = useLocation();
  const navigationRef = useRef<HTMLElement>(null);
  const configurationToggleRef = useRef<HTMLButtonElement>(null);
  const configurationItemsRef = useRef<HTMLUListElement>(null);
  const [configurationExpanded, setConfigurationExpanded] = useState(true);
  const agentReturnDescriptor =
    pathname === "/agents"
      ? agentReturnDescriptorFromSearch(search)
      : agentReturnDescriptorFromManagementSearch(search);
  const agentDestination =
    pathname !== "/agents" && agentReturnDescriptor
      ? agentReturnPath(agentReturnDescriptor)
      : "/agents";
  const navigationDestination = (item: NavigationItem) => {
    if (item.path === "/agents") {
      return agentDestination;
    }
    return agentReturnDescriptor
      ? appendAgentReturnToPath(item.path, agentReturnDescriptor)
      : item.path;
  };

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
        geometry="position"
        layoutKey={configurationExpanded}
        className="fy-side-navigation-track"
        inset={1}
      >
        {navigationGroups.map((group) => {
          if (!group.collapsible) {
            return (
              <section
                className="fy-side-navigation-group"
                data-navigation-group={group.id}
                key={group.id}
              >
                {group.items.map((item) => (
                  <NavigationLink
                    item={item}
                    destination={navigationDestination(item)}
                    key={item.id}
                  />
                ))}
              </section>
            );
          }

          const groupLabelId = `fy-side-navigation-${group.id}-label`;
          const groupActive = group.items.some(
            (item) => item.path === pathname,
          );
          const content = (
            <span className="fy-side-navigation-toggle-content">
              <span>{group.label}</span>
              <CollapsibleCaret
                open={configurationExpanded}
                className="fy-side-navigation-caret"
              >
                <CaretDownIcon
                  size={16}
                  weight="bold"
                  aria-hidden
                  data-testid="configuration-management-caret"
                />
              </CollapsibleCaret>
            </span>
          );
          const visuallyActive = groupActive && !configurationExpanded;

          return (
            <Collapsible
              asChild
              open={configurationExpanded}
              onOpenChange={setConfigurationExpanded}
              key={group.id}
            >
              <section
                className="fy-side-navigation-group fy-side-navigation-group-collapsible"
                aria-labelledby={groupLabelId}
                data-navigation-group={group.id}
              >
                <CollapsibleTrigger asChild>
                  <button
                    ref={configurationToggleRef}
                    className={classNames(
                      "fy-side-navigation-toggle",
                      groupActive && "fy-side-navigation-toggle-active",
                    )}
                    id={groupLabelId}
                    type="button"
                    data-active={groupActive || undefined}
                    data-collapsed-active={visuallyActive ? "true" : undefined}
                    data-selection-material={
                      visuallyActive
                        ? "text-only"
                        : groupActive
                          ? "context-frame"
                          : undefined
                    }
                    data-testid="configuration-management-toggle"
                  >
                    <SelectionLens active={visuallyActive} />
                    {content}
                  </button>
                </CollapsibleTrigger>
                <CollapsibleContent open={configurationExpanded}>
                  <ul
                    ref={configurationItemsRef}
                    className="fy-side-navigation-items"
                    id={configurationItemsId}
                    hidden={!configurationExpanded}
                    data-testid="configuration-management-items"
                  >
                    {group.items.map((item) => (
                      <li key={item.id}>
                        <NavigationLink
                          item={item}
                          destination={navigationDestination(item)}
                          visuallyAvailable={configurationExpanded}
                        />
                      </li>
                    ))}
                  </ul>
                </CollapsibleContent>
              </section>
            </Collapsible>
          );
        })}
      </SelectionLensGroup>
    </nav>
  );
}
