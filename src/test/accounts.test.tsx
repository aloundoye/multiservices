import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { api } from "../api";
import { AccountSelect } from "../components/AccountSelect";
import { OpeningAccountsEditor } from "../components/OpeningAccountsEditor";
import { AccountsSettings } from "../screens/AccountsSettings";
import { InventoryPage } from "../screens/InventoryPages";
import { DashboardPage } from "../screens/DashboardPage";
import type { Account, Dashboard, Inventory, InventoryPreview, OpeningAccount } from "../types";

vi.mock("../api", () => ({ api: {
  accounts: vi.fn(), createAccount: vi.fn(), updateAccount: vi.fn(), archiveAccount: vi.fn(), reactivateAccount: vi.fn(),
  previewInventory: vi.fn(), closeInventory: vi.fn()
} }));

const accounts: Account[] = [
  { accountId: "om-1", provider: "orange_money", name: "Orange comptoir", identifier: "771112233", active: true, lastBalance: 1_000_000 },
  { accountId: "om-2", provider: "orange_money", name: "Orange réserve", active: true, lastBalance: 500_000 },
  { accountId: "wave-1", provider: "wave", name: "Wave 1", active: true, lastBalance: 1_200_000 },
  { accountId: "djamo-1", provider: "djamo", name: "Djamo 1", active: true, lastBalance: 800_000 },
  { accountId: "cash", provider: "cash", name: "Espèces", active: true, lastBalance: 1_500_000 },
  { accountId: "old", provider: "wave", name: "Ancienne Wave", active: false, lastBalance: 0 }
];
const totals = { orangeMoney: 1_500_000, wave: 1_200_000, djamo: 800_000, cash: 1_500_000 };
const inventory: Inventory = {
  id: "opening", kind: "opening", closedAt: "2026-09-01T08:00:00Z", balances: totals,
  accountBalances: accounts.filter((a) => a.active).map((a) => ({ ...a, amount: a.lastBalance!, delta: null, legacy: false })),
  receivables: 0, liquidity: 5_000_000, expectedTotal: 5_000_000, actualTotal: 5_000_000, variance: 0,
  delta: { orangeMoney: 0, wave: 0, djamo: 0, cash: 0 }
};
const dashboard: Dashboard = {
  accounts, lastInventory: inventory, expectedCapital: 5_000_000, lastActualCapital: 5_000_000,
  openReceivables: 0, openDebtsCount: 0, overdueDebtsCount: 0, journalNetSinceInventory: 0,
  nextInventoryAt: "2026-09-01T12:00:00Z", inventoryOverdue: false,
  settings: { businessName: "Boutique", currency: "XOF", timezone: "Africa/Dakar", inventoryIntervalMinutes: 240, autoLockMinutes: 15, createdAt: inventory.closedAt }
};
const preview: InventoryPreview = { ...inventory, previousBalances: totals };

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.accounts).mockResolvedValue(accounts);
  vi.mocked(api.previewInventory).mockResolvedValue(preview);
  vi.mocked(api.closeInventory).mockResolvedValue({ inventory });
});
afterEach(cleanup);

describe("comptes et SIM", () => {
  it("sélectionne un ID précis dans des groupes de services et masque les comptes archivés", async () => {
    const onChange = vi.fn();
    render(<AccountSelect value="" onChange={onChange} />);
    await screen.findByRole("option", { name: "Orange réserve" });
    expect(screen.getAllByRole("group")).toHaveLength(4);
    expect(screen.queryByRole("option", { name: "Ancienne Wave" })).not.toBeInTheDocument();
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "om-2" } });
    expect(onChange).toHaveBeenCalledWith("om-2");
  });

  it("autorise Djamo pour les dettes mais pas la caisse", async () => {
    render(<AccountSelect value="" onChange={vi.fn()} mobileOnly />);
    await screen.findByRole("option", { name: "Djamo 1" });
    expect(screen.queryByRole("option", { name: "Espèces" })).not.toBeInTheDocument();
  });

  it("ajoute et retire des SIM à l’ouverture sans permettre une seconde caisse", () => {
    function Editor() {
      const [values, setValues] = useState<OpeningAccount[]>([
        { provider: "orange_money", name: "Orange 1", amount: 1_000_000 },
        { provider: "cash", name: "Espèces", amount: 4_000_000 }
      ]);
      return <OpeningAccountsEditor accounts={values} onChange={setValues} />;
    }
    render(<Editor />);
    fireEvent.click(screen.getByRole("button", { name: "Ajouter un compte / SIM" }));
    const added = within(screen.getByRole("region", { name: "Orange SIM 1" }));
    fireEvent.change(added.getByLabelText("Service"), { target: { value: "wave" } });
    expect(added.queryByRole("option", { name: "Espèces" })).not.toBeInTheDocument();
    fireEvent.change(added.getByLabelText("Solde initial (FCFA)"), { target: { value: "" } });
    expect(added.getByLabelText("Solde initial (FCFA)")).toHaveValue(null);
    const cash = within(screen.getByRole("region", { name: "Espèces" }));
    expect(cash.getByLabelText("Service")).toBeDisabled();
    expect(cash.queryByRole("button")).not.toBeInTheDocument();
    fireEvent.click(added.getByRole("button", { name: "Retirer ce compte" }));
    expect(screen.getAllByRole("spinbutton")).toHaveLength(2);
  });

  it("exige le solde d’un compte jamais relevé puis envoie tous les IDs actifs", async () => {
    const extra: Account = { accountId: "new", provider: "wave", name: "Nouvelle SIM", active: true, lastBalance: null };
    const onDone = vi.fn();
    render(<InventoryPage dashboard={{ ...dashboard, accounts: [...accounts, extra] }} onDone={onDone} />);
    expect(screen.getByText("Pas encore relevé")).toBeInTheDocument();
    await screen.findByText("Renseignez un solde pour chaque compte.");
    expect(api.previewInventory).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: "Clôturer l’inventaire" })).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Nouvelle SIM"), { target: { value: "0" } });
    const close = await screen.findByRole("button", { name: "Clôturer l’inventaire" });
    fireEvent.click(close);
    await waitFor(() => expect(onDone).toHaveBeenCalled());
    expect(api.closeInventory).toHaveBeenCalledWith({
      balances: [...accounts.filter((a) => a.active).map((a) => ({ accountId: a.accountId, amount: a.lastBalance })), { accountId: "new", amount: 0 }],
      varianceCategory: null, varianceNote: null
    });
  });

  it("affiche les totaux fournis par Rust et n’invente pas de variation initiale", async () => {
    vi.mocked(api.previewInventory).mockResolvedValue({ ...preview, balances: { ...totals, orangeMoney: 1_234_567 } });
    render(<InventoryPage dashboard={dashboard} onDone={vi.fn()} />);
    await screen.findByText(/1\s234\s567/);
    expect(api.previewInventory).toHaveBeenCalledWith({ balances: accounts.filter((a) => a.active).map((a) => ({ accountId: a.accountId, amount: a.lastBalance })) });
    expect(screen.getAllByText("—")).toHaveLength(5);
  });

  it("renomme sans changer le service et laisse l’archivage à la validation Rust", async () => {
    const onChanged = vi.fn();
    vi.mocked(api.updateAccount).mockResolvedValue({ ...accounts[0], name: "Comptoir neuf" });
    vi.mocked(api.archiveAccount).mockRejectedValue(new Error("Clôturez un inventaire avec un solde nul."));
    render(<AccountsSettings notify={vi.fn()} onChanged={onChanged} />);
    await screen.findByText("Orange comptoir — 771112233");
    fireEvent.click(screen.getAllByRole("button", { name: "Modifier" })[0]);
    const modal = within(screen.getByRole("dialog"));
    expect(modal.getByLabelText("Service")).toBeDisabled();
    fireEvent.change(modal.getByLabelText("Nom du compte"), { target: { value: "Comptoir neuf" } });
    fireEvent.click(modal.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.updateAccount).toHaveBeenCalledWith({ accountId: "om-1", name: "Comptoir neuf", identifier: "771112233" });
    fireEvent.click(screen.getAllByRole("button", { name: "Archiver" })[0]);
    expect(await screen.findByRole("alert")).toHaveTextContent("solde nul");
    expect(api.archiveAccount).toHaveBeenCalledWith("om-1");
    expect(onChanged).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "Réactiver" }));
    await waitFor(() => expect(api.reactivateAccount).toHaveBeenCalledWith("old"));
  });

  it("déplie le détail par service et signale les nouvelles SIM non relevées", () => {
    render(<DashboardPage dashboard={{ ...dashboard, accounts: [...accounts, { accountId: "new", provider: "wave", name: "Nouvelle Wave", active: true }] }} onNavigate={vi.fn()} />);
    expect(screen.getByText("Comptes Orange Money (2)")).toBeInTheDocument();
    expect(screen.getByText("Comptes Wave (2)")).toBeInTheDocument();
    expect(screen.getByText("Pas encore relevé")).toBeInTheDocument();
    expect(screen.queryByText("Ancienne Wave")).not.toBeInTheDocument();
  });
});
