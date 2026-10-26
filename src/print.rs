//! Impression native via GDI, comme le Bloc-notes Win32.
//!
//! La boîte « Imprimer » est celle de Windows (`PrintDlgW`) : tout le document,
//! une plage de pages ou la sélection, avec le nombre de copies. Chaque page
//! est rendue avec la police de l'éditeur et l'en-tête/pied de page de « Mise
//! en page », en utilisant la même pagination que l'aperçu (`page::paginate`).

use crate::header;
use crate::page::{self, Bands, PageSetup};

/// Tout ce dont le pilote d'impression a besoin.
pub struct PrintJob {
    pub text: String,
    pub title: String,
    pub setup: PageSetup,
    /// Taille de police du document, en points.
    pub font_pt: f32,
    /// Nom de la police Windows a utiliser.
    pub face: String,
    pub monospace: bool,
    /// Texte selectionne : la boite d'impression propose alors « Selection ».
    pub selection: Option<String>,
}

/// Resultat d'une demande d'impression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintOutcome {
    /// L'utilisateur a annule la boîte de dialogue.
    Cancelled,
    /// Pages et copies reellement envoyees.
    Printed { pages: usize, copies: usize },
}

#[cfg(windows)]
pub fn print(job: &PrintJob) -> Result<PrintOutcome, String> {
    use windows_sys::Win32::Graphics::Gdi::{
        CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH, DEFAULT_QUALITY,
        DeleteDC, FF_DONTCARE, FF_MODERN, FW_NORMAL, GetDeviceCaps, GetTextExtentPoint32W, HDC,
        HORZRES, LOGPIXELSX, LOGPIXELSY, OUT_DEFAULT_PRECIS, PHYSICALHEIGHT, PHYSICALOFFSETX,
        PHYSICALOFFSETY, PHYSICALWIDTH, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, VERTRES,
    };
    use windows_sys::Win32::Storage::Xps::{DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};
    use windows_sys::Win32::UI::Controls::Dialogs::{
        PD_ALLPAGES, PD_NOSELECTION, PD_PAGENUMS, PD_RETURNDC, PD_SELECTION, PRINTDLGW, PrintDlgW,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    // La boite doit appartenir a *notre* fenetre : `GetForegroundWindow` peut
    // designer la fenetre d'une autre application, la boite s'ouvrirait alors
    // derriere elle. `GetActiveWindow` interroge la file de messages du thread
    // courant, celui qui possede la fenetre eframe.
    let mut owner = unsafe { GetActiveWindow() };
    if owner.is_null() {
        owner = unsafe { GetForegroundWindow() };
    }
    let setup = job.setup.sanitized();

    let mut dialog = PRINTDLGW::default();
    dialog.lStructSize = std::mem::size_of::<PRINTDLGW>() as u32;
    dialog.hwndOwner = owner;
    dialog.nCopies = 1;
    dialog.nMinPage = 1;
    dialog.nMaxPage = 9999;

    // Premier appel sans interface : recuperer le `DEVMODE` de l'imprimante par
    // defaut, y inscrire le format et l'orientation de « Mise en page », puis
    // les presenter dans la boite. Comme le Bloc-notes, la boite affiche donc
    // l'orientation qui sera reellement utilisee.
    {
        use windows_sys::Win32::UI::Controls::Dialogs::PD_RETURNDEFAULT;
        let mut probe = PRINTDLGW::default();
        probe.lStructSize = std::mem::size_of::<PRINTDLGW>() as u32;
        probe.Flags = PD_RETURNDEFAULT;
        if unsafe { PrintDlgW(&mut probe) } != 0 {
            if !probe.hDevMode.is_null() {
                unsafe { write_page_setup(probe.hDevMode, &setup) };
            }
            if !probe.hDC.is_null() {
                unsafe { DeleteDC(probe.hDC) };
            }
            dialog.hDevMode = probe.hDevMode;
            dialog.hDevNames = probe.hDevNames;
        }
    }
    dialog.Flags = PD_RETURNDC | PD_ALLPAGES;
    if job.selection.is_none() {
        dialog.Flags |= PD_NOSELECTION;
    }

    let accepted = unsafe { PrintDlgW(&mut dialog) };
    if accepted == 0 {
        unsafe {
            free_global(dialog.hDevMode);
            free_global(dialog.hDevNames);
        }
        return Ok(PrintOutcome::Cancelled);
    }
    if dialog.hDC.is_null() {
        unsafe {
            free_global(dialog.hDevMode);
            free_global(dialog.hDevNames);
        }
        return Err("Aucune imprimante n'a été sélectionnée.".to_string());
    }

    // Choix retenus dans la boite : tout, une plage de pages, ou la selection.
    // Sans selection dans l'editeur, le bouton « Selection » est desactive.
    let flags = dialog.Flags;
    let use_selection = flags & PD_SELECTION != 0;
    let use_pages = flags & PD_PAGENUMS != 0;
    let copies = dialog.nCopies.max(1) as usize;
    let range = if use_pages {
        Some((dialog.nFromPage as usize, dialog.nToPage as usize))
    } else {
        None
    };
    let body = match (&job.selection, use_selection) {
        (Some(selection), true) => selection.clone(),
        _ => job.text.clone(),
    };

    // Le contexte d'impression reprend le `DEVMODE` retenu par la boite (que
    // l'utilisateur a pu modifier) avec notre mise en page.
    if !dialog.hDevMode.is_null() {
        unsafe { write_page_setup(dialog.hDevMode, &setup) };
    }
    let hdc: HDC = unsafe { reset_from_devmode(dialog.hDC, dialog.hDevMode) };
    unsafe {
        free_global(dialog.hDevMode);
        free_global(dialog.hDevNames);
    }

    let dpi_x = unsafe { GetDeviceCaps(hdc, LOGPIXELSX as i32) }.max(1) as f32;
    let dpi_y = unsafe { GetDeviceCaps(hdc, LOGPIXELSY as i32) }.max(1) as f32;
    let to_pt_x = |px: i32| px as f32 * 72.0 / dpi_x;
    let to_pt_y = |px: i32| px as f32 * 72.0 / dpi_y;
    let offset_x_pt = to_pt_x(unsafe { GetDeviceCaps(hdc, PHYSICALOFFSETX as i32) });
    let offset_y_pt = to_pt_y(unsafe { GetDeviceCaps(hdc, PHYSICALOFFSETY as i32) });
    let paper_pt = (
        to_pt_x(unsafe { GetDeviceCaps(hdc, PHYSICALWIDTH as i32) }),
        to_pt_y(unsafe { GetDeviceCaps(hdc, PHYSICALHEIGHT as i32) }),
    );
    let printable_pt = (
        to_pt_x(unsafe { GetDeviceCaps(hdc, HORZRES as i32) }),
        to_pt_y(unsafe { GetDeviceCaps(hdc, VERTRES as i32) }),
    );
    // Marges demandees, elargies si la zone imprimable du pilote est plus
    // petite : un texte dans la zone non imprimable serait coupe.
    let asked = setup.margins_pt();
    let left = asked[3].max(offset_x_pt);
    let top = asked[0].max(offset_y_pt);
    let right = asked[1]
        .min((paper_pt.0 - offset_x_pt - printable_pt.0).max(0.0))
        .max(0.0);
    let bottom = asked[2]
        .min((paper_pt.1 - offset_y_pt - printable_pt.1).max(0.0))
        .max(0.0);
    let margins = [top, right, bottom, left];

    // Police de l'editeur a la taille demandee (hauteur negative = hauteur de
    // caracteres, et non de cellule).
    let height = -((job.font_pt * dpi_y / 72.0).round() as i32).max(1);
    let face = wide(&job.face);
    let family = if job.monospace {
        FF_MODERN as u32
    } else {
        FF_DONTCARE as u32
    };
    let font = unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            DEFAULT_QUALITY as u32,
            DEFAULT_PITCH as u32 | family,
            face.as_ptr(),
        )
    };
    if font.is_null() {
        unsafe { DeleteDC(hdc) };
        return Err(format!("Police d'impression introuvable : {}", job.face));
    }
    let previous = unsafe { SelectObject(hdc, font) };
    unsafe {
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, 0);
    }

    let mut measure = |line: &str| -> f32 {
        let text = wide(line);
        let mut size = windows_sys::Win32::Foundation::SIZE { cx: 0, cy: 0 };
        let ok =
            unsafe { GetTextExtentPoint32W(hdc, text.as_ptr(), (text.len() - 1) as i32, &mut size) };
        if ok == 0 {
            0.0
        } else {
            size.cx as f32 * 72.0 / dpi_x
        }
    };
    let doc = page::paginate_sized(
        &body,
        paper_pt,
        margins,
        job.font_pt,
        Bands::from_setup(&setup),
        &mut measure,
    );
    // Plage demandee, bornee aux pages reellement produites.
    let pages = page::selected_pages(doc.pages.len(), range);
    if pages.is_empty() {
        finish(hdc, font, previous);
        return Err("Plage de pages vide : rien à imprimer.".to_string());
    }

    let doc_name = wide(&job.title);
    let info = DOCINFOW {
        cbSize: std::mem::size_of::<DOCINFOW>() as i32,
        lpszDocName: doc_name.as_ptr(),
        lpszOutput: std::ptr::null(),
        lpszDatatype: std::ptr::null(),
        fwType: 0,
    };
    if unsafe { StartDocW(hdc, &info) } <= 0 {
        finish(hdc, font, previous);
        return Err("Le pilote d'impression a refusé le document.".to_string());
    }

    let offset_x = unsafe { GetDeviceCaps(hdc, PHYSICALOFFSETX as i32) } as f32;
    let offset_y = unsafe { GetDeviceCaps(hdc, PHYSICALOFFSETY as i32) } as f32;
    let x_pt = doc.margins_pt[3];
    let total = doc.pages.len();
    let now = chrono::Local::now();
    let date = now.format("%d/%m/%Y").to_string();
    let time = now.format("%H:%M").to_string();
    let mut error = None;
    'copies: for _ in 0..copies {
        for &page_index in &pages {
            if unsafe { StartPage(hdc) } <= 0 {
                error = Some("Le pilote d'impression a refusé une page.".to_string());
                break 'copies;
            }
            for (line_index, line) in doc.pages[page_index].iter().enumerate() {
                let y_pt = doc.text_top_pt() + line_index as f32 * doc.line_height_pt;
                draw_line(hdc, line, x_pt, y_pt, dpi_x, dpi_y, offset_x, offset_y);
            }
            let fields = header::Fields {
                file: &job.title,
                page: page_index + 1,
                total,
                date: &date,
                time: &time,
            };
            if !setup.header.trim().is_empty() {
                draw_band(
                    hdc,
                    &setup.header,
                    &fields,
                    doc.margins_pt[0],
                    doc.size_pt,
                    doc.margins_pt,
                    dpi_x,
                    dpi_y,
                    offset_x,
                    offset_y,
                    &mut measure,
                );
            }
            if !setup.footer.trim().is_empty() {
                let top = doc.size_pt.1 - doc.margins_pt[2] - doc.footer_pt();
                draw_band(
                    hdc,
                    &setup.footer,
                    &fields,
                    top,
                    doc.size_pt,
                    doc.margins_pt,
                    dpi_x,
                    dpi_y,
                    offset_x,
                    offset_y,
                    &mut measure,
                );
            }
            if unsafe { EndPage(hdc) } <= 0 {
                error = Some("Le pilote d'impression a refusé la fin de page.".to_string());
                break 'copies;
            }
        }
    }

    let ended = if error.is_none() {
        unsafe { EndDoc(hdc) }
    } else {
        unsafe {
            windows_sys::Win32::Storage::Xps::AbortDoc(hdc);
        }
        -1
    };
    finish(hdc, font, previous);
    if let Some(error) = error {
        return Err(error);
    }
    if ended < 0 {
        return Err("L'impression a échoué (document non transmis à l'imprimante).".to_string());
    }
    Ok(PrintOutcome::Printed {
        pages: pages.len(),
        copies,
    })
}

/// Reapplique l'orientation et le format choisis dans « Mise en page » au
/// contexte d'impression, via son `DEVMODE`.
///
/// Renvoie le contexte effectif : `ResetDCW` peut en fournir un nouveau.
#[cfg(windows)]
unsafe fn reset_from_devmode(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    devmode: windows_sys::Win32::Foundation::HGLOBAL,
) -> windows_sys::Win32::Graphics::Gdi::HDC {
    use windows_sys::Win32::Graphics::Gdi::{DEVMODEW, ResetDCW};
    use windows_sys::Win32::System::Memory::GlobalLock;

    if devmode.is_null() {
        return hdc;
    }
    let pointer = unsafe { GlobalLock(devmode) } as *const DEVMODEW;
    if pointer.is_null() {
        return hdc;
    }
    let updated = unsafe { ResetDCW(hdc, pointer) };
    unsafe { windows_sys::Win32::System::Memory::GlobalUnlock(devmode) };
    if updated.is_null() { hdc } else { updated }
}

/// Inscrit le format et l'orientation de « Mise en page » dans un `DEVMODE`.
#[cfg(windows)]
unsafe fn write_page_setup(devmode: windows_sys::Win32::Foundation::HGLOBAL, setup: &PageSetup) {
    use windows_sys::Win32::Graphics::Gdi::{
        DEVMODEW, DMORIENT_LANDSCAPE, DMORIENT_PORTRAIT, DM_ORIENTATION, DM_PAPERLENGTH,
        DM_PAPERSIZE, DM_PAPERWIDTH, DMPAPER_A4, DMPAPER_A5, DMPAPER_LEGAL, DMPAPER_LETTER,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    if devmode.is_null() {
        return;
    }
    let pointer = unsafe { GlobalLock(devmode) } as *mut DEVMODEW;
    if pointer.is_null() {
        return;
    }
    // `dmPaperWidth`/`dmPaperLength` s'expriment toujours dans le sens
    // portrait, en dixiemes de millimetre ; seule `dmOrientation` decide.
    let index = setup.paper.min(page::PAPERS.len() as u8 - 1) as usize;
    let (_, portrait_w, portrait_h) = page::PAPERS[index];
    let paper = match index {
        1 => DMPAPER_LETTER,
        2 => DMPAPER_LEGAL,
        3 => DMPAPER_A5,
        _ => DMPAPER_A4,
    };
    let mode = unsafe { &mut *pointer };
    mode.dmFields |= DM_ORIENTATION | DM_PAPERSIZE | DM_PAPERWIDTH | DM_PAPERLENGTH;
    unsafe {
        let printer = &mut mode.Anonymous1.Anonymous1;
        printer.dmOrientation = if setup.landscape {
            DMORIENT_LANDSCAPE as i16
        } else {
            DMORIENT_PORTRAIT as i16
        };
        printer.dmPaperSize = paper as i16;
        printer.dmPaperWidth = (portrait_w * 10.0).round() as i16;
        printer.dmPaperLength = (portrait_h * 10.0).round() as i16;
    }
    unsafe { GlobalUnlock(devmode) };
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn draw_band(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    template: &str,
    fields: &header::Fields,
    y_pt: f32,
    size_pt: (f32, f32),
    margins_pt: [f32; 4],
    dpi_x: f32,
    dpi_y: f32,
    offset_x: f32,
    offset_y: f32,
    measure: &mut dyn FnMut(&str) -> f32,
) {
    let sections = header::resolve(template, fields);
    for (text, column) in [
        (&sections.left, 0u8),
        (&sections.center, 1u8),
        (&sections.right, 2u8),
    ] {
        if text.is_empty() {
            continue;
        }
        let width = measure(text);
        let x_pt = match column {
            0 => margins_pt[3],
            1 => (size_pt.0 - width) / 2.0,
            _ => size_pt.0 - margins_pt[1] - width,
        };
        draw_line(hdc, text, x_pt, y_pt, dpi_x, dpi_y, offset_x, offset_y);
    }
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
unsafe fn free_global(handle: windows_sys::Win32::Foundation::HGLOBAL) {
    if !handle.is_null() {
        unsafe { windows_sys::Win32::Foundation::GlobalFree(handle) };
    }
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn draw_line(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    text: &str,
    x_pt: f32,
    y_pt: f32,
    dpi_x: f32,
    dpi_y: f32,
    offset_x: f32,
    offset_y: f32,
) {
    use windows_sys::Win32::Graphics::Gdi::TextOutW;
    let x = ((x_pt * dpi_x / 72.0) - offset_x).round() as i32;
    let y = ((y_pt * dpi_y / 72.0) - offset_y).round() as i32;
    let text = wide(text);
    unsafe { TextOutW(hdc, x, y, text.as_ptr(), (text.len() - 1) as i32) };
}

#[cfg(windows)]
fn finish(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    font: windows_sys::Win32::Graphics::Gdi::HFONT,
    previous: windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
) {
    use windows_sys::Win32::Graphics::Gdi::{DeleteDC, DeleteObject, SelectObject};
    unsafe {
        if !previous.is_null() {
            SelectObject(hdc, previous);
        }
        DeleteObject(font);
        DeleteDC(hdc);
    }
}

#[cfg(not(windows))]
pub fn print(_job: &PrintJob) -> Result<PrintOutcome, String> {
    Err("L'impression n'est disponible que sous Windows.".to_string())
}
