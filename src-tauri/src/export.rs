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

fn export_csv(destination: &Path, report: &ReportData) -> AppResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b';')
        .from_path(destination)?;
    writer.write_record([
        "type",
        "date",
        "libelle",
        "compte_service",
        "montant_fcfa",
        "solde_ou_ecart_fcfa",
        "statut_note",
    ])?;
    for inventory in &report.inventories {
        writer.write_record([
            "inventaire",
            &inventory.closed_at,
            if inventory.kind == "opening" {
                "Ouverture"
            } else {
                "Inventaire"
            },
            "tous",
            &inventory.actual_total.to_string(),
            &inventory.variance.to_string(),
            inventory.variance_note.as_deref().unwrap_or(""),
        ])?;
    }
    for entry in &report.journal {
        writer.write_record([
            "journal",
            &entry.occurred_at,
            &entry.entry_type,
            &entry.payment_account,
            &entry.signed_amount.to_string(),
            "",
            entry.note.as_deref().unwrap_or(""),
        ])?;
    }
    for debt in &report.debts {
        writer.write_record([
            "dette",
            &debt.issued_at,
            &debt.customer_name,
            &debt.provider,
            &debt.principal.to_string(),
            &debt.remaining.to_string(),
            &debt.status,
        ])?;
        for payment in &debt.payments {
            writer.write_record([
                "remboursement",
                &payment.paid_at,
                &debt.customer_name,
                &payment.account,
                &payment.amount.to_string(),
                "",
                payment.note.as_deref().unwrap_or(""),
            ])?;
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
                .write_string(row, 2, &item.payment_account)
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
        }
        sheet.set_column_width(0, 14).map_err(xlsx_err)?;
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
            "Service",
            "Principal",
            "Reste",
            "Échéance",
            "Statut",
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
                .write_string(row, 3, &debt.provider)
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
        }
        for column in 0..=7 {
            sheet
                .set_column_width(column, if column == 1 { 28 } else { 18 })
                .map_err(xlsx_err)?;
        }
    }

    workbook
        .save(destination)
        .map_err(|e| AppError::Export(e.to_string()))?;
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
            item.payment_account,
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
            debt.provider,
            format_money(debt.remaining),
            debt.status
        ));
    }

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

    fn empty_report() -> ReportData {
        ReportData {
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
