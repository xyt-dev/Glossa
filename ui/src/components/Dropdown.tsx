import { useEffect, useRef, useState } from "react";
import { uiScale } from "../platform";

export interface DropdownOption {
  value: string;
  label: string;
}

interface Props {
  value: string;
  options: DropdownOption[];
  onChange: (value: string) => void;
  disabled?: boolean;
  title?: string;
  className?: string;
}

interface MenuPos {
  left: number;
  width: number;
  up: boolean;
  top: number;
  maxHeight: number;
}

/**
 * Themed replacement for native <select> — the WebKitGTK popup ignores CSS
 * entirely. The menu is position:fixed so it escapes scrolling modals, and
 * flips upward when there is more room above the button.
 */
export default function Dropdown({ value, options, onChange, disabled, title, className }: Props) {
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState<MenuPos | null>(null);
  const btnRef = useRef<HTMLButtonElement>(null);

  const current = options.find((o) => o.value === value);

  const toggle = () => {
    if (disabled) return;
    if (open) {
      setOpen(false);
      return;
    }
    // CSS zoom（web 缩放）会再乘一次 fixed 定位的 px，坐标要除回去
    const z = uiScale;
    const rect = btnRef.current!.getBoundingClientRect();
    const r = {
      left: rect.left / z,
      top: rect.top / z,
      bottom: rect.bottom / z,
      width: rect.width / z,
    };
    const viewH = window.innerHeight / z;
    const margin = 8;
    const below = viewH - r.bottom - margin;
    const above = r.top - margin;
    const desired = Math.min(options.length * 48 + 12, Math.round(viewH * 0.6));
    const up = below < desired && above > below;
    setPos({
      left: r.left,
      width: r.width,
      up,
      top: up ? r.top - 4 : r.bottom + 4,
      maxHeight: Math.min(desired, up ? above : below),
    });
    setOpen(true);
  };

  useEffect(() => {
    if (!open) return;
    const close = () => setOpen(false);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", onKey);
    window.addEventListener("resize", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("resize", close);
      window.removeEventListener("scroll", close, true);
    };
  }, [open]);

  return (
    <div className={`dropdown${className ? ` ${className}` : ""}`}>
      <button
        ref={btnRef}
        type="button"
        className="dropdown-btn"
        disabled={disabled}
        title={title}
        onClick={toggle}
      >
        <span className="dropdown-label">{current?.label ?? value}</span>
        <span className="dropdown-chevron">▾</span>
      </button>
      {open && pos && (
        <div
          className="dropdown-menu"
          style={{
            left: pos.left,
            width: pos.width,
            maxHeight: pos.maxHeight,
            ...(pos.up
              ? { bottom: window.innerHeight / uiScale - pos.top }
              : { top: pos.top }),
          }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          {options.map((o) => (
            <button
              key={o.value}
              type="button"
              className={`dropdown-item${o.value === value ? " selected" : ""}`}
              onClick={() => {
                onChange(o.value);
                setOpen(false);
              }}
            >
              {o.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
