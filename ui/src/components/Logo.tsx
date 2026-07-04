/**
 * Glossa 字标：几何 G（Google 式内收横杆）。
 * 一笔画成的字环，横杆收在环内伸到圆心，简洁现代。用 currentColor 跟随容器配色。
 */
export default function Logo({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d="M18.2 7 A7.8 7.8 0 1 0 19.8 12.6 H12.6"
        stroke="currentColor"
        strokeWidth="2.4"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
