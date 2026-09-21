import { useEffect, useRef, useState, type FormEvent } from "react";
import { Archive, ArrowDownLeft, ArrowUpRight, Eye, History, Pencil, Plus, RotateCcw, Search, ShieldCheck, Users, X } from "lucide-react";
import { api } from "../api";
import { AccountSelect, accountDisplayLabel } from "../components/AccountSelect";
import { Field, MoneyInput, SelectInput, TextArea, TextInput } from "../components/Fields";
import { Modal } from "../components/Modal";
import { formatDate, formatMoney, signed, today } from "../lib/format";
import type { CustodyCustomer, CustodyKind, CustodyMovement, CustodyOpeningPreview } from "../types";

export const custodyLabels: Record<CustodyKind, string> = { deposit: "Dépôt reçu", withdrawal: "Restitution", opening: "Reprise antérieure", reversal: "Annulation" };
type Editor = { kind: "client"; client?: CustodyCustomer } | { kind: "deposit" | "withdrawal" | "opening" } | { kind: "reverse"; movement: CustodyMovement };
const errorText = (e: unknown) => e instanceof Error ? e.message : String(e);
const blankLine = () => ({ customerId: "", amount: 0 });
const blankForm = () => ({ requestId: crypto.randomUUID(), customerId: "", name: "", phone: "", amount: 0, accountId: "", occurredAt: today(), note: "", reason: "", lines: [blankLine()] });

export function CustodyPage({ onChanged, notify, initialAction }: { onChanged: () => void | Promise<void>; notify: (message: string) => void; initialAction?: "deposit" | "withdrawal" }) {
  const [clients, setClients] = useState<CustodyCustomer[]>([]);
  const [movements, setMovements] = useState<CustodyMovement[]>([]);
  const [tab, setTab] = useState<"clients" | "history">("clients");
  const [query, setQuery] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [historyClient, setHistoryClient] = useState("");
  const [editor, setEditor] = useState<Editor | undefined>(initialAction ? { kind: initialAction } : undefined);
  const [detail, setDetail] = useState<CustodyMovement>();
  const [form, setForm] = useState(blankForm);
  const [preview, setPreview] = useState<CustodyOpeningPreview>();
  const [error, setError] = useState("");
  const [formError, setFormError] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const inFlight = useRef(false);

  async function load() {
    const [nextClients, nextMovements] = await Promise.all([api.custodyCustomers(), api.custodyMovements()]);
    setClients(nextClients); setMovements(nextMovements);
  }
  useEffect(() => { void load().catch((e) => setError(errorText(e))).finally(() => setLoading(false)); }, []);
  useEffect(() => {
    setPreview(undefined);
    if (editor?.kind !== "opening" || !form.lines.every((l) => l.customerId && Number.isSafeInteger(l.amount) && l.amount > 0)) return;
    let current = true;
    const timer = window.setTimeout(() => {
      api.previewCustodyOpening({ requestId: form.requestId, lines: form.lines }).then((result) => { if (current) { setPreview(result); setFormError(""); } }).catch((e) => { if (current) setFormError(errorText(e)); });
    }, 200);
    return () => { current = false; window.clearTimeout(timer); };
  }, [editor?.kind, form.lines, form.requestId]);

  function open(next: Editor, customerId = "") {
    const initial = blankForm(); initial.customerId = customerId;
    if (next.kind === "client" && next.client) { initial.name = next.client.name; initial.phone = next.client.phone ?? ""; }
    setForm(initial); setFormError(""); setPreview(undefined); setDetail(undefined); setEditor(next);
  }
  function close() { if (!inFlight.current) setEditor(undefined); }
  async function save(action: () => Promise<unknown>, message: string) {
    if (inFlight.current) return;
    inFlight.current = true; setBusy(true); setFormError(""); setError("");
    try {
      await action(); setEditor(undefined); setDetail(undefined); notify(message);
      try { await load(); await onChanged(); } catch (e) { setError(`Enregistrement effectué. Actualisation impossible : ${errorText(e)}`); }
    } catch (e) { if (editor) setFormError(errorText(e)); else setError(errorText(e)); }
    finally { inFlight.current = false; setBusy(false); }
  }
  function submit(e: FormEvent) {
    e.preventDefault(); if (!editor) return;
    const requestId = form.requestId;
    if (editor.kind === "client") return void save(() => api.saveCustodyCustomer({ requestId, customerId: editor.client?.id ?? null, name: form.name, phone: form.phone || null, active: editor.client?.active ?? true }), "Fiche client enregistrée.");
    if (editor.kind === "reverse") return void save(() => api.reverseCustodyMovement({ requestId, movementId: editor.movement.id, reason: form.reason }), "Contre-mouvement enregistré.");
    if (editor.kind === "opening") {
      if (!preview) return;
      return void save(() => api.recordCustodyOpening({ requestId, lines: form.lines }), "Reprise enregistrée. Le capital de la boutique a été corrigé.");
    }
    const kind = editor.kind;
    return void save(() => api.recordCustodyMovement({ requestId, customerId: form.customerId, kind, amount: form.amount, accountId: form.accountId, occurredAt: form.occurredAt, note: form.note || null }), kind === "deposit" ? "Dépôt reçu. Le capital attendu reste inchangé." : "Restitution enregistrée. Le capital attendu reste inchangé.");
  }
  const active = clients.filter((c) => c.active);
  const selected = clients.find((c) => c.id === form.customerId);
  const filtered = clients.filter((c) => (showArchived || c.active) && `${c.name} ${c.phone ?? ""} ${c.id}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  const availableOpening = active.filter((c) => !movements.some((m) => m.customerId === c.id && m.kind === "opening" && !m.reversed));
  const title = editor?.kind === "client" ? editor.client ? "Modifier le client" : "Ajouter un client" : editor?.kind === "deposit" ? "Recevoir un dépôt" : editor?.kind === "withdrawal" ? "Restituer au client" : editor?.kind === "opening" ? "Argent déjà gardé avant la mise à jour" : "Annuler le mouvement";
  const paymentValid = selected && Number.isSafeInteger(form.amount) && form.amount > 0 && Number.isSafeInteger(selected.balance + (editor?.kind === "withdrawal" ? -form.amount : form.amount)) && (editor?.kind !== "withdrawal" || form.amount <= selected.balance);

  return <div className="page custody-page">
    <header className="page-header"><div><p className="eyebrow">ARGENT CONFIÉ À LA BOUTIQUE</p><h1>Dépôts clients</h1><p>Les sommes à restituer aux clients restent séparées de votre capital.</p></div><button className="button secondary" disabled={busy || loading} onClick={() => open({ kind: "client" })}><Plus /> Ajouter un client</button></header>
    <section className="summary-strip">
      <div><span className="summary-icon neutral"><ShieldCheck /></span><p>Argent gardé pour les clients<strong>{formatMoney(clients.reduce((s, c) => s + c.balance, 0))}</strong></p></div>
      <div><span className="summary-icon neutral"><Users /></span><p>Clients ayant un solde<strong>{clients.filter((c) => c.balance > 0).length}</strong></p></div>
      <div className="custody-actions"><button className="button primary" disabled={busy || loading || !active.length} onClick={() => open({ kind: "deposit" })}><ArrowDownLeft /> Recevoir un dépôt</button><button className="button secondary" disabled={busy || !active.some((c) => c.balance > 0)} onClick={() => open({ kind: "withdrawal" })}><ArrowUpRight /> Restituer</button></div>
    </section>
    <aside className="custody-opening-banner"><ShieldCheck /><div><strong>Vous gardiez déjà de l’argent avant la mise à jour ?</strong><p>Répertoriez les montants restants pour corriger le capital, sans enregistrer de nouvel encaissement.</p></div><button className="button secondary" disabled={busy || loading || !availableOpening.length} onClick={() => open({ kind: "opening" })}>Reprendre les montants</button></aside>
    <div className="settings-tabs" role="tablist" aria-label="Dépôts clients"><button role="tab" aria-selected={tab === "clients"} className={tab === "clients" ? "active" : ""} onClick={() => setTab("clients")}><Users /> Clients</button><button role="tab" aria-selected={tab === "history"} className={tab === "history" ? "active" : ""} onClick={() => setTab("history")}><History /> Registre des mouvements</button></div>
    {error && <div className="form-error" role="alert">{error}<button className="text-button" onClick={() => { void load().then(onChanged).then(() => setError("")).catch((e) => setError(errorText(e))); }}>Actualiser</button></div>}
    {loading ? <div className="empty-state" role="status">Chargement des dépôts…</div> : tab === "clients" ? <section className="panel table-panel">
      <div className="table-toolbar"><div className="search-wrap"><Search /><TextInput aria-label="Rechercher un client" placeholder="Nom, téléphone ou identifiant…" value={query} onChange={(e) => setQuery(e.target.value)} /></div><label className="product-checkbox"><input type="checkbox" checked={showArchived} onChange={(e) => setShowArchived(e.target.checked)} /> Afficher les archivés</label></div>
      {!filtered.length ? <div className="empty-state"><Users /><h3>{clients.length ? "Aucun client correspondant" : "L’argent confié, suivi client par client"}</h3><p>Ajoutez un client, puis recevez un dépôt ou reprenez un montant déjà gardé.</p><button className="button secondary" onClick={() => open({ kind: "client" })}>Ajouter un client</button></div> : <div className="table-scroll"><table><thead><tr><th>Client</th><th>Entrées cumulées</th><th>Restitutions</th><th>Solde gardé</th><th>Actions</th></tr></thead><tbody>{filtered.map((c) => <tr key={c.id} className={c.active ? "" : "muted-row"}>
        <td><strong>{c.name}</strong><small>{c.phone || "Téléphone non renseigné"}{!c.active && " · Archivé"}</small><small className="custody-id" title={c.id}>ID : {c.id}</small></td><td>{formatMoney(c.totalReceived)}</td><td>{formatMoney(c.totalWithdrawn)}</td><td><strong>{formatMoney(c.balance)}</strong></td>
        <td><div className="product-row-actions"><button className="icon-button" aria-label={`Modifier ${c.name}`} disabled={busy} onClick={() => open({ kind: "client", client: c })}><Pencil size={17} /></button>{c.active && <><button className="icon-button" aria-label={`Recevoir pour ${c.name}`} disabled={busy} onClick={() => open({ kind: "deposit" }, c.id)}><ArrowDownLeft size={17} /></button><button className="icon-button" aria-label={`Restituer à ${c.name}`} disabled={busy || c.balance === 0} onClick={() => open({ kind: "withdrawal" }, c.id)}><ArrowUpRight size={17} /></button></>}
          <button className="icon-button" aria-label={`${c.active ? "Archiver" : "Réactiver"} ${c.name}`} title={c.balance ? "Solde nul requis pour archiver" : undefined} disabled={busy || c.balance > 0} onClick={() => void save(() => api.saveCustodyCustomer({ requestId: crypto.randomUUID(), customerId: c.id, name: c.name, phone: c.phone, active: !c.active }), c.active ? "Client archivé." : "Client réactivé.")}>{c.active ? <Archive size={17} /> : <RotateCcw size={17} />}</button>
          <button className="icon-button" aria-label={`Historique de ${c.name}`} onClick={() => { setHistoryClient(c.id); setTab("history"); }}><History size={17} /></button></div></td>
      </tr>)}</tbody></table></div>}
      <p className="custody-table-note">Entrées : dépôts et reprises, hors opérations annulées. Les restitutions annulées sont également exclues des cumuls.</p>
    </section> : <section className="panel table-panel">
      <div className="table-toolbar"><Field label="Client"><SelectInput value={historyClient} onChange={(e) => setHistoryClient(e.target.value)}><option value="">Tous les clients</option>{clients.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</SelectInput></Field><p className="stock-hint">Ordre d’enregistrement conservé, même pour une date antidatée.</p></div>
      <div className="table-scroll"><table><thead><tr><th>Date déclarée</th><th>Client</th><th>Mouvement</th><th>Compte</th><th>Variation du dépôt</th><th>Solde après</th><th></th></tr></thead><tbody>{movements.filter((m) => !historyClient || m.customerId === historyClient).map((m) => <tr key={m.id} className={m.reversed ? "muted-row" : ""}><td>{formatDate(m.occurredAt)}<small>N° {m.sequence} · {m.operator}</small></td><td>{m.customerName}<small>{m.customerPhone}</small></td><td>{custodyLabels[m.kind]}{m.reversed && <small>Annulé</small>}</td><td>{m.accountSnapshot ? accountDisplayLabel(m.accountSnapshot) : "Sans mouvement d’argent"}</td><td>{signed(m.delta)}</td><td>{formatMoney(m.balanceAfter)}</td><td><button className="icon-button" aria-label={`Détail du mouvement ${m.sequence}`} onClick={() => setDetail(m)}><Eye size={18} /></button></td></tr>)}</tbody></table></div>
      {!movements.some((m) => !historyClient || m.customerId === historyClient) && <div className="empty-state">Aucun mouvement enregistré.</div>}
    </section>}

    <Modal title={title} open={Boolean(editor)} onClose={close} wide={editor?.kind === "opening"}>
      <form className="modal-form" onSubmit={submit}><fieldset className="product-fieldset" disabled={busy}>
        {editor?.kind === "client" && <><Field label="Nom du client"><TextInput autoFocus required value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} /></Field><Field label="Téléphone (facultatif)"><TextInput type="tel" value={form.phone} onChange={(e) => setForm({ ...form, phone: e.target.value })} /></Field>{editor.client && <p className="stock-hint">Identifiant permanent : {editor.client.id}. L’historique conserve les anciens libellés.</p>}</>}
        {(editor?.kind === "deposit" || editor?.kind === "withdrawal") && <>
          <Field label="Client"><SelectInput required value={form.customerId} onChange={(e) => setForm({ ...form, customerId: e.target.value })}><option value="">Choisir un client…</option>{active.map((c) => <option key={c.id} value={c.id} disabled={editor.kind === "withdrawal" && c.balance === 0}>{c.name}{c.phone ? ` · ${c.phone}` : ""} · {formatMoney(c.balance)}</option>)}</SelectInput></Field>
          {!active.length && <p className="stock-hint">Ajoutez d’abord une fiche client depuis la page Dépôts clients.</p>}
          <Field label="Montant (FCFA)" hint={selected ? `Solde gardé : ${formatMoney(selected.balance)}` : undefined}><MoneyInput min={1} max={editor.kind === "withdrawal" ? selected?.balance ?? 0 : Number.MAX_SAFE_INTEGER} required value={form.amount || ""} onChange={(e) => setForm({ ...form, amount: Number(e.target.value) })} /></Field>
          <div className="form-grid"><Field label={editor.kind === "deposit" ? "Compte d’encaissement" : "Compte de restitution"}><AccountSelect value={form.accountId} onChange={(accountId) => setForm({ ...form, accountId })} /></Field><Field label="Date du mouvement"><TextInput type="date" required value={form.occurredAt} onChange={(e) => setForm({ ...form, occurredAt: e.target.value })} /></Field></div>
          <Field label="Note (facultatif)"><TextArea value={form.note} onChange={(e) => setForm({ ...form, note: e.target.value })} /></Field>
          {paymentValid && <div className="effect-preview"><span>Solde client après {editor.kind === "deposit" ? "dépôt" : "restitution"}</span><strong>{formatMoney(selected.balance + (editor.kind === "deposit" ? form.amount : -form.amount))}</strong></div>}
          <p className="stock-hint">Capital attendu inchangé. {editor.kind === "withdrawal" ? "Vérifiez la disponibilité sur le compte choisi : les soldes affichés dans la boutique datent du dernier inventaire." : "Service gratuit : ce dépôt n’est pas une recette."}</p>
        </>}
        {editor?.kind === "opening" && <>
          <p>Indiquez l’argent encore dû aux clients, déjà inclus dans vos anciens inventaires. Les liquidités restent inchangées ; seul le capital de la boutique est corrigé.</p>
          {form.lines.map((line, index) => <div className="custody-opening-line" key={index}><Field label={`Client ${index + 1}`}><SelectInput required value={line.customerId} onChange={(e) => setForm({ ...form, lines: form.lines.map((l, i) => i === index ? { ...l, customerId: e.target.value } : l) })}><option value="">Choisir…</option>{availableOpening.filter((c) => c.id === line.customerId || !form.lines.some((l) => l.customerId === c.id)).map((c) => <option key={c.id} value={c.id}>{c.name}{c.phone ? ` · ${c.phone}` : ""}</option>)}</SelectInput></Field><Field label={`Montant restant ${index + 1}`}><MoneyInput min={1} max={Number.MAX_SAFE_INTEGER} required value={line.amount || ""} onChange={(e) => setForm({ ...form, lines: form.lines.map((l, i) => i === index ? { ...l, amount: Number(e.target.value) } : l) })} /></Field><button type="button" className="icon-button" aria-label={`Retirer le client ${index + 1}`} disabled={form.lines.length === 1} onClick={() => setForm({ ...form, lines: form.lines.filter((_, i) => i !== index) })}><X /></button></div>)}
          <button type="button" className="text-button" disabled={form.lines.length >= availableOpening.length} onClick={() => setForm({ ...form, lines: [...form.lines, blankLine()] })}><Plus size={17} /> Ajouter un montant client</button>
          <p className="stock-hint">Les clients ayant déjà une reprise sont exclus. Pour en corriger une, annulez son mouvement dans le registre.</p>
          {preview && <div className="summary-lines custody-preview" aria-live="polite"><div><span>Capital attendu actuel</span><strong>{formatMoney(preview.expectedCapital)}</strong></div><div><span>Argent déjà gardé à reclasser</span><strong>− {formatMoney(preview.total)}</strong></div><div className="summary-separator"><span>Capital corrigé</span><strong>{formatMoney(preview.correctedCapital)}</strong></div><p>Aucun nouvel encaissement ni dépense. Tous les montants seront enregistrés ensemble.</p></div>}
        </>}
        {editor?.kind === "reverse" && <><div className="selected-entry"><span>{custodyLabels[editor.movement.kind]} · {editor.movement.customerName}</span><strong>{signed(editor.movement.delta)}</strong></div><p>Un contre-mouvement conservera la trace de l’original. {editor.movement.capitalAdjustment !== 0 ? "Le reclassement du capital sera inversé." : "Le capital attendu restera inchangé."}</p><Field label="Motif obligatoire"><TextArea required minLength={3} value={form.reason} onChange={(e) => setForm({ ...form, reason: e.target.value })} /></Field></>}
        {formError && <div className="form-error" role="alert">{formError}</div>}
        <div className="modal-actions"><button type="button" className="button secondary" onClick={close}>Fermer</button><button className="button primary" disabled={busy || (editor?.kind === "opening" && !preview) || ((editor?.kind === "deposit" || editor?.kind === "withdrawal") && !paymentValid)}>{busy ? "Enregistrement…" : editor?.kind === "opening" ? "Valider la reprise" : editor?.kind === "reverse" ? "Confirmer l’annulation" : "Enregistrer"}</button></div>
      </fieldset></form>
    </Modal>
    <Modal title="Détail du mouvement" open={Boolean(detail)} onClose={() => setDetail(undefined)}>
      {detail && <div className="modal-form"><h3>{custodyLabels[detail.kind]} · {detail.customerName}</h3><div className="summary-lines"><div><span>Variation du dépôt</span><strong>{signed(detail.delta)}</strong></div><div><span>Solde après enregistrement</span><strong>{formatMoney(detail.balanceAfter)}</strong></div><div><span>Ajustement du capital</span><strong>{signed(detail.capitalAdjustment)}</strong></div></div><p>{detail.accountSnapshot ? accountDisplayLabel(detail.accountSnapshot) : "Sans mouvement physique d’argent"}</p><p>Date déclarée : {formatDate(detail.occurredAt)}<br />Enregistré le {formatDate(detail.postedAt, true)} · N° {detail.sequence} · {detail.operator}</p><p>{detail.note}</p><small>Client : {detail.customerId}<br />Mouvement : {detail.id}</small>{detail.reversesId && <button className="text-button" onClick={() => setDetail(movements.find((m) => m.id === detail.reversesId))}>Voir le mouvement original</button>}{detail.reversed && <button className="text-button" onClick={() => setDetail(movements.find((m) => m.reversesId === detail.id))}>Voir l’annulation</button>}{!detail.reversed && detail.kind !== "reversal" && <button className="button secondary" disabled={busy} onClick={() => open({ kind: "reverse", movement: detail })}><RotateCcw /> Annuler ce mouvement</button>}</div>}
    </Modal>
  </div>;
}
