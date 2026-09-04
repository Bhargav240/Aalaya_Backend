//! A4 invoice renderer matching the approved Hamsa Vilas receipt template.
//! The layout is deliberately kept in one renderer so every generated receipt
//! has identical spacing, hierarchy, and pagination.
//!
//! Blocks flow from a running cursor rather than sitting at literal
//! coordinates: an address with six lines, or a terms list with five entries,
//! pushes what follows down instead of overprinting it. Horizontal placement
//! goes through the measured helpers in [`super::metrics`], so numeric columns
//! right-align on their own edge and centred lines are centred for their real
//! width, not for one sample string.
use super::metrics::text_width;
use pg_application::ports::storage::{ReceiptData, ReceiptRenderer, ReceiptTemplate};
use time::OffsetDateTime;

// Page frame. A4 is 595x842pt; content sits inside these margins.
const PAGE_W: f32 = 595.0;
const PAGE_H: f32 = 842.0;
const LEFT: f32 = 52.0;
const RIGHT: f32 = 543.0;
const CENTER: f32 = (LEFT + RIGHT) / 2.0;

// Item table columns. Text columns are left edges, numeric columns are right
// edges -- money must align on its last digit, not its first.
const COL_SNO: f32 = 60.0;
const COL_DESC: f32 = 92.0;
const COL_MODE: f32 = 270.0;
const COL_RATE_R: f32 = 415.0;
const COL_QTY_R: f32 = 452.0;
const COL_TOTAL_R: f32 = RIGHT;
// Totals sit in a right-hand block under the table.
const TOTALS_LABEL_X: f32 = 365.0;
/// Width available to the name/address column before the right-hand meta block
/// (INVOICE, Number, Date, Mode) starts.
const HEADER_COL_W: f32 = 270.0;

const INK: &str = "0.12 0.12 0.14";
const NAVY: &str = "0.07 0.16 0.31";
const BLUE: &str = "0.10 0.31 0.76";
const MUTED: &str = "0.40 0.45 0.55";
const GREEN: &str = "0.00 0.45 0.28";
const RULE: &str = "0.80 0.83 0.89";

pub struct SimpleReceiptRenderer;

impl ReceiptRenderer for SimpleReceiptRenderer {
    fn render(&self, template: &ReceiptTemplate, data: &ReceiptData) -> Vec<u8> {
        let mut p = PdfPage::new();

        let date = format_epoch(data.issued_epoch, "[day]/[month]/[year]");
        let issued = format_epoch(
            data.issued_epoch,
            "[day padding:zero] [month repr:short] [year], [hour]:[minute]:[second]",
        );
        let amount = format_amount(data.amount_paise);
        let amount_words = amount_in_words(data.amount_paise);

        // The full legal name goes under the badge; the short brand mark goes
        // in the badge and heads the Accommodation column. For a multi-branch
        // org the branch IS the short mark; a single-branch org falls back to
        // the display name so the badge is never empty.
        let legal_name = template.display_name.as_str();
        let brand = data.branch_name.as_deref().unwrap_or(legal_name);
        // Bill To names the unit alone; Accommodation qualifies it with the
        // branch, which is the only place the distinction carries information.
        let bill_unit = match &data.room_number {
            Some(room) => format!("Unit {room}"),
            None => "Current accommodation".to_string(),
        };
        let stay_unit = match (&data.room_number, &data.branch_name) {
            (Some(room), Some(branch)) => format!("{branch} \u{b7} Unit {room}"),
            (Some(room), None) => format!("Unit {room}"),
            _ => "Current accommodation".to_string(),
        };

        // ── Header: brand badge + legal name + address ──────────────────────
        let badge_fill = template
            .accent
            .as_deref()
            .and_then(hex_to_pdf_rgb)
            .unwrap_or_else(|| NAVY.to_string());
        // Letter-spaced like the template's wordmark, then sized to whatever
        // that actually measures rather than to a per-character guess.
        let badge_label = space_out(&brand.to_uppercase());
        let badge_size = fit_size(&badge_label, 13.0, 8.0, 286.0, true);
        let badge_w = text_width(&badge_label, badge_size, true) + 34.0;
        let badge_top = 800.0;
        let badge_h = 34.0;
        p.rect(
            LEFT,
            badge_top - badge_h,
            badge_w,
            badge_h,
            &badge_fill,
            true,
        );
        p.text(
            LEFT + 17.0,
            badge_top - badge_h + 12.0,
            badge_size,
            "1 1 1",
            true,
            &badge_label,
        );

        let mut y_left = badge_top - badge_h - 26.0;
        // Both header columns are bounded, so a long legal name wraps inside
        // its own column instead of running underneath "Date:" in the
        // right-hand meta block.
        for line in wrap_block(legal_name, HEADER_COL_W, 11.0, true) {
            p.text(LEFT, y_left, 11.0, NAVY, true, line);
            y_left -= 17.0;
        }
        y_left -= 5.0;
        // The operator's own line breaks are the address layout. Only lines too
        // wide for the column get wrapped; blank lines are dropped.
        let address = template
            .address
            .as_deref()
            .unwrap_or("Property address not configured");
        for line in wrap_block(address, HEADER_COL_W, 9.5, false) {
            p.text(LEFT, y_left, 9.5, INK, false, line);
            y_left -= 15.0;
        }

        // ── Header, right column: INVOICE + meta, right-aligned as a block ──
        let mut y_right = badge_top - 14.0;
        p.text_right(RIGHT, y_right, 25.0, NAVY, true, "INVOICE");
        y_right -= 26.0;
        for line in [
            format!("Number: {}", data.invoice_number),
            format!("Date: {date}"),
            format!("Mode: {}", data.payment_method),
        ] {
            p.text_right(RIGHT, y_right, 10.0, INK, false, line);
            y_right -= 20.0;
        }
        y_right -= 8.0;
        let paid_w = 58.0;
        let paid_h = 28.0;
        p.rect(
            RIGHT - paid_w,
            y_right - paid_h,
            paid_w,
            paid_h,
            GREEN,
            false,
        );
        p.text_center(
            RIGHT - paid_w / 2.0,
            y_right - paid_h + 10.0,
            9.0,
            GREEN,
            true,
            space_out("PAID"),
        );
        y_right -= paid_h;

        // Whichever header column runs longer sets where the body starts.
        let mut y = y_left.min(y_right) - 18.0;
        p.line(LEFT, y, RIGHT, y, RULE);

        // ── Bill To | Accommodation ─────────────────────────────────────────
        let parties_top = y - 23.0;
        let col2 = 288.0;
        let mut y_bill = parties_top;
        p.text(LEFT, y_bill, 11.0, BLUE, true, "Bill To");
        y_bill -= 25.0;
        p.text(LEFT, y_bill, 11.0, INK, true, &data.resident_name);
        y_bill -= 21.0;
        if let Some(phone) = &data.tenant_phone {
            p.text(LEFT, y_bill, 10.0, INK, false, phone);
            y_bill -= 21.0;
        }
        p.text(LEFT, y_bill, 10.0, MUTED, false, &bill_unit);
        y_bill -= 21.0;

        let mut y_stay = parties_top;
        p.text(col2, y_stay, 11.0, BLUE, true, "Accommodation");
        y_stay -= 25.0;
        p.text(col2, y_stay, 11.0, INK, true, brand.to_uppercase());
        y_stay -= 21.0;
        p.text(col2, y_stay, 10.0, INK, false, &stay_unit);
        y_stay -= 21.0;
        if let Some(floor) = data.floor_label.as_deref().filter(|f| !f.trim().is_empty()) {
            p.text(col2, y_stay, 10.0, MUTED, false, floor);
            y_stay -= 21.0;
        }
        p.text(
            col2,
            y_stay,
            10.0,
            MUTED,
            false,
            format!(
                "{} rent \u{b7} {} per month",
                data.plan_kind,
                rupees(data.amount_paise)
            ),
        );
        y_stay -= 30.0;
        let chip = space_out("CURRENT UNIT");
        let chip_w = text_width(&chip, 8.0, true) + 24.0;
        p.rect(col2, y_stay - 3.0, chip_w, 25.0, BLUE, false);
        p.text(col2 + 12.0, y_stay + 6.0, 8.0, BLUE, true, chip);
        y_stay -= 3.0;

        let parties_bottom = y_bill.min(y_stay);
        p.line(255.0, parties_top, 255.0, parties_bottom + 8.0, RULE);
        y = parties_bottom - 10.0;
        p.line(LEFT, y, RIGHT, y, RULE);

        // ── Item table ──────────────────────────────────────────────────────
        y -= 26.0;
        p.text(COL_SNO, y, 9.0, NAVY, true, "Sno.");
        p.text(COL_DESC, y, 9.0, NAVY, true, "Description");
        p.text(COL_MODE, y, 9.0, NAVY, true, "Payment Mode");
        p.text_right(COL_RATE_R, y, 9.0, NAVY, true, "Rate");
        p.text_right(COL_QTY_R, y, 9.0, NAVY, true, "Qty");
        p.text_right(COL_TOTAL_R, y, 9.0, NAVY, true, "Total");
        y -= 16.0;
        p.line(LEFT, y, RIGHT, y, NAVY);

        y -= 23.0;
        p.text(COL_SNO, y, 9.0, INK, false, "1");
        // The description shares its row with the Payment Mode column, so it
        // is clipped to the space actually available rather than running under
        // the next heading.
        let desc = format!(
            "{} \u{b7} {} \u{b7} {}",
            brand.to_uppercase(),
            stay_unit_short(data),
            data.line_item
        );
        p.text(
            COL_DESC,
            y,
            9.0,
            INK,
            false,
            ellipsize(&desc, COL_MODE - COL_DESC - 10.0, 9.0, false),
        );
        p.text(COL_MODE, y, 9.0, INK, false, &data.payment_method);
        p.text_right(COL_RATE_R, y, 9.0, INK, false, &amount);
        p.text_right(COL_QTY_R, y, 9.0, INK, false, "1");
        p.text_right(COL_TOTAL_R, y, 9.0, INK, false, &amount);
        y -= 17.0;
        p.text(
            COL_DESC,
            y,
            8.5,
            MUTED,
            false,
            // The provider's reference joins the invoice id on the same line
            // when there is one, so a resident can match the invoice against
            // their bank or UPI statement. Cash has no reference and gets none.
            match data.payment_reference.as_deref() {
                Some(reference) => {
                    format!(
                        "Invoice ID: {} \u{b7} Ref: {}",
                        data.invoice_number, reference
                    )
                }
                None => format!("Invoice ID: {}", data.invoice_number),
            },
        );

        // ── Totals ──────────────────────────────────────────────────────────
        y -= 18.0;
        p.line(LEFT, y, RIGHT, y, RULE);
        y -= 18.0;
        p.text(TOTALS_LABEL_X, y, 9.0, INK, false, "Taxable Amount");
        p.text_right(RIGHT, y, 9.0, INK, true, format!("INR {amount}"));
        y -= 12.0;
        p.line(TOTALS_LABEL_X, y, RIGHT, y, RULE);
        y -= 20.0;
        p.text(TOTALS_LABEL_X, y, 11.0, INK, true, "Total");
        p.text_right(RIGHT, y, 11.0, NAVY, true, format!("INR {amount}"));

        y -= 22.0;
        p.line(LEFT, y, RIGHT, y, RULE);
        y -= 22.0;
        p.text_center(
            CENTER,
            y,
            10.5,
            INK,
            true,
            format!("Total in Words: {amount_words}"),
        );

        // ── Terms & conditions ──────────────────────────────────────────────
        y -= 26.0;
        p.line(LEFT, y, RIGHT, y, RULE);
        y -= 22.0;
        p.text(LEFT, y, 10.0, NAVY, true, "Terms & Conditions");
        // One number per TERM, not per wrapped line. Numbering the output of
        // the wrapper turned a single sentence into "1. ...Queries: +91 90000
        // 10101 /" and "2. hello@hamsavilas.in". Operators separate terms with
        // newlines in the receipt template; a footer without any is one term.
        let footer = template
            .footer
            .as_deref()
            .unwrap_or("Rent is payable as agreed with the property management.");
        let terms: Vec<&str> = footer
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        y -= 22.0;
        for (index, term) in terms.iter().enumerate() {
            let number = format!("{}. ", index + 1);
            let indent = text_width(&number, 8.5, false);
            for (line_no, line) in wrap_block(term, RIGHT - LEFT - 4.0 - indent, 8.5, false)
                .into_iter()
                .enumerate()
            {
                if line_no == 0 {
                    p.text(LEFT + 4.0, y, 8.5, MUTED, false, format!("{number}{line}"));
                } else {
                    // Hanging indent, so continuations align under the text
                    // rather than under the number.
                    p.text(LEFT + 4.0 + indent, y, 8.5, MUTED, false, line);
                }
                y -= 14.0;
            }
        }

        // ── Footer ──────────────────────────────────────────────────────────
        y -= 8.0;
        p.line(LEFT, y, RIGHT, y, RULE);
        y -= 22.0;
        p.text_center(
            CENTER,
            y,
            10.0,
            INK,
            false,
            "Thank you for staying with us.",
        );
        y -= 22.0;
        p.text_center(
            CENTER,
            y,
            8.5,
            MUTED,
            false,
            format!("Recorded by {} \u{b7} Paid on {}", data.recorded_by, issued),
        );
        y -= 19.0;
        p.text_center(
            CENTER,
            y,
            8.5,
            MUTED,
            false,
            "This invoice is auto generated and doesn't require sign",
        );
        p.finish()
    }
}

/// The unit as it reads inside the description cell -- no branch prefix, since
/// the description already opens with the brand.
fn stay_unit_short(data: &ReceiptData) -> String {
    match &data.room_number {
        Some(room) => format!("Unit {room}"),
        None => "Stay".to_string(),
    }
}

fn format_epoch(epoch: i64, fmt: &str) -> String {
    OffsetDateTime::from_unix_timestamp(epoch)
        .ok()
        .and_then(|d| {
            d.format(&time::format_description::parse_owned::<1>(fmt).ok()?)
                .ok()
        })
        .unwrap_or_else(|| "\u{2014}".to_string())
}

/// Largest size in `[min, max]` at which `s` fits `limit`, so a long brand name
/// shrinks into its badge instead of overflowing it.
fn fit_size(s: &str, max: f32, min: f32, limit: f32, bold: bool) -> f32 {
    let mut size = max;
    while size > min && text_width(s, size, bold) > limit {
        size -= 0.5;
    }
    size
}

/// Wordmark letter-spacing: "HAMSA VILAS" -> "H A M S A   V I L A S".
fn space_out(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Truncates with an ellipsis when `s` will not fit `limit`.
fn ellipsize(s: &str, limit: f32, size: f32, bold: bool) -> String {
    if text_width(s, size, bold) <= limit {
        return s.to_string();
    }
    let mut out = String::new();
    for c in s.chars() {
        if text_width(&format!("{out}{c}..."), size, bold) > limit {
            break;
        }
        out.push(c);
    }
    out.push_str("...");
    out
}

struct PdfPage {
    content: String,
}
impl PdfPage {
    fn new() -> Self {
        Self {
            content: String::new(),
        }
    }
    fn text<T: AsRef<str>>(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        color: &str,
        bold: bool,
        value: T,
    ) {
        let s = sanitize(value.as_ref());
        if s.is_empty() {
            return;
        }
        self.content.push_str(&format!(
            "BT /{} {} Tf {} rg {} {} Td ({}) Tj ET\n",
            if bold { "F2" } else { "F1" },
            size,
            color,
            x,
            y,
            escape(&s)
        ));
    }
    /// Draws `value` so its right edge lands on `x_right`.
    fn text_right<T: AsRef<str>>(
        &mut self,
        x_right: f32,
        y: f32,
        size: f32,
        color: &str,
        bold: bool,
        value: T,
    ) {
        let s = sanitize(value.as_ref());
        self.text(
            x_right - text_width(&s, size, bold),
            y,
            size,
            color,
            bold,
            s,
        );
    }
    /// Draws `value` centred on `x_center`.
    fn text_center<T: AsRef<str>>(
        &mut self,
        x_center: f32,
        y: f32,
        size: f32,
        color: &str,
        bold: bool,
        value: T,
    ) {
        let s = sanitize(value.as_ref());
        self.text(
            x_center - text_width(&s, size, bold) / 2.0,
            y,
            size,
            color,
            bold,
            s,
        );
    }
    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: &str) {
        self.content.push_str(&format!(
            "{} RG 0.6 w {} {} m {} {} l S\n",
            color, x1, y1, x2, y2
        ));
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: &str, fill: bool) {
        // The colour needs its operator: `rg` sets the non-stroking (fill)
        // colour, `RG` the stroking one. Emitting the three components bare
        // left them as stray operands that `re` ignored, so every filled rect
        // came out black and every outlined one inherited whatever colour the
        // previous drawing call happened to leave set.
        self.content.push_str(&format!(
            "{} {} {} {} {} {} re {}\n",
            color,
            if fill { "rg" } else { "RG" },
            x,
            y,
            w,
            h,
            if fill { "f" } else { "S" }
        ));
    }
    fn finish(self) -> Vec<u8> {
        build_pdf(self.content.into_bytes())
    }
}

/// "#2E7D32" -> "0.180 0.490 0.196", the PDF operand form. Returns None for
/// anything that is not a 6-digit hex colour, so a malformed template value
/// falls back rather than producing an invalid content stream.
fn hex_to_pdf_rgb(hex: &str) -> Option<String> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = |i: usize| {
        u8::from_str_radix(&h[i..i + 2], 16)
            .ok()
            .map(|b| b as f32 / 255.0)
    };
    Some(format!("{:.3} {:.3} {:.3}", v(0)?, v(2)?, v(4)?))
}

/// Replaces characters the WinAnsi fonts cannot show with readable stand-ins,
/// BEFORE anything is measured.
///
/// Measurement and drawing must see identical text; if the rupee sign were
/// swapped for "INR" only at draw time, every right-aligned total would be
/// mispositioned by the width difference. The rupee sign is the notable case:
/// U+20B9 has no WinAnsi slot, so it becomes "INR " -- with the space, or the
/// template's "\u{20b9}100 per day" prints as "INR100 per day".
fn sanitize(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{20b9}' => {
                out.push_str("INR");
                // A separating space, but only where one is not already
                // present. Collapsing runs of spaces globally instead would
                // flatten the deliberate letter-spacing in the wordmark, so
                // "H A M S A  V I L A S" lost its word gap.
                if matches!(chars.peek(), Some(next) if !next.is_whitespace()) {
                    out.push(' ');
                }
            }
            '\u{a0}' => out.push(' '),
            c if c.is_ascii() => out.push(c),
            c if winansi_byte(c).is_some() => out.push(c),
            // Unrepresentable and with no sensible stand-in: drop it rather
            // than print "?" on a financial document.
            _ => {}
        }
    }
    out
}

/// Escapes an already-sanitised string into a PDF literal using WinAnsiEncoding.
///
/// The fonts declare `/WinAnsiEncoding`, so any codepoint that exists in that
/// set can be written as its single byte. This previously replaced every
/// non-ASCII character with "?", which is why separators rendered as
/// "Hamsa Vilas PG ? Rent" instead of "Hamsa Vilas PG - Rent".
fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            c if c.is_ascii() => out.push(c),
            c => {
                if let Some(b) = winansi_byte(c) {
                    out.push_str(&format!("\\{b:03o}"));
                }
            }
        }
    }
    out
}

/// WinAnsi (CP1252) byte for the handful of non-ASCII characters this template
/// actually uses. Anything else is dropped in [`sanitize`].
fn winansi_byte(c: char) -> Option<u8> {
    Some(match c {
        '\u{b7}' => 0xB7,   // middle dot, the separator between fields
        '\u{2013}' => 0x96, // en dash
        '\u{2014}' => 0x97, // em dash
        '\u{2018}' => 0x91,
        '\u{2019}' => 0x92,
        '\u{201c}' => 0x93,
        '\u{201d}' => 0x94,
        '\u{2022}' => 0x95,
        _ => return None,
    })
}

/// Wraps a block to `width` points, preserving the author's own line breaks.
///
/// The address and the terms are written with meaningful newlines; reflowing
/// the whole string as one paragraph ran "Telangana 500075" into
/// "Mobile: +91 96032 41180". Each source line is wrapped independently and
/// only if it is genuinely too wide.
fn wrap_block(s: &str, width: f32, size: f32, bold: bool) -> Vec<String> {
    let mut out = Vec::new();
    for source in s.lines() {
        let source = sanitize(source.trim());
        if source.is_empty() {
            continue;
        }
        let mut line = String::new();
        for word in source.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if text_width(&candidate, size, bold) > width && !line.is_empty() {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            out.push(line);
        }
    }
    out
}

/// Money as it appears on an invoice: Indian digit grouping, always two
/// decimals. 1500000 paise -> "15,000.00"; 15000000 -> "1,50,000.00".
fn format_amount(paise: i64) -> String {
    let units = (paise / 100).abs();
    let digits = units.to_string();
    let bytes = digits.as_bytes();
    let mut grouped = String::new();
    // Indian grouping: the last three digits form one group, everything above
    // it is grouped in pairs (lakh, crore), not in thousands.
    for (i, ch) in digits.chars().enumerate() {
        let remaining = bytes.len() - i;
        if i > 0 && (remaining == 3 || (remaining > 3 && (remaining - 3).is_multiple_of(2))) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let sign = if paise < 0 { "-" } else { "" };
    format!("{sign}{grouped}.{:02}", (paise % 100).abs())
}

/// The same figure with a currency marker, for prose lines rather than columns.
fn rupees(paise: i64) -> String {
    format!("INR {}", format_amount(paise))
}

fn amount_in_words(paise: i64) -> String {
    let rupees = paise / 100;
    let paise_part = (paise % 100).abs();
    let mut words = format!("Indian Rupee {}", number_words(rupees));
    if paise_part > 0 {
        words.push_str(&format!(" and {} Paise", number_words(paise_part)));
    }
    words.push_str(" only");
    words
}

/// Indian numbering: thousand, lakh, crore -- not million/billion.
///
/// The previous version fell through to raw digits above 99,999, so a
/// \u{20b9}1,50,000 settlement printed "Indian Rupee 150000 only" in the
/// amount-in-words line that exists precisely to be unambiguous.
fn number_words(n: i64) -> String {
    if n == 0 {
        return "Zero".into();
    }
    if n < 0 {
        return format!("Minus {}", number_words(-n));
    }
    let ones = [
        "",
        "One",
        "Two",
        "Three",
        "Four",
        "Five",
        "Six",
        "Seven",
        "Eight",
        "Nine",
        "Ten",
        "Eleven",
        "Twelve",
        "Thirteen",
        "Fourteen",
        "Fifteen",
        "Sixteen",
        "Seventeen",
        "Eighteen",
        "Nineteen",
    ];
    if n < 20 {
        return ones[n as usize].into();
    }
    if n < 100 {
        // Indexed from 20, NOT from 0. Indexing a table that begins with a
        // filler entry by `n / 10` shifted every value one decade up, so
        // \u{20b9}8,750 spelled out as "Eight Hundred Sixty" instead of "Fifty".
        const TENS: [&str; 8] = [
            "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
        ];
        let tens = TENS[n as usize / 10 - 2];
        return if n % 10 == 0 {
            tens.to_string()
        } else {
            format!("{tens} {}", ones[(n % 10) as usize])
        };
    }
    // (divisor, name) in descending order; the first that fits names the group.
    for (divisor, name) in [
        (10_000_000, "Crore"),
        (100_000, "Lakh"),
        (1_000, "Thousand"),
        (100, "Hundred"),
    ] {
        if n >= divisor {
            let rest = n % divisor;
            return format!(
                "{} {name}{}",
                number_words(n / divisor),
                if rest == 0 {
                    String::new()
                } else {
                    format!(" {}", number_words(rest))
                }
            );
        }
    }
    unreachable!("every n >= 100 is covered by the table above")
}

fn build_pdf(content: Vec<u8>) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] \
             /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> /Contents 4 0 R >>"
        )
        .into_bytes(),
    );
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(&content);
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);
    objects.push(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    objects.push(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(obj);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    let size = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for o in offsets {
        pdf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF").as_bytes(),
    );
    pdf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_digits_the_indian_way() {
        assert_eq!(format_amount(1_250_000), "12,500.00");
        assert_eq!(format_amount(15_000_000), "1,50,000.00");
        assert_eq!(format_amount(120_000_000), "12,00,000.00");
        assert_eq!(format_amount(1_500_000_000), "1,50,00,000.00");
        assert_eq!(format_amount(45_67), "45.67");
        assert_eq!(format_amount(0), "0.00");
    }

    #[test]
    fn spells_lakhs_and_crores_instead_of_falling_back_to_digits() {
        assert_eq!(
            amount_in_words(1_250_000),
            "Indian Rupee Twelve Thousand Five Hundred only"
        );
        assert_eq!(
            amount_in_words(15_000_000),
            "Indian Rupee One Lakh Fifty Thousand only"
        );
        assert_eq!(
            amount_in_words(1_500_000_000),
            "Indian Rupee One Crore Fifty Lakh only"
        );
        assert_eq!(amount_in_words(0), "Indian Rupee Zero only");
    }

    #[test]
    fn spells_the_paise_remainder() {
        assert_eq!(
            amount_in_words(1_250_050),
            "Indian Rupee Twelve Thousand Five Hundred and Fifty Paise only"
        );
    }

    #[test]
    fn rupee_sign_becomes_a_spaced_marker() {
        assert_eq!(
            sanitize("a late fee of \u{20b9}100 per day"),
            "a late fee of INR 100 per day"
        );
        // Already spaced in the source: do not double it.
        assert_eq!(sanitize("\u{20b9} 100"), "INR 100");
    }

    #[test]
    fn wordmark_letter_spacing_keeps_its_word_gap() {
        // The gap between words has to stay wider than the gap between
        // letters, or the badge reads "H A M S A V I L A S".
        assert_eq!(sanitize(&space_out("HAMSA VILAS")), "H A M S A   V I L A S");
    }

    #[test]
    fn address_keeps_the_operators_line_breaks() {
        let address = "Telangana 500075\nMobile: +91 96032 41180";
        assert_eq!(
            wrap_block(address, 250.0, 9.5, false),
            vec!["Telangana 500075", "Mobile: +91 96032 41180"]
        );
    }

    #[test]
    fn overlong_lines_still_wrap() {
        let lines = wrap_block(&"word ".repeat(40), 200.0, 9.5, false);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|l| text_width(l, 9.5, false) <= 200.0));
    }

    #[test]
    fn renders_a_parseable_single_page_pdf() {
        let bytes = SimpleReceiptRenderer.render(
            &ReceiptTemplate::default_template(),
            &ReceiptData {
                receipt_number: "R-1".into(),
                invoice_number: "INV-1".into(),
                resident_name: "Sahithi Reddy".into(),
                branch_name: Some("Hamsa Vilas".into()),
                amount_paise: 1_250_000,
                issued_epoch: 1_786_421_093,
                tenant_phone: Some("+91 91773 20486".into()),
                room_number: Some("204".into()),
                plan_kind: "Fixed".into(),
                line_item: "Rent".into(),
                payment_method: "PhonePe".into(),
                payment_reference: Some("T2508091142".into()),
                floor_label: Some("Floor 2".into()),
                recorded_by: "Owner".into(),
            },
        );
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.ends_with(b"%%EOF"));
    }
}
