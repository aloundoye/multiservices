import { accountLabel } from "../components/AccountSelect";
import {
  AlertTriangle,
  ArrowDownRight,
  ArrowRight,
  ArrowUpRight,
  Banknote,
  CalendarClock,
  CircleDollarSign,
  HandCoins,
  Landmark,
  Plus,
  Scale,
  ShieldCheck,
  Wallet
} from "lucide-react";
import type { Dashboard, PageId } from "../types";
import { formatDate, formatMoney, signed } from "../lib/format";

export function DashboardPage({ dashboard, onNavigate, onCustodyAction }: { dashboard: Dashboard; onNavigate: (page: PageId) => void; onCustodyAction?: (action: "deposit" | "withdrawal") => void }) {
  const inventory = dashboard.lastInventory;
  const accounts = [
    { label: "Orange Money", value: inventory.balances.orangeMoney, delta: inventory.delta.orangeMoney, color: "orange", icon: Landmark },
    { label: "Wave", value: inventory.balances.wave, delta: inventory.delta.wave, color: "wave", icon: Wallet },
    { label: "Djamo", value: inventory.balances.djamo, delta: inventory.delta.djamo, color: "djamo", icon: CircleDollarSign },
    { label: "Espèces", value: inventory.balances.cash, delta: inventory.delta.cash, color: "cash", icon: Banknote }
  ];
  const maxValue = Math.max(...accounts.map((account) => account.value), 1);

  return (
    <div className="page dashboard-page">
      <header className="page-header">
        <div><p className="eyebrow">VUE D’ENSEMBLE</p><h1>Bonjour, bienvenue à la boutique</h1><p>Situation basée sur l’inventaire du {formatDate(inventory.closedAt, true)}.</p></div>
        <button className="button primary" onClick={() => onNavigate("inventory")}><Plus size={18} /> Faire l’inventaire</button>
      </header>

      {dashboard.inventoryOverdue && (
        <button className="overdue-banner" onClick={() => onNavigate("inventory")}>
          <AlertTriangle /><div><strong>Un inventaire est attendu</strong><span>Le délai configuré depuis la dernière clôture est dépassé.</span></div><ArrowRight />
        </button>
      )}

      <section className="metric-grid">
        <article className="metric-card hero-metric">
          <div className="metric-icon green"><Scale /></div>
          <span>Capital attendu</span>
          <strong>{formatMoney(dashboard.expectedCapital)}</strong>
          <small><span className={dashboard.expectedCapital - dashboard.lastActualCapital >= 0 ? "positive" : "negative"}>{signed(dashboard.expectedCapital - dashboard.lastActualCapital)}</span> depuis le dernier inventaire</small>
        </article>
        <article className="metric-card">
          <div className="metric-icon blue"><Banknote /></div>
          <span>Dernier capital réel</span>
          <strong>{formatMoney(dashboard.lastActualCapital)}</strong>
          <small>Liquidités + créances − dépôts suivis</small>
        </article>
        <article className="metric-card">
          <div className="metric-icon amber"><HandCoins /></div>
          <span>Créances en cours</span>
          <strong>{formatMoney(dashboard.openReceivables)}</strong>
          <small>{dashboard.openDebtsCount} dette{dashboard.openDebtsCount > 1 ? "s" : ""} non soldée{dashboard.openDebtsCount > 1 ? "s" : ""}</small>
        </article>
        <article className={`metric-card ${inventory.variance !== 0 ? "attention" : ""}`}>
          <div className="metric-icon coral"><AlertTriangle /></div>
          <span>Dernier écart</span>
          <strong className={inventory.variance > 0 ? "positive" : inventory.variance < 0 ? "negative" : ""}>{signed(inventory.variance)}</strong>
          <small>{inventory.variance === 0 ? "Inventaire équilibré" : inventory.varianceNote ?? "Écart justifié"}</small>
        </article>
      </section>

      <section className="panel custody-dashboard">
        <div className="metric-icon green"><ShieldCheck /></div><div><span>Argent gardé pour les clients</span><strong>{formatMoney(dashboard.custodyTotal)}</strong><small>{dashboard.custodyCustomersCount} client(s) ayant un solde · hors capital de la boutique</small></div>
        <div className="custody-actions"><button className="button primary" onClick={() => onCustodyAction ? onCustodyAction("deposit") : onNavigate("custody")}>Recevoir un dépôt</button><button className="button secondary" onClick={() => onCustodyAction ? onCustodyAction("withdrawal") : onNavigate("custody")}>Restituer</button></div>
      </section>
      <section className="dashboard-columns">
        <article className="panel balances-panel">
          <header className="panel-header"><div><h2>Répartition des liquidités</h2><p>Soldes vérifiés au dernier inventaire, non actualisés par les opérations</p></div><span className="total-pill">{formatMoney(inventory.liquidity)}</span></header>
          <div className="account-list">
            {accounts.map(({ label, value, delta, color, icon: Icon }) => (
              <div className="account-row" key={label}>
                <div className={`account-icon ${color}`}><Icon /></div>
                <div className="account-info"><div><strong>{label}</strong><span>{formatMoney(value)}</span></div><div className="account-bar"><span className={color} style={{ width: `${Math.max(3, (value / maxValue) * 100)}%` }} /></div></div>
                <span className={`delta ${delta > 0 ? "up" : delta < 0 ? "down" : ""}`}>{delta > 0 ? <ArrowUpRight /> : delta < 0 ? <ArrowDownRight /> : null}{signed(delta)}</span>
              </div>
            ))}
            {accounts.map(({ label: service, color }) => {
              const provider = service === "Orange Money" ? "orange_money" : service === "Espèces" ? "cash" : service.toLowerCase();
              const details = inventory.accountBalances.filter((a) => a.provider === provider);
              const unmeasured = dashboard.accounts.filter((a) => a.active && a.provider === provider && !details.some((d) => d.accountId === a.accountId));
              return <details className="service-detail" key={service}>
                <summary><span className={`dot ${color}`} /> Comptes {service} ({details.length + unmeasured.length})</summary>
                {details.map((a) => <div className="read-only-row" key={a.accountId}><span>{accountLabel(a)}{a.legacy && <small>Solde historique regroupé</small>}</span><strong>{formatMoney(a.amount)}</strong></div>)}
                {unmeasured.map((a) => <div className="read-only-row" key={a.accountId}><span>{accountLabel(a)}</span><small>{a.lastBalance == null ? "Pas encore relevé" : "Réactivé — à relever au prochain inventaire"}</small></div>)}
              </details>;
            })}
          </div>
          <button className="text-button" onClick={() => onNavigate("history")}>Voir l’historique des inventaires <ArrowRight size={16} /></button>
        </article>

        <div className="side-stack">
          <article className="panel next-inventory">
            <div className="calendar-icon"><CalendarClock /></div>
            <div><span>PROCHAIN INVENTAIRE</span><strong>{formatDate(dashboard.nextInventoryAt, true)}</strong><small>Rappel selon l’intervalle configuré</small></div>
            <button className="button ghost" onClick={() => onNavigate("inventory")}>Commencer</button>
          </article>
          <article className="panel quick-actions">
            <header className="panel-header"><div><h2>Actions rapides</h2><p>Enregistrer un mouvement</p></div></header>
            <button onClick={() => onNavigate("products")}><span className="quick-icon green"><Plus /></span><div><strong>Enregistrer une vente</strong><small>Produits, stock et encaissement</small></div><ArrowRight /></button>
            <button onClick={() => onNavigate("journal")}><span className="quick-icon green"><Plus /></span><div><strong>Ajouter au journal</strong><small>Recette, achat ou dépense</small></div><ArrowRight /></button>
            <button onClick={() => onNavigate("debts")}><span className="quick-icon amber"><HandCoins /></span><div><strong>Noter une dette</strong><small>Transfert Orange Money, Wave ou Djamo</small></div><ArrowRight /></button>
            {dashboard.overdueDebtsCount > 0 && <div className="debt-alert"><AlertTriangle /><span><strong>{dashboard.overdueDebtsCount} dette{dashboard.overdueDebtsCount > 1 ? "s" : ""} en retard</strong><small>Consultez les échéances clients.</small></span></div>}
          </article>
        </div>
      </section>
    </div>
  );
}
