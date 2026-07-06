import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { uiScale } from "../platform";

/** 触发点在视口中的位置（原始坐标，未按 uiScale 归一化）。 */
export interface PopoverAnchor {
  cx: number; // 锚点水平中心
  top: number; // 锚点上边
  bottom: number; // 锚点下边
}

interface Placement {
  left: number;
  top?: number;
  bottom?: number;
  up: boolean;
  arrowLeft: number;
}

interface Props {
  anchor: PopoverAnchor;
  onClose: () => void;
  /** 主体内容，可滚动。 */
  children: ReactNode;
  /** 可选：固定在底部、不随主体滚动的内容（如操作按钮）。 */
  footer?: ReactNode;
  maxWidth?: number;
  className?: string;
}

const GAP = 10; // 锚点与浮层间距
const MARGIN = 8; // 距视口边缘的最小留白

/**
 * 通用锚定浮层。
 *
 * 协议：调用方给出 `anchor`（触发点位置）与内容（`children` 可滚动主体、
 * 可选 `footer` 固定底部），组件自行完成定位与关闭：
 *  - 智能选择在锚点上方/下方弹出：下方空间不足且上方更宽敞时自动上翻；
 *  - 水平方向夹住不出屏，箭头始终指向锚点；
 *  - 主体超高时内部滚动，footer 固定可见；
 *  - 点击外部 / Esc / 滚动 / 缩放窗口自动关闭。
 *
 * 复用点：句子翻译浮窗；未来阅读模式（epub / 网页文本）点击划词翻译等，
 * 只需按此协议塞入不同内容即可共用同一套浮层交互。
 */
export default function Popover({
  anchor,
  onClose,
  children,
  footer,
  maxWidth = 720,
  className,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState<Placement | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    // 只在“外部”滚动时关闭（锚点随之移走）；浮层内部滚动不能关，否则滚不动
    const onScroll = (e: Event) => {
      const t = e.target as Node | null;
      if (t && ref.current?.contains(t)) return;
      onClose();
    };
    window.addEventListener("mousedown", onClose);
    window.addEventListener("touchstart", onClose);
    window.addEventListener("keydown", onKey);
    window.addEventListener("resize", onClose);
    window.addEventListener("scroll", onScroll, true);
    return () => {
      window.removeEventListener("mousedown", onClose);
      window.removeEventListener("touchstart", onClose);
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("resize", onClose);
      window.removeEventListener("scroll", onScroll, true);
    };
  }, [onClose]);

  // 先隐藏渲染一次以测得真实宽高，再据此决定上下翻转与夹边（paint 前完成，无闪烁）
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const z = uiScale;
    const viewW = window.innerWidth / z;
    const viewH = window.innerHeight / z;
    const cx = anchor.cx / z;
    const aTop = anchor.top / z;
    const aBottom = anchor.bottom / z;
    const w = el.offsetWidth;
    const h = el.offsetHeight;

    const below = viewH - aBottom - GAP - MARGIN;
    const above = aTop - GAP - MARGIN;
    const up = h > below && above > below;

    const left = Math.min(Math.max(cx, w / 2 + MARGIN), viewW - w / 2 - MARGIN);
    // 箭头相对浮层左缘，夹在两端内侧，始终指向锚点中心
    const arrowLeft = Math.min(Math.max(cx - (left - w / 2), 16), w - 16);

    setPlace(
      up
        ? { left, bottom: viewH - aTop + GAP, up: true, arrowLeft }
        : { left, top: aBottom + GAP, up: false, arrowLeft },
    );
  }, [anchor]);

  return (
    <div
      ref={ref}
      className={`popover${place?.up ? " up" : ""}${className ? " " + className : ""}`}
      style={
        place
          ? { left: place.left, top: place.top, bottom: place.bottom, maxWidth }
          : { left: -9999, top: 0, maxWidth, visibility: "hidden" }
      }
      onMouseDown={(e) => e.stopPropagation()}
      onTouchStart={(e) => e.stopPropagation()}
    >
      <span
        className="popover-arrow"
        style={place ? { left: place.arrowLeft } : undefined}
      />
      <div className="popover-body">{children}</div>
      {footer && <div className="popover-footer">{footer}</div>}
    </div>
  );
}
