import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { api } from "../api";
import { CustodyPage } from "../screens/CustodyPage";
import type { CustodyCustomer, CustodyMovement } from "../types";

vi.mock("../api", () => ({ api: { custodyCustomers: vi.fn(), custodyMovements: vi.fn(), saveCustodyCustomer: vi.fn(), recordCustodyMovement: vi.fn(), reverseCustodyMovement: vi.fn(), previewCustodyOpening: vi.fn(), recordCustodyOpening: vi.fn(), accounts: vi.fn() } }));
const clients: CustodyCustomer[] = [
  { id: "awa", name: "Awa Fall", phone: "771234567", active: true, balance: 200_000, totalReceived: 250_000, totalWithdrawn: 50_000 },
  { id: "moussa", name: "Moussa", phone: null, active: true, balance: 0, totalReceived: 0, totalWithdrawn: 0 },
  { id: "old", name: "Client archivé", phone: null, active: false, balance: 0, totalReceived: 10_000, totalWithdrawn: 10_000 }
];
const account = { accountId: "wave-2", provider: "wave" as const, name: "Wave réserve", active: true };
const movement: CustodyMovement = { sequence: 1, id: "movement", customerId: "awa", customerName: "Ancien nom Awa", customerPhone: "771234567", kind: "deposit", delta: 200_000, capitalAdjustment: 0, balanceAfter: 200_000, accountSnapshot: account, occurredAt: "2026-09-01", postedAt: "2026-09-02T10:00:00Z", operator: "Gérant", note: "Pour garde", reversed: false, reversesId: null };
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.custodyCustomers).mockResolvedValue(clients);
  vi.mocked(api.custodyMovements).mockResolvedValue([movement]);
  vi.mocked(api.accounts).mockResolvedValue([account, { accountId: "cash", provider: "cash", name: "Espèces", active: true }]);
});
afterEach(cleanup);
const set = (label: string | RegExp, value: string) => fireEvent.change(screen.getByLabelText(label), { target: { value } });
async function page(initialAction?: "deposit" | "withdrawal") {
  const onChanged = vi.fn(); const notify = vi.fn();
  render(<CustodyPage onChanged={onChanged} notify={notify} initialAction={initialAction} />);
  await screen.findByText("Awa Fall"); return { onChanged, notify };
}

describe("dépôts clients", () => {
  it("enregistre une restitution partielle sur un autre compte et contrôle le solde client", async () => {
    vi.mocked(api.recordCustodyMovement).mockResolvedValue(movement);
    const { onChanged, notify } = await page();
    fireEvent.click(screen.getByRole("button", { name: "Restituer à Awa Fall" }));
    await screen.findByRole("option", { name: "Espèces" });
    set(/^Montant \(FCFA\)/, "200001"); set("Compte de restitution", "cash");
    expect(screen.getByRole("button", { name: "Enregistrer" })).toBeDisabled();
    set(/^Montant \(FCFA\)/, "80000");
    expect(within(screen.getByRole("dialog")).getByText(/120\s000/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.recordCustodyMovement).toHaveBeenCalledWith(expect.objectContaining({ customerId: "awa", kind: "withdrawal", amount: 80_000, accountId: "cash", requestId: expect.any(String) }));
    expect(notify).toHaveBeenCalledWith(expect.stringContaining("capital attendu reste inchangé"));
  });

  it("conserve la même demande après erreur et empêche les doubles envois", async () => {
    vi.mocked(api.recordCustodyMovement).mockRejectedValueOnce(new Error("Réponse perdue")).mockResolvedValueOnce(movement);
    await page("deposit");
    await screen.findByRole("option", { name: "Wave réserve" });
    set("Client", "awa"); set(/^Montant \(FCFA\)/, "50000"); set("Compte d’encaissement", "wave-2");
    const form = screen.getByRole("button", { name: "Enregistrer" }).closest("form")!;
    fireEvent.submit(form); fireEvent.submit(form);
    expect(api.recordCustodyMovement).toHaveBeenCalledOnce();
    expect(await screen.findByRole("alert")).toHaveTextContent("Réponse perdue");
    const original = vi.mocked(api.recordCustodyMovement).mock.calls[0][0];
    fireEvent.submit(form);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(vi.mocked(api.recordCustodyMovement).mock.calls[1][0]).toEqual(original);
  });

  it("prévisualise puis valide une reprise de plusieurs clients", async () => {
    vi.mocked(api.previewCustodyOpening).mockResolvedValue({ total: 200_000, expectedCapital: 1_000_000, correctedCapital: 800_000 });
    vi.mocked(api.recordCustodyOpening).mockResolvedValue([movement]);
    await page(); fireEvent.click(screen.getByRole("button", { name: "Reprendre les montants" }));
    expect(screen.getByRole("button", { name: "Valider la reprise" })).toBeDisabled();
    set("Client 1", "awa"); set("Montant restant 1", "150000");
    fireEvent.click(screen.getByRole("button", { name: "Ajouter un montant client" }));
    set("Client 2", "moussa"); set("Montant restant 2", "50000");
    await waitFor(() => expect(screen.getByRole("button", { name: "Valider la reprise" })).toBeEnabled());
    expect(within(screen.getByRole("dialog")).getByText(/800\s000/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Valider la reprise" }));
    await waitFor(() => expect(api.recordCustodyOpening).toHaveBeenCalledOnce());
    expect(api.recordCustodyOpening).toHaveBeenCalledWith(expect.objectContaining({ lines: [{ customerId: "awa", amount: 150_000 }, { customerId: "moussa", amount: 50_000 }] }));
  });

  it("affiche les libellés historiques et annule avec un motif", async () => {
    vi.mocked(api.reverseCustodyMovement).mockResolvedValue({ ...movement, kind: "reversal", reversesId: movement.id });
    await page(); fireEvent.click(screen.getByRole("tab", { name: "Registre des mouvements" }));
    expect(screen.getByText("Ancien nom Awa")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Détail du mouvement 1" }));
    expect(screen.getByRole("dialog")).toHaveTextContent("Gérant");
    fireEvent.click(screen.getByRole("button", { name: "Annuler ce mouvement" }));
    set("Motif obligatoire", "Dépôt saisi deux fois");
    fireEvent.click(screen.getByRole("button", { name: "Confirmer l’annulation" }));
    await waitFor(() => expect(api.reverseCustodyMovement).toHaveBeenCalledWith(expect.objectContaining({ movementId: "movement", reason: "Dépôt saisi deux fois" })));
  });

  it("archive uniquement les soldes nuls, recherche et crée une fiche sans téléphone", async () => {
    vi.mocked(api.saveCustodyCustomer).mockResolvedValue(clients[1]);
    await page();
    expect(screen.getByRole("button", { name: "Archiver Awa Fall" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Archiver Moussa" })).toBeEnabled();
    expect(screen.queryByText("Client archivé")).not.toBeInTheDocument();
    set("Rechercher un client", "771234567");
    expect(screen.queryByText("Moussa")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Ajouter un client" }));
    set("Nom du client", "Binta"); fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(api.saveCustodyCustomer).toHaveBeenCalledWith(expect.objectContaining({ name: "Binta", phone: null, customerId: null, active: true })));
  });
});
