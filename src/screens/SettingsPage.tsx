import { useEffect, useState, type FormEvent } from "react";
import { ArchiveRestore, DatabaseBackup, ExternalLink, FileKey2, HardDrive, History, Save, Settings2, ShieldCheck, Usb } from "lucide-react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import { Field, SelectInput, TextInput } from "../components/Fields";
import { Modal } from "../components/Modal";
import { formatDate } from "../lib/format";
import type { AuditEvent, BackupInfo, BusinessSettings } from "../types";

const auditLabels: Record<string, string> = {
  business_initialized: "Boutique initialisée",
  settings_updated: "Paramètres modifiés",
  inventory_closed: "Inventaire clôturé",
  inventory_corrected: "Inventaire corrigé par écriture liée",
  journal_entry_created: "Écriture ajoutée",
  journal_entry_reversed: "Écriture corrigée",
  debt_created: "Dette enregistrée",
  debt_payment_recorded: "Remboursement enregistré",
  debt_cancelled: "Dette annulée"
};

export function SettingsPage({ notify, onChanged }: { notify: (message: string) => void; onChanged: () => void }) {
  const [settings, setSettings] = useState<BusinessSettings>();
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [audit, setAudit] = useState<AuditEvent[]>([]);
  const [tab, setTab] = useState<"general" | "backup" | "audit">("general");
  const [restoreOpen, setRestoreOpen] = useState(false);
  const [restorePath, setRestorePath] = useState("");
  const [recoveryPassword, setRecoveryPassword] = useState("");
  const [newPin, setNewPin] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function load() {
    const [settingsValue, backupValues, auditValues] = await Promise.all([api.settings(), api.backups(), api.audit(100)]);
    setSettings(settingsValue); setBackups(backupValues); setAudit(auditValues);
  }
  useEffect(() => { void load(); }, []);

  async function saveSettings(event: FormEvent) {
    event.preventDefault(); if (!settings) return; setLoading(true); setError("");
    try { const value = await api.updateSettings(settings as unknown as Record<string, unknown>); setSettings(value); onChanged(); notify("Paramètres enregistrés."); }
    catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setLoading(false); }
  }

  async function localBackup() {
    setLoading(true); setError(""); try { await api.createBackup(); await load(); notify("Sauvegarde locale créée."); } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); } finally { setLoading(false); }
  }

  async function externalBackup() {
    const destination = await save({ title: "Copier la sauvegarde vers une clé USB", defaultPath: `ker-finance-${new Date().toISOString().slice(0, 10)}.msbackup`, filters: [{ name: "Sauvegarde Kër Finance", extensions: ["msbackup"] }] });
    if (!destination) return; setLoading(true); setError(""); try { await api.createBackup(destination); notify("Sauvegarde externe créée."); } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); } finally { setLoading(false); }
  }

  async function chooseRestore() {
    const selected = await open({ title: "Choisir une sauvegarde Kër Finance", multiple: false, filters: [{ name: "Sauvegarde Kër Finance", extensions: ["msbackup"] }] });
    if (typeof selected === "string") { setRestorePath(selected); setRestoreOpen(true); }
  }

  async function restore(event: FormEvent) {
    event.preventDefault(); if (!window.confirm("La restauration remplacera les données actuelles après avoir créé une sauvegarde de sécurité. Continuer ?")) return;
    setLoading(true); setError(""); try { await api.restoreBackup(restorePath, recoveryPassword, newPin); setRestoreOpen(false); notify("Sauvegarde restaurée. La session utilise maintenant le nouveau PIN."); await load(); onChanged(); } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); } finally { setLoading(false); }
  }

  return (
    <div className="page">
      <header className="page-header"><div><p className="eyebrow">SYSTÈME</p><h1>Paramètres et sécurité</h1><p>Configurez les rappels, sauvegardes et consultez la trace d’audit.</p></div></header>
      <div className="settings-tabs"><button className={tab === "general" ? "active" : ""} onClick={() => setTab("general")}><Settings2 /> Général</button><button className={tab === "backup" ? "active" : ""} onClick={() => setTab("backup")}><DatabaseBackup /> Sauvegardes</button><button className={tab === "audit" ? "active" : ""} onClick={() => setTab("audit")}><History /> Journal d’audit</button></div>
      {error && <div className="form-error page-error">{error}</div>}
      {tab === "general" && settings && <form className="panel settings-panel" onSubmit={saveSettings}><header className="panel-header"><div><h2>Configuration de la boutique</h2><p>Les changements prennent effet immédiatement.</p></div></header><div className="settings-form"><Field label="Nom de la boutique"><TextInput value={settings.businessName} onChange={(e) => setSettings({ ...settings, businessName: e.target.value })} /></Field><div className="form-grid"><Field label="Rappel d’inventaire"><SelectInput value={settings.inventoryIntervalMinutes} onChange={(e) => setSettings({ ...settings, inventoryIntervalMinutes: Number(e.target.value) })}><option value={60}>Toutes les heures</option><option value={120}>Toutes les 2 heures</option><option value={240}>Toutes les 4 heures</option><option value={360}>Toutes les 6 heures</option><option value={480}>Toutes les 8 heures</option><option value={720}>Toutes les 12 heures</option><option value={1440}>Une fois par jour</option></SelectInput></Field><Field label="Verrouillage automatique"><SelectInput value={settings.autoLockMinutes} onChange={(e) => setSettings({ ...settings, autoLockMinutes: Number(e.target.value) })}><option value={5}>Après 5 minutes</option><option value={10}>Après 10 minutes</option><option value={15}>Après 15 minutes</option><option value={30}>Après 30 minutes</option><option value={60}>Après 1 heure</option></SelectInput></Field></div><div className="read-only-row"><span>Devise</span><strong>{settings.currency} — Franc CFA</strong></div><div className="read-only-row"><span>Fuseau horaire</span><strong>{settings.timezone}</strong></div><button className="button primary align-self" disabled={loading}><Save /> Enregistrer</button></div></form>}
      {tab === "backup" && <div className="backup-layout"><section className="panel settings-panel"><header className="panel-header"><div><h2>Protéger les données</h2><p>Les sauvegardes contiennent la base chiffrée et sa clé de récupération protégée.</p></div></header><div className="backup-actions"><button className="backup-action" onClick={localBackup} disabled={loading}><span className="backup-action-icon"><HardDrive /></span><div><strong>Sauvegarde locale</strong><small>Créer une copie dans le dossier sécurisé de l’application</small></div><ExternalLink /></button><button className="backup-action" onClick={externalBackup} disabled={loading}><span className="backup-action-icon usb"><Usb /></span><div><strong>Copier vers une clé USB</strong><small>Choisir un dossier externe pour conserver une copie</small></div><ExternalLink /></button><button className="backup-action restore" onClick={chooseRestore} disabled={loading}><span className="backup-action-icon restore"><ArchiveRestore /></span><div><strong>Restaurer une sauvegarde</strong><small>Vérifier puis remplacer les données actuelles</small></div><ExternalLink /></button></div><div className="security-callout"><ShieldCheck /><div><strong>Chiffrement actif</strong><p>La base et les copies sont illisibles sans les clés. Gardez le mot de passe de récupération dans un lieu sûr.</p></div></div></section><section className="panel backup-history"><header className="panel-header"><div><h2>Copies locales</h2><p>Les 30 sauvegardes les plus récentes sont conservées.</p></div></header>{backups.length === 0 ? <div className="empty-inline">Aucune sauvegarde locale.</div> : backups.slice(0, 10).map((item) => <div className="backup-row" key={item.path}><FileKey2 /><div><strong>{formatDate(item.createdAt, true)}</strong><small>{(item.sizeBytes / 1024).toFixed(0)} Ko • {item.path.split(/[\\/]/).pop()}</small></div></div>)}</section></div>}
      {tab === "audit" && <section className="panel table-panel"><div className="table-scroll"><table><thead><tr><th>Date et heure</th><th>Action</th><th>Élément</th><th>Détails vérifiables</th></tr></thead><tbody>{audit.map((event) => <tr key={event.id}><td>{formatDate(event.occurredAt, true)}</td><td><strong>{auditLabels[event.action] ?? event.action}</strong></td><td>{event.entityType}{event.entityId && <small>{event.entityId.slice(0, 8)}…</small>}</td><td><code>{JSON.stringify(event.details)}</code></td></tr>)}</tbody></table></div></section>}
      <Modal title="Restaurer une sauvegarde" subtitle="Le mot de passe déchiffre la copie; choisissez ensuite le nouveau PIN quotidien." open={restoreOpen} onClose={() => setRestoreOpen(false)}><form className="modal-form" onSubmit={restore}><div className="selected-file"><FileKey2 /><span>{restorePath.split(/[\\/]/).pop()}</span></div><Field label="Mot de passe de récupération"><TextInput type="password" autoFocus value={recoveryPassword} onChange={(e) => setRecoveryPassword(e.target.value)} required /></Field><Field label="Nouveau PIN du gérant"><TextInput type="password" inputMode="numeric" value={newPin} onChange={(e) => setNewPin(e.target.value.replace(/\D/g, "").slice(0, 12))} required /></Field>{error && <div className="form-error">{error}</div>}<div className="modal-actions"><button type="button" className="button secondary" onClick={() => setRestoreOpen(false)}>Annuler</button><button className="button danger" disabled={loading}><ArchiveRestore /> Restaurer</button></div></form></Modal>
    </div>
  );
}
