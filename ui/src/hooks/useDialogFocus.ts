import { useEffect, useRef, type RefObject } from "react";

const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

type DialogFocusOptions = {
  /** Called on Escape unless `dismissDisabled` is set. */
  onDismiss?: () => void;
  /** Ignore Escape while a request is in flight. */
  dismissDisabled?: boolean;
  /** Element to focus when the dialog opens; defaults to the first control. */
  initialFocus?: RefObject<HTMLElement | null>;
  /** Turn off the Tab loop, for a dialog stacked under another one. */
  focusTrap?: boolean;
};

/**
 * Keeps keyboard focus inside an open dialog and restores it to the element
 * that opened the dialog when it closes. The dialog element needs `tabIndex`
 * of `-1` so focus has somewhere to land when it holds no controls.
 */
export function useDialogFocus<T extends HTMLElement>(
  options: DialogFocusOptions = {},
) {
  const dialogRef = useRef<T | null>(null);
  const dismissRef = useRef(options.onDismiss);
  dismissRef.current = options.onDismiss;
  const disabledRef = useRef(options.dismissDisabled);
  disabledRef.current = options.dismissDisabled;
  const initialFocusRef = useRef(options.initialFocus);
  initialFocusRef.current = options.initialFocus;
  const trapRef = useRef(options.focusTrap ?? true);
  trapRef.current = options.focusTrap ?? true;

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const previous =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    const bodyOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    function focusable(): HTMLElement[] {
      const all = Array.from(
        dialog!.querySelectorAll<HTMLElement>(FOCUSABLE),
      );
      const visible = all.filter(
        (element) =>
          element.getClientRects().length > 0 ||
          element === document.activeElement,
      );
      return visible.length > 0 ? visible : all;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        if (!disabledRef.current && dismissRef.current) {
          event.preventDefault();
          dismissRef.current();
        }
        return;
      }
      if (event.key !== "Tab" || !trapRef.current) return;
      const targets = focusable();
      if (targets.length === 0) {
        event.preventDefault();
        dialog!.focus({ preventScroll: true });
        return;
      }
      const first = targets[0];
      const last = targets[targets.length - 1];
      const active = document.activeElement;
      const inside = active instanceof Node && dialog!.contains(active);
      if (event.shiftKey && (active === first || !inside)) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && (active === last || !inside)) {
        event.preventDefault();
        first.focus();
      }
    }

    window.addEventListener("keydown", handleKeyDown, true);

    const initial = initialFocusRef.current?.current ?? null;
    const target = initial ?? focusable()[0] ?? dialog;
    target.focus({ preventScroll: true });

    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      document.body.style.overflow = bodyOverflow;
      if (previous?.isConnected) {
        previous.focus({ preventScroll: true });
      }
    };
  }, []);

  return dialogRef;
}
