import { Field, MoneyInput, SelectInput, TextInput } from "./Fields";
import { providers } from "./AccountSelect";
import { label } from "../lib/format";
import type { OpeningAccount } from "../types";

export function OpeningAccountsEditor({ accounts, onChange }: {
  accounts: OpeningAccount[]; onChange: (accounts: OpeningAccount[]) => void;
}) {
  function update(index: number, patch: Partial<OpeningAccount>) {
    onChange(accounts.map((a, i) => i === index ? { ...a, ...patch } : a));
  }
  function add() {
    let number = 1;
    while (accounts.some((a) => a.provider === "orange_money" && a.name === `Orange SIM ${number}`)) number++;
    onChange([...accounts, { provider: "orange_money", name: `Orange SIM ${number}`, amount: 0, identifier: "" }]);
  }
  return <div className="sim-editor">
    {accounts.map((a, i) => <section className="sim-editor-row" key={i} aria-label={a.name || `Compte ${i + 1}`}>
      <div className="form-grid">
        <Field label="Service"><SelectInput value={a.provider} disabled={a.provider === "cash"} onChange={(e) => update(i, { provider: e.target.value as OpeningAccount["provider"] })}>
          {providers.filter((p) => p !== "cash" || a.provider === "cash").map((p) => <option key={p} value={p}>{label(p)}</option>)}
        </SelectInput></Field>
        <Field label="Nom du compte"><TextInput required value={a.name} onChange={(e) => update(i, { name: e.target.value })} /></Field>
        {a.provider !== "cash" && <Field label="Numéro ou identifiant (facultatif)"><TextInput value={a.identifier ?? ""} onChange={(e) => update(i, { identifier: e.target.value })} /></Field>}
        <Field label="Solde initial (FCFA)"><MoneyInput required value={Number.isNaN(a.amount) ? "" : a.amount} onChange={(e) => update(i, { amount: e.target.value === "" ? NaN : Number(e.target.value) })} /></Field>
      </div>
      {a.provider !== "cash" && <button type="button" className="text-button" onClick={() => onChange(accounts.filter((_, index) => index !== i))}>Retirer ce compte</button>}
    </section>)}
    <button type="button" className="button secondary" onClick={add}>Ajouter un compte / SIM</button>
  </div>;
}
