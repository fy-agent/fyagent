import {
  createContext,
  useContext,
  useEffect,
  useRef,
  type HTMLAttributes,
  type ReactNode,
} from "react";

type HiddenRootProps = HTMLAttributes<HTMLDivElement> & { inert?: "" };

const PersistentVisibilityContext = createContext(true);

export function usePersistentVisibility(): boolean {
  return useContext(PersistentVisibilityContext);
}

export function PersistentSurface({
  active,
  children,
  className,
}: {
  active: boolean;
  children: ReactNode;
  className?: string;
}) {
  const ancestorVisible = useContext(PersistentVisibilityContext);
  const visible = active && ancestorVisible;
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (visible) return;
    const focused = document.activeElement;
    if (focused instanceof HTMLElement && ref.current?.contains(focused)) {
      focused.blur();
    }
  }, [visible]);

  const hiddenProps: HiddenRootProps = visible
    ? {}
    : { hidden: true, "aria-hidden": true, inert: "" };

  return (
    <PersistentVisibilityContext.Provider value={visible}>
      <div ref={ref} className={className} {...hiddenProps}>
        {children}
      </div>
    </PersistentVisibilityContext.Provider>
  );
}
