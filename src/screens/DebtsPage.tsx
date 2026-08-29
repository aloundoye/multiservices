import { useEffect, useMemo, useState, type FormEvent } from "react";
import { AlertTriangle, CalendarDays, CheckCircle2, Eye, HandCoins, Phone, Plus, Search, UserRound, WalletCards } from "lucide-react";
import { api } from "../api";
import { Field, MoneyInput, SelectInput, TextArea, TextInput } from "../components/Fields";
import { Modal } from "../components/Modal";
import { formatDate, formatMoney, label, today } from "../lib/format";
import type { Debt } from "../types";

const initialDebt = { customerName: "", phone: "", provider: "wave", amount: 0, issuedAt: today(), dueDate: "", note: "" };

export function DebtsPage({ onChanged, notify }: { onChanged: () => void; notify: (message: string) => void }) {
  const [items, setItems] = useState<Debt[]>([]);
  const [createOpen, setCreateOpen] = useState(false);
  const [form, setForm] = useState(initialDebt);
  const [selected, setSelected] = useState<Debt>();
  const [paymentOpen, setPaymentOpen] = useState(false);
  const [payment, setPayment] = useState({ amount: 0, account: "cash", paidAt: today(), note: "" });
  const [cancelOpen, setCancelOpen] = useState(false);
  const [cancelReason, setCancelReason] = useState("");
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("active");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function load(selectId?: string) {
    const debts = await api.debts(); setItems(debts);
    if (selectId) setSelected(debts.find((debt) => debt.id === selectId));
  }
  useEffect(() => { void load(); }, []);

  const visible = useMemo(() => items.filter((debt) => {
    const statusMatch = filter === "all" || (filter === "active" ? ["open", "partial", "overdue"].includes(debt.status) : debt.status === filter);
    return statusMatch && `${debt.customerName} ${debt.phone}`.toLowerCase().includes(query.toLowerCase());
  }), [items, query, filter]);
  const totalOpen = items.filter((debt) => ["open", "partial", "overdue"].includes(debt.status)).reduce((sum, debt) => sum + debt.remaining, 0);
  const overdue = items.filter((debt) => debt.status === "overdue");

  async function create(event: FormEvent) {
    event.preventDefault(); setLoading(true); setError("");
    try {
      await api.createDebt({ ...form, dueDate: form.dueDate || null, note: form.note || null });
      setCreateOpen(false); setForm(initialDebt); await load(); onChanged(); notify("Dette client enregistrée dans les créances.");
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setLoading(false); }
  }

  async function pay(event: FormEvent) {
    event.preventDefault(); if (!selected) return; setLoading(true); setError("");
    try {
      await api.payDebt({ debtId: selected.id, ...payment, note: payment.note || null });
      setPaymentOpen(false); setPayment({ amount: 0, account: "cash", paidAt: today(), note: "" }); await load(selected.id); onChanged(); notify("Remboursement enregistré.");
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setLoading(false); }
  }

  async function cancel(event: FormEvent) {
    event.preventDefault(); if (!selected) return; setLoading(true); setError("");
    try {
      await api.cancelDebt(selected.id, cancelReason); setCancelOpen(false); setCancelReason(""); await load(selected.id); onChanged(); notify("Dette annulée avec trace d’audit.");
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setLoading(false); }
  }

  return (
    <div className="page">
      <header className="page-header"><div><p className="eyebrow">CRÉANCES CLIENTS</p><h1>Dettes clients</h1><p>Les montants encore dus sont inclus dans le capital réel lors de l’inventaire.</p></div><button className="button primary" onClick={() => { setError(""); setCreateOpen(true); }}><Plus /> Noter un transfert à crédit</button></header>
      <section className="metric-grid debt-metrics"><article className="metric-card hero-metric"><div className="metric-icon amber"><HandCoins /></div><span>Total restant dû</span><strong>{formatMoney(totalOpen)}</strong><small>{items.filter((debt) => ["open", "partial", "overdue"].includes(debt.status)).length} créance(s) active(s)</small></article><article className="metric-card"><div className="metric-icon coral"><AlertTriangle /></div><span>En retard</span><strong>{overdue.length}</strong><small>{formatMoney(overdue.reduce((sum, debt) => sum + debt.remaining, 0))} à relancer</small></article><article className="metric-card"><div className="metric-icon green"><CheckCircle2 /></div><span>Dettes soldées</span><strong>{items.filter((debt) => debt.status === "paid").length}</strong><small>Historique conservé</small></article></section>
      <section className="panel table-panel">
        <div className="table-toolbar"><div className="search-wrap"><Search /><input className="input search" placeholder="Nom ou téléphone du client…" value={query} onChange={(e) => setQuery(e.target.value)} /></div><select className="input compact" value={filter} onChange={(e) => setFilter(e.target.value)}><option value="active">Créances actives</option><option value="overdue">En retard</option><option value="paid">Payées</option><option value="cancelled">Annulées</option><option value="all">Toutes</option></select></div>
        {visible.length === 0 ? <div className="empty-state"><HandCoins /><h3>Aucune dette dans cette vue</h3><p>Les transferts à crédit apparaîtront ici.</p></div> : <div className="table-scroll"><table><thead><tr><th>Client</th><th>Service</th><th>Date / échéance</th><th>Montant initial</th><th>Reste dû</th><th>Statut</th><th></th></tr></thead><tbody>{visible.map((debt) => <tr key={debt.id}><td><strong>{debt.customerName}</strong><small><Phone size={12} /> {debt.phone}</small></td><td><span className={`provider-badge ${debt.provider}`}>{label(debt.provider)}</span></td><td>{formatDate(debt.issuedAt)}<small>{debt.dueDate ? `Échéance ${formatDate(debt.dueDate)}` : "Sans échéance"}</small></td><td>{formatMoney(debt.principal)}</td><td><strong>{formatMoney(debt.remaining)}</strong><small>{debt.principal > 0 ? `${Math.round(((debt.principal - debt.remaining) / debt.principal) * 100)} % remboursé` : ""}</small></td><td><span className={`status-chip ${debt.status}`}>{label(debt.status)}</span></td><td><button className="icon-button" onClick={() => setSelected(debt)}><Eye size={18} /></button></td></tr>)}</tbody></table></div>}
      </section>

      <Modal title="Nouveau transfert à crédit" subtitle="La somme restera comptée comme une créance dans le capital." open={createOpen} onClose={() => setCreateOpen(false)}>
        <form className="modal-form" onSubmit={create}><div className="form-grid"><Field label="Nom complet du client"><TextInput autoFocus value={form.customerName} onChange={(e) => setForm({ ...form, customerName: e.target.value })} required /></Field><Field label="Téléphone"><TextInput type="tel" value={form.phone} onChange={(e) => setForm({ ...form, phone: e.target.value })} placeholder="77 123 45 67" required /></Field></div><div className="form-grid"><Field label="Service utilisé"><SelectInput value={form.provider} onChange={(e) => setForm({ ...form, provider: e.target.value })}><option value="wave">Wave</option><option value="orange_money">Orange Money</option></SelectInput></Field><Field label="Montant transféré"><MoneyInput value={form.amount} onChange={(e) => setForm({ ...form, amount: Number(e.target.value) })} required /></Field></div><div className="form-grid"><Field label="Date du transfert"><TextInput type="date" value={form.issuedAt} onChange={(e) => setForm({ ...form, issuedAt: e.target.value })} required /></Field><Field label="Échéance (facultatif)"><TextInput type="date" min={form.issuedAt} value={form.dueDate} onChange={(e) => setForm({ ...form, dueDate: e.target.value })} /></Field></div><Field label="Note (facultatif)"><TextArea value={form.note} onChange={(e) => setForm({ ...form, note: e.target.value })} placeholder="Motif, accord avec le client…" /></Field><div className="info-callout"><WalletCards /><span>Le transfert ne diminue pas le capital total : {formatMoney(form.amount)} passeront du portefeuille à la liste des créances.</span></div>{error && <div className="form-error">{error}</div>}<div className="modal-actions"><button type="button" className="button secondary" onClick={() => setCreateOpen(false)}>Annuler</button><button className="button primary" disabled={loading}>Enregistrer la dette</button></div></form>
      </Modal>

      <Modal title={selected?.customerName ?? "Détail de la dette"} subtitle={selected ? `${label(selected.provider)} • ${selected.phone}` : ""} open={Boolean(selected)} onClose={() => setSelected(undefined)} wide>
        {selected && <div className="debt-detail"><div className="debt-profile"><div className="client-avatar"><UserRound /></div><div><span className={`status-chip ${selected.status}`}>{label(selected.status)}</span><h3>{formatMoney(selected.remaining)} restant</h3><p>sur {formatMoney(selected.principal)} transférés le {formatDate(selected.issuedAt)}</p></div><div className="debt-detail-actions">{["open", "partial", "overdue"].includes(selected.status) && <button className="button primary" onClick={() => { setError(""); setPaymentOpen(true); }}>Enregistrer un paiement</button>}{selected.status !== "cancelled" && <button className="button danger-ghost" onClick={() => { setError(""); setCancelOpen(true); }}>Annuler par correction</button>}</div></div><div className="debt-facts"><div><CalendarDays /><span>Échéance<strong>{selected.dueDate ? formatDate(selected.dueDate) : "Non définie"}</strong></span></div><div><Phone /><span>Téléphone<strong>{selected.phone}</strong></span></div><div><WalletCards /><span>Service<strong>{label(selected.provider)}</strong></span></div></div>{selected.note && <div className="debt-note"><strong>Note</strong><p>{selected.note}</p></div>}<h3 className="subsection-title">Historique des remboursements</h3>{selected.payments.length === 0 ? <div className="empty-inline">Aucun remboursement enregistré.</div> : <div className="payment-list">{selected.payments.map((item) => <div key={item.id}><span className="payment-check"><CheckCircle2 /></span><div><strong>{formatMoney(item.amount)}</strong><small>{formatDate(item.paidAt)} • reçu en {label(item.account)}</small>{item.note && <p>{item.note}</p>}</div></div>)}</div>}</div>}
      </Modal>

      <Modal title="Enregistrer un remboursement" subtitle={selected ? `${selected.customerName} • reste ${formatMoney(selected.remaining)}` : ""} open={paymentOpen} onClose={() => setPaymentOpen(false)}>
        <form className="modal-form" onSubmit={pay}><Field label="Montant reçu"><MoneyInput autoFocus max={selected?.remaining} value={payment.amount} onChange={(e) => setPayment({ ...payment, amount: Number(e.target.value) })} required /></Field><div className="form-grid"><Field label="Reçu sur"><SelectInput value={payment.account} onChange={(e) => setPayment({ ...payment, account: e.target.value })}><option value="cash">Espèces</option><option value="orange_money">Orange Money</option><option value="wave">Wave</option><option value="djamo">Djamo</option></SelectInput></Field><Field label="Date"><TextInput type="date" value={payment.paidAt} onChange={(e) => setPayment({ ...payment, paidAt: e.target.value })} required /></Field></div><Field label="Note"><TextArea value={payment.note} onChange={(e) => setPayment({ ...payment, note: e.target.value })} /></Field>{error && <div className="form-error">{error}</div>}<div className="modal-actions"><button type="button" className="button secondary" onClick={() => setPaymentOpen(false)}>Annuler</button><button className="button primary" disabled={loading}>Valider le paiement</button></div></form>
      </Modal>

      <Modal title="Annuler cette dette par correction" subtitle="Cette action mettra le reste dû à zéro sans supprimer l’historique." open={cancelOpen} onClose={() => setCancelOpen(false)}>
        <form className="modal-form" onSubmit={cancel}><Field label="Motif obligatoire"><TextArea autoFocus value={cancelReason} onChange={(e) => setCancelReason(e.target.value)} placeholder="Doublon, erreur de saisie…" required /></Field>{error && <div className="form-error">{error}</div>}<div className="modal-actions"><button type="button" className="button secondary" onClick={() => setCancelOpen(false)}>Retour</button><button className="button danger" disabled={loading}>Confirmer l’annulation</button></div></form>
      </Modal>
    </div>
  );
}
