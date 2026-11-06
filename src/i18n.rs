//! Interface languages. Texts are written in English in the code and looked
//! up here with `t("…")`; a text missing from the table shows in English.
//! Flags are drawn in code (egui's fonts have no flag emoji).

use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Rect, Sense, Stroke};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub enum Lang {
    #[default]
    En,
    Es,
    Ca,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::En, Lang::Es, Lang::Ca];

    /// The language's own name for itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::Ca => "Català",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "EN",
            Lang::Es => "ES",
            Lang::Ca => "CA",
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::Es,
        2 => Lang::Ca,
        _ => Lang::En,
    }
}

/// English text → (Spanish from Spain, Catalan).
const TABLE: &[(&str, &str, &str)] = &[
    ("Show {}", "Mostrar {}", "Mostra {}"),
    ("Capture Area", "Capturar área", "Captura una àrea"),
    ("Capture Fullscreen", "Capturar pantalla completa", "Captura la pantalla completa"),
    ("Fullscreen after 10s", "Pantalla completa en 10 s", "Pantalla completa d'aquí a 10 s"),
    ("Capture Frozen Area", "Capturar área congelada", "Captura una àrea congelada"),
    ("Record Video", "Grabar vídeo", "Enregistra un vídeo"),
    ("Stop Recording", "Detener grabación", "Atura l'enregistrament"),
    ("Open…", "Abrir…", "Obre…"),
    ("Open from Clipboard", "Abrir desde el portapapeles", "Obre des del porta-retalls"),
    ("Upload from Clipboard", "Subir desde el portapapeles", "Puja des del porta-retalls"),
    ("Open Latest Image", "Abrir la última imagen", "Obre l'última imatge"),
    ("Preferences…", "Ajustes…", "Configuració…"),
    ("Quit", "Salir", "Surt"),
    ("Recording the screen…", "Grabando la pantalla…", "S'està enregistrant la pantalla…"),
    ("Video saved to {}", "Vídeo guardado en {}", "Vídeo desat a {}"),
    ("Open", "Abrir", "Obre"),
    ("Images", "Imágenes", "Imatges"),
    ("Can't open this file: {}", "No se puede abrir este archivo: {}", "No es pot obrir aquest fitxer: {}"),
    ("There is no screenshot yet", "Todavía no hay ninguna captura", "Encara no hi ha cap captura"),
    ("Uploading needs a connected account; opening the editor instead", "Para subir hace falta una cuenta conectada; se abre el editor", "Per pujar cal un compte connectat; s'obre l'editor"),
    ("Saved to {}", "Guardado en {}", "Desat a {}"),
    ("Save image", "Guardar imagen", "Desa la imatge"),
    ("Image copied to clipboard", "Imagen copiada al portapapeles", "Imatge copiada al porta-retalls"),
    ("Take a screenshot, mark it up, save or copy it.", "Haz una captura, anótala y guárdala o cópiala.", "Fes una captura, anota-la i desa-la o copia-la."),
    ("Not set", "Sin asignar", "Sense assignar"),
    ("The same commands are in the menu-bar icon.", "Los mismos comandos están en el icono de la barra de menús.", "Les mateixes ordres són a la icona de la barra de menús."),
    ("Recording {}", "Grabando {}", "Enregistrant {}"),
    ("Stop", "Detener", "Atura"),
    ("All settings have been reset", "Se han restablecido todos los ajustes", "S'ha restablert tota la configuració"),
    ("About {}", "Acerca de {}", "Quant a {}"),
    ("Version {}", "Versión {}", "Versió {}"),
    ("Screenshots, annotations and screen recordings.", "Capturas, anotaciones y grabaciones de pantalla.", "Captures, anotacions i enregistraments de pantalla."),
    ("Language", "Idioma", "Idioma"),
    ("Language:", "Idioma:", "Idioma:"),
    ("Could not start the capture tool: {}", "No se ha podido iniciar la herramienta de captura: {}", "No s'ha pogut iniciar l'eina de captura: {}"),
    ("Could not read the capture: {}", "No se ha podido leer la captura: {}", "No s'ha pogut llegir la captura: {}"),
    ("Can't create {}: {}", "No se puede crear {}: {}", "No es pot crear {}: {}"),
    ("Screenshot copied to clipboard", "Captura copiada al portapapeles", "Captura copiada al porta-retalls"),
    ("Path copied to clipboard", "Ruta copiada al portapapeles", "Camí copiat al porta-retalls"),
    ("Nothing to copy", "No hay nada que copiar", "No hi ha res per copiar"),
    ("Clipboard doesn't contain image", "El portapapeles no contiene ninguna imagen", "El porta-retalls no conté cap imatge"),
    ("Could not start recording: {}", "No se ha podido iniciar la grabación: {}", "No s'ha pogut iniciar l'enregistrament: {}"),
    ("The recording was cancelled", "Se ha cancelado la grabación", "S'ha cancel·lat l'enregistrament"),
    ("Drag to select an area · Click to keep the whole screen · Hold ⇧ for a square · Esc to cancel", "Arrastra para seleccionar un área · Haz clic para quedarte toda la pantalla · Mantén ⇧ para un cuadrado · Esc para cancelar", "Arrossega per seleccionar una àrea · Fes clic per quedar-te tota la pantalla · Mantén ⇧ per a un quadrat · Esc per cancel·lar"),
    ("Select & move", "Seleccionar y mover", "Selecciona i mou"),
    ("Click an item to select it, drag to move it\nDouble-click a text to edit it · Delete removes it", "Haz clic en un elemento para seleccionarlo y arrástralo para moverlo\nDoble clic en un texto para editarlo · Suprimir lo elimina", "Fes clic en un element per seleccionar-lo i arrossega'l per moure'l\nDoble clic en un text per editar-lo · Suprimeix l'elimina"),
    ("Arrow", "Flecha", "Fletxa"),
    ("Hold ⇧ to draw at 45°\nHold ⌘ to draw from arrow’s head to tail", "Mantén ⇧ para dibujar a 45°\nMantén ⌘ para dibujar de la punta a la cola", "Mantén ⇧ per dibuixar a 45°\nMantén ⌘ per dibuixar de la punta a la cua"),
    ("Rectangle", "Rectángulo", "Rectangle"),
    ("Hold ⇧ to draw a square\nHold ⌥ to dark all except selected area\nHold ⌘ to draw filled rectangle", "Mantén ⇧ para dibujar un cuadrado\nMantén ⌥ para oscurecer todo excepto el área seleccionada\nMantén ⌘ para dibujar un rectángulo relleno", "Mantén ⇧ per dibuixar un quadrat\nMantén ⌥ per enfosquir-ho tot excepte l'àrea seleccionada\nMantén ⌘ per dibuixar un rectangle ple"),
    ("Line", "Línea", "Línia"),
    ("Hold ⇧ to draw at 45°", "Mantén ⇧ para dibujar a 45°", "Mantén ⇧ per dibuixar a 45°"),
    ("Ellipse", "Elipse", "El·lipse"),
    ("Hold ⇧ to draw a circle\nHold ⌘ to draw filled ellipse", "Mantén ⇧ para dibujar un círculo\nMantén ⌘ para dibujar una elipse rellena", "Mantén ⇧ per dibuixar un cercle\nMantén ⌘ per dibuixar una el·lipse plena"),
    ("Pen", "Lápiz", "Llapis"),
    ("Hold ⌘ to convert into highlighter", "Mantén ⌘ para convertirlo en rotulador fluorescente", "Mantén ⌘ per convertir-lo en retolador fluorescent"),
    ("Text", "Texto", "Text"),
    ("Click to type · click an existing text to edit it", "Haz clic para escribir · haz clic en un texto existente para editarlo", "Fes clic per escriure · fes clic en un text existent per editar-lo"),
    ("Text with arrow", "Texto con flecha", "Text amb fletxa"),
    ("Drag from the point to where the note goes, then type", "Arrastra desde el punto hasta donde va la nota y escribe", "Arrossega des del punt fins on va la nota i escriu"),
    ("Blur", "Desenfocar", "Difumina"),
    ("Drag over what you want to hide\nHold ⇧ for a square", "Arrastra sobre lo que quieras ocultar\nMantén ⇧ para un cuadrado", "Arrossega per sobre del que vulguis amagar\nMantén ⇧ per a un quadrat"),
    ("Crop", "Recortar", "Retalla"),
    ("Hold ⇧ for a square\nEnter to apply, Esc to cancel", "Mantén ⇧ para un cuadrado\nIntro para aplicar, Esc para cancelar", "Mantén ⇧ per a un quadrat\nRetorn per aplicar, Esc per cancel·lar"),
    ("Home", "Inicio", "Inici"),
    ("Back to the capture screen", "Volver a la pantalla de captura", "Torna a la pantalla de captura"),
    ("Rotate clockwise", "Girar a la derecha", "Gira cap a la dreta"),
    ("Hold ⌘ to rotate counterclockwise", "Mantén ⌘ para girar a la izquierda", "Mantén ⌘ per girar cap a l'esquerra"),
    ("Resize", "Cambiar tamaño", "Canvia la mida"),
    ("Undo", "Deshacer", "Desfés"),
    ("Redo", "Rehacer", "Refés"),
    ("Copy", "Copiar", "Copia"),
    ("Copy image  ⌘C", "Copiar imagen  ⌘C", "Copia la imatge  ⌘C"),
    ("Open in…", "Abrir en…", "Obre amb…"),
    ("External editor  ⌘E", "Editor externo  ⌘E", "Editor extern  ⌘E"),
    ("Upload", "Subir", "Puja"),
    ("Uploading needs a connected account, which this version doesn't include", "Para subir hace falta una cuenta conectada, que esta versión no incluye", "Per pujar cal un compte connectat, que aquesta versió no inclou"),
    ("Save", "Guardar", "Desa"),
    ("Save  ⌘S", "Guardar  ⌘S", "Desa  ⌘S"),
    ("Selected arrow", "Flecha seleccionada", "Fletxa seleccionada"),
    ("Selected line", "Línea seleccionada", "Línia seleccionada"),
    ("Selected rectangle", "Rectángulo seleccionado", "Rectangle seleccionat"),
    ("Selected ellipse", "Elipse seleccionada", "El·lipse seleccionada"),
    ("Selected drawing", "Dibujo seleccionado", "Dibuix seleccionat"),
    ("Selected text", "Texto seleccionado", "Text seleccionat"),
    ("Selected note", "Nota seleccionada", "Nota seleccionada"),
    ("Selected blur area", "Zona desenfocada seleccionada", "Zona difuminada seleccionada"),
    ("Typing — Esc to finish", "Escribiendo — Esc para terminar", "Escrivint — Esc per acabar"),
    ("Click on the picture to type", "Haz clic en la imagen para escribir", "Fes clic a la imatge per escriure"),
    ("Click an item to change it", "Haz clic en un elemento para cambiarlo", "Fes clic en un element per canviar-lo"),
    ("New arrow", "Nueva flecha", "Fletxa nova"),
    ("New rectangle", "Nuevo rectángulo", "Rectangle nou"),
    ("New line", "Nueva línea", "Línia nova"),
    ("New ellipse", "Nueva elipse", "El·lipse nova"),
    ("New drawing", "Nuevo dibujo", "Dibuix nou"),
    ("New text with arrow", "Nuevo texto con flecha", "Text amb fletxa nou"),
    ("New blur area", "Nueva zona desenfocada", "Zona difuminada nova"),
    ("Crop area", "Área de recorte", "Àrea de retall"),
    ("Drag over anything you want to hide · ⇧ for a square", "Arrastra sobre lo que quieras ocultar · ⇧ para un cuadrado", "Arrossega per sobre del que vulguis amagar · ⇧ per a un quadrat"),
    ("Any colour", "Cualquier color", "Qualsevol color"),
    ("Size of lines, arrows and shapes  ⌘[ ⌘]", "Grosor de líneas, flechas y formas  ⌘[ ⌘]", "Gruix de línies, fletxes i formes  ⌘[ ⌘]"),
    ("Text size", "Tamaño del texto", "Mida del text"),
    ("Plain text", "Texto simple", "Text simple"),
    ("Text with an outline", "Texto con contorno", "Text amb contorn"),
    ("Text on a coloured box", "Texto sobre un recuadro de color", "Text sobre un requadre de color"),
    ("Shadow", "Sombra", "Ombra"),
    ("Shadow & outline under new items", "Sombra y contorno bajo los nuevos elementos", "Ombra i contorn sota els elements nous"),
    ("Zoom out  ⌘−", "Alejar  ⌘−", "Allunya  ⌘−"),
    ("Zoom in  ⌘+", "Acercar  ⌘+", "Apropa  ⌘+"),
    ("Fit", "Ajustar", "Ajusta"),
    ("Fit to window  ⌘0", "Ajustar a la ventana  ⌘0", "Ajusta a la finestra  ⌘0"),
    ("Original size", "Tamaño original", "Mida original"),
    ("Type text…", "Escribe el texto…", "Escriu el text…"),
    ("Cancel", "Cancelar", "Cancel·la"),
    ("Width:", "Ancho:", "Amplada:"),
    ("Height:", "Alto:", "Alçada:"),
    ("Save proportions", "Mantener proporciones", "Mantén les proporcions"),
    ("OK", "Aceptar", "D'acord"),
    ("Needs a connected account, which this version doesn't include", "Necesita una cuenta conectada, que esta versión no incluye", "Necessita un compte connectat, que aquesta versió no inclou"),
    ("General", "General", "General"),
    ("Advanced", "Avanzado", "Avançat"),
    ("Appearance", "Apariencia", "Aparença"),
    ("Hotkeys", "Atajos", "Dreceres"),
    ("Fonts", "Tipos de letra", "Tipus de lletra"),
    ("Reset all settings", "Restablecer todos los ajustes", "Restableix tota la configuració"),
    ("ALL settings in the application will be reset", "Se restablecerán TODOS los ajustes de la aplicación", "Es restablirà TOTA la configuració de l'aplicació"),
    ("Reset", "Restablecer", "Restableix"),
    ("After screenshot:", "Después de capturar:", "Després de capturar:"),
    ("Sound effects", "Efectos de sonido", "Efectes de so"),
    ("After upload:", "Después de subir:", "Després de pujar:"),
    ("Do not copy", "No copiar", "Sense copiar"),
    ("Copy site link", "Copiar el enlace a la página", "Copia l'enllaç a la pàgina"),
    ("Copy direct image link", "Copiar el enlace directo a la imagen", "Copia l'enllaç directe a la imatge"),
    ("Open in browser", "Abrir en el navegador", "Obre al navegador"),
    ("Short links", "Enlaces cortos", "Enllaços curts"),
    ("Apply text recognition (in development)", "Reconocer texto (en desarrollo)", "Reconeix el text (en desenvolupament)"),
    ("External Image Editor:", "Editor de imágenes externo:", "Editor d'imatges extern:"),
    ("Choose application...", "Elegir aplicación...", "Tria una aplicació..."),
    ("Choose application", "Elegir aplicación", "Tria una aplicació"),
    ("Application", "Aplicación", "Aplicació"),
    ("Status bar:", "Barra de menús:", "Barra de menús:"),
    ("Launch at login", "Abrir al iniciar sesión", "Obre en iniciar la sessió"),
    ("Image File Format:", "Formato de imagen:", "Format d'imatge:"),
    ("Auto (smaller size)", "Automático (menor tamaño)", "Automàtic (mida més petita)"),
    ("JPEG Quality:", "Calidad JPEG:", "Qualitat JPEG:"),
    ("Save folder:", "Carpeta de destino:", "Carpeta de destinació:"),
    ("Choose...", "Elegir...", "Tria..."),
    ("Choose save folder", "Elegir la carpeta de destino", "Tria la carpeta de destinació"),
    ("Autosave screenshots", "Guardar capturas automáticamente", "Desa les captures automàticament"),
    ("Shrink Retina snaps", "Reducir capturas Retina", "Redueix les captures Retina"),
    ("Video File Format:", "Formato de vídeo:", "Format de vídeo:"),
    ("GIF Quality:", "Calidad GIF:", "Qualitat GIF:"),
    ("Autosave to \"Movies\" folder", "Guardar automáticamente en «Películas»", "Desa automàticament a «Pel·lícules»"),
    ("Drag and drop copy:", "Copia al arrastrar y soltar:", "Còpia en arrossegar i deixar anar:"),
    ("Copy to clipboard:", "Copiar al portapapeles:", "Copia al porta-retalls:"),
    ("Filename template:", "Plantilla de nombre de archivo:", "Plantilla de nom de fitxer:"),
    ("%Y — 4-digit year,  %y — 2-digit year, %m — month, %d — day,\n%H — hour, %M — minutes, %S — seconds, %W — window title,\n%R — 5 random symbols, %T - file type", "%Y — año (4 cifras),  %y — año (2 cifras), %m — mes, %d — día,\n%H — hora, %M — minutos, %S — segundos, %W — título de la ventana,\n%R — 5 símbolos aleatorios, %T - tipo de archivo", "%Y — any (4 xifres),  %y — any (2 xifres), %m — mes, %d — dia,\n%H — hora, %M — minuts, %S — segons, %W — títol de la finestra,\n%R — 5 símbols aleatoris, %T - tipus de fitxer"),
    ("Apply to reopened files", "Aplicar a los archivos reabiertos", "Aplica als fitxers reoberts"),
    ("Theme:", "Tema:", "Tema:"),
    ("Automatic", "Automático", "Automàtic"),
    ("Light", "Claro", "Clar"),
    ("Dark", "Oscuro", "Fosc"),
    ("Capture area:", "Captura de área:", "Captura d'àrea:"),
    ("Show magnifier", "Mostrar lupa", "Mostra la lupa"),
    ("Highlight window under cursor", "Resaltar la ventana bajo el cursor", "Ressalta la finestra sota el cursor"),
    ("Add shadow to window", "Añadir sombra a la ventana", "Afegeix ombra a la finestra"),
    ("Click to snap a window", "Clic para capturar una ventana", "Fes clic per capturar una finestra"),
    ("Show hints", "Mostrar consejos", "Mostra consells"),
    ("Hide animation", "Ocultar animación", "Amaga l'animació"),
    ("Hide all notifications", "Ocultar todas las notificaciones", "Amaga totes les notificacions"),
    ("Editor:", "Editor:", "Editor:"),
    ("Show tooltips", "Mostrar ayudas emergentes", "Mostra els indicadors de funció"),
    ("Close the editor after uploading", "Cerrar el editor después de subir", "Tanca l'editor després de pujar"),
    ("Use default browser to sign in", "Iniciar sesión en el navegador", "Inicia la sessió al navegador"),
    ("Video record hotkeys:", "Atajos de grabación de vídeo:", "Dreceres d'enregistrament de vídeo:"),
    ("Default", "Por defecto", "Per defecte"),
    ("Restore the default hotkeys", "Restaurar los atajos por defecto", "Restaura les dreceres per defecte"),
    ("Press hotkey with ⌘/⌃", "Pulsa un atajo con ⌘/⌃", "Prem una drecera amb ⌘/⌃"),
    ("Delete hotkey", "Eliminar atajo", "Elimina la drecera"),
    ("Main font:", "Tipo de letra principal:", "Tipus de lletra principal:"),
    ("Additional fonts:", "Tipos de letra adicionales:", "Tipus de lletra addicionals:"),
    ("No additional fonts", "Sin tipos de letra adicionales", "Cap tipus de lletra addicional"),
    ("Add font", "Añadir tipo de letra", "Afegeix un tipus de lletra"),
    ("Font", "Tipo de letra", "Tipus de lletra"),
    ("Remove font", "Quitar tipo de letra", "Treu el tipus de lletra"),
    ("Attention: some fonts could not correctly displayed in the editor.", "Atención: algunos tipos de letra pueden no verse bien en el editor.", "Atenció: alguns tipus de lletra poden no veure's bé a l'editor."),
    ("Reset fonts", "Restablecer tipos de letra", "Restableix els tipus de lletra"),
    ("Image", "Imagen", "Imatge"),
    ("Path", "Ruta", "Camí"),
    ("Open {} editor", "Abrir el editor de {}", "Obre l'editor de {}"),
    ("Save screenshot", "Guardar la captura", "Desa la captura"),
    ("Upload screenshot", "Subir la captura", "Puja la captura"),
    ("Open external editor", "Abrir el editor externo", "Obre l'editor extern"),
    ("Copy image to clipboard", "Copiar la imagen al portapapeles", "Copia la imatge al porta-retalls"),
    ("Capture frozen area:", "Captura de área congelada:", "Captura d'àrea congelada:"),
    ("Capture area & Upload:", "Captura de área y subida:", "Captura d'àrea i pujada:"),
    ("Capture fullscreen:", "Captura de pantalla completa:", "Captura de pantalla completa:"),
    ("Fullscreen after 10s:", "Pantalla completa en 10 s:", "Pantalla completa d'aquí a 10 s:"),
    ("Open Latest Image:", "Abrir la última imagen:", "Obre l'última imatge:"),
    ("Record video:", "Grabar vídeo:", "Enregistra un vídeo:"),
    ("Capture previous area:", "Capturar el área anterior:", "Captura l'àrea anterior:"),
    ("Open from clipboard:", "Abrir desde el portapapeles:", "Obre des del porta-retalls:"),
    ("Record/Pause:", "Grabar/Pausa:", "Enregistra/Pausa:"),
    ("Stop:", "Detener:", "Atura:"),
    ("Arrow:", "Flecha:", "Fletxa:"),
    ("Rectangle:", "Rectángulo:", "Rectangle:"),
    ("Fullscreen / Windowed mode:", "Pantalla completa / ventana:", "Pantalla completa / finestra:"),
];

fn table() -> &'static HashMap<&'static str, (&'static str, &'static str)> {
    static MAP: OnceLock<HashMap<&'static str, (&'static str, &'static str)>> = OnceLock::new();
    MAP.get_or_init(|| TABLE.iter().map(|(en, es, ca)| (*en, (*es, *ca))).collect())
}

/// The text in the current language.
pub fn t(en: &'static str) -> &'static str {
    match current() {
        Lang::En => en,
        lang => match table().get(en) {
            Some((es, ca)) => {
                if lang == Lang::Es {
                    es
                } else {
                    ca
                }
            }
            None => en,
        },
    }
}

/// `t` for texts with `{}` placeholders, filled in order.
pub fn tf(en: &'static str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut rest = t(en);
    for a in args {
        match rest.find("{}") {
            Some(i) => {
                out.push_str(&rest[..i]);
                out.push_str(&a.to_string());
                rest = &rest[i + 2..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}

/// Draws the language's flag filling `r`.
pub fn paint_flag(p: &Painter, r: Rect, lang: Lang) {
    let p = p.with_clip_rect(r.intersect(p.clip_rect()));
    let (w, h) = (r.width(), r.height());
    match lang {
        Lang::Ca => {
            // the senyera: nine stripes, four of them red
            p.rect_filled(r, 0.0, Color32::from_rgb(252, 221, 9));
            let s = h / 9.0;
            for k in [1.0, 3.0, 5.0, 7.0] {
                p.rect_filled(Rect::from_min_size(pos2(r.min.x, r.min.y + k * s), vec2(w, s)), 0.0, Color32::from_rgb(218, 18, 26));
            }
        }
        Lang::Es => {
            p.rect_filled(r, 0.0, Color32::from_rgb(170, 21, 27));
            p.rect_filled(Rect::from_min_size(pos2(r.min.x, r.min.y + h / 4.0), vec2(w, h / 2.0)), 0.0, Color32::from_rgb(241, 191, 0));
        }
        Lang::En => {
            // the Union Flag, simplified for small sizes
            let blue = Color32::from_rgb(1, 33, 105);
            let red = Color32::from_rgb(200, 16, 46);
            p.rect_filled(r, 0.0, blue);
            let u = h / 30.0;
            for (a, b) in [(r.left_top(), r.right_bottom()), (r.left_bottom(), r.right_top())] {
                p.line_segment([a, b], Stroke::new(6.0 * u, Color32::WHITE));
                p.line_segment([a, b], Stroke::new(2.0 * u, red));
            }
            let c = r.center();
            p.rect_filled(Rect::from_center_size(c, vec2(w, 10.0 * u)), 0.0, Color32::WHITE);
            p.rect_filled(Rect::from_center_size(c, vec2(10.0 * u, h)), 0.0, Color32::WHITE);
            p.rect_filled(Rect::from_center_size(c, vec2(w, 6.0 * u)), 0.0, red);
            p.rect_filled(Rect::from_center_size(c, vec2(6.0 * u, h)), 0.0, red);
        }
    }
    p.rect_stroke(r, 0.0, Stroke::new(0.5_f32, Color32::from_black_alpha(60)), egui::StrokeKind::Inside);
}

const FLAG: egui::Vec2 = egui::Vec2::new(20.0, 14.0);

/// A button showing the current language's flag and name (or its two-letter
/// code when `compact`) that opens a menu with all of them. Returns true when
/// the language changed.
pub fn lang_picker(ui: &mut egui::Ui, lang: &mut Lang, compact: bool) -> bool {
    let label = if compact { lang.code() } else { lang.name() };
    let font = FontId::proportional(13.0);
    let text_w = ui.fonts(|f| f.layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE).size().x);
    let (r, resp) = ui.allocate_exact_size(vec2(8.0 + FLAG.x + 6.0 + text_w + 18.0, 28.0), Sense::click());
    let vis = ui.style().interact(&resp);
    let p = ui.painter();
    p.rect(r, 8.0, vis.weak_bg_fill, vis.bg_stroke, egui::StrokeKind::Inside);
    let fr = Rect::from_min_size(pos2(r.min.x + 8.0, r.center().y - FLAG.y / 2.0), FLAG);
    paint_flag(p, fr, *lang);
    p.text(pos2(fr.max.x + 6.0, r.center().y), Align2::LEFT_CENTER, label, font.clone(), vis.text_color());
    // a small chevron
    let c = pos2(r.max.x - 10.0, r.center().y);
    p.line_segment([c + vec2(-3.5, -1.5), c + vec2(0.0, 2.0)], Stroke::new(1.3_f32, vis.text_color()));
    p.line_segment([c + vec2(0.0, 2.0), c + vec2(3.5, -1.5)], Stroke::new(1.3_f32, vis.text_color()));
    let resp = resp.on_hover_text(t("Language"));

    let mut changed = false;
    egui::Popup::menu(&resp).show(|ui| {
        for l in Lang::ALL {
            let (rr, rsp) = ui.allocate_exact_size(vec2(150.0, 26.0), Sense::click());
            let p = ui.painter();
            if rsp.hovered() {
                p.rect_filled(rr, 6.0, ui.visuals().widgets.hovered.weak_bg_fill);
            }
            let fr = Rect::from_min_size(pos2(rr.min.x + 6.0, rr.center().y - FLAG.y / 2.0), FLAG);
            paint_flag(p, fr, l);
            let col = ui.visuals().text_color();
            p.text(pos2(fr.max.x + 8.0, rr.center().y), Align2::LEFT_CENTER, l.name(), font.clone(), col);
            if l == *lang {
                p.text(pos2(rr.max.x - 8.0, rr.center().y), Align2::RIGHT_CENTER, "✓", font.clone(), col);
            }
            if rsp.clicked() && l != *lang {
                *lang = l;
                changed = true;
            }
        }
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translations_keep_placeholders_and_fall_back_to_english() {
        for (en, es, ca) in TABLE {
            assert_eq!(en.matches("{}").count(), es.matches("{}").count(), "{en}");
            assert_eq!(en.matches("{}").count(), ca.matches("{}").count(), "{en}");
        }
        set(Lang::Es);
        assert_eq!(t("Save"), "Guardar");
        assert_eq!(tf("Saved to {}", &[&"/tmp/x.png"]), "Guardado en /tmp/x.png");
        assert_eq!(t("not in the table"), "not in the table");
        set(Lang::Ca);
        assert_eq!(t("Quit"), "Surt");
        set(Lang::En);
        assert_eq!(t("Quit"), "Quit");
    }
}
