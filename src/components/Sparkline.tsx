import React from "react";

interface SparklineProps {
  /** Recent download samples (bytes/sec), oldest first. */
  download: number[];
  /** Recent upload samples (bytes/sec), oldest first. */
  upload: number[];
  /** Max samples to render (trims the head). Default 30 (~1 min at 2s poll). */
  maxPoints?: number;
}

/**
 * Live throughput sparkline. Draws two polylines (download in signal-green,
 * upload in muted text) inside a fixed viewBox so it scales to its container.
 * The flat baseline at the bottom is intentional — a still line means a still
 * tunnel, which is itself information.
 */
export const Sparkline: React.FC<SparklineProps> = ({
  download,
  upload,
  maxPoints = 30,
}) => {
  const W = 300;
  const H = 56;
  const padY = 4;

  // Trim to the most recent N samples so the line scrolls naturally.
  const dl = download.slice(-maxPoints);
  const ul = upload.slice(-maxPoints);
  const len = Math.max(dl.length, ul.length);
  const n = Math.max(len, 2);

  const allValues = [...dl, ...ul, 1];
  const max = Math.max(...allValues, 1);

  const toPoints = (data: number[]): string => {
    // Right-align: a half-filled buffer plots against the right edge so the
    // newest sample is always at x = W.
    const offset = n - data.length;
    return data
      .map((v, i) => {
        const x = ((offset + i) / (n - 1)) * W;
        const y = H - padY - (v / max) * (H - padY * 2);
        return `${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(" ");
  };

  const dlPoints = toPoints(dl);
  const ulPoints = toPoints(ul);

  return (
    <svg
      className="sparkline-svg"
      viewBox={`0 0 ${W} ${H}`}
      preserveAspectRatio="none"
      role="img"
      aria-label="Throughput history"
    >
      {/* baseline */}
      <line
        x1="0"
        y1={H - 0.5}
        x2={W}
        y2={H - 0.5}
        stroke="var(--line)"
        strokeWidth="1"
      />
      {/* upload (muted, drawn first so download sits on top) */}
      {ul.length > 1 && (
        <polyline
          points={ulPoints}
          fill="none"
          stroke="var(--txt-2)"
          strokeWidth="1.5"
          strokeLinejoin="round"
          strokeLinecap="round"
          opacity="0.5"
        />
      )}
      {/* download (signal green) */}
      {dl.length > 1 && (
        <polyline
          points={dlPoints}
          fill="none"
          stroke="var(--signal)"
          strokeWidth="1.75"
          strokeLinejoin="round"
          strokeLinecap="round"
        />
      )}
    </svg>
  );
};
