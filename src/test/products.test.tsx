import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { api } from "../api";
import { ProductsPage } from "../screens/ProductsPage";
import { JournalPage } from "../screens/JournalPage";
import type { Product, ProductOperation } from "../types";

vi.mock("../api", () => ({ api: {
  products: vi.fn(), productOperations: vi.fn(), stockMovements: vi.fn(), accounts: vi.fn(),
  createProduct: vi.fn(), updateProduct: vi.fn(), archiveProduct: vi.fn(), adjustStock: vi.fn(),
  createSale: vi.fn(), receiveStock: vi.fn(), cancelProductOperation: vi.fn(), journal: vi.fn(), reverseJournal: vi.fn()
} }));
const products: Product[] = [
  { id: "cable", name: "Câble USB", price: 2000, stock: 10, active: true },
  { id: "book", name: "Cahier", price: 500, stock: 3, active: true },
  { id: "empty", name: "Chargeur", price: 3000, stock: 0, active: true },
  { id: "old", name: "Ancien produit", price: 1000, stock: 0, active: false }
];
const account = { accountId: "cash", provider: "cash" as const, name: "Espèces", active: true };
const operation: ProductOperation = { id: "sale", kind: "sale", journalEntryId: "journal-sale", accountSnapshot: account,
  occurredAt: "2026-09-07", createdAt: "2026-09-07T10:00:00Z", amount: 4000, note: "Vente", cancelledAt: null, cancellationReason: null,
  lines: [{ productId: "cable", productName: "Ancien nom conservé", quantity: 2, unitPrice: 2000, total: 4000 }] };
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.products).mockResolvedValue(products);
  vi.mocked(api.productOperations).mockResolvedValue([]);
  vi.mocked(api.stockMovements).mockResolvedValue([]);
  vi.mocked(api.accounts).mockResolvedValue([account]);
});
afterEach(cleanup);
async function page() {
  const onChanged = vi.fn(); const notify = vi.fn();
  render(<ProductsPage onChanged={onChanged} notify={notify} />);
  await screen.findByText("Câble USB");
  return { onChanged, notify };
}
const set = (name: string | RegExp, value: string) => fireEvent.change(screen.getByLabelText(name), { target: { value } });

describe("produits et ventes", () => {
  it("crée un produit et réserve le stock initial aux marchandises déjà détenues", async () => {
    vi.mocked(api.createProduct).mockResolvedValue(products[0]);
    const { onChanged } = await page();
    expect(screen.queryByText("Ancien produit")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Archiver Câble USB" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Ajouter un produit" }));
    expect(screen.getByText(/aucun achat ne sera ajouté/)).toBeInTheDocument();
    set("Nom du produit", "Stylo"); set("Prix de vente unitaire", "200"); set(/Quantité déjà en stock/, "15");
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.createProduct).toHaveBeenCalledWith({ requestId: expect.any(String), name: "Stylo", price: 200, initialStock: 15 });
    expect(api.receiveStock).not.toHaveBeenCalled();
  });

  it("vend un panier avec prix modifié et actualise le budget", async () => {
    vi.mocked(api.createSale).mockResolvedValue(operation);
    const { onChanged } = await page();
    fireEvent.click(screen.getByRole("button", { name: "Nouvelle vente" }));
    await screen.findByRole("option", { name: "Espèces" });
    set("Produit 1", "cable");
    expect(screen.getByLabelText("Prix unitaire 1")).toHaveValue(2000);
    set(/Quantité 1/, "2"); set("Prix unitaire 1", "1750");
    fireEvent.click(screen.getByRole("button", { name: "Ajouter un article" }));
    set("Produit 2", "book"); set("Quantité 2", "1"); set("Compte d’encaissement", "cash");
    const modal = within(screen.getByRole("dialog"));
    expect(modal.getByText(/\+4\s000/)).toBeInTheDocument();
    fireEvent.click(modal.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.createSale).toHaveBeenCalledWith(expect.objectContaining({ accountId: "cash", requestId: expect.any(String), lines: [
      { productId: "cable", quantity: 2, unitPrice: 1750 }, { productId: "book", quantity: 1, unitPrice: 500 }
    ] }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("bloque les quantités excessives et conserve la demande pour réessayer après erreur", async () => {
    vi.mocked(api.createSale).mockRejectedValueOnce(new Error("Connexion interrompue")).mockResolvedValueOnce(operation);
    await page();
    fireEvent.click(screen.getByRole("button", { name: "Nouvelle vente" }));
    await screen.findByRole("option", { name: "Espèces" });
    set("Produit 1", "cable"); set("Quantité 1", "11"); set("Compte d’encaissement", "cash");
    expect(screen.getByRole("alert")).toHaveTextContent("Stock insuffisant");
    expect(screen.getByRole("button", { name: "Enregistrer" })).toBeDisabled();
    set("Quantité 1", "2");
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Connexion interrompue");
    const input = vi.mocked(api.createSale).mock.calls[0][0];
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(vi.mocked(api.createSale).mock.calls[1][0]).toEqual(input);
  });

  it("désactive le formulaire pendant l’enregistrement et ignore un deuxième envoi", async () => {
    let finish!: (op: ProductOperation) => void;
    vi.mocked(api.createSale).mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
    await page();
    fireEvent.click(screen.getByRole("button", { name: "Nouvelle vente" }));
    await screen.findByRole("option", { name: "Espèces" });
    set("Produit 1", "cable"); set("Compte d’encaissement", "cash");
    const form = screen.getByRole("button", { name: "Enregistrer" }).closest("form")!;
    fireEvent.submit(form); fireEvent.submit(form);
    expect(api.createSale).toHaveBeenCalledOnce();
    expect(screen.getByLabelText("Quantité 1")).toBeDisabled();
    finish(operation);
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("enregistre un achat payé puis une correction de comptage séparée", async () => {
    vi.mocked(api.receiveStock).mockResolvedValue({ ...operation, kind: "receipt" });
    vi.mocked(api.adjustStock).mockResolvedValue(products[0]);
    const { onChanged } = await page();
    fireEvent.click(screen.getByRole("button", { name: "Réapprovisionner Câble USB" }));
    await screen.findByRole("option", { name: "Espèces" });
    set("Quantité reçue", "5"); set("Montant total payé", "6000"); set("Compte de paiement", "cash");
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.receiveStock).toHaveBeenCalledWith(expect.objectContaining({ productId: "cable", quantity: 5, amount: 6000, accountId: "cash" }));
    fireEvent.click(screen.getByRole("button", { name: "Compter Câble USB" }));
    set(/Quantité réellement comptée/, "8"); set("Motif obligatoire", "Deux articles abîmés");
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledTimes(2));
    expect(api.adjustStock).toHaveBeenCalledWith(expect.objectContaining({ productId: "cable", quantity: 8, reason: "Deux articles abîmés" }));
  });

  it("conserve les noms historiques et propose une annulation complète avec motif", async () => {
    vi.mocked(api.productOperations).mockResolvedValue([operation]);
    vi.mocked(api.cancelProductOperation).mockResolvedValue({ ...operation, cancelledAt: "2026-09-07T11:00:00Z" });
    const { onChanged } = await page();
    fireEvent.click(screen.getByRole("tab", { name: "Ventes et achats" }));
    fireEvent.click(screen.getByRole("button", { name: "Détails" }));
    expect(within(screen.getByRole("dialog")).getByText("Ancien nom conservé")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Annuler cette opération" }));
    set("Motif obligatoire", "Saisie en double");
    fireEvent.click(screen.getByRole("button", { name: "Confirmer l’annulation" }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.cancelProductOperation).toHaveBeenCalledWith(expect.objectContaining({ operationId: "sale", reason: "Saisie en double" }));
  });

  it("annonce la correction du stock lorsqu’une vente est annulée depuis le journal", async () => {
    vi.mocked(api.journal).mockResolvedValue([{ id: "j", accountSnapshot: account, entryType: "sale", amount: 4000, signedAmount: 4000, paymentAccount: "cash", occurredAt: "2026-09-07", postedAt: "2026-09-07T10:00:00Z", reversed: false, productOperation: { id: "sale", kind: "sale" } }]);
    render(<JournalPage notify={vi.fn()} onChanged={vi.fn()} />);
    await screen.findByText("Vente de produits");
    fireEvent.click(screen.getByTitle("Créer une contre-écriture"));
    expect(screen.getByText(/Cette annulation corrige le stock et le budget/)).toBeInTheDocument();
  });
});
