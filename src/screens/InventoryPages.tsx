import { useEffect, useMemo, useState, type FormEvent } from "react";
import { AlertCircle, ArrowRight, CheckCircle2, Clock3, Eye, History, LoaderCircle, RotateCcw, Scale, ShieldCheck } from "lucide-react";
import { api } from "../api";
import { Field, MoneyInput, SelectInput, TextArea } from "../components/Fields";
import { Modal } from "../components/Modal";
import { formatDate, formatMoney, label, signed } from "../lib/format";
import type { AccountBalances, CloseInventoryResult, Dashboard, Inventory, InventoryPreview } from "../types";

type BalanceKey = keyof AccountBalances;

const accountMeta: Array<{ key: BalanceKey; name: string; color: string }> = [
  { key: "orangeMoney", name: "Orange Money", color: "orange" },
  { key: "wave", name: "Wave", color: "wave" },
  { key: "djamo", name: "Djamo", color: "djamo" },
  { key: "cash", name: "Espèces", color: "cash" }
];

export function InventoryPage({ dashboard, onDone }: { dashboard: Dashboard; onDone: (message: string) => void }) {
  const [balances, setBalances] = useState<AccountBalances>(dashboard.lastInventory.balances);
  const [preview, setPreview] = useState<InventoryPreview>();
  const [category, setCategory] = useState("");
  const [note, setNote] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [closed, setClosed] = useState<CloseInventoryResult>();

  useEffect(() => {
    const timer = window.setTimeout(async () => {
      try {
        setPreview(await api.previewInventory(balances as unknown as Record<string, number>));
        setError("");
      } catch (reason) {
        setPreview(undefined);
        setError(reason instanceof Error ? reason.message : String(reason));
      }
    }, 250);
    return () => window.clearTimeout(timer);
  }, [balances]);

  function update(key: BalanceKey, value: string) {
    setBalances((current) => ({ ...current, [key]: Number(value) }));
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    setLoading(true);
    setError("");
    try {
      const result = await api.closeInventory({
        ...balances,
        varianceCategory: category || null,
        varianceNote: note || null
      });
      setClosed(result);
      onDone(result.backupWarning ? `Inventaire clôturé. Attention: ${result.backupWarning}` : "Inventaire clôturé et sauvegardé.");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  if (closed) {
    return (
      <div className="page completion-page">
        <div className="completion-card"><div className="success-ring"><CheckCircle2 /></div><p className="eyebrow">INVENTAIRE CLÔTURÉ</p><h1>Les soldes sont enregistrés</h1><p>Le capital réel de {formatMoney(closed.inventory.actualTotal)} devient la nouvelle référence.</p><div className="completion-metrics"><div><span>Capital attendu</span><strong>{formatMoney(closed.inventory.expectedTotal)}</strong></div><div><span>Capital réel</span><strong>{formatMoney(closed.inventory.actualTotal)}</strong></div><div><span>Écart conservé</span><strong className={closed.inventory.variance < 0 ? "negative" : "positive"}>{signed(closed.inventory.variance)}</strong></div></div>{closed.backup && <div className="backup-confirm"><ShieldCheck /> Sauvegarde automatique créée avec succès.</div>}<button className="button primary" onClick={() => window.location.reload()}>Retour au tableau de bord <ArrowRight /></button></div>
      </div>
    );
  }

  return (
    <div className="page inventory-page">
      <header className="page-header"><div><p className="eyebrow">CONTRÔLE DES SOLDES</p><h1>Nouvel inventaire</h1><p>Comptez chaque compte. Les créances ouvertes sont ajoutées automatiquement.</p></div><div className="reference-chip"><Clock3 /><span>Dernière clôture<strong>{formatDate(dashboard.lastInventory.closedAt, true)}</strong></span></div></header>
      <form onSubmit={submit} className="inventory-layout">
        <section className="panel inventory-form-panel">
          <header className="panel-header"><div><h2>Soldes réellement constatés</h2><p>Saisissez les montants affichés sur chaque compte et les espèces comptées.</p></div></header>
          <div className="inventory-account-grid">
            {accountMeta.map(({ key, name, color }) => (
              <div className="inventory-account" key={key}>
                <div className={`account-badge ${color}`}><span></span>{name}</div>
                <MoneyInput value={balances[key]} onChange={(event) => update(key, event.target.value)} required />
                <div className="comparison"><span>Précédent: {formatMoney(dashboard.lastInventory.balances[key])}</span><strong className={(preview?.delta[key] ?? 0) > 0 ? "positive" : (preview?.delta[key] ?? 0) < 0 ? "negative" : ""}>{signed(preview?.delta[key] ?? 0)}</strong></div>
              </div>
            ))}
          </div>
          {error && <div className="form-error">{error}</div>}
        </section>

        <aside className="inventory-summary panel">
          <div className="summary-title"><Scale /><div><h2>Contrôle d’équilibre</h2><p>Calculé par le moteur comptable</p></div></div>
          {!preview ? <div className="preview-loading"><LoaderCircle className="spin" /> Calcul en cours…</div> : <>
            <div className="summary-lines">
              <div><span>Liquidités saisies</span><strong>{formatMoney(preview.liquidity)}</strong></div>
              <div><span>Créances ouvertes</span><strong>{formatMoney(preview.receivables)}</strong></div>
              <div className="summary-separator"><span>Capital réel</span><strong>{formatMoney(preview.actualTotal)}</strong></div>
              <div><span>Capital attendu</span><strong>{formatMoney(preview.expectedTotal)}</strong></div>
            </div>
            <div className={`variance-box ${preview.variance === 0 ? "balanced" : preview.variance > 0 ? "surplus" : "shortage"}`}>
              {preview.variance === 0 ? <CheckCircle2 /> : <AlertCircle />}
              <div><span>{preview.variance === 0 ? "Inventaire équilibré" : preview.variance > 0 ? "Surplus constaté" : "Manquant constaté"}</span><strong>{signed(preview.variance)}</strong></div>
            </div>
            {preview.variance !== 0 && <div className="variance-fields"><Field label="Catégorie de l’écart"><SelectInput value={category} onChange={(e) => setCategory(e.target.value)} required><option value="">Choisir…</option><option value="commission_mobile">Commission mobile</option><option value="surplus_caisse">Surplus de caisse</option><option value="manquant_caisse">Manquant de caisse</option><option value="erreur_saisie">Erreur de saisie</option><option value="autre">Autre</option></SelectInput></Field><Field label="Explication obligatoire"><TextArea value={note} onChange={(e) => setNote(e.target.value)} placeholder="Décrivez l’origine probable de l’écart…" required /></Field></div>}
            <button className="button primary full" disabled={loading}>{loading ? "Clôture…" : "Clôturer l’inventaire"}<ArrowRight /></button>
            <p className="immutable-note"><ShieldCheck /> Une clôture ne peut plus être modifiée.</p>
          </>}
        </aside>
      </form>
    </div>
  );
}

export function InventoryHistoryPage({ onChanged, notify }: { onChanged: () => void; notify: (message: string) => void }) {
  const [items, setItems] = useState<Inventory[]>([]);
  const [selected, setSelected] = useState<Inventory>();
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [correctionOpen, setCorrectionOpen] = useState(false);
  const [correction, setCorrection] = useState({ amount: 0, direction: "increase", paymentAccount: "cash", reason: "" });
  const [correctionError, setCorrectionError] = useState("");

  useEffect(() => { api.inventories().then(setItems).finally(() => setLoading(false)); }, []);
  const filtered = useMemo(() => items.filter((item) => item.closedAt.includes(query) || item.varianceNote?.toLowerCase().includes(query.toLowerCase())), [items, query]);

  async function submitCorrection(event: FormEvent) {
    event.preventDefault();
    if (!selected) return;
    setCorrectionError("");
    try {
      await api.correctInventory({ inventoryId: selected.id, ...correction });
      setCorrectionOpen(false);
      setCorrection({ amount: 0, direction: "increase", paymentAccount: "cash", reason: "" });
      onChanged();
      notify("Correction liée à l’inventaire ajoutée au journal.");
    } catch (reason) {
      setCorrectionError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  return (
    <div className="page">
      <header className="page-header"><div><p className="eyebrow">TRAÇABILITÉ</p><h1>Historique des inventaires</h1><p>Chaque clôture est immuable et comparée à la précédente.</p></div><div className="header-stat"><History /><span>{items.length}<small>clôtures enregistrées</small></span></div></header>
      <section className="panel table-panel">
        <div className="table-toolbar"><input className="input search" placeholder="Rechercher une date ou une note…" value={query} onChange={(e) => setQuery(e.target.value)} /><span>{filtered.length} résultat{filtered.length > 1 ? "s" : ""}</span></div>
        {loading ? <div className="empty-state"><LoaderCircle className="spin" /> Chargement…</div> : <div className="table-scroll"><table><thead><tr><th>Date de clôture</th><th>Liquidités</th><th>Créances</th><th>Capital attendu</th><th>Capital réel</th><th>Écart</th><th></th></tr></thead><tbody>{filtered.map((item) => <tr key={item.id}><td><strong>{item.kind === "opening" ? "Ouverture" : formatDate(item.closedAt, true)}</strong>{item.kind === "opening" && <small>{formatDate(item.closedAt, true)}</small>}</td><td>{formatMoney(item.liquidity)}</td><td>{formatMoney(item.receivables)}</td><td>{formatMoney(item.expectedTotal)}</td><td><strong>{formatMoney(item.actualTotal)}</strong></td><td><span className={`status-chip ${item.variance === 0 ? "paid" : item.variance > 0 ? "open" : "overdue"}`}>{signed(item.variance)}</span></td><td><button className="icon-button" onClick={() => setSelected(item)}><Eye size={18} /></button></td></tr>)}</tbody></table></div>}
      </section>
      <Modal title="Détail de l’inventaire" subtitle={selected ? formatDate(selected.closedAt, true) : ""} open={Boolean(selected)} onClose={() => setSelected(undefined)} wide>
        {selected && <div className="inventory-detail"><div className="detail-cards">{accountMeta.map(({ key, name, color }) => <div key={key}><span className={`dot ${color}`}></span><small>{name}</small><strong>{formatMoney(selected.balances[key])}</strong><em>{signed(selected.delta[key])} depuis le précédent</em></div>)}</div><div className="detail-summary"><div><span>Liquidités</span><strong>{formatMoney(selected.liquidity)}</strong></div><div><span>Créances</span><strong>{formatMoney(selected.receivables)}</strong></div><div><span>Attendu</span><strong>{formatMoney(selected.expectedTotal)}</strong></div><div><span>Réel</span><strong>{formatMoney(selected.actualTotal)}</strong></div><div><span>Écart</span><strong>{signed(selected.variance)}</strong></div></div>{selected.variance !== 0 && <div className="justification"><strong>{label(selected.varianceCategory ?? "autre")}</strong><p>{selected.varianceNote}</p></div>}<button className="button secondary correction-button" onClick={() => { setCorrectionError(""); setCorrectionOpen(true); }}><RotateCcw /> Ajouter une correction liée</button></div>}
      </Modal>
      <Modal title="Correction liée à l’inventaire" subtitle="L’inventaire reste intact; une écriture auditée ajuste le capital attendu." open={correctionOpen} onClose={() => setCorrectionOpen(false)}>
        <form className="modal-form" onSubmit={submitCorrection}><div className="form-grid"><Field label="Sens"><SelectInput value={correction.direction} onChange={(e) => setCorrection({ ...correction, direction: e.target.value })}><option value="increase">Augmenter le capital</option><option value="decrease">Diminuer le capital</option></SelectInput></Field><Field label="Montant"><MoneyInput value={correction.amount} onChange={(e) => setCorrection({ ...correction, amount: Number(e.target.value) })} required /></Field></div><Field label="Compte concerné"><SelectInput value={correction.paymentAccount} onChange={(e) => setCorrection({ ...correction, paymentAccount: e.target.value })}><option value="cash">Espèces</option><option value="orange_money">Orange Money</option><option value="wave">Wave</option><option value="djamo">Djamo</option></SelectInput></Field><Field label="Motif obligatoire"><TextArea value={correction.reason} onChange={(e) => setCorrection({ ...correction, reason: e.target.value })} required /></Field>{correctionError && <div className="form-error">{correctionError}</div>}<div className="modal-actions"><button type="button" className="button secondary" onClick={() => setCorrectionOpen(false)}>Annuler</button><button className="button primary">Enregistrer la correction</button></div></form>
      </Modal>
    </div>
  );
}
