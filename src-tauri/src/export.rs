use std::{fs, path::Path};

use printpdf::{
    ops::PdfFontHandle, BuiltinFont, Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Point, Pt,
    TextItem,
};
use rust_xlsxwriter::{Color as XlsxColor, Format, Workbook};

use crate::{
    error::{AppError, AppResult},
    models::{ExportInput, Money, ReportData},
};

fn ensure_destination(path: &Path) -> AppResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Export("Chemin d’export invalide.".into()))?;
    fs::create_dir_all(parent)?;
    Ok(())
}

fn format_money(value: Money) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let mut output = String::new();
    for (index, character) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            output.push(' ');
        }
        output.push(character);
    }
    let formatted: String = output.chars().rev().collect();
    format!("{}{} FCFA", if negative { "-" } else { "" }, formatted)
}

pub fn export_report(input: &ExportInput, report: &ReportData) -> AppResult<String> {
    let destination = Path::new(&input.destination);
    ensure_destination(destination)?;
    match input.format.as_str() {
        "pdf" => export_pdf(destination, report)?,
        "xlsx" => export_xlsx(destination, report)?,
        "csv" => export_csv(destination, report)?,
        _ => return Err(AppError::Validation("Format d’export non reconnu.".into())),
    }
    Ok(destination.to_string_lossy().to_string())
}

const LEGACY_CUSTODY: &str = "Dépôts clients non suivis à cette date";
fn custody_label(kind: &str) -> &str {
    match kind {
        "deposit" => "Dépôt reçu",
        "withdrawal" => "Restitution",
        "opening" => "Reprise antérieure",
        "reversal" => "Annulation",
        other => other,
    }
}
fn csv_row<const N: usize>(writer: &mut csv::Writer<fs::File>, values: [&str; N]) -> AppResult<()> {
    let mut row = values.to_vec();
    row.resize(25, "");
    writer.write_record(row)?;
    Ok(())
}

fn export_csv(destination: &Path, report: &ReportData) -> AppResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b';')
        .from_path(destination)?;
    csv_row(
        &mut writer,
        [
            "type",
            "date",
            "libelle",
            "compte_service",
            "compte_id",
            "compte_nom",
            "compte_identifiant",
            "montant_fcfa",
            "solde_ou_ecart_fcfa",
            "statut_note",
            "client_id",
            "client_telephone",
            "mouvement_id",
            "sequence",
            "enregistre_le",
            "auteur",
            "annule_original",
            "reclassement_capital_fcfa",
            "solde_depart_fcfa",
            "augmentations_fcfa",
            "diminutions_fcfa",
            "solde_fin_fcfa",
            "inventaire_id",
            "sequence_cloture",
            "nature_mouvement",
        ],
    )?;
    for inventory in &report.inventories {
        csv_row(
            &mut writer,
            [
                "inventaire",
                &inventory.closed_at,
                if inventory.kind == "opening" {
                    "Ouverture"
                } else {
                    "Inventaire"
                },
                "tous",
                "",
                "",
                "",
                &inventory.actual_total.to_string(),
                &inventory.variance.to_string(),
                inventory.variance_note.as_deref().unwrap_or(""),
            ],
        )?;
    }
    for inventory in &report.inventories {
        for detail in &inventory.account_balances {
            csv_row(
                &mut writer,
                [
                    "solde_compte",
                    &inventory.closed_at,
                    "Détail inventaire (hors synthèse)",
                    &detail.account.provider,
                    &detail.account.account_id,
                    &detail.account.name,
                    detail.account.identifier.as_deref().unwrap_or(""),
                    &detail.amount.to_string(),
                    &detail.delta.map(|v| v.to_string()).unwrap_or_default(),
                    if detail.legacy {
                        "Solde historique regroupé"
                    } else {
                        ""
                    },
                ],
            )?;
        }
    }
    for entry in &report.journal {
        csv_row(
            &mut writer,
            [
                "journal",
                &entry.occurred_at,
                &entry.entry_type,
                &entry.payment_account,
                &entry.account_snapshot.account_id,
                &entry.account_snapshot.name,
                entry.account_snapshot.identifier.as_deref().unwrap_or(""),
                &entry.signed_amount.to_string(),
                "",
                entry.note.as_deref().unwrap_or(""),
            ],
        )?;
    }
    for debt in &report.debts {
        csv_row(
            &mut writer,
            [
                "dette",
                &debt.issued_at,
                &debt.customer_name,
                &debt.provider,
                &debt.account_snapshot.account_id,
                &debt.account_snapshot.name,
                debt.account_snapshot.identifier.as_deref().unwrap_or(""),
                &debt.principal.to_string(),
                &debt.remaining.to_string(),
                &debt.status,
            ],
        )?;
        for payment in &debt.payments {
            csv_row(
                &mut writer,
                [
                    "remboursement",
                    &payment.paid_at,
                    &debt.customer_name,
                    &payment.account,
                    &payment.account_snapshot.account_id,
                    &payment.account_snapshot.name,
                    payment.account_snapshot.identifier.as_deref().unwrap_or(""),
                    &payment.amount.to_string(),
                    "",
                    payment.note.as_deref().unwrap_or(""),
                ],
            )?;
        }
    }
    for b in &report.custody.balances {
        csv_row(
            &mut writer,
            [
                "depot_solde_periode",
                report.filters.to.as_deref().unwrap_or(""),
                &b.customer_name,
                "",
                "",
                "",
                "",
                "",
                "",
                "Selon les dates déclarées; reprises et annulations comprises",
                &b.customer_id,
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                &b.opening_balance.to_string(),
                &b.increases.to_string(),
                &b.decreases.to_string(),
                &b.closing_balance.to_string(),
            ],
        )?;
    }
    for m in &report.custody.movements {
        let account = m.account_snapshot.as_ref();
        csv_row(
            &mut writer,
            [
                "depot_mouvement",
                &m.occurred_at,
                &m.customer_name,
                account.map(|a| a.provider.as_str()).unwrap_or(""),
                account.map(|a| a.account_id.as_str()).unwrap_or(""),
                account.map(|a| a.name.as_str()).unwrap_or(""),
                account.and_then(|a| a.identifier.as_deref()).unwrap_or(""),
                &m.delta.to_string(),
                &m.balance_after.to_string(),
                &format!(
                    "{}{}",
                    if m.reversed { "Annulé. " } else { "" },
                    m.note.as_deref().unwrap_or("")
                ),
                &m.customer_id,
                m.customer_phone.as_deref().unwrap_or(""),
                &m.id,
                &m.sequence.to_string(),
                &m.posted_at,
                &m.operator,
                m.reverses_id.as_deref().unwrap_or(""),
                &m.capital_adjustment.to_string(),
                "",
                "",
                "",
                "",
                "",
                "",
                custody_label(&m.kind),
            ],
        )?;
    }
    for i in &report.inventories {
        csv_row(
            &mut writer,
            [
                "depot_total_cloture",
                &i.closed_at,
                "Dépôts à restituer",
                "",
                "",
                "",
                "",
                &i.custody_total.map(|v| v.to_string()).unwrap_or_default(),
                "",
                if i.custody_total.is_none() {
                    LEGACY_CUSTODY
                } else {
                    "Montant figé"
                },
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                "",
                &i.id,
                &i.custody_sequence.to_string(),
            ],
        )?;
        for b in &i.custody_balances {
            csv_row(
                &mut writer,
                [
                    "depot_client_cloture",
                    &i.closed_at,
                    &b.customer_name,
                    "",
                    "",
                    "",
                    "",
                    &b.balance.to_string(),
                    "",
                    "Montant figé",
                    &b.customer_id,
                    b.customer_phone.as_deref().unwrap_or(""),
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    "",
                    &i.id,
                    &i.custody_sequence.to_string(),
                ],
            )?;
        }
    }
    writer.flush()?;
    Ok(())
}

fn export_xlsx(destination: &Path, report: &ReportData) -> AppResult<()> {
    let mut workbook = Workbook::new();
    let header = Format::new()
        .set_bold()
        .set_font_color(XlsxColor::White)
        .set_background_color(XlsxColor::RGB(0x0F3D32));
    let money = Format::new().set_num_format("# ##0 \"FCFA\"");

    {
        let sheet = workbook.add_worksheet();
        sheet
            .set_name("Inventaires")
            .map_err(|e| AppError::Export(e.to_string()))?;
        let titles = [
            "Date",
            "Orange Money",
            "Wave",
            "Djamo",
            "Espèces",
            "Créances",
            "Capital attendu",
            "Capital réel",
            "Écart",
            "Justification",
            "Dépôts à restituer",
        ];
        for (column, title) in titles.iter().enumerate() {
            sheet
                .write_string_with_format(0, column as u16, *title, &header)
                .map_err(|e| AppError::Export(e.to_string()))?;
        }
        for (index, item) in report.inventories.iter().enumerate() {
            let row = (index + 1) as u32;
            sheet
                .write_string(row, 0, &item.closed_at)
                .map_err(xlsx_err)?;
            let values = [
                item.balances.orange_money,
                item.balances.wave,
                item.balances.djamo,
                item.balances.cash,
                item.receivables,
                item.expected_total,
                item.actual_total,
                item.variance,
            ];
            for (offset, value) in values.iter().enumerate() {
                sheet
                    .write_number_with_format(row, (offset + 1) as u16, *value as f64, &money)
                    .map_err(xlsx_err)?;
            }
            sheet
                .write_string(row, 9, item.variance_note.as_deref().unwrap_or(""))
                .map_err(xlsx_err)?;
        }
        for (index, item) in report.inventories.iter().enumerate() {
            let row = (index + 1) as u32;
            if let Some(amount) = item.custody_total {
                sheet
                    .write_number_with_format(row, 10, amount as f64, &money)
                    .map_err(xlsx_err)?;
            } else {
                sheet
                    .write_string(row, 10, LEGACY_CUSTODY)
                    .map_err(xlsx_err)?;
            }
        }
        sheet.set_column_width(10, 45).map_err(xlsx_err)?;
        sheet.set_column_width(0, 22).map_err(xlsx_err)?;
        sheet.set_column_width(9, 38).map_err(xlsx_err)?;
        for column in 1..=8 {
            sheet.set_column_width(column, 18).map_err(xlsx_err)?;
        }
    }

    {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Journal").map_err(xlsx_err)?;
        let titles = [
            "Date",
            "Type",
            "Compte",
            "Montant",
            "Référence",
            "Note",
            "Correction",
            "Compte ID",
        ];
        for (column, title) in titles.iter().enumerate() {
            sheet
                .write_string_with_format(0, column as u16, *title, &header)
                .map_err(xlsx_err)?;
        }
        for (index, item) in report.journal.iter().enumerate() {
            let row = (index + 1) as u32;
            sheet
                .write_string(row, 0, &item.occurred_at)
                .map_err(xlsx_err)?;
            sheet
                .write_string(row, 1, &item.entry_type)
                .map_err(xlsx_err)?;
            sheet
                .write_string(row, 2, item.account_snapshot.display_name())
                .map_err(xlsx_err)?;
            sheet
                .write_number_with_format(row, 3, item.signed_amount as f64, &money)
                .map_err(xlsx_err)?;
            sheet
                .write_string(row, 4, item.reference.as_deref().unwrap_or(""))
                .map_err(xlsx_err)?;
            sheet
                .write_string(row, 5, item.note.as_deref().unwrap_or(""))
                .map_err(xlsx_err)?;
            sheet
                .write_string(row, 6, item.reverses_id.as_deref().unwrap_or(""))
                .map_err(xlsx_err)?;
            sheet
                .write_string(row, 7, &item.account_snapshot.account_id)
                .map_err(xlsx_err)?;
        }
        sheet.set_column_width(0, 14).map_err(xlsx_err)?;
        sheet.set_column_width(2, 48).map_err(xlsx_err)?;
        sheet.set_column_width(3, 20).map_err(xlsx_err)?;
        sheet.set_column_width(7, 38).map_err(xlsx_err)?;
        sheet.set_column_width(4, 24).map_err(xlsx_err)?;
        sheet.set_column_width(5, 38).map_err(xlsx_err)?;
    }

    {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Dettes").map_err(xlsx_err)?;
        let titles = [
            "Date",
            "Client",
            "Téléphone",
            "Compte",
            "Principal",
            "Reste",
            "Échéance",
            "Statut",
            "Compte ID",
        ];
        for (column, title) in titles.iter().enumerate() {
            sheet
                .write_string_with_format(0, column as u16, *title, &header)
                .map_err(xlsx_err)?;
        }
        for (index, debt) in report.debts.iter().enumerate() {
            let row = (index + 1) as u32;
            sheet
                .write_string(row, 0, &debt.issued_at)
                .map_err(xlsx_err)?;
            sheet
                .write_string(row, 1, &debt.customer_name)
                .map_err(xlsx_err)?;
            sheet.write_string(row, 2, &debt.phone).map_err(xlsx_err)?;
            sheet
                .write_string(row, 3, debt.account_snapshot.display_name())
                .map_err(xlsx_err)?;
            sheet
                .write_number_with_format(row, 4, debt.principal as f64, &money)
                .map_err(xlsx_err)?;
            sheet
                .write_number_with_format(row, 5, debt.remaining as f64, &money)
                .map_err(xlsx_err)?;
            sheet
                .write_string(row, 6, debt.due_date.as_deref().unwrap_or(""))
                .map_err(xlsx_err)?;
            sheet.write_string(row, 7, &debt.status).map_err(xlsx_err)?;
            sheet
                .write_string(row, 8, &debt.account_snapshot.account_id)
                .map_err(xlsx_err)?;
        }
        sheet.set_column_width(8, 38).map_err(xlsx_err)?;
        for column in 0..=7 {
            sheet
                .set_column_width(
                    column,
                    if column == 3 {
                        48
                    } else if column == 1 {
                        28
                    } else {
                        18
                    },
                )
                .map_err(xlsx_err)?;
        }
    }

    {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Soldes par compte").map_err(xlsx_err)?;
        for (column, title) in [
            "Date",
            "Compte ID",
            "Service",
            "Compte",
            "Identifiant",
            "Solde FCFA",
            "Variation FCFA",
            "Origine",
        ]
        .iter()
        .enumerate()
        {
            sheet
                .write_string_with_format(0, column as u16, *title, &header)
                .map_err(xlsx_err)?;
            sheet
                .set_column_width(column as u16, 24)
                .map_err(xlsx_err)?;
        }
        let mut row = 1;
        for inventory in &report.inventories {
            for d in &inventory.account_balances {
                for (column, value) in [
                    &inventory.closed_at,
                    &d.account.account_id,
                    &d.account.provider,
                    &d.account.name,
                    d.account.identifier.as_deref().unwrap_or(""),
                ]
                .iter()
                .enumerate()
                {
                    sheet
                        .write_string(row, column as u16, *value)
                        .map_err(xlsx_err)?;
                }
                sheet
                    .write_number_with_format(row, 5, d.amount as f64, &money)
                    .map_err(xlsx_err)?;
                if let Some(delta) = d.delta {
                    sheet
                        .write_number_with_format(row, 6, delta as f64, &money)
                        .map_err(xlsx_err)?;
                }
                sheet
                    .write_string(
                        row,
                        7,
                        if d.legacy {
                            "Historique regroupé"
                        } else {
                            "Relevé par compte"
                        },
                    )
                    .map_err(xlsx_err)?;
                row += 1;
            }
        }
    }
    {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Remboursements").map_err(xlsx_err)?;
        for (column, title) in [
            "Date",
            "Dette ID",
            "Client",
            "Compte ID",
            "Compte",
            "Montant FCFA",
            "Note",
        ]
        .iter()
        .enumerate()
        {
            sheet
                .write_string_with_format(0, column as u16, *title, &header)
                .map_err(xlsx_err)?;
            sheet
                .set_column_width(column as u16, 26)
                .map_err(xlsx_err)?;
        }
        let mut row = 1;
        for debt in &report.debts {
            for payment in &debt.payments {
                for (column, value) in [
                    &payment.paid_at,
                    &debt.id,
                    &debt.customer_name,
                    &payment.account_snapshot.account_id,
                    &payment.account_snapshot.display_name(),
                ]
                .iter()
                .enumerate()
                {
                    sheet
                        .write_string(row, column as u16, *value)
                        .map_err(xlsx_err)?;
                }
                sheet
                    .write_number_with_format(row, 5, payment.amount as f64, &money)
                    .map_err(xlsx_err)?;
                sheet
                    .write_string(row, 6, payment.note.as_deref().unwrap_or(""))
                    .map_err(xlsx_err)?;
                row += 1;
            }
        }
    }
    custody_sheets(&mut workbook, report, &header, &money)?;
    workbook
        .save(destination)
        .map_err(|e| AppError::Export(e.to_string()))?;
    Ok(())
}

fn custody_sheets(
    workbook: &mut Workbook,
    report: &ReportData,
    header: &Format,
    money: &Format,
) -> AppResult<()> {
    {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Dépôts clients").map_err(xlsx_err)?;
        for (col, title) in [
            "Client ID",
            "Client",
            "Solde de départ",
            "Augmentations",
            "Diminutions",
            "Solde de fin",
            "Base de calcul",
        ]
        .iter()
        .enumerate()
        {
            sheet
                .write_string_with_format(0, col as u16, *title, header)
                .map_err(xlsx_err)?;
            sheet.set_column_width(col as u16, 28).map_err(xlsx_err)?;
        }
        for (idx, b) in report.custody.balances.iter().enumerate() {
            let row = (idx + 1) as u32;
            sheet
                .write_string(row, 0, &b.customer_id)
                .map_err(xlsx_err)?;
            sheet
                .write_string(row, 1, &b.customer_name)
                .map_err(xlsx_err)?;
            for (col, value) in [
                b.opening_balance,
                b.increases,
                b.decreases,
                b.closing_balance,
            ]
            .iter()
            .enumerate()
            {
                sheet
                    .write_number_with_format(row, (col + 2) as u16, *value as f64, money)
                    .map_err(xlsx_err)?;
            }
            sheet
                .write_string(row, 6, "Dates déclarées; reprises et annulations comprises")
                .map_err(xlsx_err)?;
        }
    }
    {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Registre dépôts").map_err(xlsx_err)?;
        for (col, title) in [
            "Séquence",
            "Mouvement ID",
            "Date déclarée",
            "Enregistré le",
            "Client ID",
            "Client à cette date",
            "Téléphone",
            "Nature",
            "Compte ID",
            "Compte à cette date",
            "Variation dépôt",
            "Solde après",
            "Reclassement capital",
            "Auteur",
            "Note",
            "Annule le mouvement",
            "État",
        ]
        .iter()
        .enumerate()
        {
            sheet
                .write_string_with_format(0, col as u16, *title, header)
                .map_err(xlsx_err)?;
            sheet.set_column_width(col as u16, 26).map_err(xlsx_err)?;
        }
        for (idx, m) in report.custody.movements.iter().enumerate() {
            let row = (idx + 1) as u32;
            sheet
                .write_number(row, 0, m.sequence as f64)
                .map_err(xlsx_err)?;
            for (col, value) in [
                (1, m.id.clone()),
                (2, m.occurred_at.clone()),
                (3, m.posted_at.clone()),
                (4, m.customer_id.clone()),
                (5, m.customer_name.clone()),
                (6, m.customer_phone.clone().unwrap_or_default()),
                (7, custody_label(&m.kind).into()),
                (
                    8,
                    m.account_snapshot
                        .as_ref()
                        .map(|a| a.account_id.clone())
                        .unwrap_or_default(),
                ),
                (
                    9,
                    m.account_snapshot
                        .as_ref()
                        .map(|a| a.display_name())
                        .unwrap_or_else(|| "Sans mouvement d’argent".into()),
                ),
                (13, m.operator.clone()),
                (14, m.note.clone().unwrap_or_default()),
                (15, m.reverses_id.clone().unwrap_or_default()),
                (
                    16,
                    if m.reversed {
                        "Annulé".into()
                    } else {
                        "Enregistré".into()
                    },
                ),
            ] {
                sheet.write_string(row, col, value).map_err(xlsx_err)?;
            }
            for (col, value) in [
                (10, m.delta),
                (11, m.balance_after),
                (12, m.capital_adjustment),
            ] {
                sheet
                    .write_number_with_format(row, col, value as f64, money)
                    .map_err(xlsx_err)?;
            }
        }
    }
    {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Dépôts aux clôtures").map_err(xlsx_err)?;
        for (col, title) in [
            "Inventaire ID",
            "Date de clôture",
            "Séquence limite",
            "Type",
            "Client ID",
            "Client à la clôture",
            "Téléphone",
            "Dépôts à restituer",
            "Suivi",
        ]
        .iter()
        .enumerate()
        {
            sheet
                .write_string_with_format(0, col as u16, *title, header)
                .map_err(xlsx_err)?;
            sheet.set_column_width(col as u16, 28).map_err(xlsx_err)?;
        }
        let mut row = 1;
        for i in &report.inventories {
            for (col, value) in [
                (0, i.id.as_str()),
                (1, i.closed_at.as_str()),
                (3, "Total"),
                (
                    8,
                    if i.custody_total.is_some() {
                        "Montant figé"
                    } else {
                        LEGACY_CUSTODY
                    },
                ),
            ] {
                sheet.write_string(row, col, value).map_err(xlsx_err)?;
            }
            sheet
                .write_number(row, 2, i.custody_sequence as f64)
                .map_err(xlsx_err)?;
            if let Some(amount) = i.custody_total {
                sheet
                    .write_number_with_format(row, 7, amount as f64, money)
                    .map_err(xlsx_err)?;
            }
            row += 1;
            for b in &i.custody_balances {
                for (col, value) in [
                    (0, i.id.as_str()),
                    (1, i.closed_at.as_str()),
                    (3, "Détail hors total"),
                    (4, b.customer_id.as_str()),
                    (5, b.customer_name.as_str()),
                    (6, b.customer_phone.as_deref().unwrap_or("")),
                ] {
                    sheet.write_string(row, col, value).map_err(xlsx_err)?;
                }
                sheet
                    .write_number_with_format(row, 7, b.balance as f64, money)
                    .map_err(xlsx_err)?;
                row += 1;
            }
        }
    }
    Ok(())
}

fn xlsx_err(error: rust_xlsxwriter::XlsxError) -> AppError {
    AppError::Export(error.to_string())
}

fn export_pdf(destination: &Path, report: &ReportData) -> AppResult<()> {
    let mut lines = vec![
        "RAPPORT KËR FINANCE".to_string(),
        format!("Généré le {}", report.generated_at),
        format!(
            "Période: {} au {}",
            report.filters.from.as_deref().unwrap_or("début"),
            report.filters.to.as_deref().unwrap_or("aujourd’hui")
        ),
        String::new(),
        format!(
            "Recettes et apports: {}",
            format_money(report.total_positive)
        ),
        format!(
            "Achats, dépenses et retraits: {}",
            format_money(report.total_negative)
        ),
        format!("Écarts cumulés: {}", format_money(report.total_variance)),
        format!(
            "Créances en cours: {}",
            format_money(report.outstanding_receivables)
        ),
        String::new(),
        "INVENTAIRES".to_string(),
    ];
    for item in &report.inventories {
        lines.push(format!(
            "{} | Réel {} | Attendu {} | Écart {}",
            item.closed_at.get(..16).unwrap_or(&item.closed_at),
            format_money(item.actual_total),
            format_money(item.expected_total),
            format_money(item.variance)
        ));
        lines.push(format!(
            "  Liquidités {} | Créances {}",
            format_money(item.liquidity),
            format_money(item.receivables)
        ));
        lines.push(
            item.custody_total
                .map(|v| {
                    format!(
                        "  Dépôts à restituer {} | Séquence {}",
                        format_money(v),
                        item.custody_sequence
                    )
                })
                .unwrap_or_else(|| LEGACY_CUSTODY.into()),
        );
        for b in &item.custody_balances {
            lines.push(format!(
                "  {} ({}) | {} | {}",
                b.customer_name,
                b.customer_id,
                b.customer_phone.as_deref().unwrap_or(""),
                format_money(b.balance)
            ));
        }
        for d in &item.account_balances {
            lines.push(format!(
                "  {} | Solde {} | Variation {}{}",
                d.account.display_name(),
                format_money(d.amount),
                d.delta.map(format_money).unwrap_or_else(|| "—".into()),
                if d.legacy {
                    " | Historique regroupé"
                } else {
                    ""
                }
            ));
        }
        if let Some(note) = &item.variance_note {
            lines.push(format!("  Motif: {note}"));
        }
    }
    lines.push(String::new());
    lines.push("JOURNAL".to_string());
    for item in &report.journal {
        lines.push(format!(
            "{} | {} | {} | {}",
            item.occurred_at,
            item.entry_type,
            item.account_snapshot.display_name(),
            format_money(item.signed_amount)
        ));
    }
    lines.push(String::new());
    lines.push("DETTES CLIENTS".to_string());
    for debt in &report.debts {
        lines.push(format!(
            "{} | {} ({}) | {} | Reste {} | {}",
            debt.issued_at,
            debt.customer_name,
            debt.phone,
            debt.account_snapshot.display_name(),
            format_money(debt.remaining),
            debt.status
        ));
        for payment in &debt.payments {
            lines.push(format!(
                "  Reçu le {} | {} | {}",
                payment.paid_at,
                payment.account_snapshot.display_name(),
                format_money(payment.amount)
            ));
        }
    }

    lines.push(String::new());
    lines.push("DÉPÔTS CLIENTS - HORS RECETTES ET DÉPENSES".into());
    lines.push("Dates déclarées; reprises et annulations comprises.".into());
    lines.push("Les mouvements antidatés peuvent modifier les soldes de période.".into());
    lines.push(format!(
        "Solde de départ {} | Solde de fin {}",
        format_money(report.custody.opening_balance),
        format_money(report.custody.closing_balance)
    ));
    for b in &report.custody.balances {
        lines.push(format!("{} | {}", b.customer_name, b.customer_id));
        lines.push(format!(
            "  Départ {} | + {} | - {} | Fin {}",
            format_money(b.opening_balance),
            format_money(b.increases),
            format_money(b.decreases),
            format_money(b.closing_balance)
        ));
    }
    for m in &report.custody.movements {
        lines.push(format!(
            "N° {} | {} | {} | {}",
            m.sequence,
            m.occurred_at,
            m.customer_name,
            custody_label(&m.kind)
        ));
        lines.push(format!(
            "  {} | Variation {} | Solde après {}",
            m.account_snapshot
                .as_ref()
                .map(|a| a.display_name())
                .unwrap_or_else(|| "Sans mouvement d’argent".into()),
            format_money(m.delta),
            format_money(m.balance_after)
        ));
        lines.push(format!(
            "  Enregistré {} par {} | Capital {}",
            m.posted_at,
            m.operator,
            format_money(m.capital_adjustment)
        ));
        lines.push(format!("  ID {} | Client {}", m.id, m.customer_id));
        if let Some(phone) = &m.customer_phone {
            lines.push(format!("  Téléphone: {phone}"));
        }
        if let Some(id) = &m.reverses_id {
            lines.push(format!("  Annule: {id}"));
        }
        if m.reversed {
            lines.push("  Mouvement annulé".into());
        }
        if let Some(note) = &m.note {
            lines.push(format!("  Note: {note}"));
        }
    }

    let lines: Vec<String> = lines
        .into_iter()
        .flat_map(|line| {
            let mut wrapped = Vec::new();
            let mut current = String::new();
            for word in line.split_whitespace() {
                if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > 82 {
                    wrapped.push(std::mem::take(&mut current));
                }
                // Very long unbroken notes must also stay inside the page.
                for chunk in word.chars().collect::<Vec<_>>().chunks(82) {
                    if !current.is_empty() {
                        current.push(' ');
                    }
                    current.extend(chunk);
                    if current.chars().count() >= 82 {
                        wrapped.push(std::mem::take(&mut current));
                    }
                }
            }
            if !current.is_empty() || wrapped.is_empty() {
                wrapped.push(current);
            }
            wrapped
        })
        .collect();
    let mut document = PdfDocument::new("Rapport Kër Finance");
    let mut pages = Vec::new();
    for chunk in lines.chunks(40) {
        let mut operations = vec![
            Op::StartTextSection,
            Op::SetFont {
                font: PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                size: Pt(10.0),
            },
            Op::SetLineHeight { lh: Pt(17.0) },
            Op::SetTextCursor {
                pos: Point::new(Mm(18.0), Mm(278.0)),
            },
        ];
        for (index, line) in chunk.iter().enumerate() {
            if index > 0 {
                operations.push(Op::AddLineBreak);
            }
            operations.push(Op::ShowText {
                items: vec![TextItem::Text(line.clone())],
            });
        }
        operations.push(Op::EndTextSection);
        pages.push(PdfPage::new(Mm(210.0), Mm(297.0), operations));
    }
    document.with_pages(pages);
    let bytes = document.save(&PdfSaveOptions::default(), &mut Vec::new());
    fs::write(destination, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ReportFilters;
    use crate::{
        accounts,
        db::{self, test_support::*},
        models::UpdateAccountInput,
    };

    #[test]
    fn exports_match_rust_totals_and_historical_account_details() {
        use std::io::Read;
        let mut db = multi_database();
        let djamo = id(&db, "Djamo 1");
        let wave = id(&db, "Wave 2");
        journal(&mut db, &wave);
        let loan = debt(&mut db, &djamo);
        payment(&mut db, &loan.id, &wave, 20_000);
        accounts::update(
            &mut db,
            UpdateAccountInput {
                account_id: wave.clone(),
                name: "Nouveau nom".into(),
                identifier: None,
            },
        )
        .unwrap();
        let report = db::get_report(
            &db,
            ReportFilters {
                from: None,
                to: None,
            },
        )
        .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let csv_path = temp.path().join("report.csv");
        export_csv(&csv_path, &report).unwrap();
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_path(csv_path)
            .unwrap();
        let records = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
        let summaries: Vec<_> = records.iter().filter(|r| &r[0] == "inventaire").collect();
        let details: Vec<_> = records.iter().filter(|r| &r[0] == "solde_compte").collect();
        assert_eq!(summaries.len(), 1);
        assert_eq!(details.len(), 6);
        assert_eq!(
            &summaries[0][7],
            &report.inventories[0].actual_total.to_string()
        );
        assert_eq!(
            details
                .iter()
                .map(|r| r[7].parse::<i64>().unwrap())
                .sum::<i64>(),
            report.inventories[0].liquidity
        );
        for detail in &report.inventories[0].account_balances {
            let row = details
                .iter()
                .find(|r| r[4] == detail.account.account_id)
                .unwrap();
            assert_eq!(&row[5], detail.account.name);
            assert_eq!(&row[6], detail.account.identifier.as_deref().unwrap());
            assert_eq!(&row[7], detail.amount.to_string());
            assert_eq!(&row[8], ""); // No invented variation on the first reading.
        }
        for kind in ["journal", "remboursement"] {
            let row = records.iter().find(|r| &r[0] == kind).unwrap();
            assert_eq!(&row[4], wave);
            assert_eq!(&row[5], "Wave 2");
        }
        let row = records.iter().find(|r| &r[0] == "dette").unwrap();
        assert_eq!(&row[4], djamo);
        assert_eq!(&row[7], "50000");
        assert_eq!(&row[8], "30000");

        let xlsx_path = temp.path().join("report.xlsx");
        export_xlsx(&xlsx_path, &report).unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(xlsx_path).unwrap()).unwrap();
        fn xml(zip: &mut zip::ZipArchive<fs::File>, name: &str) -> String {
            let mut result = String::new();
            zip.by_name(name)
                .unwrap()
                .read_to_string(&mut result)
                .unwrap();
            result
        }
        let strings = xml(&mut zip, "xl/sharedStrings.xml");
        for detail in &report.inventories[0].account_balances {
            assert!(strings.contains(&detail.account.account_id));
            assert!(strings.contains(&detail.account.name));
            assert!(strings.contains(detail.account.identifier.as_deref().unwrap()));
        }
        assert!(!strings.contains("Nouveau nom"));
        let summary = xml(&mut zip, "xl/worksheets/sheet1.xml");
        assert_eq!(summary.matches("<row ").count(), 2); // one summary, not six account rows
        assert!(summary.contains("<v>5000000</v>"));
        let account_sheet = xml(&mut zip, "xl/worksheets/sheet4.xml");
        assert_eq!(account_sheet.matches("<row ").count(), 7);
        assert!(account_sheet.contains("<v>1000000</v>"));
        let payments = xml(&mut zip, "xl/worksheets/sheet5.xml");
        assert_eq!(payments.matches("<row ").count(), 2);
        assert!(payments.contains("<v>20000</v>"));

        let pdf_path = temp.path().join("report.pdf");
        export_pdf(&pdf_path, &report).unwrap();
        let pdf = PdfDocument::parse(
            &fs::read(pdf_path).unwrap(),
            &printpdf::PdfParseOptions::default(),
            &mut Vec::new(),
        )
        .unwrap();
        let text = pdf
            .extract_text()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        for name in ["Orange 1", "Orange 2", "Wave 1", "Wave 2", "Djamo 1"] {
            assert!(text.contains(name), "PDF omitted {name}: {text}");
        }
        for amount in [
            "5 000 000 FCFA",
            "1 000 000 FCFA",
            "20 000 FCFA",
            "30 000 FCFA",
        ] {
            assert!(text.contains(amount), "PDF omitted {amount}: {text}");
        }
        assert!(!text.contains("Nouveau nom"));
    }

    #[test]
    fn custody_exports_include_periods_ledger_and_frozen_closures_without_revenue() {
        use crate::{custody, models::*};
        use std::io::Read;
        let mut db = multi_database();
        let customer = custody::save_customer(
            &mut db,
            SaveCustodyCustomerInput {
                request_id: uuid::Uuid::new_v4().to_string(),
                customer_id: None,
                name: "Awa Fall".into(),
                phone: Some("771234567".into()),
                active: true,
            },
        )
        .unwrap();
        custody::opening(
            &mut db,
            CustodyOpeningInput {
                request_id: uuid::Uuid::new_v4().to_string(),
                lines: vec![CustodyOpeningLine {
                    customer_id: customer.id.clone(),
                    amount: 200_000,
                }],
            },
        )
        .unwrap();
        let cash = id(&db, "Espèces");
        let deposit = custody::record(
            &mut db,
            CreateCustodyMovementInput {
                request_id: uuid::Uuid::new_v4().to_string(),
                customer_id: customer.id.clone(),
                kind: "deposit".into(),
                amount: 50_000,
                account_id: cash.clone(),
                occurred_at: "2026-09-21".into(),
                note: Some("Garde gratuite".into()),
            },
        )
        .unwrap();
        let values = balances(&db);
        let closed = close(&mut db, values);
        custody::reverse(
            &mut db,
            ReverseCustodyMovementInput {
                request_id: uuid::Uuid::new_v4().to_string(),
                movement_id: deposit.id.clone(),
                reason: "Erreur de saisie".into(),
            },
        )
        .unwrap();
        custody::save_customer(
            &mut db,
            SaveCustodyCustomerInput {
                request_id: uuid::Uuid::new_v4().to_string(),
                customer_id: Some(customer.id.clone()),
                name: "Awa nouveau nom".into(),
                phone: None,
                active: true,
            },
        )
        .unwrap();
        let report = db::get_report(
            &db,
            ReportFilters {
                from: None,
                to: None,
            },
        )
        .unwrap();
        assert_eq!(report.total_positive, 0);
        assert_eq!(report.total_negative, 0);
        assert_eq!(report.custody.closing_balance, 200_000);
        let temp = tempfile::tempdir().unwrap();
        let csv_path = temp.path().join("deposits.csv");
        export_csv(&csv_path, &report).unwrap();
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .from_path(&csv_path)
            .unwrap();
        assert_eq!(reader.headers().unwrap().len(), 25);
        let records = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
        let period = records
            .iter()
            .find(|r| &r[0] == "depot_solde_periode")
            .unwrap();
        assert_eq!(&period[10], customer.id);
        assert_eq!(&period[18], "0");
        assert_eq!(&period[21], "200000");
        assert_eq!(
            records
                .iter()
                .filter(|r| &r[0] == "depot_mouvement")
                .count(),
            3
        );
        let frozen = records
            .iter()
            .find(|r| &r[0] == "depot_client_cloture" && r[22] == closed.id)
            .unwrap();
        assert_eq!(&frozen[7], "250000");
        assert_eq!(&frozen[2], "Awa Fall");
        let xlsx_path = temp.path().join("deposits.xlsx");
        export_xlsx(&xlsx_path, &report).unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(&xlsx_path).unwrap()).unwrap();
        let mut strings = String::new();
        zip.by_name("xl/sharedStrings.xml")
            .unwrap()
            .read_to_string(&mut strings)
            .unwrap();
        assert!(strings.contains("Awa Fall"));
        assert!(!strings.contains("Awa nouveau nom"));
        assert!(strings.contains(&deposit.id));
        assert!(strings.contains("Garde gratuite"));
        let mut ledger = String::new();
        zip.by_name("xl/worksheets/sheet7.xml")
            .unwrap()
            .read_to_string(&mut ledger)
            .unwrap();
        assert_eq!(ledger.matches("<row ").count(), 4);
        assert!(ledger.contains("<v>-50000</v>"));
        let pdf_path = temp.path().join("deposits.pdf");
        export_pdf(&pdf_path, &report).unwrap();
        let pdf = PdfDocument::parse(
            &fs::read(&pdf_path).unwrap(),
            &printpdf::PdfParseOptions::default(),
            &mut Vec::new(),
        )
        .unwrap();
        let text = pdf
            .extract_text()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        for expected in [
            "Awa Fall",
            "250 000 FCFA",
            "200 000 FCFA",
            "Garde gratuite",
            "Annulation",
        ] {
            assert!(text.contains(expected), "Missing {expected}: {text}");
        }
        assert!(!text.contains("Awa nouveau nom"));
        // Optional local QA artifacts contain synthetic data only.
        if let Ok(directory) = std::env::var("KER_FINANCE_TEST_EXPORT_DIR") {
            let dir = Path::new(&directory);
            fs::create_dir_all(dir).unwrap();
            for path in [&csv_path, &xlsx_path, &pdf_path] {
                fs::copy(path, dir.join(path.file_name().unwrap())).unwrap();
            }
        }
    }

    fn empty_report() -> ReportData {
        ReportData {
            custody: crate::models::CustodyReport {
                balances: vec![],
                movements: vec![],
                opening_balance: 0,
                closing_balance: 0,
            },
            generated_at: "2026-08-26T12:00:00Z".into(),
            filters: ReportFilters {
                from: None,
                to: None,
            },
            inventories: vec![],
            journal: vec![],
            debts: vec![],
            total_positive: 100_000,
            total_negative: -30_000,
            total_variance: 0,
            outstanding_receivables: 50_000,
        }
    }

    #[test]
    fn all_report_formats_are_written() {
        let temp = tempfile::tempdir().unwrap();
        let report = empty_report();
        for format in ["pdf", "xlsx", "csv"] {
            let destination = temp.path().join(format!("report.{format}"));
            let input = ExportInput {
                format: format.into(),
                destination: destination.to_string_lossy().to_string(),
                filters: report.filters.clone(),
            };
            export_report(&input, &report).unwrap();
            let bytes = fs::read(&destination).unwrap();
            assert!(bytes.len() > 20);
            if format == "pdf" {
                assert!(bytes.starts_with(b"%PDF"));
            }
            if format == "xlsx" {
                assert!(bytes.starts_with(b"PK"));
            }
        }
    }
}
