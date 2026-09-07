import { useEffect, useRef, useState, type FormEvent } from "react";
import { Archive, ClipboardCheck, History, Package, Pencil, Plus, RotateCcw, Search, ShoppingCart, Truck, X } from "lucide-react";
import { api } from "../api";
import { AccountSelect, accountDisplayLabel } from "../components/AccountSelect";
import { Field, MoneyInput, SelectInput, TextArea, TextInput } from "../components/Fields";
import { Modal } from "../components/Modal";
import { formatDate, formatMoney, signed, today } from "../lib/format";
import type { Product, ProductOperation, SaleLineInput, StockMovement } from "../types";

type Tab = "catalogue" | "history" | "movements";
type Editor = { kind: "create" | "sale" } | { kind: "edit" | "receipt" | "adjust"; product: Product } | { kind: "cancel"; operation: ProductOperation };
const movementLabels = { initial: "Stock initial", sale: "Vente", receipt: "Réapprovisionnement", adjustment: "Comptage", cancellation: "Annulation" };
const blankLine = (): SaleLineInput => ({ productId: "", quantity: 1, unitPrice: 0 });
function blankForm() {
  return { requestId: crypto.randomUUID(), name: "", price: 0, initialStock: 0, quantity: 1, amount: 0,
    accountId: "", occurredAt: today(), note: "", reason: "", lines: [blankLine()] };
}
const errorText = (e: unknown) => e instanceof Error ? e.message : String(e);

export function ProductsPage({ onChanged, notify }: { onChanged: () => void | Promise<void>; notify: (message: string) => void }) {
  const [products, setProducts] = useState<Product[]>([]);
  const [operations, setOperations] = useState<ProductOperation[]>([]);
  const [movements, setMovements] = useState<StockMovement[]>([]);
  const [tab, setTab] = useState<Tab>("catalogue");
  const [query, setQuery] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  const [movementProduct, setMovementProduct] = useState("");
  const [editor, setEditor] = useState<Editor>();
  const [detail, setDetail] = useState<ProductOperation>();
  const [form, setForm] = useState(blankForm);
  const [error, setError] = useState("");
  const [formError, setFormError] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const inFlight = useRef(false);

  async function load() {
    const [nextProducts, nextOperations, nextMovements] = await Promise.all([api.products(), api.productOperations(), api.stockMovements()]);
    setProducts(nextProducts); setOperations(nextOperations); setMovements(nextMovements);
  }
  useEffect(() => { void load().catch((e) => setError(errorText(e))).finally(() => setLoading(false)); }, []);

  function open(next: Editor) {
    const initial = blankForm();
    if ("product" in next) {
      initial.name = next.product.name; initial.price = next.product.price;
      if (next.kind === "adjust") initial.quantity = next.product.stock;
    }
    setForm(initial); setFormError(""); setEditor(next); setDetail(undefined);
  }
  function close() { if (!inFlight.current) { setEditor(undefined); setFormError(""); } }
  async function save(action: () => Promise<unknown>, message: string) {
    if (inFlight.current) return;
    inFlight.current = true; setBusy(true); setFormError(""); setError("");
    try {
      await action();
      setEditor(undefined); setDetail(undefined); notify(message);
      try { await load(); await onChanged(); }
      catch (e) { setError(`Enregistrement effectué. Actualisation impossible : ${errorText(e)}`); }
    } catch (e) { if (editor) setFormError(errorText(e)); else setError(errorText(e)); }
    finally { inFlight.current = false; setBusy(false); }
  }
  async function submit(event: FormEvent) {
    event.preventDefault(); if (!editor) return;
    const requestId = form.requestId;
    const payment = { requestId, accountId: form.accountId, occurredAt: form.occurredAt, note: form.note };
    switch (editor.kind) {
      case "create": return save(() => api.createProduct({ requestId, name: form.name, price: form.price, initialStock: form.initialStock }), "Produit ajouté au catalogue.");
      case "edit": return save(() => api.updateProduct({ requestId, productId: editor.product.id, name: form.name, price: form.price }), "Produit mis à jour.");
      case "sale": return save(() => api.createSale({ ...payment, lines: form.lines }), "Vente enregistrée. Le stock et le budget sont mis à jour.");
      case "receipt": return save(() => api.receiveStock({ ...payment, productId: editor.product.id, quantity: form.quantity, amount: form.amount }), "Réapprovisionnement et achat enregistrés.");
      case "adjust": return save(() => api.adjustStock({ requestId, productId: editor.product.id, quantity: form.quantity, reason: form.reason }), "Stock corrigé après comptage.");
      case "cancel": return save(() => api.cancelProductOperation({ requestId, operationId: editor.operation.id, reason: form.reason }), "Opération annulée. Le stock et le budget sont corrigés.");
    }
  }
  function changeLine(index: number, changes: Partial<SaleLineInput>) {
    setForm((current) => ({ ...current, lines: current.lines.map((line, i) => i === index ? { ...line, ...changes } : line) }));
  }
  const active = products.filter((p) => p.active);
  const visible = products.filter((p) => (showArchived || p.active) && p.name.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  const total = form.lines.reduce((sum, line) => sum + line.quantity * line.unitPrice, 0);
  const saleValid = form.lines.length > 0 && Number.isSafeInteger(total) && total > 0 && form.lines.every((line) => {
    const p = active.find((p) => p.id === line.productId);
    return p && Number.isSafeInteger(line.quantity) && line.quantity > 0 && line.quantity <= p.stock && Number.isSafeInteger(line.unitPrice) && line.unitPrice > 0;
  });
  const title = editor?.kind === "create" ? "Ajouter un produit" : editor?.kind === "edit" ? "Modifier le produit" : editor?.kind === "sale" ? "Nouvelle vente" : editor?.kind === "receipt" ? "Recevoir du stock" : editor?.kind === "adjust" ? "Compter le stock" : "Annuler l’opération";

  return <div className="page products-page">
    <header className="page-header">
      <div><p className="eyebrow">BOUTIQUE</p><h1>Produits et ventes</h1><p>Vos articles, leurs stocks et chaque encaissement au même endroit.</p></div>
      <div className="product-actions"><button className="button secondary" disabled={loading || busy} onClick={() => open({ kind: "create" })}><Plus /> Ajouter un produit</button><button className="button primary" disabled={loading || busy || !active.some((p) => p.stock > 0)} onClick={() => open({ kind: "sale" })}><ShoppingCart /> Nouvelle vente</button></div>
    </header>
    <section className="summary-strip">
      <div><span className="summary-icon neutral"><Package /></span><p>Produits au catalogue<strong>{active.length}</strong></p></div>
      <div><span className="summary-icon negative"><Package /></span><p>Produits en rupture<strong>{active.filter((p) => p.stock === 0).length}</strong></p></div>
      <div><span className="summary-icon positive"><ShoppingCart /></span><p>Ventes du jour, hors annulations<strong>{formatMoney(operations.filter((o) => o.kind === "sale" && !o.cancelledAt && o.occurredAt === today()).reduce((sum, o) => sum + o.amount, 0))}</strong></p></div>
    </section>
    <div className="settings-tabs" role="tablist" aria-label="Produits et ventes">
      {([ ["catalogue", "Catalogue", Package], ["history", "Ventes et achats", History], ["movements", "Mouvements de stock", ClipboardCheck] ] as const).map(([value, label, Icon]) => <button key={value} role="tab" aria-selected={tab === value} className={tab === value ? "active" : ""} onClick={() => setTab(value)}><Icon />{label}</button>)}
    </div>
    {error && <div className="form-error product-error" role="alert">{error}<button className="text-button" onClick={() => { setLoading(true); void load().then(onChanged).then(() => setError("")).catch((e) => setError(errorText(e))).finally(() => setLoading(false)); }}>Actualiser</button></div>}
    {loading ? <div className="empty-state" role="status">Chargement des produits…</div> : <>
      {tab === "catalogue" && <section className="panel table-panel">
        <div className="table-toolbar"><div className="search-wrap"><Search /><input className="input search" aria-label="Rechercher un produit" placeholder="Rechercher un produit…" value={query} onChange={(e) => setQuery(e.target.value)} /></div><label className="product-checkbox"><input type="checkbox" checked={showArchived} onChange={(e) => setShowArchived(e.target.checked)} /> Afficher les archivés</label></div>
        {visible.length === 0 ? <div className="empty-state"><Package /><h3>{products.length ? "Aucun produit correspondant" : "Votre catalogue commence ici"}</h3><p>Ajoutez un article, son prix et les quantités déjà en boutique.</p>{!products.length && <button className="button secondary" onClick={() => open({ kind: "create" })}>Créer le premier produit</button>}</div> : <div className="table-scroll"><table><thead><tr><th>Produit</th><th>Prix de vente</th><th>Stock</th><th>Actions</th></tr></thead><tbody>{visible.map((p) => <tr key={p.id} className={!p.active ? "muted-row" : ""}>
          <td><strong>{p.name}</strong>{!p.active && <small>Archivé</small>}</td><td>{formatMoney(p.price)}</td><td><span className={`status-chip ${p.stock === 0 ? "overdue" : "paid"}`}>{p.stock === 0 ? "Rupture · 0" : `${p.stock} unité${p.stock > 1 ? "s" : ""}`}</span></td>
          <td><div className="product-row-actions">{p.active && <>
            <button className="icon-button" aria-label={`Modifier ${p.name}`} title="Modifier" disabled={busy} onClick={() => open({ kind: "edit", product: p })}><Pencil size={16} /></button>
            <button className="icon-button" aria-label={`Réapprovisionner ${p.name}`} title="Recevoir du stock" disabled={busy} onClick={() => open({ kind: "receipt", product: p })}><Truck size={17} /></button>
            <button className="icon-button" aria-label={`Compter ${p.name}`} title="Compter le stock" disabled={busy} onClick={() => open({ kind: "adjust", product: p })}><ClipboardCheck size={17} /></button>
            <button className="icon-button" aria-label={`Archiver ${p.name}`} title={p.stock ? "Le stock doit être nul pour archiver" : "Archiver"} disabled={busy || p.stock !== 0} onClick={() => void save(() => api.archiveProduct({ requestId: crypto.randomUUID(), productId: p.id }), "Produit archivé.")}><Archive size={17} /></button>
          </>}<button className="icon-button" aria-label={`Historique du stock de ${p.name}`} title="Historique du stock" onClick={() => { setMovementProduct(p.id); setTab("movements"); }}><History size={17} /></button></div></td>
        </tr>)}</tbody></table></div>}
      </section>}
      {tab === "history" && <section className="panel table-panel">
        {operations.length === 0 ? <div className="empty-state"><History /><h3>Aucune vente ni réapprovisionnement</h3><p>Chaque opération enregistrée apparaîtra ici.</p></div> : <div className="table-scroll"><table><thead><tr><th>Date</th><th>Opération</th><th>Articles</th><th>Compte</th><th>Budget</th><th></th></tr></thead><tbody>{operations.map((o) => <tr key={o.id} className={o.cancelledAt ? "muted-row" : ""}>
          <td>{formatDate(o.occurredAt)}</td><td><strong>{o.kind === "sale" ? "Vente" : "Réapprovisionnement"}</strong><small>{o.cancelledAt ? "Annulé" : "Enregistré"}</small></td><td>{o.lines.map((l) => `${l.quantity} × ${l.productName}`).join(", ")}</td><td>{accountDisplayLabel(o.accountSnapshot)}</td><td><strong className={o.kind === "sale" ? "positive" : "negative"}>{signed(o.kind === "sale" ? o.amount : -o.amount)}</strong>{o.cancelledAt && <small>Compensé par une annulation</small>}</td><td><button className="text-button" onClick={() => setDetail(o)}>Détails</button></td>
        </tr>)}</tbody></table></div>}
      </section>}
      {tab === "movements" && <section className="panel table-panel">
        <div className="table-toolbar"><Field label="Produit"><SelectInput value={movementProduct} onChange={(e) => setMovementProduct(e.target.value)}><option value="">Tous les produits</option>{products.map((p) => <option key={p.id} value={p.id}>{p.name}{!p.active ? " (archivé)" : ""}</option>)}</SelectInput></Field><p className="stock-hint">Le comptage des produits ajuste les quantités. Les achats s’enregistrent avec « Recevoir du stock ».</p></div>
        <div className="table-scroll"><table><thead><tr><th>Date</th><th>Produit</th><th>Mouvement</th><th>Quantité</th><th>Stock après</th><th>Motif</th></tr></thead><tbody>{movements.filter((m) => !movementProduct || m.productId === movementProduct).map((m) => <tr key={m.id}><td>{formatDate(m.createdAt, true)}<small>Opération du {formatDate(m.occurredAt)}</small></td><td>{m.productName}</td><td>{movementLabels[m.kind]}{m.operationId && <button className="text-button" onClick={() => setDetail(operations.find((o) => o.id === m.operationId))}>Voir l’opération</button>}</td><td className={m.quantity >= 0 ? "positive" : "negative"}>{m.quantity > 0 ? "+" : ""}{m.quantity}</td><td>{m.balanceAfter}</td><td>{m.reason || "—"}</td></tr>)}</tbody></table></div>
        {!movements.some((m) => !movementProduct || m.productId === movementProduct) && <div className="empty-inline">Aucun mouvement de stock.</div>}
      </section>}
    </>}
    <Modal title={title} subtitle={editor && "product" in editor ? `${editor.product.name} · ${editor.product.stock} unité(s) en stock` : undefined} open={Boolean(editor)} onClose={close} wide={editor?.kind === "sale"}>
      <form className="modal-form" onSubmit={(e) => void submit(e)}>
        <fieldset className="product-fieldset" disabled={busy}>
          {(editor?.kind === "create" || editor?.kind === "edit") && <>
            <Field label="Nom du produit"><TextInput autoFocus required value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="Chargeur, câble, cahier…" /></Field>
            <Field label="Prix de vente unitaire"><MoneyInput required min={1} max={Number.MAX_SAFE_INTEGER} value={form.price} onChange={(e) => setForm({ ...form, price: Number(e.target.value) })} /></Field>
            {editor.kind === "create" && <Field label="Quantité déjà en stock" hint="Marchandises déjà détenues : aucun achat ne sera ajouté au budget. Pour un nouvel achat, créez le produit à 0 puis recevez le stock."><TextInput type="number" min={0} step={1} max={Number.MAX_SAFE_INTEGER} required value={form.initialStock} onChange={(e) => setForm({ ...form, initialStock: Number(e.target.value) })} /></Field>}
          </>}
          {editor?.kind === "sale" && <>
            <div className="sale-lines">
              {form.lines.map((line, index) => {
                const p = active.find((p) => p.id === line.productId);
                const tooMany = p && line.quantity > p.stock;
                return <div className="sale-line" key={index}>
                  <Field label={`Produit ${index + 1}`} hint={p ? `${p.stock} unité(s) disponible(s)` : undefined}><SelectInput required value={line.productId} onChange={(e) => { const next = active.find((p) => p.id === e.target.value); changeLine(index, { productId: e.target.value, unitPrice: next?.price ?? 0 }); }}>
                    <option value="">Choisir un produit…</option>{active.filter((p) => p.id === line.productId || !form.lines.some((l) => l.productId === p.id)).map((p) => <option disabled={p.stock === 0} key={p.id} value={p.id}>{p.name}{p.stock === 0 ? " — rupture" : ""}</option>)}
                  </SelectInput></Field>
                  <Field label={`Quantité ${index + 1}`}><TextInput type="number" min={1} max={p?.stock ?? Number.MAX_SAFE_INTEGER} step={1} required value={line.quantity} aria-invalid={Boolean(tooMany)} onChange={(e) => changeLine(index, { quantity: Number(e.target.value) })} /></Field>
                  <Field label={`Prix unitaire ${index + 1}`}><MoneyInput required min={1} max={Number.MAX_SAFE_INTEGER} value={line.unitPrice} onChange={(e) => changeLine(index, { unitPrice: Number(e.target.value) })} /></Field>
                  <button type="button" className="icon-button" aria-label={`Retirer le produit ${index + 1}`} disabled={form.lines.length === 1} onClick={() => setForm({ ...form, lines: form.lines.filter((_, i) => i !== index) })}><X size={17} /></button>
                  {tooMany && <span className="sale-line-error" role="alert">Stock insuffisant pour {p.name}.</span>}
                </div>;
              })}
              <button type="button" className="text-button" disabled={!active.some((p) => p.stock > 0 && !form.lines.some((l) => l.productId === p.id))} onClick={() => setForm({ ...form, lines: [...form.lines, blankLine()] })}><Plus size={16} /> Ajouter un article</button>
            </div>
            <p className="stock-hint">Le prix est prérempli depuis le catalogue. Vous pouvez l’adapter pour cette vente.</p>
          </>}
          {editor?.kind === "receipt" && <div className="form-grid">
            <Field label="Quantité reçue"><TextInput autoFocus type="number" min={1} step={1} max={Number.MAX_SAFE_INTEGER} required value={form.quantity} onChange={(e) => setForm({ ...form, quantity: Number(e.target.value) })} /></Field>
            <Field label="Montant total payé"><MoneyInput min={1} max={Number.MAX_SAFE_INTEGER} required value={form.amount} onChange={(e) => setForm({ ...form, amount: Number(e.target.value) })} /></Field>
          </div>}
          {(editor?.kind === "sale" || editor?.kind === "receipt") && <>
            <div className="form-grid"><Field label={editor.kind === "sale" ? "Compte d’encaissement" : "Compte de paiement"}><AccountSelect value={form.accountId} onChange={(accountId) => setForm({ ...form, accountId })} /></Field><Field label="Date de l’opération"><TextInput type="date" required value={form.occurredAt} onChange={(e) => setForm({ ...form, occurredAt: e.target.value })} /></Field></div>
            <Field label="Note (facultatif)"><TextArea value={form.note} onChange={(e) => setForm({ ...form, note: e.target.value })} /></Field>
            <div className={`effect-preview ${editor.kind === "sale" ? "positive" : "negative"}`}><span>{editor.kind === "sale" ? "Montant encaissé · Capital attendu" : "Achat payé · Capital attendu"}</span><strong>{editor.kind === "sale" ? (Number.isSafeInteger(total) ? signed(total) : "Total trop élevé") : signed(form.amount === 0 ? 0 : -form.amount)}</strong></div>
            <p className="stock-hint">Paiement intégral. Cette opération sera automatiquement inscrite au journal de boutique.</p>
          </>}
          {editor?.kind === "adjust" && <>
            <Field label="Quantité réellement comptée" hint="Ce comptage corrige uniquement le stock, sans mouvement d’argent."><TextInput autoFocus type="number" min={0} step={1} max={Number.MAX_SAFE_INTEGER} required value={form.quantity} onChange={(e) => setForm({ ...form, quantity: Number(e.target.value) })} /></Field>
            <div className="selected-entry"><span>Écart de quantité</span><strong>{form.quantity - editor.product.stock > 0 ? "+" : ""}{form.quantity - editor.product.stock}</strong></div>
          </>}
          {editor?.kind === "cancel" && <>
            <div className="selected-entry"><span>{editor.operation.kind === "sale" ? "Annulation de vente" : "Annulation de réapprovisionnement"}</span><strong>{signed(editor.operation.kind === "sale" ? -editor.operation.amount : editor.operation.amount)}</strong></div>
            <p className="stock-hint">{editor.operation.kind === "sale" ? "Les articles sont remis en stock et le montant intégral est retiré du capital attendu. Utilisez cette annulation si la vente a été saisie par erreur ou si tous les articles ont été repris et le client remboursé." : "Les quantités reçues sont retirées du stock et le montant intégral est rétabli dans le capital attendu. Utilisez cette annulation si l’achat a été saisi par erreur ou si les articles ont été rendus et le montant récupéré."}</p>
          </>}
          {(editor?.kind === "adjust" || editor?.kind === "cancel") && <Field label="Motif obligatoire"><TextArea required minLength={3} value={form.reason} onChange={(e) => setForm({ ...form, reason: e.target.value })} placeholder="Expliquez la correction…" /></Field>}
        </fieldset>
        {formError && <div className="form-error" role="alert">{formError}</div>}
        <div className="modal-actions"><button type="button" className="button secondary" disabled={busy} onClick={close}>Fermer</button><button className="button primary" disabled={busy || (editor?.kind === "sale" && !saleValid)}>{busy ? "Enregistrement…" : editor?.kind === "cancel" ? "Confirmer l’annulation" : "Enregistrer"}</button></div>
      </form>
    </Modal>
    <Modal title={detail?.kind === "sale" ? "Détail de la vente" : "Détail du réapprovisionnement"} subtitle={detail ? `${formatDate(detail.occurredAt)} · ${accountDisplayLabel(detail.accountSnapshot)}` : undefined} open={Boolean(detail)} onClose={() => setDetail(undefined)} wide>
      {detail && <div className="product-operation-detail">
        <div className="selected-entry"><span>{detail.cancelledAt ? "Opération annulée" : "Paiement enregistré"}</span><strong>{formatMoney(detail.amount)}</strong></div>
        <div className="table-scroll"><table><thead><tr><th>Produit</th><th>Quantité</th><th>Prix unitaire</th><th>Total</th></tr></thead><tbody>{detail.lines.map((line) => <tr key={line.productId}><td>{line.productName}</td><td>{line.quantity}</td><td>{line.unitPrice == null ? "—" : formatMoney(line.unitPrice)}</td><td>{formatMoney(line.total)}</td></tr>)}</tbody></table></div>
        {detail.note && <p className="stock-hint">{detail.note}</p>}
        {detail.cancelledAt ? <div className="justification"><strong>Annulée le {formatDate(detail.cancelledAt, true)}</strong><p>{detail.cancellationReason}</p></div> : <button className="button secondary" disabled={busy} onClick={() => open({ kind: "cancel", operation: detail })}><RotateCcw size={17} /> Annuler cette opération</button>}
      </div>}
    </Modal>
  </div>;
}
