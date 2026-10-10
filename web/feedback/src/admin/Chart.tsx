// A month of days as bars: one series, or two stacked. Drawn in SVG at the
// plot's real width (no stretched marks), with a tooltip on hover, touch
// and the arrow keys, and the same numbers as a table one click away.

import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";

import { formatCount, formatDay, t, type Lang } from "../i18n";

export interface Series<K extends string = string> {
  key: K;
  label: string;
  /** `c1` or `c2`: the chart's two validated hues. */
  hue: "c1" | "c2";
}

/** One UTC day with a number under each series key. */
export type DayOf<K extends string> = { day: string } & Record<K, number>;

const HEIGHT = 140;
const GAP = 2;

/** The smallest of 1, 2, 5 × 10ⁿ at or above `n`. */
export function niceMax(n: number): number {
  if (n <= 1) return 1;
  const power = 10 ** Math.floor(Math.log10(n));
  for (const step of [1, 2, 5, 10]) {
    if (step * power >= n) return step * power;
  }
  return 10 * power;
}

/** A bar of width `w` and height `h` at `x`,`y` with its top corners rounded. */
function bar(x: number, y: number, w: number, h: number): string {
  const r = Math.min(3, w / 2, h);
  return `M${x},${y + h}V${y + r}Q${x},${y} ${x + r},${y}H${x + w - r}Q${x + w},${y} ${x + w},${y + r}V${y + h}Z`;
}

/**
 * The plot's real width, measured. Nothing is drawn until it is: a plot
 * drawn at a guessed width before the first measurement can be wider than
 * a phone and leave the page scrolling sideways. Where nothing measures
 * (a test's DOM) the guess stands.
 */
export function useWidth(): [React.RefObject<HTMLDivElement | null>, number] {
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(() => (typeof ResizeObserver === "undefined" ? 600 : 0));
  useEffect(() => {
    const el = ref.current;
    if (el === null || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width;
      if (w !== undefined && w > 0) setWidth(Math.round(w));
    });
    observer.observe(el);
    return () => {
      observer.disconnect();
    };
  }, []);
  return [ref, width];
}

export function DayChart<K extends string>({
  lang,
  title,
  days,
  series,
}: {
  lang: Lang;
  title: string;
  days: DayOf<K>[];
  series: Series<K>[];
}) {
  const id = useId();
  const [ref, width] = useWidth();
  const [active, setActive] = useState<number | null>(null);
  const n = (v: number) => formatCount(lang, v);

  const totals = useMemo(() => days.map((d) => series.reduce((sum, s) => sum + d[s.key], 0)), [days, series]);
  const max = niceMax(Math.max(0, ...totals));
  const sum = totals.reduce((a, b) => a + b, 0);
  const peak = Math.max(0, ...totals);
  const slot = days.length === 0 ? width : width / days.length;
  const barWidth = Math.max(1, slot - GAP);
  const y = (v: number) => (v / max) * HEIGHT;

  const pick = (event: PointerEvent<HTMLDivElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    const i = Math.floor(((event.clientX - box.left) / Math.max(1, box.width)) * days.length);
    setActive(Math.min(days.length - 1, Math.max(0, i)));
  };
  const step = (event: KeyboardEvent<HTMLInputElement>) => {
    const last = days.length - 1;
    const now = active ?? last;
    const next =
      event.key === "ArrowLeft"
        ? Math.max(0, now - 1)
        : event.key === "ArrowRight"
          ? Math.min(last, now + 1)
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? last
              : null;
    if (next !== null) {
      event.preventDefault();
      setActive(next);
    } else if (event.key === "Escape") setActive(null);
  };

  const shown = active === null ? null : days[active];
  const valueText = (i: number) => {
    const d = days[i];
    if (d === undefined) return "";
    return `${formatDay(lang, d.day)}: ${series.map((s) => `${s.label} ${n(d[s.key])}`).join(", ")}`;
  };
  const tipLeft = active === null ? 0 : Math.min(Math.max((active + 0.5) * slot, 70), width - 70);

  return (
    <figure className="chart" aria-labelledby={`${id}-t`}>
      <figcaption className="chart-head">
        <span className="chart-title" id={`${id}-t`}>
          {title}
        </span>
        <span className="chart-sum muted small">
          {t(lang, "chart.sum", { n: n(sum) })} · {t(lang, "chart.peak", { n: n(peak) })}
        </span>
        {series.length > 1 && (
          <span className="legend">
            {series.map((s) => (
              <span key={s.key} className="legend-item">
                <span className={`swatch ${s.hue}`} aria-hidden="true" />
                {s.label}
              </span>
            ))}
          </span>
        )}
      </figcaption>
      <div
        className="chart-plot"
        ref={ref}
        onPointerMove={pick}
        onPointerDown={pick}
        onPointerLeave={() => {
          setActive(null);
        }}
      >
        <input
          type="range"
          className="chart-range"
          min={0}
          max={Math.max(0, days.length - 1)}
          value={active ?? Math.max(0, days.length - 1)}
          aria-label={`${title}. ${t(lang, "chart.hint")}`}
          aria-valuetext={valueText(active ?? days.length - 1)}
          onChange={(event) => {
            setActive(Number(event.target.value));
          }}
          onKeyDown={step}
          onFocus={() => {
            setActive(days.length - 1);
          }}
          onBlur={() => {
            setActive(null);
          }}
        />
        <span className="chart-y chart-y-top">{n(max)}</span>
        <span className="chart-y chart-y-mid">{n(max / 2)}</span>
        <svg width={width} height={HEIGHT} aria-hidden="true" focusable="false">
          <line className="grid" x1={0} x2={width} y1={0.5} y2={0.5} />
          <line className="grid" x1={0} x2={width} y1={HEIGHT / 2} y2={HEIGHT / 2} />
          <line className="axis" x1={0} x2={width} y1={HEIGHT - 0.5} y2={HEIGHT - 0.5} />
          {active !== null && <rect className="hover-band" x={active * slot} y={0} width={slot} height={HEIGHT} />}
          {days.map((d, i) => {
            const x = i * slot + GAP / 2;
            let base = HEIGHT;
            return series.map((s, k) => {
              const value = d[s.key];
              if (value <= 0) return null;
              // A 2px gap of surface between stacked segments.
              const gap = k > 0 && base < HEIGHT ? GAP : 0;
              const h = Math.max(1, y(value) - gap);
              const top = base - gap - h;
              base = top;
              return (
                <path
                  key={`${d.day}-${s.key}`}
                  className={`mark ${s.hue}${active !== null && active !== i ? " dim" : ""}`}
                  d={bar(x, top, barWidth, h)}
                />
              );
            });
          })}
        </svg>
        {shown !== null && shown !== undefined && (
          <output className="chart-tip" style={{ left: `${tipLeft}px` }}>
            <strong>{formatDay(lang, shown.day)}</strong>
            {series.map((s) => (
              <span key={s.key} className="tip-row">
                <span className={`swatch ${s.hue}`} aria-hidden="true" />
                {s.label}: <b>{n(shown[s.key])}</b>
              </span>
            ))}
          </output>
        )}
      </div>
      <div className="chart-x muted small" aria-hidden="true">
        <span>{days[0] === undefined ? "" : formatDay(lang, days[0].day)}</span>
        <span>{days.at(-1) === undefined ? "" : formatDay(lang, days.at(-1)?.day ?? "")}</span>
      </div>
      <details className="chart-table">
        <summary>{t(lang, "chart.table")}</summary>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">{t(lang, "chart.day")}</th>
                {series.map((s) => (
                  <th scope="col" key={s.key}>
                    {s.label}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {days.toReversed().map((d) => (
                <tr key={d.day}>
                  <th scope="row">{formatDay(lang, d.day)}</th>
                  {series.map((s) => (
                    <td key={s.key}>{n(d[s.key])}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
    </figure>
  );
}
