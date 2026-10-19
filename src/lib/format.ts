// Utilities used by the Reddit-style UI.

/** Format a timestamp as a relative "2h ago" string, matching Reddit's display. */
export function timeAgo(iso: string): string {
  const date = new Date(iso);
  const now = new Date();
  const sec = Math.floor((now.getTime() - date.getTime()) / 1000);
  if (sec < 60) return 'just now';
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  if (day < 30) return `${day}d ago`;
  const mo = Math.floor(day / 30);
  if (mo < 12) return `${mo}mo ago`;
  const yr = Math.floor(mo / 12);
  return `${yr}y ago`;
}

/** Format a number with K/M suffixes — 12345 → "12.3k", matching Reddit. */
export function formatCount(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) {
    const v = n / 1000;
    return `${v < 10 ? v.toFixed(1) : Math.floor(v)}k`;
  }
  const v = n / 1_000_000;
  return `${v < 10 ? v.toFixed(1) : Math.floor(v)}M`;
}

/** A deterministic color from a subreddit name — used when no icon is set. */
export function colorFromString(s: string): string {
  let hash = 0;
  for (let i = 0; i < s.length; i++) {
    hash = (hash << 5) - hash + s.charCodeAt(i);
    hash |= 0;
  }
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue} 65% 45%)`;
}

/** Initials for avatar fallback. */
export function initials(name: string): string {
  const parts = name.replace(/[^a-zA-Z0-9 ]/g, '').split(/\s+/).filter(Boolean);
  if (parts.length === 0) return '?';
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[1][0]).toUpperCase();
}

/** Reddit-style score display: hidden when negative, with +/- suffix. */
export function formatScore(score: number): string {
  if (score > 9999) return formatCount(score);
  return String(score);
}

/** Truncate a string to N chars with ellipsis. */
export function truncate(s: string, n: number): string {
  if (s.length <= n) return s;
  return s.slice(0, n - 1) + '…';
}
