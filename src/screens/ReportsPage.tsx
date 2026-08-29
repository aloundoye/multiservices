import { useEffect, useState } from "react";
import { BarChart3, Download, FileSpreadsheet, FileText, RefreshCw, Table2 } from "lucide-react";
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import { Field, TextInput } from "../components/Fields";
import { formatMoney, today } from "../lib/format";
import type { ReportData, ReportFilters } from "../types";

function monthStart() {
  const date = today();
  return `${date.slice(0, 8)}01`;
}

export function ReportsPage({ notify }: { notify: (message: string) => void }) {
  const [filters, setFilters] = useState<ReportFilters>({ from: monthStart(), to: today() });
  const [report, setReport] = useState<ReportData>();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function load() {
    setLoading(true); setError("");
    try { setReport(await api.report(filters)); }
    catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setLoading(false); }
  }
  useEffect(() => { void load(); }, []);

  async function exportAs(format: "pdf" | "xlsx" | "csv") {
    const destination = await save({
      title: `Exporter le rapport ${format.toUpperCase()}`,
      defaultPath: `rapport-ker-finance-${today()}.${format}`,
      filters: [{ name: format.toUpperCase(), extensions: [format] }]
    });
    if (!destination) return;
    setLoading(true); setError("");
    try { await api.exportReport(format, destination, filters); notify(`Rapport ${format.toUpperCase()} créé avec succès.`); }
    catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setLoading(false); }
  }

  return (
    <div className="page">
      <header className="page-header"><div><p className="eyebrow">ANALYSE</p><h1>Rapports et exports</h1><p>Analysez les inventaires, mouvements et créances sur une période.</p></div></header>
      <section className="panel report-controls"><div className="date-filters"><Field label="Du"><TextInput type="date" value={filters.from ?? ""} onChange={(e) => setFilters({ ...filters, from: e.target.value || undefined })} /></Field><Field label="Au"><TextInput type="date" value={filters.to ?? ""} onChange={(e) => setFilters({ ...filters, to: e.target.value || undefined })} /></Field><button className="button secondary filter-button" onClick={load} disabled={loading}><RefreshCw className={loading ? "spin" : ""} /> Actualiser</button></div><div className="export-buttons"><button className="button export pdf" onClick={() => exportAs("pdf")}><FileText /> PDF</button><button className="button export xlsx" onClick={() => exportAs("xlsx")}><FileSpreadsheet /> Excel</button><button className="button export csv" onClick={() => exportAs("csv")}><Table2 /> CSV</button></div></section>
      {error && <div className="form-error page-error">{error}</div>}
      {report && <>
        <section className="metric-grid report-metrics"><article className="metric-card"><span>Recettes et apports</span><strong className="positive">{formatMoney(report.totalPositive)}</strong><small>Écritures positives de la période</small></article><article className="metric-card"><span>Achats et sorties</span><strong className="negative">{formatMoney(report.totalNegative)}</strong><small>Écritures négatives de la période</small></article><article className="metric-card"><span>Écarts cumulés</span><strong className={report.totalVariance >= 0 ? "positive" : "negative"}>{formatMoney(report.totalVariance)}</strong><small>{report.inventories.length} inventaire(s)</small></article><article className="metric-card"><span>Créances actuelles</span><strong>{formatMoney(report.outstandingReceivables)}</strong><small>Reste dû à ce jour</small></article></section>
        <section className="report-grid"><article className="panel report-card"><header><span className="report-icon"><BarChart3 /></span><div><h2>Inventaires</h2><p>{report.inventories.length} clôture(s) sur la période</p></div></header><div className="mini-bars">{report.inventories.slice(0, 10).reverse().map((item) => { const width = report.inventories.length ? Math.max(6, Math.min(100, (item.actualTotal / Math.max(...report.inventories.map((value) => value.actualTotal), 1)) * 100)) : 0; return <div key={item.id}><span>{item.closedAt.slice(5, 10)}</span><div><i style={{ width: `${width}%` }}></i></div><strong>{formatMoney(item.actualTotal)}</strong></div>; })}</div>{report.inventories.length === 0 && <div className="empty-inline">Aucun inventaire dans cette période.</div>}</article><article className="panel report-card"><header><span className="report-icon amber"><Download /></span><div><h2>Composition du rapport</h2><p>Données incluses dans chaque export</p></div></header><ul className="report-content-list"><li><span>Inventaires et écarts</span><strong>{report.inventories.length}</strong></li><li><span>Écritures du journal</span><strong>{report.journal.length}</strong></li><li><span>Dettes créées</span><strong>{report.debts.length}</strong></li><li><span>Remboursements</span><strong>{report.debts.reduce((sum, debt) => sum + debt.payments.length, 0)}</strong></li></ul></article></section>
      </>}
    </div>
  );
}
