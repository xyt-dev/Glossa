/**
 * Glossa 字标：开口 G 环 + 注解点。
 * G = Glossa；横杆延伸出环，像一行被标注的文本；右上的点即 "gloss"（页边注）。
 * 用 currentColor 绘制，跟随容器配色。
 */
export default function Logo({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path
        d="M18.3 6.5 A8.4 8.4 0 1 0 20.4 12.4"
        stroke="currentColor"
        strokeWidth="2.4"
        strokeLinecap="round"
      />
      <path
        d="M13.6 12.4 H20.4"
        stroke="currentColor"
        strokeWidth="2.4"
        strokeLinecap="round"
      />
      <circle cx="19.6" cy="4.9" r="1.7" fill="currentColor" />
    </svg>
  );
}
