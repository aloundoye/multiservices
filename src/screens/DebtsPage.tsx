import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { AlertTriangle, Eye, HandCoins, Plus, Search, Users } from "lucide-react";
import { api } from "../api";
import { AccountSelect, accountDisplayLabel } from "../components/AccountSelect";
import { Field, MoneyInput, TextArea, TextInput } from "../components/Fields";
import { Modal } from "../components/Modal";
import { formatDate, formatMoney, label, today } from "../lib/format";
import type { CustomerRepayment, Debt, DebtCustomer, RepaymentAllocation, RepaymentPreview, SaveDebtCustomerInput } from "../types";

const debtForm = (customerId = "") => ({ requestId: crypto.randomUUID(), customerId, accountId: "", amount: 0, issuedAt: today(), dueDate: "", note: "" });
const paymentForm = (customerId = "") => ({ requestId: crypto.randomUUID(), customerId, accountId: "", amount: 0, paidAt: today(), note: "" });
const message = (reason: unknown) => reason instanceof Error ? reason.message : String(reason);
const validMoney = (amount: number) => Number.isSafeInteger(amount) && amount > 0;

function CustomerSelect({ customers, value, onChange }: { customers: DebtCustomer[]; value: string; onChange: (id: string) => void }) {
  const [search, setSearch] = useState("");
  const options = customers.filter((c) => c.active && (c.id === value || `${c.name} ${c.phone}`.toLowerCase().includes(search.toLowerCase())));
  return <><Field label="Rechercher un client"><TextInput type="search" placeholder="Nom ou téléphone…" value={search} onChange={(e) => setSearch(e.target.value)} /></Field><Field label="Client"><select className="input" required value={value} onChange={(e) => onChange(e.target.value)}><option value="">Choisir un client</option>{options.map((c) => <option key={c.id} value={c.id}>{c.name} · {c.phone}</option>)}</select></Field></>;
}
function Allocations({ items }: { items: RepaymentAllocation[] }) {
  return <div className="table-scroll"><table><thead><tr><th>Dette du</th><th>Affectation</th><th>Reste après</th></tr></thead><tbody>{items.map((a) => <tr key={a.debtId}><td>{formatDate(a.issuedAt)}<small className="debt-reference">Réf. {a.debtId}</small></td><td>{formatMoney(a.amount)}</td><td>{a.remainingAfter === null ? "Non suivi à cette date" : formatMoney(a.remainingAfter)}</td></tr>)}</tbody></table></div>;
}

export function DebtsPage({ onChanged, notify }: { onChanged: () => void | Promise<void>; notify: (message: string) => void }) {
  const [tab, setTab] = useState<"clients" | "debts" | "repayments">("clients");
  const [clients, setClients] = useState<DebtCustomer[]>([]);
  const [debts, setDebts] = useState<Debt[]>([]);
  const [receipts, setReceipts] = useState<CustomerRepayment[]>([]);
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("active");
  const [showArchived, setShowArchived] = useState(false);
  const [period, setPeriod] = useState({ from: "", to: "" });
  const [clientId, setClientId] = useState<string>();
  const [debtId, setDebtId] = useState<string>();
  const [receiptId, setReceiptId] = useState<string>();
  const [customerForm, setCustomerForm] = useState<SaveDebtCustomerInput>();
  const [createOpen, setCreateOpen] = useState(false);
  const [form, setForm] = useState(debtForm);
  const [payOpen, setPayOpen] = useState(false);
  const [payment, setPayment] = useState(paymentForm);
  const [preview, setPreview] = useState<RepaymentPreview>();
  const [previewError, setPreviewError] = useState("");
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewRevision, setPreviewRevision] = useState(0);
  const [cancelOpen, setCancelOpen] = useState(false);
  const [cancelReason, setCancelReason] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const inFlight = useRef(false);
  const selectedClient = clients.find((c) => c.id === clientId);
  const selectedDebt = debts.find((d) => d.id === debtId);
  const selectedReceipt = receipts.find((p) => p.id === receiptId);

  async function load() {
    const [c, d, p] = await Promise.all([api.debtCustomers(), api.debts(), api.customerRepayments()]);
    setClients(c); setDebts(d); setReceipts(p);
  }
  useEffect(() => { void load().catch((e) => setError(message(e))); }, []);
  useEffect(() => {
    let current = true;
    setPreview(undefined); setPreviewError(""); setPreviewLoading(false);
    if (!payOpen || !payment.customerId || !validMoney(payment.amount) || !payment.paidAt) return;
    setPreviewLoading(true);
    const timer = setTimeout(() => {
      void api.previewRepayment({ customerId: payment.customerId, amount: payment.amount, paidAt: payment.paidAt })
        .then((result) => { if (current) setPreview(result); })
        .catch((e) => { if (current) setPreviewError(message(e)); })
        .finally(() => { if (current) setPreviewLoading(false); });
    }, 200);
    return () => { current = false; clearTimeout(timer); };
  }, [payOpen, payment.customerId, payment.amount, payment.paidAt, previewRevision]);

  async function save(work: () => Promise<unknown>, finish: () => void, success: string) {
    if (inFlight.current) return;
    inFlight.current = true; setBusy(true); setError("");
    try {
      await work(); finish(); notify(success);
      try { await load(); await onChanged(); }
      catch (e) { setError(`Enregistrement effectué. Actualisation impossible : ${message(e)}`); }
    } catch (e) { setError(message(e)); }
    finally { inFlight.current = false; setBusy(false); }
  }
  function addDebt(id = "") { setClientId(undefined); setDebtId(undefined); setForm(debtForm(id)); setCreateOpen(true); setError(""); }
  function repay(id = "") { setClientId(undefined); setDebtId(undefined); setPayment(paymentForm(id)); setPreview(undefined); setPayOpen(true); setError(""); }
  function editClient(client?: DebtCustomer) {
    setError(""); setCustomerForm({ requestId: crypto.randomUUID(), customerId: client?.id ?? null, name: client?.name ?? "", phone: client?.phone ?? "", active: client?.active ?? true });
  }
  function saveClient(e: FormEvent) {
    e.preventDefault(); if (!customerForm) return;
    void save(async () => {
      const client = await api.saveDebtCustomer(customerForm);
      if (createOpen) setForm((f) => ({ ...f, customerId: client.id }));
    }, () => setCustomerForm(undefined), "Fiche client enregistrée.");
  }
  function create(e: FormEvent) {
    e.preventDefault(); if (!validMoney(form.amount)) { setError("Saisissez un montant FCFA entier supérieur à zéro."); return; }
    void save(() => api.createDebt({ ...form, dueDate: form.dueDate || null, note: form.note || null }), () => setCreateOpen(false), "Dette enregistrée dans les créances.");
  }
  function pay(e: FormEvent) {
    e.preventDefault(); if (!preview || previewLoading || !validMoney(payment.amount)) return;
    void save(() => api.repayCustomer({ ...payment, note: payment.note || null, previewToken: preview.token }), () => setPayOpen(false), "Remboursement et répartition enregistrés.");
  }
  function archive(client: DebtCustomer) {
    void save(() => api.saveDebtCustomer({ requestId: crypto.randomUUID(), customerId: client.id, name: client.name, phone: client.phone, active: !client.active }), () => {}, client.active ? "Client archivé." : "Client réactivé.");
  }
  const matches = (name: string, phone: string) => `${name} ${phone}`.toLowerCase().includes(query.toLowerCase());
  const visibleDebts = debts.filter((d) => matches(d.customerName, d.phone) && (status === "all" || (status === "active" ? ["open", "partial", "overdue"].includes(d.status) : status === d.status)));
  const visibleClients = clients.filter((c) => (showArchived || c.active) && matches(c.name, c.phone));
  const visibleReceipts = receipts.filter((p) => matches(p.customerName, p.customerPhone) && (!period.from || p.paidAt >= period.from) && (!period.to || p.paidAt <= period.to));
  const totals = useMemo(() => ({ remaining: clients.reduce((s, c) => s + c.remaining, 0), overdue: clients.reduce((s, c) => s + c.overdueCount, 0) }), [clients]);
  const modalError = error && <div className="form-error" role="alert">{error}</div>;
  const close = (fn: () => void) => { if (!inFlight.current) fn(); };

  return <div className="page">
    <header className="page-header"><div><p className="eyebrow">CRÉANCES CLIENTS</p><h1>Dettes clients</h1><p>Un répertoire pour vos clients, leurs dettes et leurs remboursements.</p></div><div className="product-row-actions"><button className="button secondary" onClick={() => repay()}>Enregistrer un remboursement</button><button className="button primary" onClick={() => addDebt()}><Plus /> Ajouter une dette</button></div></header>
    <section className="metric-grid debt-metrics"><article className="metric-card hero-metric"><div className="metric-icon amber"><HandCoins /></div><span>Total restant dû</span><strong>{formatMoney(totals.remaining)}</strong><small>Créances incluses dans le capital réel</small></article><article className="metric-card"><div className="metric-icon coral"><AlertTriangle /></div><span>Dettes en retard</span><strong>{totals.overdue}</strong><small>Clients à relancer</small></article><article className="metric-card"><div className="metric-icon green"><Users /></div><span>Clients actifs</span><strong>{clients.filter((c) => c.active).length}</strong><small>Répertoire indépendant des dépôts</small></article></section>
    <div className="settings-tabs" role="tablist" aria-label="Dettes clients">{([['clients', 'Clients'], ['debts', 'Dettes'], ['repayments', 'Remboursements']] as const).map(([key, title]) => <button key={key} role="tab" aria-selected={tab === key} className={tab === key ? "active" : ""} onClick={() => setTab(key)}>{title}</button>)}</div>
    {error && !customerForm && !createOpen && !payOpen && !cancelOpen && <div className="form-error" role="alert">{error}<button className="button secondary" onClick={() => { setError(""); void load().catch((e) => setError(message(e))); }}>Actualiser</button></div>}
    <section className="panel table-panel">
      <div className="table-toolbar"><div className="search-wrap"><Search /><input className="input search" aria-label="Rechercher dans le répertoire" placeholder="Nom ou téléphone du client…" value={query} onChange={(e) => setQuery(e.target.value)} /></div>
        {tab === "clients" && <><label className="checkbox-label"><input type="checkbox" checked={showArchived} onChange={(e) => setShowArchived(e.target.checked)} /> Inclure les clients archivés</label><button className="button primary" onClick={() => editClient()}><Plus /> Ajouter un client</button></>}
        {tab === "debts" && <select className="input compact" aria-label="Statut des dettes" value={status} onChange={(e) => setStatus(e.target.value)}><option value="active">Créances actives</option><option value="overdue">En retard</option><option value="paid">Soldées</option><option value="cancelled">Annulées</option><option value="all">Toutes</option></select>}
        {tab === "repayments" && <div className="form-grid"><Field label="Paiements du"><TextInput type="date" value={period.from} onChange={(e) => setPeriod({ ...period, from: e.target.value })} /></Field><Field label="Au"><TextInput type="date" value={period.to} onChange={(e) => setPeriod({ ...period, to: e.target.value })} /></Field></div>}
      </div>
      {tab === "clients" && <><div className="table-scroll"><table><thead><tr><th>Client</th><th>Reste dû</th><th>Remboursements cumulés</th><th>En retard</th><th></th></tr></thead><tbody>{visibleClients.map((c) => <tr key={c.id}><td><strong>{c.name}</strong><small className="debt-reference">{c.phone}{!c.active && " · Archivé"}</small></td><td>{formatMoney(c.remaining)}</td><td>{formatMoney(c.totalRepaid)}</td><td>{c.overdueCount} · {formatMoney(c.overdueAmount)}</td><td><button className="button secondary" onClick={() => { setClientId(c.id); setError(""); }}>Ouvrir la fiche</button></td></tr>)}</tbody></table></div>{!visibleClients.length && <div className="empty-state"><Users /><h3>Aucun client dans cette vue</h3><p>Ajoutez ses coordonnées une seule fois pour ses prochaines dettes.</p></div>}</>}
      {tab === "debts" && <><div className="table-scroll"><table><thead><tr><th>Client à la date du prêt</th><th>Compte</th><th>Date / échéance</th><th>Montant initial</th><th>Reste dû</th><th>Statut</th><th></th></tr></thead><tbody>{visibleDebts.map((d) => <tr key={d.id}><td><strong>{d.customerName}</strong><small className="debt-reference">{d.phone}</small></td><td>{accountDisplayLabel(d.accountSnapshot)}</td><td>{formatDate(d.issuedAt)}<small className="debt-reference">{d.dueDate ? formatDate(d.dueDate) : "Sans échéance"}</small></td><td>{formatMoney(d.principal)}</td><td>{formatMoney(d.remaining)}</td><td><span className={`status-chip ${d.status}`}>{label(d.status)}</span></td><td><button className="icon-button" aria-label={`Détail de la dette de ${d.customerName}`} onClick={() => { setDebtId(d.id); setError(""); }}><Eye /></button></td></tr>)}</tbody></table></div>{!visibleDebts.length && <p className="empty-inline">Aucune dette dans cette vue.</p>}</>}
      {tab === "repayments" && <><p className="custody-table-note">Un total par paiement, avec sa répartition sur les dettes. Ces montants ne sont pas des recettes du journal.</p><div className="table-scroll"><table><thead><tr><th>Date</th><th>Client au paiement</th><th>Compte</th><th>Total reçu</th><th>Type</th><th></th></tr></thead><tbody>{visibleReceipts.map((p) => <tr key={p.id}><td>{formatDate(p.paidAt)}</td><td><strong>{p.customerName}</strong><small className="debt-reference">{p.customerPhone}</small></td><td>{accountDisplayLabel(p.accountSnapshot)}</td><td>{formatMoney(p.amount)}</td><td>{p.legacy ? "Unitaire historique" : `${p.allocations.length} dette(s)`}</td><td><button className="button secondary" onClick={() => setReceiptId(p.id)}>Voir la répartition</button></td></tr>)}</tbody></table></div>{!visibleReceipts.length && <p className="empty-inline">Aucun remboursement dans cette vue.</p>}</>}
    </section>

    <Modal title={selectedClient?.name ?? "Fiche client"} subtitle={selectedClient?.phone} open={Boolean(selectedClient) && !customerForm} onClose={() => close(() => setClientId(undefined))} wide>
      {selectedClient && <div className="modal-form"><p className="debt-reference">Identifiant permanent : {selectedClient.id}</p><div className="summary-strip"><div><p>Reste dû<strong>{formatMoney(selectedClient.remaining)}</strong></p></div><div><p>Remboursements cumulés<strong>{formatMoney(selectedClient.totalRepaid)}</strong></p></div><div><p>En retard<strong>{selectedClient.overdueCount} dette(s)</strong></p></div></div><div className="product-row-actions"><button disabled={busy} className="button secondary" onClick={() => editClient(selectedClient)}>Modifier les coordonnées</button><button disabled={busy || (selectedClient.active && selectedClient.remaining > 0)} className="button secondary" onClick={() => archive(selectedClient)}>{selectedClient.active ? "Archiver" : "Réactiver"}</button>{selectedClient.active && <><button className="button primary" onClick={() => addDebt(selectedClient.id)}>Ajouter une dette</button><button className="button primary" disabled={selectedClient.remaining === 0} onClick={() => repay(selectedClient.id)}>Enregistrer un remboursement</button></>}</div>{selectedClient.active && selectedClient.remaining > 0 && <p className="field-hint">L’archivage sera disponible quand le solde sera nul.</p>}<h3>Dettes de ce client</h3>{debts.filter((d) => d.customerId === selectedClient.id).map((d) => <button className="debt-history-row" key={d.id} onClick={() => { setClientId(undefined); setDebtId(d.id); }}><span>{formatDate(d.issuedAt)} · {label(d.status)}</span><strong>{formatMoney(d.remaining)} restant</strong></button>)}{modalError}</div>}
    </Modal>

    <Modal title={customerForm?.customerId ? "Modifier le client" : "Ajouter un client"} subtitle="Répertoire des dettes uniquement. Les coordonnées passées restent inchangées." open={Boolean(customerForm)} onClose={() => close(() => setCustomerForm(undefined))}>
      {customerForm && <form onSubmit={saveClient}><fieldset className="operation-fields" disabled={busy}><Field label="Nom complet"><TextInput autoFocus required minLength={2} value={customerForm.name} onChange={(e) => setCustomerForm({ ...customerForm, name: e.target.value })} /></Field><Field label="Téléphone"><TextInput type="tel" required minLength={6} value={customerForm.phone} onChange={(e) => setCustomerForm({ ...customerForm, phone: e.target.value })} /></Field>{modalError}<div className="modal-actions"><button type="button" className="button secondary" onClick={() => setCustomerForm(undefined)}>Retour</button><button className="button primary">Enregistrer le client</button></div></fieldset></form>}
    </Modal>

    <Modal title="Ajouter une dette" subtitle="La somme reste une créance incluse dans le capital réel." open={createOpen && !customerForm} onClose={() => close(() => setCreateOpen(false))}>
      <form onSubmit={create}><fieldset className="operation-fields" disabled={busy}><CustomerSelect customers={clients} value={form.customerId} onChange={(customerId) => setForm({ ...form, customerId })} /><button type="button" className="button secondary" onClick={() => editClient()}><Plus /> Ajouter un client</button><div className="form-grid"><Field label="Compte émetteur"><AccountSelect mobileOnly value={form.accountId} onChange={(accountId) => setForm({ ...form, accountId })} /></Field><Field label="Montant transféré"><MoneyInput min={1} max={Number.MAX_SAFE_INTEGER} required value={form.amount} onChange={(e) => setForm({ ...form, amount: Number(e.target.value) })} /></Field></div><div className="form-grid"><Field label="Date du transfert"><TextInput type="date" required value={form.issuedAt} onChange={(e) => setForm({ ...form, issuedAt: e.target.value })} /></Field><Field label="Échéance (facultatif)"><TextInput type="date" min={form.issuedAt} value={form.dueDate} onChange={(e) => setForm({ ...form, dueDate: e.target.value })} /></Field></div><Field label="Note (facultatif)"><TextArea value={form.note} onChange={(e) => setForm({ ...form, note: e.target.value })} /></Field>{modalError}<div className="modal-actions"><button type="button" className="button secondary" onClick={() => setCreateOpen(false)}>Annuler</button><button className="button primary">Enregistrer la dette</button></div></fieldset></form>
    </Modal>

    <Modal title="Enregistrer un remboursement" subtitle="Le paiement rembourse les dettes les plus anciennes de ce client, à la date choisie." open={payOpen} onClose={() => close(() => setPayOpen(false))} wide>
      <form onSubmit={pay}><fieldset className="operation-fields" disabled={busy}><CustomerSelect customers={clients} value={payment.customerId} onChange={(customerId) => setPayment({ ...payment, customerId })} /><div className="form-grid"><Field label="Montant reçu"><MoneyInput min={1} max={Number.MAX_SAFE_INTEGER} required value={payment.amount} onChange={(e) => setPayment({ ...payment, amount: Number(e.target.value) })} /></Field><Field label="Date du remboursement"><TextInput type="date" required value={payment.paidAt} onChange={(e) => setPayment({ ...payment, paidAt: e.target.value })} /></Field></div><Field label="Reçu sur"><AccountSelect value={payment.accountId} onChange={(accountId) => setPayment({ ...payment, accountId })} /></Field><Field label="Note (facultatif)"><TextArea value={payment.note} onChange={(e) => setPayment({ ...payment, note: e.target.value })} /></Field><section aria-live="polite"><h3>Répartition avant validation</h3>{previewLoading && <p>Calcul de la répartition…</p>}{previewError && <p className="form-error">{previewError}</p>}{preview && <><p>Total reçu : <strong>{formatMoney(payment.amount)}</strong> sur {formatMoney(preview.eligibleTotal)} remboursables à cette date.</p><Allocations items={preview.allocations} /></>}{!preview && !previewError && !previewLoading && <p className="field-hint">Choisissez un client, un montant et une date pour voir la répartition.</p>}<button type="button" className="button secondary" disabled={previewLoading} onClick={() => { setPreview(undefined); setPreviewRevision((v) => v + 1); setError(""); }}>Actualiser l’aperçu</button></section><p className="field-hint">Le capital attendu reste inchangé. Aucun excédent ne sera enregistré comme avance ou dépôt.</p>{modalError}<div className="modal-actions"><button type="button" className="button secondary" onClick={() => setPayOpen(false)}>Annuler</button><button className="button primary" disabled={!preview || previewLoading}>Valider le remboursement</button></div></fieldset></form>
    </Modal>

    <Modal title={selectedDebt?.customerName ?? "Détail de la dette"} subtitle={selectedDebt?.phone} open={Boolean(selectedDebt) && !cancelOpen} onClose={() => close(() => setDebtId(undefined))} wide>
      {selectedDebt && <div className="modal-form"><p>{formatDate(selectedDebt.issuedAt)} · {accountDisplayLabel(selectedDebt.accountSnapshot)} · {label(selectedDebt.status)}</p><h3>{formatMoney(selectedDebt.remaining)} restant sur {formatMoney(selectedDebt.principal)}</h3><p>Échéance : {selectedDebt.dueDate ? formatDate(selectedDebt.dueDate) : "Non définie"}</p>{selectedDebt.note && <p>{selectedDebt.note}</p>}<div className="product-row-actions">{selectedDebt.remaining > 0 && <button className="button primary" onClick={() => repay(selectedDebt.customerId)}>Rembourser ce client</button>}{selectedDebt.status !== "cancelled" && <button className="button danger-ghost" onClick={() => { setCancelReason(""); setCancelOpen(true); setError(""); }}>Annuler par correction</button>}</div><h3>Montants affectés à cette dette</h3>{selectedDebt.payments.map((p) => <div className="debt-history-row" key={p.id}><span>{formatDate(p.paidAt)} · {accountDisplayLabel(p.accountSnapshot)}{p.note && <small className="debt-reference">{p.note}</small>}</span><strong>{formatMoney(p.amount)}</strong></div>)}{!selectedDebt.payments.length && <p>Aucun remboursement enregistré.</p>}{modalError}</div>}
    </Modal>
    <Modal title="Annuler cette dette par correction" subtitle="Le reste dû sera mis à zéro. L’historique sera conservé." open={cancelOpen} onClose={() => close(() => setCancelOpen(false))}>
      <form onSubmit={(e) => { e.preventDefault(); if (selectedDebt) void save(() => api.cancelDebt(selectedDebt.id, cancelReason), () => setCancelOpen(false), "Dette annulée avec trace d’audit."); }}><fieldset className="operation-fields" disabled={busy}><Field label="Motif obligatoire"><TextArea required minLength={3} value={cancelReason} onChange={(e) => setCancelReason(e.target.value)} /></Field>{modalError}<div className="modal-actions"><button type="button" className="button secondary" onClick={() => setCancelOpen(false)}>Retour</button><button className="button danger">Confirmer l’annulation</button></div></fieldset></form>
    </Modal>
    <Modal title="Détail du remboursement" subtitle={selectedReceipt ? `${selectedReceipt.customerName} · ${selectedReceipt.customerPhone}` : ""} open={Boolean(selectedReceipt)} onClose={() => setReceiptId(undefined)} wide>
      {selectedReceipt && <div className="modal-form"><h3>Total reçu : {formatMoney(selectedReceipt.amount)}</h3><p>{formatDate(selectedReceipt.paidAt)} · {accountDisplayLabel(selectedReceipt.accountSnapshot)}</p><p className="debt-reference">Reçu {selectedReceipt.id} · Enregistré le {formatDate(selectedReceipt.createdAt)}{selectedReceipt.legacy && " · Remboursement unitaire historique"}</p>{selectedReceipt.note && <p>{selectedReceipt.note}</p>}<Allocations items={selectedReceipt.allocations} /></div>}
    </Modal>
  </div>;
}
