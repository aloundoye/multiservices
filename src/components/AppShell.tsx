import {
  BarChart3,
  BookOpenText,
  Clock3,
  FileClock,
  HandCoins,
  LayoutDashboard,
  LockKeyhole,
  Settings,
  WalletCards
} from "lucide-react";
import type { Dashboard, PageId } from "../types";
import { formatDate } from "../lib/format";

const navigation: Array<{ id: PageId; label: string; icon: typeof LayoutDashboard }> = [
  { id: "dashboard", label: "Tableau de bord", icon: LayoutDashboard },
  { id: "inventory", label: "Nouvel inventaire", icon: Clock3 },
  { id: "history", label: "Historique", icon: FileClock },
  { id: "journal", label: "Journal boutique", icon: BookOpenText },
  { id: "debts", label: "Dettes clients", icon: HandCoins },
  { id: "reports", label: "Rapports", icon: BarChart3 },
  { id: "settings", label: "Paramètres", icon: Settings }
];

export function AppShell({
  page,
  dashboard,
  onNavigate,
  onLock,
  children
}: {
  page: PageId;
  dashboard: Dashboard;
  onNavigate: (page: PageId) => void;
  onLock: () => void;
  children: React.ReactNode;
}) {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="sidebar-brand"><div className="brand-mark"><WalletCards /></div><div><strong>Kër Finance</strong><span>Gestion multiservices</span></div></div>
        <nav>
          <p>GESTION</p>
          {navigation.slice(0, 5).map(({ id, label, icon: Icon }) => (
            <button className={page === id ? "active" : ""} key={id} onClick={() => onNavigate(id)}><Icon size={19} /><span>{label}</span>{id === "debts" && dashboard.overdueDebtsCount > 0 && <b>{dashboard.overdueDebtsCount}</b>}</button>
          ))}
          <p>ANALYSE & SYSTÈME</p>
          {navigation.slice(5).map(({ id, label, icon: Icon }) => (
            <button className={page === id ? "active" : ""} key={id} onClick={() => onNavigate(id)}><Icon size={19} /><span>{label}</span></button>
          ))}
        </nav>
        <div className="sidebar-footer">
          <div className="manager-avatar">G</div>
          <div><strong>Gérant</strong><span>{dashboard.settings.businessName}</span></div>
          <button className="icon-button dark" title="Verrouiller" onClick={onLock}><LockKeyhole size={18} /></button>
        </div>
      </aside>
      <main className="main-area">
        <header className="topbar">
          <div><span className="status-dot"></span> Données locales chiffrées</div>
          <div className={dashboard.inventoryOverdue ? "inventory-status overdue" : "inventory-status"}>
            <Clock3 size={16} /> {dashboard.inventoryOverdue ? "Inventaire en retard" : `Prochain inventaire ${formatDate(dashboard.nextInventoryAt, true)}`}
          </div>
        </header>
        <div className="page-content">{children}</div>
      </main>
    </div>
  );
}
