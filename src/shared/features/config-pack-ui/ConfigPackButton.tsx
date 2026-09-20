import { lazy, Suspense, useRef } from "react";
import { Button } from "../../ui/Button";
import { Spinner } from "../../ui/primitives";
import { useDialogState } from "../../ui/useDialogState";
import { usePersistentVisibility } from "../../ui/PersistentSurface";
import type { ConfigPackFormFill } from "../config-pack";

const ConfigPackDialog = lazy(() =>
  import("./ConfigPackDialog").then((m) => ({ default: m.ConfigPackDialog })),
);

export function ConfigPackButton({
  onFillModelForm,
}: {
  onFillModelForm?: ConfigPackFormFill;
}) {
  const [open, setOpen, session] = useDialogState<boolean>();
  const origin = useRef<HTMLElement | null>(null);
  const visible = usePersistentVisibility();
  return (
    <>
      <Button dialogOriginRef={origin} onClick={() => setOpen(true)}>
        导入 / 导出连接
      </Button>
      {open && visible && (
        <Suspense fallback={<Spinner label="正在打开配置迁移" />}>
          <ConfigPackDialog
            key={session}
            open={!!open}
            originRef={origin}
            onFillModelForm={onFillModelForm}
            onClose={() => setOpen(null)}
          />
        </Suspense>
      )}
    </>
  );
}
