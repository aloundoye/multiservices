import { useEffect, useMemo, useState, type FormEvent } from "react";
import { ArrowDownRight, ArrowUpRight, BookOpenText, Plus, RotateCcw, Search } from "lucide-react";
import { api } from "../api";
import { Field, MoneyInput, SelectInput, TextArea, TextInput } from "../components/Fields";
import { Modal } from "../components/Modal";
import { formatDate, formatMoney, label, signed, today } from "../lib/format";
import type { JournalEntry } from "../types";

const initialForm = {
  entryType: "sale",
  amount: 0,
  paymentAccount: "cash",
  occurredAt: today(),
  reference: "",
  note: ""
};

export function JournalPage({ onChanged, notify }: { onChanged: () => void; notify: (message: string) => void }) {
  const [items, setItems] = useState<JournalEntry[]>([]);
  const [open, setOpen] = useState(false);
  const [form, setForm] = useState(initialForm);
  const [query, setQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState("all");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [reverseTarget, setReverseTarget] = useState<JournalEntry>();
  const [reason, setReason] = useState("");

  async function load() { setItems(await api.journal()); }
  useEffect(() => { void load(); }, []);

  const visible = useMemo(() => items.filter((item) => {
    const typeMatch = typeFilter === "all" || (typeFilter === "positive" ? item.signedAmount > 0 : item.signedAmount < 0);
    const text = `${label(item.entryType)} ${item.reference ?? ""} ${item.note ?? ""}`.toLowerCase();
    return typeMatch && text.includes(query.toLowerCase());
  }), [items, query, typeFilter]);
  const totals = useMemo(() => ({
    positive: items.filter((item) => item.signedAmount > 0).reduce((sum, item) => sum + item.signedAmount, 0),
    negative: items.filter((item) => item.signedAmount < 0).reduce((sum, item) => sum + item.signedAmount, 0)
  }), [items]);

  async function submit(event: FormEvent) {
    event.preventDefault(); setLoading(true); setError("");
    try {
      await api.createJournal(form);
      setOpen(false); setForm(initialForm); await load(); onChanged(); notify("Écriture ajoutée au journal.");
    } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setLoading(false); }
  }

  async function reverse(event: FormEvent) {
    event.preventDefault(); if (!reverseTarget) return; setLoading(true); setError("");
    try {
      await api.reverseJournal(reverseTarget.id, reason);
      setReverseTarget(undefined); setReason(""); await load(); onChanged(); notify("Contre-écriture créée. L’original reste dans l’historique.");
    } catch (error) { setError(error instanceof Error ? error.message : String(error)); }
    finally { setLoading(false); }
  }

  return (
    <div className="page">
      <header className="page-header"><div><p className="eyebrow">MOUVEMENTS DE CAPITAL</p><h1>Journal de boutique</h1><p>Recettes, achats, dépenses et mouvements de capital hors opérations clients.</p></div><button className="button primary" onClick={() => { setError(""); setOpen(true); }}><Plus /> Nouvelle écriture</button></header>
      <section className="summary-strip"><div><span className="summary-icon positive"><ArrowUpRight /></span><p>Entrées enregistrées<strong>{formatMoney(totals.positive)}</strong></p></div><div><span className="summary-icon negative"><ArrowDownRight /></span><p>Sorties enregistrées<strong>{formatMoney(totals.negative)}</strong></p></div><div><span className="summary-icon neutral"><BookOpenText /></span><p>Impact net<strong className={totals.positive + totals.negative >= 0 ? "positive" : "negative"}>{signed(totals.positive + totals.negative)}</strong></p></div></section>
      <section className="panel table-panel">
        <div className="table-toolbar"><div className="search-wrap"><Search /><input className="input search" placeholder="Rechercher une note ou référence…" value={query} onChange={(e) => setQuery(e.target.value)} /></div><select className="input compact" value={typeFilter} onChange={(e) => setTypeFilter(e.target.value)}><option value="all">Toutes les écritures</option><option value="positive">Entrées seulement</option><option value="negative">Sorties seulement</option></select></div>
        {visible.length === 0 ? <div className="empty-state"><BookOpenText /><h3>Aucune écriture</h3><p>Ajoutez une recette, un achat ou une dépense.</p></div> : <div className="table-scroll"><table><thead><tr><th>Date</th><th>Type</th><th>Compte</th><th>Référence / note</th><th>Montant</th><th></th></tr></thead><tbody>{visible.map((item) => <tr className={item.reversed || item.reversesId ? "muted-row" : ""} key={item.id}><td>{formatDate(item.occurredAt)}</td><td><span className={`entry-type ${item.signedAmount >= 0 ? "income" : "outcome"}`}>{label(item.entryType)}</span>{item.reversed && <small>Corrigée</small>}</td><td>{label(item.paymentAccount)}</td><td><strong>{item.reference || "—"}</strong><small>{item.note || "Aucune note"}</small></td><td><strong className={item.signedAmount >= 0 ? "positive" : "negative"}>{signed(item.signedAmount)}</strong></td><td>{!item.reversed && !item.reversesId && <button className="icon-button" title="Créer une contre-écriture" onClick={() => { setError(""); setReverseTarget(item); }}><RotateCcw size={17} /></button>}</td></tr>)}</tbody></table></div>}
      </section>

      <Modal title="Nouvelle écriture" subtitle="Le capital attendu sera ajusté automatiquement." open={open} onClose={() => setOpen(false)}>
        <form className="modal-form" onSubmit={submit}>
          <Field label="Type d’écriture"><SelectInput value={form.entryType} onChange={(e) => setForm({ ...form, entryType: e.target.value })}><optgroup label="Augmente le capital"><option value="sale">Recette boutique</option><option value="commission">Commission mobile</option><option value="capital_contribution">Apport de capital</option></optgroup><optgroup label="Diminue le capital"><option value="purchase">Achat</option><option value="expense">Dépense</option><option value="capital_withdrawal">Retrait de capital</option></optgroup></SelectInput></Field>
          <div className="form-grid"><Field label="Montant"><MoneyInput autoFocus value={form.amount} onChange={(e) => setForm({ ...form, amount: Number(e.target.value) })} required /></Field><Field label="Compte utilisé"><SelectInput value={form.paymentAccount} onChange={(e) => setForm({ ...form, paymentAccount: e.target.value })}><option value="cash">Espèces</option><option value="orange_money">Orange Money</option><option value="wave">Wave</option><option value="djamo">Djamo</option></SelectInput></Field></div>
          <div className="form-grid"><Field label="Date"><TextInput type="date" value={form.occurredAt} onChange={(e) => setForm({ ...form, occurredAt: e.target.value })} required /></Field><Field label="Référence (facultatif)"><TextInput value={form.reference} onChange={(e) => setForm({ ...form, reference: e.target.value })} placeholder="Facture, reçu…" /></Field></div>
          <Field label="Note (facultatif)"><TextArea value={form.note} onChange={(e) => setForm({ ...form, note: e.target.value })} placeholder="Détails utiles…" /></Field>
          <div className={`effect-preview ${["sale", "commission", "capital_contribution"].includes(form.entryType) ? "positive" : "negative"}`}><span>Impact sur le capital attendu</span><strong>{["sale", "commission", "capital_contribution"].includes(form.entryType) ? "+" : "-"}{formatMoney(form.amount)}</strong></div>
          {error && <div className="form-error">{error}</div>}
          <div className="modal-actions"><button type="button" className="button secondary" onClick={() => setOpen(false)}>Annuler</button><button className="button primary" disabled={loading}>{loading ? "Enregistrement…" : "Enregistrer"}</button></div>
        </form>
      </Modal>

      <Modal title="Corriger cette écriture" subtitle="Une contre-écriture sera ajoutée; l’original restera visible." open={Boolean(reverseTarget)} onClose={() => setReverseTarget(undefined)}>
        <form className="modal-form" onSubmit={reverse}>{reverseTarget && <div className="selected-entry"><span>{label(reverseTarget.entryType)}</span><strong>{signed(reverseTarget.signedAmount)}</strong></div>}<Field label="Motif de la correction"><TextArea autoFocus value={reason} onChange={(e) => setReason(e.target.value)} required placeholder="Expliquez pourquoi cette écriture doit être annulée…" /></Field>{error && <div className="form-error">{error}</div>}<div className="modal-actions"><button type="button" className="button secondary" onClick={() => setReverseTarget(undefined)}>Retour</button><button className="button danger" disabled={loading}><RotateCcw /> Créer la contre-écriture</button></div></form>
      </Modal>
    </div>
  );
}
