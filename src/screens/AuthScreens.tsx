import { OpeningAccountsEditor } from "../components/OpeningAccountsEditor";
import { useEffect, useState, type FormEvent } from "react";
import { ArrowRight, KeyRound, LockKeyhole, ShieldCheck, Store, WalletCards } from "lucide-react";
import type { Dashboard, SetupInput, OpeningPreview } from "../types";
import { api } from "../api";
import { Field, MoneyInput, TextInput } from "../components/Fields";
import { formatMoney } from "../lib/format";

export function LoginScreen({ onSuccess }: { onSuccess: (dashboard: Dashboard) => void }) {
  const [pin, setPin] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setLoading(true);
    setError("");
    try {
      onSuccess(await api.login(pin));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="auth-page">
      <div className="auth-brand">
        <div className="brand-mark large"><WalletCards /></div>
        <span>Kër Finance</span>
      </div>
      <section className="auth-card login-card">
        <div className="auth-icon"><LockKeyhole size={28} /></div>
        <p className="eyebrow">ESPACE SÉCURISÉ</p>
        <h1>Bon retour</h1>
        <p className="auth-copy">Entrez votre PIN de gérant pour accéder aux comptes de la boutique.</p>
        <form onSubmit={submit} className="auth-form">
          <Field label="PIN du gérant">
            <TextInput
              autoFocus
              type="password"
              inputMode="numeric"
              autoComplete="current-password"
              placeholder="••••••"
              value={pin}
              onChange={(event) => setPin(event.target.value.replace(/\D/g, "").slice(0, 12))}
            />
          </Field>
          {error && <div className="form-error">{error}</div>}
          <button className="button primary full" disabled={loading || pin.length < 4}>
            {loading ? "Ouverture…" : "Ouvrir la boutique"} <ArrowRight size={18} />
          </button>
        </form>
        <p className="security-note"><ShieldCheck size={16} /> Vos données restent chiffrées sur cet ordinateur.</p>
      </section>
    </main>
  );
}

const initial: SetupInput = {
  businessName: "Mon multiservices",
  pin: "",
  recoveryPassword: "",
  initialCapital: 5_000_000,
  accounts: [
    { provider: "orange_money", name: "Orange Money — Principal", amount: 0 },
    { provider: "wave", name: "Wave — Principal", amount: 0 },
    { provider: "djamo", name: "Djamo — Principal", amount: 0 },
    { provider: "cash", name: "Espèces", amount: 0 }
  ]
};

export function SetupScreen({ onSuccess }: { onSuccess: (dashboard: Dashboard) => void }) {
  const [step, setStep] = useState(1);
  const [form, setForm] = useState(initial);
  const [pinConfirm, setPinConfirm] = useState("");
  const [recoveryConfirm, setRecoveryConfirm] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [allocation, setAllocation] = useState<OpeningPreview>();
  const [allocationError, setAllocationError] = useState("");
  useEffect(() => {
    let current = true;
    setAllocation(undefined);
    const timer = setTimeout(() => {
      if (form.accounts.some((a) => !Number.isSafeInteger(a.amount) || a.amount < 0)) {
        setAllocationError("Renseignez un montant entier positif ou nul sur chaque compte."); return;
      }
      api.previewOpening(form.accounts, form.initialCapital).then((result) => {
        if (current) { setAllocation(result); setAllocationError(""); }
      }).catch((e) => { if (current) setAllocationError(String(e)); });
    }, 200);
    return () => { current = false; clearTimeout(timer); };
  }, [form.accounts, form.initialCapital]);

  function patchValue(key: keyof SetupInput, value: string | number) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function next() {
    setError("");
    if (step === 1 && form.businessName.trim().length < 2) return setError("Indiquez le nom de la boutique.");
    if (step === 2 && (!allocation || allocation.difference !== 0 || form.initialCapital <= 0)) return setError("La répartition doit être exactement égale au capital initial.");
    setStep((value) => Math.min(3, value + 1));
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (form.pin !== pinConfirm) return setError("Les deux PIN ne correspondent pas.");
    if (form.recoveryPassword !== recoveryConfirm) return setError("Les mots de passe de récupération ne correspondent pas.");
    setLoading(true);
    setError("");
    try {
      onSuccess(await api.setup(form));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="setup-page">
      <aside className="setup-aside">
        <div className="auth-brand inverse">
          <div className="brand-mark"><WalletCards /></div>
          <span>Kër Finance</span>
        </div>
        <div className="setup-message">
          <p className="eyebrow light">BIENVENUE</p>
          <h1>Votre boutique,<br />toujours équilibrée.</h1>
          <p>Configurez votre capital de départ et protégez vos données en quelques minutes.</p>
        </div>
        <div className="setup-promise">
          <ShieldCheck />
          <div><strong>100 % local et chiffré</strong><span>Aucune donnée envoyée sur Internet</span></div>
        </div>
      </aside>
      <section className="setup-main">
        <div className="stepper">
          {["Boutique", "Capital", "Sécurité"].map((label, index) => (
            <div className={`step ${step >= index + 1 ? "active" : ""}`} key={label}>
              <span>{index + 1}</span><small>{label}</small>
            </div>
          ))}
        </div>

        {step === 1 && (
          <div className="setup-form-section">
            <div className="section-icon"><Store /></div>
            <p className="eyebrow">ÉTAPE 1 SUR 3</p>
            <h2>Identifiez votre boutique</h2>
            <p>Ce nom apparaîtra sur votre tableau de bord et vos rapports.</p>
            <div className="form-stack spaced">
              <Field label="Nom de la boutique">
                <TextInput autoFocus value={form.businessName} onChange={(e) => patchValue("businessName", e.target.value)} />
              </Field>
              <div className="read-only-row"><span>Devise</span><strong>Franc CFA (XOF)</strong></div>
              <div className="read-only-row"><span>Fuseau horaire</span><strong>Afrique / Dakar</strong></div>
            </div>
          </div>
        )}

        {step === 2 && (
          <div className="setup-form-section wide-form">
            <div className="section-icon"><WalletCards /></div>
            <p className="eyebrow">ÉTAPE 2 SUR 3</p>
            <h2>Répartissez le capital initial</h2>
            <p>Ajoutez vos SIM et répartissez le capital entre tous les comptes et la caisse.</p>
            <div className="capital-total-field">
              <Field label="Capital initial">
                <MoneyInput value={form.initialCapital} onChange={(e) => patchValue("initialCapital", Number(e.target.value))} />
              </Field>
              <span>FCFA</span>
            </div>
            <OpeningAccountsEditor accounts={form.accounts} onChange={(accounts) => { setAllocation(undefined); setForm((current) => ({ ...current, accounts })); }} />
            {allocationError && <div className="form-error" role="alert">{allocationError}</div>}
            <div className="allocation-summary">
              <div><span>Réparti</span><strong>{allocation ? formatMoney(allocation.liquidity) : "Calcul…"}</strong></div>
              <div className={allocation?.difference === 0 ? "balanced" : "unbalanced"}><span>Reste à répartir</span><strong>{allocation ? formatMoney(allocation.difference) : "—"}</strong></div>
            </div>
          </div>
        )}

        {step === 3 && (
          <form className="setup-form-section" onSubmit={submit}>
            <div className="section-icon"><KeyRound /></div>
            <p className="eyebrow">ÉTAPE 3 SUR 3</p>
            <h2>Protégez vos données</h2>
            <p>Le PIN sert chaque jour. Le mot de passe permet de restaurer une sauvegarde sur un autre PC.</p>
            <div className="form-grid spaced">
              <Field label="PIN du gérant" hint="4 à 12 chiffres">
                <TextInput type="password" inputMode="numeric" value={form.pin} onChange={(e) => patchValue("pin", e.target.value.replace(/\D/g, "").slice(0, 12))} />
              </Field>
              <Field label="Confirmer le PIN">
                <TextInput type="password" inputMode="numeric" value={pinConfirm} onChange={(e) => setPinConfirm(e.target.value.replace(/\D/g, "").slice(0, 12))} />
              </Field>
              <Field label="Mot de passe de récupération" hint="12 caractères minimum">
                <TextInput type="password" value={form.recoveryPassword} onChange={(e) => patchValue("recoveryPassword", e.target.value)} />
              </Field>
              <Field label="Confirmer le mot de passe">
                <TextInput type="password" value={recoveryConfirm} onChange={(e) => setRecoveryConfirm(e.target.value)} />
              </Field>
            </div>
            <div className="recovery-warning"><ShieldCheck /><span>Conservez ce mot de passe hors de l’ordinateur. Sans lui, une sauvegarde ne pourra pas être restaurée ailleurs.</span></div>
            {error && <div className="form-error">{error}</div>}
            <div className="setup-actions">
              <button type="button" className="button secondary" onClick={() => setStep(2)}>Retour</button>
              <button className="button primary" disabled={loading}>{loading ? "Création…" : "Créer mon espace"}<ArrowRight size={18} /></button>
            </div>
          </form>
        )}

        {step < 3 && (
          <>
            {error && <div className="form-error setup-error">{error}</div>}
            <div className="setup-actions fixed">
              {step > 1 && <button className="button secondary" onClick={() => setStep(step - 1)}>Retour</button>}
              <button className="button primary" onClick={next}>Continuer <ArrowRight size={18} /></button>
            </div>
          </>
        )}
      </section>
    </main>
  );
}
