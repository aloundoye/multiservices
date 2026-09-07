import { useEffect, useState, type FormEvent } from "react";
import { api } from "../api";
import { accountLabel, providers } from "../components/AccountSelect";
import { Field, SelectInput, TextInput } from "../components/Fields";
import { Modal } from "../components/Modal";
import { formatMoney, formatDate, label } from "../lib/format";
import type { Account, MobileProvider } from "../types";

export function AccountsSettings({ notify, onChanged }: { notify: (message: string) => void; onChanged: () => void }) {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState<Account>();
  const [form, setForm] = useState({ provider: "orange_money" as MobileProvider, name: "", identifier: "" });
  async function load() { setAccounts(await api.accounts()); }
  useEffect(() => { load().catch((e) => setError(String(e))); }, []);
  function start(account?: Account) {
    setEditing(account); setError("");
    setForm({ provider: (account?.provider ?? "orange_money") as MobileProvider, name: account?.name ?? "", identifier: account?.identifier ?? "" });
    setOpen(true);
  }
  async function save(e: FormEvent) {
    e.preventDefault(); setError(""); setBusy(true);
    try {
      if (editing) await api.updateAccount({ accountId: editing.accountId, name: form.name, identifier: form.identifier || null });
      else await api.createAccount({ ...form, identifier: form.identifier || null });
      setOpen(false); await load(); onChanged(); notify(editing ? "Compte mis à jour. L’historique est conservé." : "Compte créé. Son solde sera saisi au prochain inventaire.");
    } catch (reason) { setError(String(reason)); } finally { setBusy(false); }
  }
  async function toggle(account: Account) {
    setError(""); setBusy(true);
    try {
      if (account.active) await api.archiveAccount(account.accountId);
      else await api.reactivateAccount(account.accountId);
      await load(); onChanged(); notify(account.active ? "Compte archivé." : "Compte réactivé.");
    } catch (reason) { setError(String(reason)); } finally { setBusy(false); }
  }
  return <section className="panel settings-panel accounts-settings">
    <header className="panel-header"><div><h2>Comptes et SIM</h2><p>Plusieurs comptes par service, une seule caisse espèces.</p></div><button className="button primary" onClick={() => start()}>Ajouter un compte</button></header>
    <p className="accounts-help">Créer un compte ne modifie pas le capital. Renseignez son solde au prochain inventaire ; enregistrez tout apport dans le journal.</p>
    {error && <div role="alert" className="form-error">{error}</div>}
    {providers.map((provider) => <section className="account-group" key={provider}>
      <h3>{label(provider)}</h3>
      {accounts.filter((a) => a.provider === provider).map((a) => <div className="managed-account" key={a.accountId}>
        <div><strong>{accountLabel(a)}</strong><small>{a.active ? "Actif" : "Archivé"} · {a.lastBalance == null ? "Pas encore relevé" : `${formatMoney(a.lastBalance)} au ${formatDate(a.lastMeasuredAt ?? "")}`}</small></div>
        <div className="account-actions"><button className="button secondary" disabled={busy} onClick={() => start(a)}>Modifier</button>
          {a.provider !== "cash" && <button className="button secondary" disabled={busy} onClick={() => toggle(a)}>{a.active ? "Archiver" : "Réactiver"}</button>}</div>
      </div>)}
      {!accounts.some((a) => a.provider === provider) && <p className="empty-inline">Aucun compte pour ce service.</p>}
    </section>)}
    <p className="accounts-help">Pour archiver un compte utilisé, clôturez un inventaire avec un solde nul. Aucune nouvelle écriture ne doit avoir été enregistrée sur ce compte depuis ce relevé.</p>
    <Modal title={editing ? "Modifier le compte" : "Ajouter un compte / SIM"} open={open} onClose={() => setOpen(false)}>
      <form className="modal-form" onSubmit={save}>
        <Field label="Service"><SelectInput disabled={Boolean(editing)} value={editing?.provider ?? form.provider} onChange={(e) => setForm({ ...form, provider: e.target.value as MobileProvider })}>{providers.filter((p) => p !== "cash" || editing?.provider === "cash").map((p) => <option key={p} value={p}>{label(p)}</option>)}</SelectInput></Field>
        <Field label="Nom du compte"><TextInput autoFocus required value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="Ex. Orange SIM comptoir" /></Field>
        {editing?.provider !== "cash" && <Field label="Numéro ou identifiant (facultatif)"><TextInput value={form.identifier} onChange={(e) => setForm({ ...form, identifier: e.target.value })} placeholder="77 123 45 67" /></Field>}
        {error && <div role="alert" className="form-error">{error}</div>}
        <div className="modal-actions"><button type="button" className="button secondary" onClick={() => setOpen(false)}>Annuler</button><button className="button primary" disabled={busy}>Enregistrer</button></div>
      </form>
    </Modal>
  </section>;
}
