import { classNames } from "../design-system/classNames";
import {
  AnimatePresence,
  fySurfaceEase,
  motion,
  motionDuration,
  useIsPresent,
  useReducedMotion,
} from "./motion";

export interface ToastMessage {
  id: number;
  tone: "success" | "error" | "info";
  title: string;
  description?: string;
}

function ToastItem({ message }: { message: ToastMessage }) {
  const reduce = useReducedMotion();
  const present = useIsPresent();
  const duration = reduce ? 0 : motionDuration("toast");
  return (
    <motion.div
      layout="position"
      initial={duration === 0 ? false : { opacity: 0, y: 8, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: reduce ? 0 : 4, scale: reduce ? 1 : 0.98 }}
      transition={{ duration, ease: [...fySurfaceEase] }}
      className={classNames("fy-toast", `fy-toast-${message.tone}`)}
      role="status"
      aria-hidden={present ? undefined : true}
    >
      <strong>{message.title}</strong>
      {message.description && <span>{message.description}</span>}
    </motion.div>
  );
}

/** Presentation only. FeatureProvider retains notification timing and state. */
export function ToastViewport({
  messages,
}: {
  messages: readonly ToastMessage[];
}) {
  return (
    <div className="fy-toast-host" aria-live="polite" aria-atomic="false">
      <AnimatePresence initial={false}>
        {messages.map((message) => (
          <ToastItem key={message.id} message={message} />
        ))}
      </AnimatePresence>
    </div>
  );
}
