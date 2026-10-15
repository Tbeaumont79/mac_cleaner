export function ageLabel(days: number | null): string {
  if (days === null) return "Inconnu";
  if (days < 1) return "aujourd'hui";
  if (days < 2) return "hier";
  if (days < 30) return `il y a ${days} j`;
  if (days < 365) return `il y a ${Math.round(days / 30)} mois`;
  const years = Math.round(days / 365);
  return `il y a ${years} an${years > 1 ? "s" : ""}`;
}
