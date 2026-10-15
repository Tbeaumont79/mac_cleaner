export const ICONS = {
  scan: "M12 3v3M12 18v3M3 12h3M18 12h3M6.3 6.3l2.1 2.1M15.6 15.6l2.1 2.1M6.3 17.7l2.1-2.1M15.6 8.4l2.1-2.1",
  junk: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
  prot: "M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6l8-3z",
  speed: "M4 17a8 8 0 1 1 16 0M12 17l4.5-5.5",
  apps: "M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z",
  files: "M14 3H6v18h12V7l-4-4zM14 3v4h4M9 13h6M9 17h4",
  trash: "M3 6h18M8 6V4h8v2M5 6l1.2 14h11.6L19 6M10 10l4 6M14 10l-4 6",
} as const;

export type IconName = keyof typeof ICONS;

export const GRADIENTS: Record<IconName, string> = {
  scan: "linear-gradient(135deg, #5CE1FF, #2E8DE0)",
  junk: "linear-gradient(135deg, #3FE0B8, #17988A)",
  prot: "linear-gradient(135deg, #7DB2FF, #3E5BD9)",
  speed: "linear-gradient(135deg, #FFC25C, #EE7A2E)",
  apps: "linear-gradient(135deg, #FF8AD8, #C2389A)",
  files: "linear-gradient(135deg, #C59BFF, #8045DD)",
  trash: "linear-gradient(135deg, #FF8AD8, #C2389A)",
};
