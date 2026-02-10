import { useToastStore } from "../store/toastStore";
import type { ToastType } from "../lib/types";

const typeStyles: Record<ToastType, string> = {
  info: "border-blue-500/30 bg-blue-500/10 text-blue-300",
  success: "border-green-500/30 bg-green-500/10 text-green-300",
  error: "border-red-500/30 bg-red-500/10 text-red-300",
  warning: "border-amber-500/30 bg-amber-500/10 text-amber-300",
};

const typeIcons: Record<ToastType, string> = {
  info: "i",
  success: "\u2713",
  error: "!",
  warning: "\u26A0",
};

export function ToastContainer() {
  const toasts = useToastStore((s) => s.toasts);
  const removeToast = useToastStore((s) => s.removeToast);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed right-4 top-4 z-50 flex flex-col gap-2">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`flex items-start gap-2 rounded-lg border px-4 py-3 shadow-lg backdrop-blur-sm animate-in slide-in-from-right ${typeStyles[toast.type]}`}
        >
          <span className="mt-0.5 text-sm font-bold">
            {typeIcons[toast.type]}
          </span>
          <p className="flex-1 text-sm">{toast.message}</p>
          <button
            onClick={() => removeToast(toast.id)}
            className="ml-2 text-sm opacity-60 hover:opacity-100"
          >
            x
          </button>
        </div>
      ))}
    </div>
  );
}
