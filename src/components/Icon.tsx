import type { ReactNode, SVGProps } from "react";

export type IconName = "archive" | "activity" | "clock" | "cookie" | "download" | "file" | "folder"
  | "key" | "moon" | "pulse" | "search" | "settings" | "shield" | "star" | "sun" | "terminal" | "x";

const paths: Record<IconName, ReactNode> = {
  archive: <><path d="M4 7.5h16" /><path d="M5.5 7.5V19h13V7.5" /><path d="M3.5 4h17v3.5h-17z" /><path d="M9.5 12h5" /></>,
  activity: <><path d="M3 12h4l2.2-6 4 12 2.1-6H21" /></>,
  clock: <><circle cx="12" cy="12" r="8.5" /><path d="M12 7.5V12l3 2" /></>,
  cookie: <><path d="M19.6 13.2A4.2 4.2 0 0 1 14 7.3 4.2 4.2 0 0 1 9.5 3.6 8.7 8.7 0 1 0 20.4 14c-.3-.2-.5-.5-.8-.8Z" /><path d="M8.5 10.5h.01M11 16h.01M6.5 15h.01" /></>,
  download: <><path d="M12 3v12" /><path d="m7.5 10.5 4.5 4.5 4.5-4.5" /><path d="M4 20h16" /></>,
  file: <><path d="M6 3.5h8l4 4V20H6z" /><path d="M14 3.5V8h4M9 12h6M9 16h5" /></>,
  folder: <><path d="M3.5 6.5h6l2-2h9v15h-17z" /></>,
  key: <><circle cx="8" cy="12" r="4" /><path d="M12 12h9M17 12v3M20 12v2" /></>,
  moon: <path d="M19.2 15.2A8.3 8.3 0 0 1 8.8 4.8 8.5 8.5 0 1 0 19.2 15.2Z" />,
  pulse: <><path d="M3 12h4l2-5 4.2 10 2.3-5H21" /></>,
  search: <><circle cx="10.5" cy="10.5" r="6.5" /><path d="m15.5 15.5 5 5" /></>,
  settings: <><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4V21a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9A1.7 1.7 0 0 0 3 14H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1A1.7 1.7 0 0 0 9 4.6 1.7 1.7 0 0 0 10 3V2.8h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1Z" /></>,
  shield: <><path d="M12 3 5 6v5.5c0 4.4 2.8 7.5 7 9.5 4.2-2 7-5.1 7-9.5V6z" /><path d="m9 12 2 2 4-4" /></>,
  star: <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-2.9-5.6 2.9 1.1-6.2L3 9.6l6.2-.9z" />,
  sun: <><circle cx="12" cy="12" r="3.5" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></>,
  terminal: <><path d="m5 7 4 4-4 4M11 16h7" /></>,
  x: <><path d="m6 6 12 12M18 6 6 18" /></>,
};

export function Icon({ name, ...props }: { name: IconName } & SVGProps<SVGSVGElement>) {
  return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" {...props}>{paths[name]}</svg>;
}

export function ReqVaultMark({ className }: { className?: string }) {
  return <svg className={className} viewBox="0 0 32 32" fill="none" aria-hidden="true"><rect width="32" height="32" rx="9" fill="currentColor" /><path d="M9 10.5h7.3c3 0 4.7 1.5 4.7 4 0 1.8-.9 3-2.5 3.7l3 4.3h-4.2l-2.4-3.7h-2v3.7H9v-12Zm3.9 3v2.7h3c.9 0 1.4-.5 1.4-1.4s-.5-1.3-1.4-1.3h-3Z" fill="white" /></svg>;
}
