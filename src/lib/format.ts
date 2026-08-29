export const money = new Intl.NumberFormat("fr-SN", {
  style: "currency",
  currency: "XOF",
  maximumFractionDigits: 0
});

export function formatMoney(value: number): string {
  return money.format(value).replace("F CFA", "FCFA");
}

export function formatDate(value?: string, withTime = false): string {
  if (!value) return "—";
  const date = value.length === 10 ? new Date(`${value}T00:00:00Z`) : new Date(value);
  return new Intl.DateTimeFormat("fr-SN", {
    timeZone: "Africa/Dakar",
    day: "2-digit",
    month: "short",
    year: "numeric",
    ...(withTime ? { hour: "2-digit", minute: "2-digit" } : {})
  }).format(date);
}

export function today(): string {
  return new Intl.DateTimeFormat("en-CA", {
    timeZone: "Africa/Dakar",
    year: "numeric",
    month: "2-digit",
    day: "2-digit"
  }).format(new Date());
}

export function signed(value: number): string {
  return `${value > 0 ? "+" : ""}${formatMoney(value)}`;
}

export const labels: Record<string, string> = {
  cash: "Espèces",
  orange_money: "Orange Money",
  wave: "Wave",
  djamo: "Djamo",
  sale: "Recette boutique",
  commission: "Commission mobile",
  capital_contribution: "Apport de capital",
  purchase: "Achat",
  expense: "Dépense",
  capital_withdrawal: "Retrait de capital",
  reversal: "Contre-écriture",
  inventory_correction: "Correction d’inventaire",
  open: "Ouverte",
  partial: "Partiellement payée",
  paid: "Payée",
  overdue: "En retard",
  cancelled: "Annulée",
  commission_mobile: "Commission mobile",
  surplus_caisse: "Surplus de caisse",
  manquant_caisse: "Manquant de caisse",
  erreur_saisie: "Erreur de saisie",
  autre: "Autre"
};

export function label(value: string): string {
  return labels[value] ?? value;
}
