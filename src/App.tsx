import { useCallback, useEffect, useState } from "react";
import { CheckCircle2, LoaderCircle, X } from "lucide-react";
import { api } from "./api";
import { AppShell } from "./components/AppShell";
import { LoginScreen, SetupScreen } from "./screens/AuthScreens";
import { DashboardPage } from "./screens/DashboardPage";
import { DebtsPage } from "./screens/DebtsPage";
import { InventoryHistoryPage, InventoryPage } from "./screens/InventoryPages";
import { JournalPage } from "./screens/JournalPage";
import { ReportsPage } from "./screens/ReportsPage";
import { SettingsPage } from "./screens/SettingsPage";
import type { Dashboard, PageId } from "./types";

type Phase = "loading" | "setup" | "login" | "ready" | "error";

export default function App() {
  const [phase, setPhase] = useState<Phase>("loading");
  const [dashboard, setDashboard] = useState<Dashboard>();
  const [page, setPage] = useState<PageId>("dashboard");
  const [fatalError, setFatalError] = useState("");
  const [toast, setToast] = useState("");

  useEffect(() => {
    api.checkSetup()
      .then((status) => setPhase(status.initialized ? "login" : "setup"))
      .catch((reason) => { setFatalError(reason instanceof Error ? reason.message : String(reason)); setPhase("error"); });
  }, []);

  const refresh = useCallback(async () => {
    try { setDashboard(await api.dashboard()); }
    catch (reason) {
      if (reason instanceof Error && reason.message.toLowerCase().includes("session verrouillée")) {
        setDashboard(undefined); setPhase("login");
      } else throw reason;
    }
  }, []);

  useEffect(() => {
    if (phase !== "ready" || !dashboard) return;
    let timer = window.setTimeout(() => { void handleLock(); }, dashboard.settings.autoLockMinutes * 60_000);
    const reset = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => { void handleLock(); }, dashboard.settings.autoLockMinutes * 60_000);
    };
    window.addEventListener("pointerdown", reset);
    window.addEventListener("keydown", reset);
    return () => { window.clearTimeout(timer); window.removeEventListener("pointerdown", reset); window.removeEventListener("keydown", reset); };
  }, [phase, dashboard?.settings.autoLockMinutes]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(""), 4500);
    return () => window.clearTimeout(timer);
  }, [toast]);

  function authenticated(value: Dashboard) {
    setDashboard(value); setPage("dashboard"); setPhase("ready");
  }

  async function handleLock() {
    try { await api.lock(); } finally { setDashboard(undefined); setPhase("login"); }
  }

  if (phase === "loading") return <main className="splash"><div className="brand-mark large"><LoaderCircle className="spin" /></div><strong>Kër Finance</strong><span>Ouverture de l’espace sécurisé…</span></main>;
  if (phase === "setup") return <SetupScreen onSuccess={authenticated} />;
  if (phase === "login") return <LoginScreen onSuccess={authenticated} />;
  if (phase === "error") return <main className="fatal-page"><h1>Impossible d’ouvrir Kër Finance</h1><p>{fatalError}</p><button className="button primary" onClick={() => window.location.reload()}>Réessayer</button></main>;
  if (!dashboard) return null;

  const content = (() => {
    switch (page) {
      case "dashboard": return <DashboardPage dashboard={dashboard} onNavigate={setPage} />;
      case "inventory": return <InventoryPage dashboard={dashboard} onDone={setToast} />;
      case "history": return <InventoryHistoryPage onChanged={() => void refresh()} notify={setToast} />;
      case "journal": return <JournalPage onChanged={() => void refresh()} notify={setToast} />;
      case "debts": return <DebtsPage onChanged={() => void refresh()} notify={setToast} />;
      case "reports": return <ReportsPage notify={setToast} />;
      case "settings": return <SettingsPage notify={setToast} onChanged={() => void refresh()} />;
    }
  })();

  return (
    <>
      <AppShell page={page} dashboard={dashboard} onNavigate={(next) => { setPage(next); void refresh(); }} onLock={handleLock}>{content}</AppShell>
      {toast && <div className="toast"><CheckCircle2 /><span>{toast}</span><button onClick={() => setToast("")}><X /></button></div>}
    </>
  );
}
