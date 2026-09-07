import { useEffect, useState } from "react";
import { api } from "../api";
import type { Account, AccountSnapshot, Provider } from "../types";
import { label } from "../lib/format";
import { SelectInput } from "./Fields";

export const providers: Provider[] = ["orange_money", "wave", "djamo", "cash"];
export function accountLabel(account: AccountSnapshot) {
  return account.name + (account.identifier ? ` — ${account.identifier}` : "");
}
export function accountDisplayLabel(account: AccountSnapshot) {
  return `${label(account.provider)} — ${accountLabel(account)}`;
}

export function AccountSelect({ value, onChange, mobileOnly = false }: {
  value: string; onChange: (id: string) => void; mobileOnly?: boolean;
}) {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [error, setError] = useState("");
  useEffect(() => {
    let current = true;
    api.accounts().then((items) => { if (current) setAccounts(items); })
      .catch((e) => { if (current) setError(String(e)); });
    return () => { current = false; };
  }, []);
  return <>
    <SelectInput required value={value} onChange={(e) => onChange(e.target.value)}>
      <option value="">Choisir un compte…</option>
      {providers.filter((p) => !mobileOnly || p !== "cash").map((provider) => {
        const available = accounts.filter((a) => a.active && a.provider === provider);
        return available.length > 0 && <optgroup key={provider} label={label(provider)}>
          {available.map((a) => <option key={a.accountId} value={a.accountId}>{accountLabel(a)}</option>)}
        </optgroup>;
      })}
    </SelectInput>
    {error && <span role="alert" className="form-error">{error}</span>}
  </>;
}
