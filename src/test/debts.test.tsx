import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { api } from "../api";
import { DebtsPage } from "../screens/DebtsPage";
import type { CustomerRepayment, Debt, DebtCustomer, RepaymentPreview } from "../types";

vi.mock("../api", () => ({ api: { debtCustomers: vi.fn(), debts: vi.fn(), customerRepayments: vi.fn(), saveDebtCustomer: vi.fn(), createDebt: vi.fn(), previewRepayment: vi.fn(), repayCustomer: vi.fn(), cancelDebt: vi.fn(), accounts: vi.fn() } }));
const account = { accountId: "wave", provider: "wave" as const, name: "Wave réserve", active: true };
const clients: DebtCustomer[] = [
  { id: "awa", name: "Awa Fall", phone: "771234567", active: true, remaining: 50_000, totalRepaid: 20_000, overdueCount: 1, overdueAmount: 20_000 },
  { id: "binta", name: "Binta Fall", phone: "781234567", active: true, remaining: 0, totalRepaid: 10_000, overdueCount: 0, overdueAmount: 0 },
  { id: "old", name: "Client archivé", phone: "761234567", active: false, remaining: 0, totalRepaid: 0, overdueCount: 0, overdueAmount: 0 }
];
const debts: Debt[] = [
  { id: "first", customerId: "awa", customerName: "Awa ancien nom", phone: "771234567", provider: "wave", accountSnapshot: account, principal: 20_000, remaining: 20_000, issuedAt: "2026-01-01", dueDate: "2026-01-02", status: "overdue", createdAt: "2026-01-01", note: null, payments: [] },
  { id: "second", customerId: "awa", customerName: "Awa ancien nom", phone: "771234567", provider: "wave", accountSnapshot: account, principal: 30_000, remaining: 30_000, issuedAt: "2026-01-02", dueDate: null, status: "open", createdAt: "2026-01-02", note: null, payments: [] }
];
const preview: RepaymentPreview = { token: "allocation-token", eligibleTotal: 50_000, allocations: [{ debtId: "first", issuedAt: "2026-01-01", amount: 20_000, remainingAfter: 0 }, { debtId: "second", issuedAt: "2026-01-02", amount: 15_000, remainingAfter: 15_000 }] };
const receipt: CustomerRepayment = { id: "receipt", customerId: "awa", customerName: "Awa ancien nom", customerPhone: "771234567", amount: 35_000, accountSnapshot: account, paidAt: "2026-02-01", note: null, createdAt: "2026-02-01", legacy: false, allocations: preview.allocations };
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.debtCustomers).mockResolvedValue(clients);
  vi.mocked(api.debts).mockResolvedValue(debts);
  vi.mocked(api.customerRepayments).mockResolvedValue([receipt]);
  vi.mocked(api.accounts).mockResolvedValue([account, { accountId: "cash", provider: "cash", name: "Espèces", active: true }]);
  vi.mocked(api.previewRepayment).mockResolvedValue(preview);
  vi.mocked(api.repayCustomer).mockResolvedValue(receipt);
});
afterEach(cleanup);
const set = (label: string | RegExp, value: string) => fireEvent.change(screen.getByLabelText(label), { target: { value } });
async function page() {
  const onChanged = vi.fn(); render(<DebtsPage onChanged={onChanged} notify={vi.fn()} />);
  await screen.findByText("Awa Fall"); return onChanged;
}
async function openPayment() {
  fireEvent.click(screen.getByRole("button", { name: "Enregistrer un remboursement" }));
  await screen.findByRole("option", { name: "Espèces" });
  set("Client", "awa"); set("Montant reçu", "35000"); set("Date du remboursement", "2026-02-01"); set("Reçu sur", "cash");
  await waitFor(() => expect(screen.getByRole("button", { name: "Valider le remboursement" })).toBeEnabled());
}

describe("clients des dettes et remboursements globaux", () => {
  it("prévisualise les allocations puis rembourse le client une seule fois sur la caisse", async () => {
    const changed = await page(); await openPayment();
    expect(within(screen.getByRole("dialog")).getAllByText(/15\s000/).length).toBe(2);
    fireEvent.click(screen.getByRole("button", { name: "Valider le remboursement" }));
    await waitFor(() => expect(changed).toHaveBeenCalledOnce());
    expect(api.repayCustomer).toHaveBeenCalledWith(expect.objectContaining({ customerId: "awa", amount: 35_000, accountId: "cash", paidAt: "2026-02-01", previewToken: "allocation-token", requestId: expect.any(String) }));
  });
  it("réutilise la même requête après réponse perdue et empêche les doubles soumissions", async () => {
    vi.mocked(api.repayCustomer).mockRejectedValueOnce(new Error("Réponse perdue")).mockResolvedValueOnce(receipt);
    await page(); await openPayment();
    const form = screen.getByRole("button", { name: "Valider le remboursement" }).closest("form")!;
    fireEvent.submit(form); fireEvent.submit(form);
    expect(api.repayCustomer).toHaveBeenCalledOnce(); await screen.findByText("Réponse perdue");
    const original = vi.mocked(api.repayCustomer).mock.calls[0][0]; fireEvent.submit(form);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(vi.mocked(api.repayCustomer).mock.calls[1][0]).toEqual(original);
  });
  it("invalide un aperçu après changement de date et affiche les refus du serveur", async () => {
    await page(); await openPayment();
    vi.mocked(api.previewRepayment).mockRejectedValue(new Error("Le montant dépasse le total remboursable à cette date."));
    set("Date du remboursement", "2025-12-31");
    expect(screen.getByRole("button", { name: "Valider le remboursement" })).toBeDisabled();
    await screen.findByText(/Le montant dépasse/);
    expect(api.repayCustomer).not.toHaveBeenCalled();
    vi.mocked(api.previewRepayment).mockResolvedValue({ ...preview, token: "fresh" });
    set("Date du remboursement", "2026-02-01");
    await waitFor(() => expect(screen.getByRole("button", { name: "Valider le remboursement" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "Valider le remboursement" }));
    await waitFor(() => expect(api.repayCustomer).toHaveBeenCalledWith(expect.objectContaining({ previewToken: "fresh" })));
  });
  it("ajoute un client pendant la saisie de dette et conserve le montant saisi", async () => {
    vi.mocked(api.createDebt).mockResolvedValue(debts[0]);
    vi.mocked(api.saveDebtCustomer).mockResolvedValue({ ...clients[1], id: "new", name: "Moussa" });
    await page(); fireEvent.click(screen.getByRole("button", { name: "Ajouter une dette" }));
    await screen.findByRole("option", { name: "Wave réserve" });
    set("Montant transféré", "20000"); set("Compte émetteur", "wave");
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Ajouter un client" }));
    set("Nom complet", "Moussa"); set("Téléphone", "781234567");
    vi.mocked(api.debtCustomers).mockResolvedValue([...clients, { ...clients[1], id: "new", name: "Moussa" }]);
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer le client" }));
    await screen.findByRole("dialog", { name: "Ajouter une dette" });
    await screen.findByRole("option", { name: "Moussa · 781234567" });
    expect(screen.getByLabelText("Client")).toHaveValue("new"); expect(screen.getByLabelText("Montant transféré")).toHaveValue(20000);
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer la dette" }));
    await waitFor(() => expect(api.createDebt).toHaveBeenCalledWith(expect.objectContaining({ customerId: "new", amount: 20000, requestId: expect.any(String) })));
    expect(vi.mocked(api.createDebt).mock.calls[0][0]).not.toHaveProperty("customerName");
  });
  it("ouvre le paiement global depuis une dette avec le client présélectionné", async () => {
    await page(); fireEvent.click(screen.getByRole("tab", { name: "Dettes" }));
    fireEvent.click(screen.getAllByRole("button", { name: "Détail de la dette de Awa ancien nom" })[1]);
    fireEvent.click(screen.getByRole("button", { name: "Rembourser ce client" }));
    expect(screen.getByLabelText("Client")).toHaveValue("awa");
    expect(screen.getByRole("dialog")).toHaveTextContent("les dettes les plus anciennes");
  });
  it("recherche les fiches, protège l’archivage et permet de réactiver un client", async () => {
    vi.mocked(api.saveDebtCustomer).mockResolvedValue(clients[2]);
    await page(); set("Rechercher dans le répertoire", "771234567");
    fireEvent.click(screen.getByRole("button", { name: "Ouvrir la fiche" }));
    expect(screen.getByRole("button", { name: "Archiver" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Fermer" })); set("Rechercher dans le répertoire", "Client archivé");
    fireEvent.click(screen.getByRole("checkbox", { name: "Inclure les clients archivés" }));
    fireEvent.click(screen.getByRole("button", { name: "Ouvrir la fiche" }));
    fireEvent.click(screen.getByRole("button", { name: "Réactiver" }));
    await waitFor(() => expect(api.saveDebtCustomer).toHaveBeenCalledWith(expect.objectContaining({ customerId: "old", active: true })));
  });
  it("filtre les reçus par leur date propre et montre le total et le détail historique", async () => {
    await page(); fireEvent.click(screen.getByRole("tab", { name: "Remboursements" }));
    set("Paiements du", "2026-02-01"); set("Au", "2026-02-28");
    fireEvent.click(screen.getByRole("button", { name: "Voir la répartition" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("Awa ancien nom");
    expect(screen.getByRole("dialog")).toHaveTextContent(/35\s000/);
    expect(within(screen.getByRole("dialog")).getAllByRole("row")).toHaveLength(3);
    fireEvent.click(screen.getByRole("button", { name: "Fermer" }));
    set("Paiements du", "2026-03-01"); expect(screen.queryByRole("button", { name: "Voir la répartition" })).not.toBeInTheDocument();
  });
});
