//! Interface languages. Texts are written in English in the code and looked
//! up here with `t("…")`; a text missing from the table shows in English.
//! Flags are drawn in code (egui's fonts have no flag emoji).

use egui::{pos2, vec2, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Stroke};
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
    Fr,
    De,
    Pt,
    It,
    Gl,
    Eu,
    PtBr,
    Nl,
    Pl,
    Ja,
    Zh,
    Ko,
}

impl Lang {
    /// In the order of the translation table's columns (English first).
    const BY_INDEX: [Lang; 15] = [
        Lang::En,
        Lang::Es,
        Lang::Ca,
        Lang::Fr,
        Lang::De,
        Lang::Pt,
        Lang::It,
        Lang::Gl,
        Lang::Eu,
        Lang::PtBr,
        Lang::Nl,
        Lang::Pl,
        Lang::Ja,
        Lang::Zh,
        Lang::Ko,
    ];

    /// In the order of the language menu: English, the languages of Spain,
    /// the rest of Europe, then Asia.
    pub const ALL: [Lang; 15] = [
        Lang::En,
        Lang::Es,
        Lang::Ca,
        Lang::Gl,
        Lang::Eu,
        Lang::Fr,
        Lang::De,
        Lang::It,
        Lang::Pt,
        Lang::PtBr,
        Lang::Nl,
        Lang::Pl,
        Lang::Ja,
        Lang::Zh,
        Lang::Ko,
    ];

    /// Chinese, Japanese and Korean need a font of their own (see app.rs).
    pub fn is_cjk(self) -> bool {
        matches!(self, Lang::Ja | Lang::Zh | Lang::Ko)
    }

    /// The language's own name for itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::Ca => "Català",
            Lang::Fr => "Français",
            Lang::De => "Deutsch",
            Lang::Pt => "Português",
            Lang::It => "Italiano",
            Lang::Gl => "Galego",
            Lang::Eu => "Euskara",
            Lang::PtBr => "Português (Brasil)",
            Lang::Nl => "Nederlands",
            Lang::Pl => "Polski",
            Lang::Ja => "日本語",
            Lang::Zh => "简体中文",
            Lang::Ko => "한국어",
        }
    }

    /// The language's name in `ui`, the language the interface is in.
    pub fn name_in(self, ui: Lang) -> &'static str {
        NAMES[ui as usize][self as usize]
    }

    /// Every language, sorted alphabetically by its name in `ui` (the order
    /// changes with the interface language). Japanese sorts by the kana
    /// reading, Chinese by pinyin, Korean by hangul order, the others
    /// ignoring accents.
    pub fn sorted_in(ui: Lang) -> Vec<Lang> {
        let mut v = Lang::ALL.to_vec();
        v.sort_by_cached_key(|l| match ui {
            Lang::Ja => JA_READINGS[*l as usize].to_string(),
            Lang::Zh => ZH_PINYIN[*l as usize].to_string(),
            _ => sort_key(l.name_in(ui)),
        });
        v
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "EN",
            Lang::Es => "ES",
            Lang::Ca => "CA",
            Lang::Fr => "FR",
            Lang::De => "DE",
            Lang::Pt => "PT",
            Lang::It => "IT",
            Lang::Gl => "GL",
            Lang::Eu => "EU",
            Lang::PtBr => "BR",
            Lang::Nl => "NL",
            Lang::Pl => "PL",
            Lang::Ja => "JA",
            Lang::Zh => "ZH",
            Lang::Ko => "KO",
        }
    }
}

/// Language names: one row per interface language, one column per language
/// named, both in the order of `Lang::BY_INDEX` (en es ca fr de pt it gl eu
/// pt-BR nl pl ja zh ko).
const NAMES: [[&str; 15]; 15] = [
    ["English", "Spanish", "Catalan", "French", "German", "Portuguese (Portugal)", "Italian", "Galician", "Basque", "Portuguese (Brazil)", "Dutch", "Polish", "Japanese", "Chinese (Simplified)", "Korean"],
    ["Inglés", "Español", "Catalán", "Francés", "Alemán", "Portugués (Portugal)", "Italiano", "Gallego", "Euskera", "Portugués (Brasil)", "Neerlandés", "Polaco", "Japonés", "Chino simplificado", "Coreano"],
    ["Anglès", "Espanyol", "Català", "Francès", "Alemany", "Portuguès (Portugal)", "Italià", "Gallec", "Basc", "Portuguès (Brasil)", "Neerlandès", "Polonès", "Japonès", "Xinès simplificat", "Coreà"],
    ["Anglais", "Espagnol", "Catalan", "Français", "Allemand", "Portugais (Portugal)", "Italien", "Galicien", "Basque", "Portugais (Brésil)", "Néerlandais", "Polonais", "Japonais", "Chinois simplifié", "Coréen"],
    ["Englisch", "Spanisch", "Katalanisch", "Französisch", "Deutsch", "Portugiesisch (Portugal)", "Italienisch", "Galicisch", "Baskisch", "Portugiesisch (Brasilien)", "Niederländisch", "Polnisch", "Japanisch", "Chinesisch (vereinfacht)", "Koreanisch"],
    ["Inglês", "Espanhol", "Catalão", "Francês", "Alemão", "Português (Portugal)", "Italiano", "Galego", "Basco", "Português (Brasil)", "Neerlandês", "Polaco", "Japonês", "Chinês simplificado", "Coreano"],
    ["Inglese", "Spagnolo", "Catalano", "Francese", "Tedesco", "Portoghese (Portogallo)", "Italiano", "Galiziano", "Basco", "Portoghese (Brasile)", "Olandese", "Polacco", "Giapponese", "Cinese semplificato", "Coreano"],
    ["Inglés", "Español", "Catalán", "Francés", "Alemán", "Portugués (Portugal)", "Italiano", "Galego", "Éuscaro", "Portugués (Brasil)", "Neerlandés", "Polaco", "Xaponés", "Chinés simplificado", "Coreano"],
    ["Ingelesa", "Gaztelania", "Katalana", "Frantsesa", "Alemana", "Portugesa (Portugal)", "Italiera", "Galiziera", "Euskara", "Portugesa (Brasil)", "Nederlandera", "Poloniera", "Japoniera", "Txinera sinplifikatua", "Koreera"],
    ["Inglês", "Espanhol", "Catalão", "Francês", "Alemão", "Português (Portugal)", "Italiano", "Galego", "Basco", "Português (Brasil)", "Holandês", "Polonês", "Japonês", "Chinês simplificado", "Coreano"],
    ["Engels", "Spaans", "Catalaans", "Frans", "Duits", "Portugees (Portugal)", "Italiaans", "Galicisch", "Baskisch", "Portugees (Brazilië)", "Nederlands", "Pools", "Japans", "Chinees (vereenvoudigd)", "Koreaans"],
    ["Angielski", "Hiszpański", "Kataloński", "Francuski", "Niemiecki", "Portugalski (Portugalia)", "Włoski", "Galicyjski", "Baskijski", "Portugalski (Brazylia)", "Niderlandzki", "Polski", "Japoński", "Chiński uproszczony", "Koreański"],
    ["英語", "スペイン語", "カタルーニャ語", "フランス語", "ドイツ語", "ポルトガル語(ポルトガル)", "イタリア語", "ガリシア語", "バスク語", "ポルトガル語(ブラジル)", "オランダ語", "ポーランド語", "日本語", "簡体中国語", "韓国語"],
    ["英语", "西班牙语", "加泰罗尼亚语", "法语", "德语", "葡萄牙语(葡萄牙)", "意大利语", "加利西亚语", "巴斯克语", "葡萄牙语(巴西)", "荷兰语", "波兰语", "日语", "简体中文", "韩语"],
    ["영어", "스페인어", "카탈루냐어", "프랑스어", "독일어", "포르투갈어(포르투갈)", "이탈리아어", "갈리시아어", "바스크어", "포르투갈어(브라질)", "네덜란드어", "폴란드어", "일본어", "중국어(간체)", "한국어"],
];

/// Kana readings of the Japanese names (dictionary order goes by reading),
/// written without voicing marks (ガ as カ, ド as ト…) and with long vowels
/// spelled out, as Japanese dictionary order treats them.
const JA_READINGS: [&str; 15] = [
    "えいこ", "すへいんこ", "かたるうにやこ", "ふらんすこ", "といつこ", "ほるとかるこ ほるとかる", "いたりあこ", "かりしあこ", "はすくこ",
    "ほるとかるこ ふらしる", "おらんたこ", "ほおらんとこ", "にほんこ", "かんたいちゆうこくこ", "かんこくこ",
];

/// Pinyin of the Chinese names (dictionary order goes by pinyin).
const ZH_PINYIN: [&str; 15] = [
    "yingyu", "xibanyayu", "jiatailuoniyayu", "fayu", "deyu", "putaoyayu putaoya", "yidaliyu", "jialixiyayu", "basikeyu",
    "putaoyayu baxi", "helanyu", "bolanyu", "riyu", "jiantizhongwen", "hanyu",
];

/// Sort key that ignores case and accents (á, ç, ł, ß…), so "Éuscaro"
/// sorts with the E's.
fn sort_key(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().flat_map(char::to_lowercase) {
        let base = match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ą' => "a",
            'ç' | 'ć' => "c",
            'é' | 'è' | 'ê' | 'ë' | 'ę' => "e",
            'í' | 'ì' | 'î' | 'ï' => "i",
            'ñ' | 'ń' => "n",
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => "o",
            'ú' | 'ù' | 'û' | 'ü' => "u",
            'ł' => "l",
            'ś' => "s",
            'ż' | 'ź' => "z",
            'ß' => "ss",
            '·' => "",
            _ => {
                out.push(c);
                continue;
            }
        };
        out.push_str(base);
    }
    out
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    let i = CURRENT.load(Ordering::Relaxed) as usize;
    Lang::BY_INDEX.get(i).copied().unwrap_or(Lang::En)
}

/// English text → its translations, in the order of `Lang::BY_INDEX` after
/// English: Spanish (Spain), Catalan, French, German, Portuguese (Portugal),
/// Italian, Galician, Basque, Portuguese (Brazil), Dutch, Polish, Japanese,
/// Chinese (simplified) and Korean.
const TABLE: &[(&str, [&str; 14])] = &[
    ("Show {}", ["Mostrar {}", "Mostra {}", "Afficher {}", "{} anzeigen", "Mostrar {}", "Mostra {}", "Amosar {}", "{} erakutsi", "Mostrar {}", "Toon {}", "Pokaż {}", "{}を表示", "显示 {}", "{} 보기"]),
    ("Capture Area", ["Capturar área", "Captura una àrea", "Capturer une zone", "Bereich aufnehmen", "Capturar área", "Cattura un'area", "Capturar área", "Kapturatu area", "Capturar área", "Leg gebied vast", "Zrzut obszaru", "範囲をキャプチャ", "截取区域", "영역 캡처"]),
    ("Capture Fullscreen", ["Capturar pantalla completa", "Captura la pantalla completa", "Capturer l'écran entier", "Ganzer Bildschirm", "Capturar ecrã inteiro", "Cattura lo schermo intero", "Capturar toda a pantalla", "Kapturatu pantaila osoa", "Capturar tela inteira", "Leg volledig scherm vast", "Zrzut całego ekranu", "全画面をキャプチャ", "截取全屏", "전체 화면 캡처"]),
    ("Fullscreen after 10s", ["Pantalla completa en 10 s", "Pantalla completa d'aquí a 10 s", "Écran entier dans 10 s", "Ganzer Bildschirm in 10 s", "Ecrã inteiro daqui a 10 s", "Schermo intero tra 10 s", "Pantalla completa en 10 s", "Pantaila osoa 10 s barru", "Tela inteira em 10 s", "Volledig scherm na 10 s", "Cały ekran po 10 s", "10秒後に全画面", "10 秒后截取全屏", "10초 후 전체 화면"]),
    ("Capture Frozen Area", ["Capturar área congelada", "Captura una àrea congelada", "Capturer une zone figée", "Eingefrorener Bereich", "Capturar área congelada", "Cattura un'area congelata", "Capturar área conxelada", "Kapturatu area izoztua", "Capturar área congelada", "Leg bevroren gebied vast", "Zrzut zamrożonego obszaru", "静止画面で範囲を撮影", "截取冻结区域", "정지 화면 영역 캡처"]),
    ("Record Video", ["Grabar vídeo", "Enregistra un vídeo", "Enregistrer une vidéo", "Video aufnehmen", "Gravar vídeo", "Registra un video", "Gravar vídeo", "Grabatu bideoa", "Gravar vídeo", "Neem video op", "Nagraj wideo", "ビデオを収録", "录制视频", "비디오 녹화"]),
    ("Stop Recording", ["Detener grabación", "Atura l'enregistrament", "Arrêter l'enregistrement", "Aufnahme beenden", "Parar a gravação", "Interrompi la registrazione", "Deter a gravación", "Gelditu grabazioa", "Parar gravação", "Stop opname", "Zatrzymaj nagrywanie", "収録を停止", "停止录制", "녹화 중단"]),
    ("Open…", ["Abrir…", "Obre…", "Ouvrir…", "Öffnen…", "Abrir…", "Apri…", "Abrir…", "Ireki…", "Abrir…", "Open…", "Otwórz…", "開く…", "打开…", "열기…"]),
    ("Open from Clipboard", ["Abrir desde el portapapeles", "Obre des del porta-retalls", "Ouvrir depuis le presse-papiers", "Aus Zwischenablage öffnen", "Abrir da área de transferência", "Apri dagli appunti", "Abrir do portapapeis", "Ireki arbeletik", "Abrir da Área de Transferência", "Open van klembord", "Otwórz ze schowka", "クリップボードから開く", "从剪贴板打开", "클립보드에서 열기"]),
    ("Open Latest Image", ["Abrir la última imagen", "Obre l'última imatge", "Ouvrir la dernière image", "Letztes Bild öffnen", "Abrir a última imagem", "Apri l'ultima immagine", "Abrir a última imaxe", "Ireki azken irudia", "Abrir última imagem", "Open laatste afbeelding", "Otwórz ostatni obraz", "最新のイメージを開く", "打开最新图像", "최근 이미지 열기"]),
    ("Preferences…", ["Ajustes…", "Configuració…", "Réglages…", "Einstellungen…", "Definições…", "Impostazioni…", "Axustes…", "Hobespenak…", "Ajustes…", "Instellingen…", "Ustawienia…", "設定…", "设置…", "설정…"]),
    ("Quit", ["Salir", "Surt", "Quitter", "Beenden", "Sair", "Esci", "Saír", "Irten", "Encerrar", "Stop", "Zakończ", "終了", "退出", "종료"]),
    ("Recording the screen…", ["Grabando la pantalla…", "S'està enregistrant la pantalla…", "Enregistrement de l'écran…", "Bildschirm wird aufgenommen…", "A gravar o ecrã…", "Registrazione dello schermo…", "Gravando a pantalla…", "Pantaila grabatzen…", "Gravando a tela…", "Scherm wordt opgenomen…", "Nagrywanie ekranu…", "画面を収録中…", "正在录制屏幕…", "화면 녹화 중…"]),
    ("Video saved to {}", ["Vídeo guardado en {}", "Vídeo desat a {}", "Vidéo enregistrée dans {}", "Video gespeichert unter {}", "Vídeo guardado em {}", "Video salvato in {}", "Vídeo gardado en {}", "Bideoa hemen gorde da: {}", "Vídeo salvo em {}", "Video bewaard in {}", "Zapisano wideo w {}", "ビデオを{}に保存しました", "视频已存储到 {}", "비디오를 {}에 저장함"]),
    ("Open", ["Abrir", "Obre", "Ouvrir", "Öffnen", "Abrir", "Apri", "Abrir", "Ireki", "Abrir", "Open", "Otwórz", "開く", "打开", "열기"]),
    ("Images", ["Imágenes", "Imatges", "Images", "Bilder", "Imagens", "Immagini", "Imaxes", "Irudiak", "Imagens", "Afbeeldingen", "Obrazy", "イメージ", "图像", "이미지"]),
    ("Can't open this file: {}", ["No se puede abrir este archivo: {}", "No es pot obrir aquest fitxer: {}", "Impossible d'ouvrir ce fichier : {}", "Diese Datei kann nicht geöffnet werden: {}", "Não é possível abrir este ficheiro: {}", "Impossibile aprire questo file: {}", "Non se pode abrir este ficheiro: {}", "Ezin da fitxategi hau ireki: {}", "Não é possível abrir este arquivo: {}", "Kan dit bestand niet openen: {}", "Nie można otworzyć pliku: {}", "このファイルを開けません: {}", "无法打开此文件：{}", "이 파일을 열 수 없음: {}"]),
    ("There is no screenshot yet", ["Todavía no hay ninguna captura", "Encara no hi ha cap captura", "Il n'y a encore aucune capture", "Es gibt noch kein Bildschirmfoto", "Ainda não há nenhuma captura", "Non c'è ancora nessuna istantanea", "Aínda non hai ningunha captura", "Oraindik ez dago pantaila-argazkirik", "Ainda não há nenhuma captura de tela", "Er is nog geen schermafbeelding", "Nie ma jeszcze zrzutu ekranu", "スクリーンショットはまだありません", "还没有截屏", "아직 스크린샷이 없습니다"]),
    ("Saved to {}", ["Guardado en {}", "Desat a {}", "Enregistré dans {}", "Gespeichert unter {}", "Guardado em {}", "Salvato in {}", "Gardado en {}", "Hemen gorde da: {}", "Salvo em {}", "Bewaard in {}", "Zapisano w {}", "{}に保存しました", "已存储到 {}", "{}에 저장함"]),
    ("Save image", ["Guardar imagen", "Desa la imatge", "Enregistrer l'image", "Bild speichern", "Guardar imagem", "Salva l'immagine", "Gardar imaxe", "Gorde irudia", "Salvar imagem", "Bewaar afbeelding", "Zachowaj obraz", "イメージを保存", "存储图像", "이미지 저장"]),
    ("Image copied to clipboard", ["Imagen copiada al portapapeles", "Imatge copiada al porta-retalls", "Image copiée dans le presse-papiers", "Bild in die Zwischenablage kopiert", "Imagem copiada para a área de transferência", "Immagine copiata negli appunti", "Imaxe copiada no portapapeis", "Irudia arbelean kopiatu da", "Imagem copiada para a Área de Transferência", "Afbeelding gekopieerd naar klembord", "Skopiowano obraz do schowka", "イメージをクリップボードにコピーしました", "图像已拷贝到剪贴板", "이미지를 클립보드에 복사함"]),
    ("Take a screenshot, mark it up, save or copy it.", ["Haz una captura, anótala y guárdala o cópiala.", "Fes una captura, anota-la i desa-la o copia-la.", "Faites une capture, annotez-la, enregistrez-la ou copiez-la.", "Bildschirmfoto aufnehmen, beschriften, speichern oder kopieren.", "Faça uma captura, anote-a, guarde-a ou copie-a.", "Fai un'istantanea, annotala, salvala o copiala.", "Fai unha captura, anótaa e gárdaa ou cópiaa.", "Atera pantaila-argazki bat, egin oharrak eta gorde edo kopiatu.", "Faça uma captura de tela, anote-a e salve ou copie.", "Maak een schermafbeelding, annoteer, bewaar of kopieer hem.", "Zrób zrzut ekranu, dodaj adnotacje, zachowaj lub skopiuj.", "スクリーンショットを撮って、注釈を付け、保存またはコピーできます。", "截屏、标注，然后存储或拷贝。", "스크린샷을 찍고, 주석을 달고, 저장하거나 복사하십시오."]),
    ("Not set", ["Sin asignar", "Sense assignar", "Non défini", "Nicht festgelegt", "Não definido", "Non impostato", "Sen asignar", "Esleitu gabe", "Não definido", "Niet ingesteld", "Nie ustawiono", "未設定", "未设置", "설정 안 됨"]),
    ("The same commands are in the menu-bar icon.", ["Los mismos comandos están en el icono de la barra de menús.", "Les mateixes ordres són a la icona de la barra de menús.", "Les mêmes commandes se trouvent dans l'icône de la barre des menus.", "Dieselben Befehle findest du im Symbol in der Menüleiste.", "Os mesmos comandos estão no ícone da barra de menus.", "Gli stessi comandi sono nell'icona della barra dei menu.", "Os mesmos comandos están na icona da barra de menús.", "Komando berak menu-barrako ikonoan daude.", "Os mesmos comandos estão no ícone da barra de menus.", "Dezelfde opdrachten vind je in het menubalksymbool.", "Te same polecenia są dostępne w ikonie na pasku menu.", "同じコマンドはメニューバーのアイコンにもあります。", "菜单栏图标中也有相同的命令。", "같은 명령을 메뉴 막대 아이콘에서도 사용할 수 있습니다."]),
    ("Recording {}", ["Grabando {}", "Enregistrant {}", "Enregistrement {}", "Aufnahme {}", "A gravar {}", "Registrazione {}", "Gravando {}", "Grabatzen {}", "Gravando {}", "Opname {}", "Nagrywanie {}", "{}を収録中", "正在录制 {}", "{} 녹화 중"]),
    ("Stop", ["Detener", "Atura", "Arrêter", "Stoppen", "Parar", "Interrompi", "Deter", "Gelditu", "Parar", "Stop", "Zatrzymaj", "停止", "停止", "중단"]),
    ("All settings have been reset", ["Se han restablecido todos los ajustes", "S'ha restablert tota la configuració", "Tous les réglages ont été réinitialisés", "Alle Einstellungen wurden zurückgesetzt", "Todas as definições foram repostas", "Tutte le impostazioni sono state ripristinate", "Restabelecéronse todos os axustes", "Ezarpen guztiak berrezarri dira", "Todos os ajustes foram redefinidos", "Alle instellingen zijn hersteld", "Zresetowano wszystkie ustawienia", "すべての設定をリセットしました", "所有设置已重置", "모든 설정이 재설정되었습니다"]),
    ("About {}", ["Acerca de {}", "Quant a {}", "À propos de {}", "Über {}", "Acerca de {}", "Informazioni su {}", "Sobre {}", "{} aplikazioari buruz", "Sobre o {}", "Over {}", "O programie {}", "{}について", "关于 {}", "{}에 관하여"]),
    ("Version {}", ["Versión {}", "Versió {}", "Version {}", "Version {}", "Versão {}", "Versione {}", "Versión {}", "{} bertsioa", "Versão {}", "Versie {}", "Wersja {}", "バージョン {}", "版本 {}", "버전 {}"]),
    ("Screenshots, annotations and screen recordings.", ["Capturas, anotaciones y grabaciones de pantalla.", "Captures, anotacions i enregistraments de pantalla.", "Captures d'écran, annotations et enregistrements de l'écran.", "Bildschirmfotos, Anmerkungen und Bildschirmaufnahmen.", "Capturas de ecrã, anotações e gravações do ecrã.", "Istantanee, annotazioni e registrazioni dello schermo.", "Capturas, anotacións e gravacións de pantalla.", "Pantaila-argazkiak, oharrak eta pantaila-grabazioak.", "Capturas de tela, anotações e gravações de tela.", "Schermafbeeldingen, annotaties en schermopnamen.", "Zrzuty ekranu, adnotacje i nagrania ekranu.", "スクリーンショット、注釈、画面収録。", "截屏、标注与屏幕录制。", "스크린샷, 주석 및 화면 녹화."]),
    ("Language", ["Idioma", "Idioma", "Langue", "Sprache", "Idioma", "Lingua", "Idioma", "Hizkuntza", "Idioma", "Taal", "Język", "言語", "语言", "언어"]),
    ("Language:", ["Idioma:", "Idioma:", "Langue :", "Sprache:", "Idioma:", "Lingua:", "Idioma:", "Hizkuntza:", "Idioma:", "Taal:", "Język:", "言語:", "语言：", "언어:"]),
    ("Could not start the capture tool: {}", ["No se ha podido iniciar la herramienta de captura: {}", "No s'ha pogut iniciar l'eina de captura: {}", "Impossible de lancer l'outil de capture : {}", "Das Aufnahmewerkzeug konnte nicht gestartet werden: {}", "Não foi possível iniciar a ferramenta de captura: {}", "Impossibile avviare lo strumento di cattura: {}", "Non se puido iniciar a ferramenta de captura: {}", "Ezin izan da kaptura-tresna abiarazi: {}", "Não foi possível iniciar a ferramenta de captura: {}", "Kan schermafbeeldingstool niet starten: {}", "Nie można uruchomić narzędzia do zrzutów: {}", "キャプチャツールを起動できませんでした: {}", "无法启动截屏工具：{}", "캡처 도구를 시작할 수 없음: {}"]),
    ("Could not read the capture: {}", ["No se ha podido leer la captura: {}", "No s'ha pogut llegir la captura: {}", "Impossible de lire la capture : {}", "Die Aufnahme konnte nicht gelesen werden: {}", "Não foi possível ler a captura: {}", "Impossibile leggere l'istantanea: {}", "Non se puido ler a captura: {}", "Ezin izan da kaptura irakurri: {}", "Não foi possível ler a captura: {}", "Kan de schermafbeelding niet lezen: {}", "Nie można odczytać zrzutu: {}", "キャプチャを読み込めませんでした: {}", "无法读取截屏：{}", "캡처를 읽을 수 없음: {}"]),
    ("Can't create {}: {}", ["No se puede crear {}: {}", "No es pot crear {}: {}", "Impossible de créer {} : {}", "{} kann nicht erstellt werden: {}", "Não é possível criar {}: {}", "Impossibile creare {}: {}", "Non se pode crear {}: {}", "Ezin da {} sortu: {}", "Não é possível criar {}: {}", "Kan {} niet aanmaken: {}", "Nie można utworzyć {}: {}", "{}を作成できません: {}", "无法创建 {}：{}", "{}을(를) 생성할 수 없음: {}"]),
    ("Screenshot copied to clipboard", ["Captura copiada al portapapeles", "Captura copiada al porta-retalls", "Capture copiée dans le presse-papiers", "Bildschirmfoto in die Zwischenablage kopiert", "Captura copiada para a área de transferência", "Istantanea copiata negli appunti", "Captura copiada no portapapeis", "Pantaila-argazkia arbelean kopiatu da", "Captura de tela copiada para a Área de Transferência", "Schermafbeelding gekopieerd naar klembord", "Skopiowano zrzut ekranu do schowka", "スクリーンショットをクリップボードにコピーしました", "截屏已拷贝到剪贴板", "스크린샷을 클립보드에 복사함"]),
    ("Path copied to clipboard", ["Ruta copiada al portapapeles", "Camí copiat al porta-retalls", "Chemin copié dans le presse-papiers", "Pfad in die Zwischenablage kopiert", "Caminho copiado para a área de transferência", "Percorso copiato negli appunti", "Ruta copiada no portapapeis", "Bide-izena arbelean kopiatu da", "Caminho copiado para a Área de Transferência", "Pad gekopieerd naar klembord", "Skopiowano ścieżkę do schowka", "パスをクリップボードにコピーしました", "路径已拷贝到剪贴板", "경로를 클립보드에 복사함"]),
    ("Nothing to copy", ["No hay nada que copiar", "No hi ha res per copiar", "Rien à copier", "Nichts zu kopieren", "Nada para copiar", "Niente da copiare", "Non hai nada que copiar", "Ez dago kopiatzeko ezer", "Não há nada para copiar", "Niets om te kopiëren", "Nic do skopiowania", "コピーするものがありません", "没有可拷贝的内容", "복사할 항목이 없음"]),
    ("Clipboard doesn't contain image", ["El portapapeles no contiene ninguna imagen", "El porta-retalls no conté cap imatge", "Le presse-papiers ne contient aucune image", "Die Zwischenablage enthält kein Bild", "A área de transferência não contém nenhuma imagem", "Gli appunti non contengono immagini", "O portapapeis non contén ningunha imaxe", "Arbelean ez dago irudirik", "A Área de Transferência não contém imagem", "Klembord bevat geen afbeelding", "Schowek nie zawiera obrazu", "クリップボードにイメージがありません", "剪贴板中没有图像", "클립보드에 이미지가 없음"]),
    ("Could not start recording: {}", ["No se ha podido iniciar la grabación: {}", "No s'ha pogut iniciar l'enregistrament: {}", "Impossible de démarrer l'enregistrement : {}", "Die Aufnahme konnte nicht gestartet werden: {}", "Não foi possível iniciar a gravação: {}", "Impossibile avviare la registrazione: {}", "Non se puido iniciar a gravación: {}", "Ezin izan da grabazioa hasi: {}", "Não foi possível iniciar a gravação: {}", "Kan opname niet starten: {}", "Nie można rozpocząć nagrywania: {}", "収録を開始できませんでした: {}", "无法开始录制：{}", "녹화를 시작할 수 없음: {}"]),
    ("The recording was cancelled", ["Se ha cancelado la grabación", "S'ha cancel·lat l'enregistrament", "L'enregistrement a été annulé", "Die Aufnahme wurde abgebrochen", "A gravação foi cancelada", "La registrazione è stata annullata", "Cancelouse a gravación", "Grabazioa bertan behera utzi da", "A gravação foi cancelada", "De opname is geannuleerd", "Nagrywanie zostało anulowane", "収録はキャンセルされました", "录制已取消", "녹화가 취소되었습니다"]),
    ("Drag to select an area · Click to keep the whole screen · Hold ⇧ for a square · Esc to cancel", ["Arrastra para seleccionar un área · Haz clic para quedarte toda la pantalla · Mantén ⇧ para un cuadrado · Esc para cancelar", "Arrossega per seleccionar una àrea · Fes clic per quedar-te tota la pantalla · Mantén ⇧ per a un quadrat · Esc per cancel·lar", "Faites glisser pour sélectionner une zone · Cliquez pour garder tout l'écran · Maintenez ⇧ pour un carré · Échap pour annuler", "Ziehen, um einen Bereich auszuwählen · Klicken für den ganzen Bildschirm · ⇧ halten für ein Quadrat · Esc zum Abbrechen", "Arraste para selecionar uma área · Clique para ficar com o ecrã inteiro · Mantenha ⇧ para um quadrado · Esc para cancelar", "Trascina per selezionare un'area · Fai clic per tenere tutto lo schermo · Tieni premuto ⇧ per un quadrato · Esc per annullare", "Arrastra para seleccionar unha área · Fai clic para quedar con toda a pantalla · Mantén ⇧ para un cadrado · Esc para cancelar", "Arrastatu area bat hautatzeko · Egin klik pantaila osoa hartzeko · Eutsi ⇧ sakatuta karratu baterako · Esc uzteko", "Arraste para selecionar uma área · Clique para manter a tela inteira · Mantenha ⇧ para um quadrado · Esc para cancelar", "Sleep om een gebied te selecteren · Klik om het hele scherm te behouden · Houd ⇧ ingedrukt voor een vierkant · Esc om te annuleren", "Przeciągnij, aby zaznaczyć obszar · Kliknij, aby zachować cały ekran · Przytrzymaj ⇧, aby uzyskać kwadrat · Esc, aby anulować", "ドラッグで範囲を選択 · クリックで画面全体 · ⇧を押したままで正方形 · Escでキャンセル", "拖移以选择区域 · 点按以保留整个屏幕 · 按住 ⇧ 绘制正方形 · 按 Esc 取消", "드래그하여 영역 선택 · 클릭하여 전체 화면 · ⇧ 누르고 정사각형 · Esc로 취소"]),
    ("Select & move", ["Seleccionar y mover", "Selecciona i mou", "Sélectionner et déplacer", "Auswählen und bewegen", "Selecionar e mover", "Seleziona e sposta", "Seleccionar e mover", "Hautatu eta mugitu", "Selecionar e mover", "Selecteer en verplaats", "Zaznacz i przesuń", "選択と移動", "选择并移动", "선택 및 이동"]),
    ("Click an item to select it, drag to move it\nDouble-click a text to edit it · Delete removes it", ["Haz clic en un elemento para seleccionarlo y arrástralo para moverlo\nDoble clic en un texto para editarlo · Suprimir lo elimina", "Fes clic en un element per seleccionar-lo i arrossega'l per moure'l\nDoble clic en un text per editar-lo · Suprimeix l'elimina", "Cliquez sur un élément pour le sélectionner, faites-le glisser pour le déplacer\nDouble-cliquez sur un texte pour le modifier · Supprimer l'efface", "Klicke auf ein Element, um es auszuwählen, und ziehe es, um es zu bewegen\nDoppelklick auf einen Text, um ihn zu bearbeiten · Entfernen löscht ihn", "Clique num elemento para o selecionar e arraste-o para o mover\nFaça duplo clique num texto para o editar · Apagar remove-o", "Fai clic su un elemento per selezionarlo, trascinalo per spostarlo\nFai doppio clic su un testo per modificarlo · Elimina lo rimuove", "Fai clic nun elemento para seleccionalo e arrástrao para movelo\nDobre clic nun texto para editalo · Suprimir elimínao", "Egin klik elementu batean hautatzeko, eta arrastatu mugitzeko\nEgin klik bikoitza testu batean editatzeko · Ezabatu teklak kentzen du", "Clique em um item para selecioná-lo e arraste para movê-lo\nClique duas vezes em um texto para editá-lo · Delete o remove", "Klik op een onderdeel om het te selecteren, sleep om het te verplaatsen\nDubbelklik op een tekst om die te wijzigen · Delete verwijdert het", "Kliknij element, aby go zaznaczyć, przeciągnij, aby go przesunąć\nKliknij dwukrotnie tekst, aby go edytować · Delete go usuwa", "項目をクリックして選択、ドラッグして移動\nテキストをダブルクリックして編集 · Deleteで削除", "点按项目以选择，拖移以移动\n连按文本以编辑 · 按删除键将其移除", "항목을 클릭하여 선택, 드래그하여 이동\n텍스트를 이중 클릭하여 편집 · Delete로 삭제"]),
    ("Arrow", ["Flecha", "Fletxa", "Flèche", "Pfeil", "Seta", "Freccia", "Frecha", "Gezia", "Seta", "Pijl", "Strzałka", "矢印", "箭头", "화살표"]),
    ("Hold ⇧ to draw at 45°\nHold ⌘ to draw from arrow’s head to tail", ["Mantén ⇧ para dibujar a 45°\nMantén ⌘ para dibujar de la punta a la cola", "Mantén ⇧ per dibuixar a 45°\nMantén ⌘ per dibuixar de la punta a la cua", "Maintenez ⇧ pour tracer à 45°\nMaintenez ⌘ pour tracer de la pointe vers la queue", "⇧ halten, um im 45°-Winkel zu zeichnen\n⌘ halten, um von der Spitze zum Ende zu zeichnen", "Mantenha ⇧ para desenhar a 45°\nMantenha ⌘ para desenhar da ponta para a cauda", "Tieni premuto ⇧ per disegnare a 45°\nTieni premuto ⌘ per disegnare dalla punta alla coda", "Mantén ⇧ para debuxar a 45°\nMantén ⌘ para debuxar da punta á cola", "Eutsi ⇧ sakatuta 45°-an marrazteko\nEutsi ⌘ sakatuta geziaren puntatik isatsera marrazteko", "Mantenha ⇧ para desenhar a 45°\nMantenha ⌘ para desenhar da ponta à cauda da seta", "Houd ⇧ ingedrukt om onder 45° te tekenen\nHoud ⌘ ingedrukt om van punt naar staart te tekenen", "Przytrzymaj ⇧, aby rysować pod kątem 45°\nPrzytrzymaj ⌘, aby rysować od grotu do końca", "⇧を押したままで45°単位で描画\n⌘を押したままで矢印の先端から末尾へ描画", "按住 ⇧ 以 45° 绘制\n按住 ⌘ 从箭头尖端向尾部绘制", "⇧ 누르고 45° 단위로 그리기\n⌘ 누르고 화살촉에서 꼬리 방향으로 그리기"]),
    ("Rectangle", ["Rectángulo", "Rectangle", "Rectangle", "Rechteck", "Retângulo", "Rettangolo", "Rectángulo", "Laukizuzena", "Retângulo", "Rechthoek", "Prostokąt", "長方形", "矩形", "직사각형"]),
    ("Hold ⇧ to draw a square\nHold ⌥ to dark all except selected area\nHold ⌘ to draw filled rectangle", ["Mantén ⇧ para dibujar un cuadrado\nMantén ⌥ para oscurecer todo excepto el área seleccionada\nMantén ⌘ para dibujar un rectángulo relleno", "Mantén ⇧ per dibuixar un quadrat\nMantén ⌥ per enfosquir-ho tot excepte l'àrea seleccionada\nMantén ⌘ per dibuixar un rectangle ple", "Maintenez ⇧ pour tracer un carré\nMaintenez ⌥ pour assombrir tout sauf la zone sélectionnée\nMaintenez ⌘ pour tracer un rectangle plein", "⇧ halten für ein Quadrat\n⌥ halten, um alles außer dem ausgewählten Bereich abzudunkeln\n⌘ halten für ein gefülltes Rechteck", "Mantenha ⇧ para desenhar um quadrado\nMantenha ⌥ para escurecer tudo exceto a área selecionada\nMantenha ⌘ para desenhar um retângulo preenchido", "Tieni premuto ⇧ per disegnare un quadrato\nTieni premuto ⌥ per scurire tutto tranne l'area selezionata\nTieni premuto ⌘ per disegnare un rettangolo pieno", "Mantén ⇧ para debuxar un cadrado\nMantén ⌥ para escurecer todo agás a área seleccionada\nMantén ⌘ para debuxar un rectángulo recheo", "Eutsi ⇧ sakatuta karratu bat marrazteko\nEutsi ⌥ sakatuta hautatutako area izan ezik dena iluntzeko\nEutsi ⌘ sakatuta laukizuzen bete bat marrazteko", "Mantenha ⇧ para desenhar um quadrado\nMantenha ⌥ para escurecer tudo exceto a área selecionada\nMantenha ⌘ para desenhar um retângulo preenchido", "Houd ⇧ ingedrukt voor een vierkant\nHoud ⌥ ingedrukt om alles buiten het gebied te verduisteren\nHoud ⌘ ingedrukt voor een gevulde rechthoek", "Przytrzymaj ⇧, aby narysować kwadrat\nPrzytrzymaj ⌥, aby przyciemnić wszystko poza zaznaczeniem\nPrzytrzymaj ⌘, aby narysować wypełniony prostokąt", "⇧を押したままで正方形を描画\n⌥を押したままで選択範囲以外を暗くする\n⌘を押したままで塗りつぶした長方形を描画", "按住 ⇧ 绘制正方形\n按住 ⌥ 使所选区域以外的部分变暗\n按住 ⌘ 绘制填充矩形", "⇧ 누르고 정사각형 그리기\n⌥ 누르고 선택 영역 외 모두 어둡게\n⌘ 누르고 채워진 직사각형 그리기"]),
    ("Line", ["Línea", "Línia", "Ligne", "Linie", "Linha", "Linea", "Liña", "Lerroa", "Linha", "Lijn", "Linia", "直線", "直线", "선"]),
    ("Hold ⇧ to draw at 45°", ["Mantén ⇧ para dibujar a 45°", "Mantén ⇧ per dibuixar a 45°", "Maintenez ⇧ pour tracer à 45°", "⇧ halten, um im 45°-Winkel zu zeichnen", "Mantenha ⇧ para desenhar a 45°", "Tieni premuto ⇧ per disegnare a 45°", "Mantén ⇧ para debuxar a 45°", "Eutsi ⇧ sakatuta 45°-an marrazteko", "Mantenha ⇧ para desenhar a 45°", "Houd ⇧ ingedrukt om onder 45° te tekenen", "Przytrzymaj ⇧, aby rysować pod kątem 45°", "⇧を押したままで45°単位で描画", "按住 ⇧ 以 45° 绘制", "⇧ 누르고 45° 단위로 그리기"]),
    ("Ellipse", ["Elipse", "El·lipse", "Ellipse", "Ellipse", "Elipse", "Ellisse", "Elipse", "Elipsea", "Elipse", "Ellips", "Elipsa", "楕円", "椭圆", "타원"]),
    ("Hold ⇧ to draw a circle\nHold ⌘ to draw filled ellipse", ["Mantén ⇧ para dibujar un círculo\nMantén ⌘ para dibujar una elipse rellena", "Mantén ⇧ per dibuixar un cercle\nMantén ⌘ per dibuixar una el·lipse plena", "Maintenez ⇧ pour tracer un cercle\nMaintenez ⌘ pour tracer une ellipse pleine", "⇧ halten für einen Kreis\n⌘ halten für eine gefüllte Ellipse", "Mantenha ⇧ para desenhar um círculo\nMantenha ⌘ para desenhar uma elipse preenchida", "Tieni premuto ⇧ per disegnare un cerchio\nTieni premuto ⌘ per disegnare un'ellisse piena", "Mantén ⇧ para debuxar un círculo\nMantén ⌘ para debuxar unha elipse rechea", "Eutsi ⇧ sakatuta zirkulu bat marrazteko\nEutsi ⌘ sakatuta elipse bete bat marrazteko", "Mantenha ⇧ para desenhar um círculo\nMantenha ⌘ para desenhar uma elipse preenchida", "Houd ⇧ ingedrukt voor een cirkel\nHoud ⌘ ingedrukt voor een gevulde ellips", "Przytrzymaj ⇧, aby narysować okrąg\nPrzytrzymaj ⌘, aby narysować wypełnioną elipsę", "⇧を押したままで円を描画\n⌘を押したままで塗りつぶした楕円を描画", "按住 ⇧ 绘制圆形\n按住 ⌘ 绘制填充椭圆", "⇧ 누르고 원 그리기\n⌘ 누르고 채워진 타원 그리기"]),
    ("Pen", ["Lápiz", "Llapis", "Crayon", "Stift", "Lápis", "Matita", "Lapis", "Arkatza", "Caneta", "Pen", "Pióro", "ペン", "画笔", "펜"]),
    ("Hold ⌘ to convert into highlighter", ["Mantén ⌘ para convertirlo en rotulador fluorescente", "Mantén ⌘ per convertir-lo en retolador fluorescent", "Maintenez ⌘ pour le transformer en surligneur", "⌘ halten, um ihn in einen Textmarker zu verwandeln", "Mantenha ⌘ para o transformar em marcador", "Tieni premuto ⌘ per trasformarla in evidenziatore", "Mantén ⌘ para convertelo en marcador fluorescente", "Eutsi ⌘ sakatuta nabarmentzaile bihurtzeko", "Mantenha ⌘ para usar como marca-texto", "Houd ⌘ ingedrukt voor een markeerstift", "Przytrzymaj ⌘, aby zmienić w zakreślacz", "⌘を押したままで蛍光ペンに切り替え", "按住 ⌘ 转为荧光笔", "⌘ 누르고 형광펜으로 전환"]),
    ("Text", ["Texto", "Text", "Texte", "Text", "Texto", "Testo", "Texto", "Testua", "Texto", "Tekst", "Tekst", "テキスト", "文本", "텍스트"]),
    ("Click to type · click an existing text to edit it", ["Haz clic para escribir · haz clic en un texto existente para editarlo", "Fes clic per escriure · fes clic en un text existent per editar-lo", "Cliquez pour écrire · cliquez sur un texte existant pour le modifier", "Klicken zum Schreiben · auf einen vorhandenen Text klicken, um ihn zu bearbeiten", "Clique para escrever · clique num texto existente para o editar", "Fai clic per scrivere · fai clic su un testo esistente per modificarlo", "Fai clic para escribir · fai clic nun texto existente para editalo", "Egin klik idazteko · egin klik lehendik dagoen testu batean editatzeko", "Clique para digitar · clique em um texto existente para editá-lo", "Klik om te typen · klik op bestaande tekst om die te wijzigen", "Kliknij, aby pisać · kliknij istniejący tekst, aby go edytować", "クリックして入力 · 既存のテキストをクリックして編集", "点按以输入 · 点按现有文本以编辑", "클릭하여 입력 · 기존 텍스트를 클릭하여 편집"]),
    ("Text with arrow", ["Texto con flecha", "Text amb fletxa", "Texte avec flèche", "Text mit Pfeil", "Texto com seta", "Testo con freccia", "Texto con frecha", "Gezidun testua", "Texto com seta", "Tekst met pijl", "Tekst ze strzałką", "矢印付きテキスト", "带箭头的文本", "화살표 텍스트"]),
    ("Drag from the point to where the note goes, then type", ["Arrastra desde el punto hasta donde va la nota y escribe", "Arrossega des del punt fins on va la nota i escriu", "Faites glisser du point jusqu'à l'emplacement de la note, puis écrivez", "Vom Punkt dorthin ziehen, wo die Notiz stehen soll, dann schreiben", "Arraste do ponto até onde fica a nota e escreva", "Trascina dal punto a dove va la nota, poi scrivi", "Arrastra desde o punto ata onde vai a nota e escribe", "Arrastatu puntutik oharra joango den lekura, eta idatzi", "Arraste do ponto até onde vai a nota e digite", "Sleep van het punt naar de plek van de notitie en typ", "Przeciągnij od punktu do miejsca notatki, a następnie pisz", "指し示す点からメモの位置までドラッグして入力", "从指向点拖移到注释位置，然后输入", "가리킬 지점에서 메모 위치로 드래그한 다음 입력"]),
    ("Blur", ["Desenfocar", "Difumina", "Flouter", "Weichzeichnen", "Desfocar", "Sfoca", "Desenfocar", "Lausotu", "Desfocar", "Vervaag", "Rozmyj", "ぼかし", "模糊", "흐리게"]),
    ("Drag over what you want to hide\nHold ⇧ for a square", ["Arrastra sobre lo que quieras ocultar\nMantén ⇧ para un cuadrado", "Arrossega per sobre del que vulguis amagar\nMantén ⇧ per a un quadrat", "Faites glisser sur ce que vous voulez masquer\nMaintenez ⇧ pour un carré", "Über das ziehen, was verborgen werden soll\n⇧ halten für ein Quadrat", "Arraste sobre o que quer ocultar\nMantenha ⇧ para um quadrado", "Trascina sopra ciò che vuoi nascondere\nTieni premuto ⇧ per un quadrato", "Arrastra sobre o que queiras ocultar\nMantén ⇧ para un cadrado", "Arrastatu ezkutatu nahi duzunaren gainetik\nEutsi ⇧ sakatuta karratu baterako", "Arraste sobre o que deseja ocultar\nMantenha ⇧ para um quadrado", "Sleep over wat je wilt verbergen\nHoud ⇧ ingedrukt voor een vierkant", "Przeciągnij nad tym, co chcesz ukryć\nPrzytrzymaj ⇧, aby uzyskać kwadrat", "隠したい部分の上をドラッグ\n⇧を押したままで正方形", "在要隐藏的内容上拖移\n按住 ⇧ 绘制正方形", "가릴 부분 위로 드래그\n⇧ 누르고 정사각형"]),
    ("Crop", ["Recortar", "Retalla", "Recadrer", "Zuschneiden", "Recortar", "Ritaglia", "Recortar", "Moztu", "Recortar", "Snij bij", "Przytnij", "切り取り", "裁剪", "자르기"]),
    ("Hold ⇧ for a square\nEnter to apply, Esc to cancel", ["Mantén ⇧ para un cuadrado\nIntro para aplicar, Esc para cancelar", "Mantén ⇧ per a un quadrat\nRetorn per aplicar, Esc per cancel·lar", "Maintenez ⇧ pour un carré\nEntrée pour appliquer, Échap pour annuler", "⇧ halten für ein Quadrat\nEingabe zum Anwenden, Esc zum Abbrechen", "Mantenha ⇧ para um quadrado\nEnter para aplicar, Esc para cancelar", "Tieni premuto ⇧ per un quadrato\nInvio per applicare, Esc per annullare", "Mantén ⇧ para un cadrado\nIntro para aplicar, Esc para cancelar", "Eutsi ⇧ sakatuta karratu baterako\nSartu aplikatzeko, Esc uzteko", "Mantenha ⇧ para um quadrado\nReturn para aplicar, Esc para cancelar", "Houd ⇧ ingedrukt voor een vierkant\nEnter om toe te passen, Esc om te annuleren", "Przytrzymaj ⇧, aby uzyskać kwadrat\nEnter, aby zastosować, Esc, aby anulować", "⇧を押したままで正方形\nEnterで適用、Escでキャンセル", "按住 ⇧ 绘制正方形\n按 Enter 应用，按 Esc 取消", "⇧ 누르고 정사각형\nEnter로 적용, Esc로 취소"]),
    ("Home", ["Inicio", "Inici", "Accueil", "Start", "Início", "Home", "Inicio", "Hasiera", "Início", "Begin", "Start", "ホーム", "主页", "홈"]),
    ("Back to the capture screen", ["Volver a la pantalla de captura", "Torna a la pantalla de captura", "Retour à l'écran de capture", "Zurück zum Aufnahmebildschirm", "Voltar ao ecrã de captura", "Torna alla schermata di cattura", "Volver á pantalla de captura", "Itzuli kaptura-pantailara", "Voltar à tela de captura", "Terug naar het vastlegscherm", "Powrót do ekranu przechwytywania", "キャプチャ画面に戻る", "返回截屏界面", "캡처 화면으로 돌아가기"]),
    ("Rotate clockwise", ["Girar a la derecha", "Gira cap a la dreta", "Faire pivoter vers la droite", "Im Uhrzeigersinn drehen", "Rodar para a direita", "Ruota in senso orario", "Xirar á dereita", "Biratu erlojuaren norabidean", "Girar em sentido horário", "Roteer rechtsom", "Obróć w prawo", "右に回転", "顺时针旋转", "오른쪽으로 회전"]),
    ("Hold ⌘ to rotate counterclockwise", ["Mantén ⌘ para girar a la izquierda", "Mantén ⌘ per girar cap a l'esquerra", "Maintenez ⌘ pour pivoter vers la gauche", "⌘ halten, um gegen den Uhrzeigersinn zu drehen", "Mantenha ⌘ para rodar para a esquerda", "Tieni premuto ⌘ per ruotare in senso antiorario", "Mantén ⌘ para xirar á esquerda", "Eutsi ⌘ sakatuta erlojuaren aurkako norabidean biratzeko", "Mantenha ⌘ para girar em sentido anti-horário", "Houd ⌘ ingedrukt om linksom te roteren", "Przytrzymaj ⌘, aby obrócić w lewo", "⌘を押したままで左に回転", "按住 ⌘ 逆时针旋转", "⌘ 누르고 왼쪽으로 회전"]),
    ("Resize", ["Cambiar tamaño", "Canvia la mida", "Redimensionner", "Größe ändern", "Redimensionar", "Ridimensiona", "Cambiar tamaño", "Aldatu tamaina", "Redimensionar", "Wijzig grootte", "Zmień wielkość", "サイズを変更", "调整大小", "크기 조절"]),
    ("Undo", ["Deshacer", "Desfés", "Annuler", "Widerrufen", "Desfazer", "Annulla", "Desfacer", "Desegin", "Desfazer", "Herstel", "Cofnij", "取り消す", "撤销", "실행 취소"]),
    ("Redo", ["Rehacer", "Refés", "Rétablir", "Wiederholen", "Refazer", "Ripeti", "Refacer", "Berregin", "Refazer", "Opnieuw", "Ponów", "やり直す", "重做", "실행 복귀"]),
    ("Copy", ["Copiar", "Copia", "Copier", "Kopieren", "Copiar", "Copia", "Copiar", "Kopiatu", "Copiar", "Kopieer", "Kopiuj", "コピー", "拷贝", "복사"]),
    ("Copy image  ⌘C", ["Copiar imagen  ⌘C", "Copia la imatge  ⌘C", "Copier l'image  ⌘C", "Bild kopieren  ⌘C", "Copiar imagem  ⌘C", "Copia l'immagine  ⌘C", "Copiar imaxe  ⌘C", "Kopiatu irudia  ⌘C", "Copiar imagem  ⌘C", "Kopieer afbeelding  ⌘C", "Kopiuj obraz  ⌘C", "イメージをコピー  ⌘C", "拷贝图像  ⌘C", "이미지 복사  ⌘C"]),
    ("Open in…", ["Abrir en…", "Obre amb…", "Ouvrir avec…", "Öffnen mit…", "Abrir com…", "Apri con…", "Abrir con…", "Ireki honekin…", "Abrir em…", "Open in…", "Otwórz w…", "このアプリで開く…", "打开方式…", "다음으로 열기…"]),
    ("External editor  ⌘E", ["Editor externo  ⌘E", "Editor extern  ⌘E", "Éditeur externe  ⌘E", "Externer Editor  ⌘E", "Editor externo  ⌘E", "Editor esterno  ⌘E", "Editor externo  ⌘E", "Kanpoko editorea  ⌘E", "Editor externo  ⌘E", "Externe editor  ⌘E", "Zewnętrzny edytor  ⌘E", "外部エディタ  ⌘E", "外部编辑器  ⌘E", "외부 편집기  ⌘E"]),
    ("Save", ["Guardar", "Desa", "Enregistrer", "Speichern", "Guardar", "Salva", "Gardar", "Gorde", "Salvar", "Bewaar", "Zachowaj", "保存", "存储", "저장"]),
    ("Save  ⌘S", ["Guardar  ⌘S", "Desa  ⌘S", "Enregistrer  ⌘S", "Speichern  ⌘S", "Guardar  ⌘S", "Salva  ⌘S", "Gardar  ⌘S", "Gorde  ⌘S", "Salvar  ⌘S", "Bewaar  ⌘S", "Zachowaj  ⌘S", "保存  ⌘S", "存储  ⌘S", "저장  ⌘S"]),
    ("Selected arrow", ["Flecha seleccionada", "Fletxa seleccionada", "Flèche sélectionnée", "Ausgewählter Pfeil", "Seta selecionada", "Freccia selezionata", "Frecha seleccionada", "Gezi hautatua", "Seta selecionada", "Gekozen pijl", "Zaznaczona strzałka", "選択中の矢印", "所选箭头", "선택한 화살표"]),
    ("Selected line", ["Línea seleccionada", "Línia seleccionada", "Ligne sélectionnée", "Ausgewählte Linie", "Linha selecionada", "Linea selezionata", "Liña seleccionada", "Lerro hautatua", "Linha selecionada", "Gekozen lijn", "Zaznaczona linia", "選択中の直線", "所选直线", "선택한 선"]),
    ("Selected rectangle", ["Rectángulo seleccionado", "Rectangle seleccionat", "Rectangle sélectionné", "Ausgewähltes Rechteck", "Retângulo selecionado", "Rettangolo selezionato", "Rectángulo seleccionado", "Laukizuzen hautatua", "Retângulo selecionado", "Gekozen rechthoek", "Zaznaczony prostokąt", "選択中の長方形", "所选矩形", "선택한 직사각형"]),
    ("Selected ellipse", ["Elipse seleccionada", "El·lipse seleccionada", "Ellipse sélectionnée", "Ausgewählte Ellipse", "Elipse selecionada", "Ellisse selezionata", "Elipse seleccionada", "Elipse hautatua", "Elipse selecionada", "Gekozen ellips", "Zaznaczona elipsa", "選択中の楕円", "所选椭圆", "선택한 타원"]),
    ("Selected drawing", ["Dibujo seleccionado", "Dibuix seleccionat", "Dessin sélectionné", "Ausgewählte Zeichnung", "Desenho selecionado", "Disegno selezionato", "Debuxo seleccionado", "Marrazki hautatua", "Desenho selecionado", "Gekozen tekening", "Zaznaczony rysunek", "選択中の描画", "所选绘图", "선택한 그림"]),
    ("Selected text", ["Texto seleccionado", "Text seleccionat", "Texte sélectionné", "Ausgewählter Text", "Texto selecionado", "Testo selezionato", "Texto seleccionado", "Testu hautatua", "Texto selecionado", "Gekozen tekst", "Zaznaczony tekst", "選択中のテキスト", "所选文本", "선택한 텍스트"]),
    ("Selected note", ["Nota seleccionada", "Nota seleccionada", "Note sélectionnée", "Ausgewählte Notiz", "Nota selecionada", "Nota selezionata", "Nota seleccionada", "Ohar hautatua", "Nota selecionada", "Gekozen notitie", "Zaznaczona notatka", "選択中のメモ", "所选注释", "선택한 메모"]),
    ("Selected blur area", ["Zona desenfocada seleccionada", "Zona difuminada seleccionada", "Zone floutée sélectionnée", "Ausgewählter Weichzeichner", "Zona desfocada selecionada", "Area sfocata selezionata", "Desenfoque seleccionado", "Lausotze hautatua", "Desfoque selecionado", "Gekozen vervaging", "Zaznaczone rozmycie", "選択中のぼかし", "所选模糊区域", "선택한 흐림 영역"]),
    ("Typing — Esc to finish", ["Escribiendo — Esc para terminar", "Escrivint — Esc per acabar", "Saisie — Échap pour terminer", "Eingabe — Esc zum Beenden", "A escrever — Esc para terminar", "Scrittura — Esc per finire", "Escribindo — Esc para rematar", "Idazten — Esc amaitzeko", "Digitando — Esc para concluir", "Typen — Esc om te stoppen", "Pisanie — Esc kończy", "入力中 — Escで終了", "正在输入 — 按 Esc 完成", "입력 중 — Esc로 종료"]),
    ("Click on the picture to type", ["Haz clic en la imagen para escribir", "Fes clic a la imatge per escriure", "Cliquez sur l'image pour écrire", "Zum Schreiben ins Bild klicken", "Clique na imagem para escrever", "Fai clic sull'immagine per scrivere", "Fai clic na imaxe para escribir", "Egin klik irudian idazteko", "Clique na imagem para digitar", "Klik op de afbeelding om te typen", "Kliknij obraz, aby pisać", "画像をクリックして入力", "点按图片以输入", "이미지를 클릭하여 입력"]),
    ("Click an item to change it", ["Haz clic en un elemento para cambiarlo", "Fes clic en un element per canviar-lo", "Cliquez sur un élément pour le modifier", "Klicke auf ein Element, um es zu ändern", "Clique num elemento para o alterar", "Fai clic su un elemento per modificarlo", "Fai clic para cambiar", "Egin klik aldatzeko", "Clique para alterar", "Klik om te wijzigen", "Kliknij, aby zmienić", "項目をクリックして変更", "点按项目以更改", "항목을 클릭하여 변경"]),
    ("New arrow", ["Nueva flecha", "Fletxa nova", "Nouvelle flèche", "Neuer Pfeil", "Nova seta", "Nuova freccia", "Nova frecha", "Gezi berria", "Nova seta", "Nieuwe pijl", "Nowa strzałka", "新規矢印", "新箭头", "새 화살표"]),
    ("New rectangle", ["Nuevo rectángulo", "Rectangle nou", "Nouveau rectangle", "Neues Rechteck", "Novo retângulo", "Nuovo rettangolo", "Novo rectángulo", "Laukizuzen berria", "Novo retângulo", "Nieuwe rechthoek", "Nowy prostokąt", "新規長方形", "新矩形", "새 직사각형"]),
    ("New line", ["Nueva línea", "Línia nova", "Nouvelle ligne", "Neue Linie", "Nova linha", "Nuova linea", "Nova liña", "Lerro berria", "Nova linha", "Nieuwe lijn", "Nowa linia", "新規直線", "新直线", "새 선"]),
    ("New ellipse", ["Nueva elipse", "El·lipse nova", "Nouvelle ellipse", "Neue Ellipse", "Nova elipse", "Nuova ellisse", "Nova elipse", "Elipse berria", "Nova elipse", "Nieuwe ellips", "Nowa elipsa", "新規楕円", "新椭圆", "새 타원"]),
    ("New drawing", ["Nuevo dibujo", "Dibuix nou", "Nouveau dessin", "Neue Zeichnung", "Novo desenho", "Nuovo disegno", "Novo debuxo", "Marrazki berria", "Novo desenho", "Nieuwe tekening", "Nowy rysunek", "新規描画", "新绘图", "새 그림"]),
    ("New text with arrow", ["Nuevo texto con flecha", "Text amb fletxa nou", "Nouveau texte avec flèche", "Neuer Text mit Pfeil", "Novo texto com seta", "Nuovo testo con freccia", "Novo texto con frecha", "Gezidun testu berria", "Novo texto com seta", "Nieuwe tekst met pijl", "Nowy tekst ze strzałką", "新規矢印付きテキスト", "新带箭头文本", "새 화살표 텍스트"]),
    ("New blur area", ["Nueva zona desenfocada", "Zona difuminada nova", "Nouvelle zone floutée", "Neuer Weichzeichner", "Nova zona desfocada", "Nuova area sfocata", "Novo desenfoque", "Lausotze berria", "Novo desfoque", "Nieuwe vervaging", "Nowe rozmycie", "新規ぼかし", "新模糊区域", "새 흐림 영역"]),
    ("Crop area", ["Área de recorte", "Àrea de retall", "Zone de recadrage", "Zuschneidebereich", "Área de recorte", "Area di ritaglio", "Área de recorte", "Mozteko area", "Área de recorte", "Bijsnijgebied", "Obszar przycięcia", "切り取り範囲", "裁剪区域", "자르기 영역"]),
    ("Drag over anything you want to hide · ⇧ for a square", ["Arrastra sobre lo que quieras ocultar · ⇧ para un cuadrado", "Arrossega per sobre del que vulguis amagar · ⇧ per a un quadrat", "Faites glisser sur ce que vous voulez masquer · ⇧ pour un carré", "Über alles ziehen, was verborgen werden soll · ⇧ für ein Quadrat", "Arraste sobre o que quer ocultar · ⇧ para um quadrado", "Trascina sopra ciò che vuoi nascondere · ⇧ per un quadrato", "Arrastra sobre o que queiras ocultar · ⇧ para un cadrado", "Arrastatu ezkutatu nahi duzunaren gainetik · ⇧ karratu baterako", "Arraste sobre o que deseja ocultar · ⇧ para um quadrado", "Sleep over wat je wilt verbergen · ⇧ voor een vierkant", "Przeciągnij nad tym, co chcesz ukryć · ⇧ – kwadrat", "隠したい部分の上をドラッグ · ⇧で正方形", "在要隐藏的内容上拖移 · ⇧ 绘制正方形", "가릴 부분 위로 드래그 · ⇧ 정사각형"]),
    ("Any colour", ["Cualquier color", "Qualsevol color", "Toute couleur", "Beliebige Farbe", "Qualquer cor", "Qualsiasi colore", "Calquera cor", "Edozein kolore", "Qualquer cor", "Elke kleur", "Dowolny kolor", "任意の色", "任意颜色", "모든 색상"]),
    ("Size of lines, arrows and shapes  ⌘[ ⌘]", ["Grosor de líneas, flechas y formas  ⌘[ ⌘]", "Gruix de línies, fletxes i formes  ⌘[ ⌘]", "Épaisseur des lignes, flèches et formes  ⌘[ ⌘]", "Stärke von Linien, Pfeilen und Formen  ⌘[ ⌘]", "Espessura de linhas, setas e formas  ⌘[ ⌘]", "Spessore di linee, frecce e forme  ⌘[ ⌘]", "Grosor de liñas, frechas e formas  ⌘[ ⌘]", "Lerroen, gezien eta formen lodiera  ⌘[ ⌘]", "Espessura de linhas, setas e formas  ⌘[ ⌘]", "Dikte van lijnen, pijlen en vormen  ⌘[ ⌘]", "Grubość linii, strzałek i kształtów  ⌘[ ⌘]", "線、矢印、図形の太さ  ⌘[ ⌘]", "线条、箭头和形状的粗细  ⌘[ ⌘]", "선, 화살표, 도형 두께  ⌘[ ⌘]"]),
    ("Text size", ["Tamaño del texto", "Mida del text", "Taille du texte", "Textgröße", "Tamanho do texto", "Dimensione del testo", "Tamaño do texto", "Testuaren tamaina", "Tamanho do texto", "Tekstgrootte", "Rozmiar tekstu", "テキストサイズ", "文本大小", "텍스트 크기"]),
    ("Plain text", ["Texto simple", "Text simple", "Texte simple", "Einfacher Text", "Texto simples", "Testo semplice", "Texto simple", "Testu arrunta", "Texto simples", "Normale tekst", "Zwykły tekst", "プレーンテキスト", "纯文本", "일반 텍스트"]),
    ("Text with an outline", ["Texto con contorno", "Text amb contorn", "Texte avec contour", "Text mit Kontur", "Texto com contorno", "Testo con contorno", "Texto con contorno", "Testua ingeradarekin", "Texto com contorno", "Tekst met contour", "Tekst z obrysem", "縁取り付きテキスト", "带轮廓的文本", "윤곽선 텍스트"]),
    ("Text on a coloured box", ["Texto sobre un recuadro de color", "Text sobre un requadre de color", "Texte sur un cadre de couleur", "Text auf farbigem Kasten", "Texto sobre uma caixa de cor", "Testo su un riquadro colorato", "Texto sobre un recadro de cor", "Testua koloretako koadroan", "Texto sobre caixa colorida", "Tekst op gekleurd vak", "Tekst na kolorowym tle", "色付きボックス上のテキスト", "彩色底框上的文本", "색상 상자 위 텍스트"]),
    ("Shadow", ["Sombra", "Ombra", "Ombre", "Schatten", "Sombra", "Ombra", "Sombra", "Itzala", "Sombra", "Schaduw", "Cień", "影", "阴影", "그림자"]),
    ("Drop shadow around the picture and under the annotations", ["Sombra alrededor de la imagen y bajo las anotaciones", "Ombra al voltant de la imatge i sota les anotacions", "Ombre portée autour de l'image et sous les annotations", "Schlagschatten um das Bild und unter den Anmerkungen", "Sombra à volta da imagem e por baixo das anotações", "Ombra attorno all'immagine e sotto le annotazioni", "Sombra arredor da imaxe e baixo as anotacións", "Itzala irudiaren inguruan eta oharren azpian", "Sombra ao redor da imagem e sob as anotações", "Slagschaduw rond de afbeelding en onder de annotaties", "Cień wokół obrazu i pod adnotacjami", "画像の周囲と注釈の下にドロップシャドウ", "在图片周围和标注下方添加投影", "이미지 주위와 주석 아래에 그림자"]),
    ("Zoom out  ⌘−", ["Alejar  ⌘−", "Allunya  ⌘−", "Zoom arrière  ⌘−", "Verkleinern  ⌘−", "Reduzir  ⌘−", "Riduci  ⌘−", "Reducir  ⌘−", "Txikiagotu  ⌘−", "Reduzir  ⌘−", "Zoom uit  ⌘−", "Pomniejsz  ⌘−", "縮小  ⌘−", "缩小  ⌘−", "축소  ⌘−"]),
    ("Zoom in  ⌘+", ["Acercar  ⌘+", "Apropa  ⌘+", "Zoom avant  ⌘+", "Vergrößern  ⌘+", "Ampliar  ⌘+", "Ingrandisci  ⌘+", "Ampliar  ⌘+", "Handiagotu  ⌘+", "Ampliar  ⌘+", "Zoom in  ⌘+", "Powiększ  ⌘+", "拡大  ⌘+", "放大  ⌘+", "확대  ⌘+"]),
    ("Fit", ["Ajustar", "Ajusta", "Ajuster", "Einpassen", "Ajustar", "Adatta", "Axustar", "Doitu", "Ajustar", "Pas in", "Dopasuj", "フィット", "适合", "맞춤"]),
    ("Fit to window  ⌘0", ["Ajustar a la ventana  ⌘0", "Ajusta a la finestra  ⌘0", "Ajuster à la fenêtre  ⌘0", "An Fenster anpassen  ⌘0", "Ajustar à janela  ⌘0", "Adatta alla finestra  ⌘0", "Axustar á xanela  ⌘0", "Doitu leihora  ⌘0", "Ajustar à janela  ⌘0", "Pas in venster  ⌘0", "Dopasuj do okna  ⌘0", "ウインドウに合わせる  ⌘0", "缩放至窗口大小  ⌘0", "윈도우에 맞추기  ⌘0"]),
    ("Original size", ["Tamaño original", "Mida original", "Taille réelle", "Originalgröße", "Tamanho original", "Dimensioni originali", "Tamaño orixinal", "Jatorrizko tamaina", "Tamanho original", "Ware grootte", "Rozmiar oryginalny", "実際のサイズ", "原始大小", "실제 크기"]),
    ("Zoom in", ["Acercar", "Apropa", "Zoom avant", "Vergrößern", "Ampliar", "Ingrandisci", "Ampliar", "Handiagotu", "Ampliar", "Zoom in", "Powiększ", "拡大", "放大", "확대"]),
    ("Zoom out", ["Alejar", "Allunya", "Zoom arrière", "Verkleinern", "Reduzir", "Riduci", "Reducir", "Txikiagotu", "Reduzir", "Zoom uit", "Pomniejsz", "縮小", "缩小", "축소"]),
    ("Fit to window", ["Ajustar a la ventana", "Ajusta a la finestra", "Ajuster à la fenêtre", "An Fenster anpassen", "Ajustar à janela", "Adatta alla finestra", "Axustar á xanela", "Doitu leihora", "Ajustar à janela", "Pas in venster", "Dopasuj do okna", "ウインドウに合わせる", "缩放至窗口大小", "윈도우에 맞추기"]),
    ("Zoom  ·  double-click: fit / 100%", ["Zoom  ·  doble clic: ajustar / 100%", "Zoom  ·  doble clic: ajusta / 100%", "Zoom  ·  double-clic : ajuster / 100 %", "Zoom  ·  Doppelklick: einpassen / 100 %", "Zoom  ·  duplo clique: ajustar / 100%", "Zoom  ·  doppio clic: adatta / 100%", "Zoom  ·  dobre clic: axustar / 100%", "Zooma  ·  klik bikoitza: doitu / % 100", "Zoom  ·  clique duplo: ajustar / 100%", "Zoom  ·  dubbelklik: pas in / 100%", "Powiększenie  ·  dwuklik: dopasuj / 100%", "ズーム  ·  ダブルクリック: フィット / 100%", "缩放  ·  连按：适合 / 100%", "확대/축소  ·  이중 클릭: 맞춤 / 100%"]),
    ("Type text…", ["Escribe el texto…", "Escriu el text…", "Saisissez le texte…", "Text eingeben…", "Escreva o texto…", "Scrivi il testo…", "Escribe o texto…", "Idatzi testua…", "Digite o texto…", "Typ tekst…", "Wpisz tekst…", "テキストを入力…", "输入文本…", "텍스트 입력…"]),
    ("Cancel", ["Cancelar", "Cancel·la", "Annuler", "Abbrechen", "Cancelar", "Annulla", "Cancelar", "Utzi", "Cancelar", "Annuleer", "Anuluj", "キャンセル", "取消", "취소"]),
    ("Width:", ["Ancho:", "Amplada:", "Largeur :", "Breite:", "Largura:", "Larghezza:", "Largura:", "Zabalera:", "Largura:", "Breedte:", "Szerokość:", "幅:", "宽度：", "너비:"]),
    ("Height:", ["Alto:", "Alçada:", "Hauteur :", "Höhe:", "Altura:", "Altezza:", "Altura:", "Altuera:", "Altura:", "Hoogte:", "Wysokość:", "高さ:", "高度：", "높이:"]),
    ("Save proportions", ["Mantener proporciones", "Mantén les proporcions", "Conserver les proportions", "Proportionen beibehalten", "Manter as proporções", "Mantieni le proporzioni", "Manter as proporcións", "Mantendu proportzioak", "Manter proporções", "Behoud verhoudingen", "Zachowaj proporcje", "縦横比を固定", "保持比例", "비율 유지"]),
    ("OK", ["Aceptar", "D'acord", "OK", "OK", "OK", "OK", "Aceptar", "Ados", "OK", "OK", "OK", "OK", "好", "확인"]),
    ("General", ["General", "General", "Général", "Allgemein", "Geral", "Generali", "Xeral", "Orokorra", "Geral", "Algemeen", "Ogólne", "一般", "通用", "일반"]),
    ("Advanced", ["Avanzado", "Avançat", "Avancé", "Erweitert", "Avançado", "Avanzate", "Avanzado", "Aurreratua", "Avançado", "Geavanceerd", "Zaawansowane", "詳細", "高级", "고급"]),
    ("Appearance", ["Apariencia", "Aparença", "Apparence", "Darstellung", "Aspeto", "Aspetto", "Aparencia", "Itxura", "Aparência", "Weergave", "Wygląd", "外観", "外观", "모양"]),
    ("Hotkeys", ["Atajos", "Dreceres", "Raccourcis", "Kurzbefehle", "Atalhos", "Abbreviazioni", "Atallos", "Laster-teklak", "Atalhos", "Sneltoetsen", "Skróty", "ショートカット", "快捷键", "단축키"]),
    ("Fonts", ["Tipos de letra", "Tipus de lletra", "Polices", "Schriften", "Tipos de letra", "Font", "Tipografía", "Letra-tipoak", "Fontes", "Lettertypen", "Czcionki", "フォント", "字体", "서체"]),
    ("Reset all settings", ["Restablecer todos los ajustes", "Restableix tota la configuració", "Réinitialiser tous les réglages", "Alle Einstellungen zurücksetzen", "Repor todas as definições", "Ripristina tutte le impostazioni", "Restabelecer todos os axustes", "Berrezarri ezarpen guztiak", "Redefinir todos os ajustes", "Herstel alle instellingen", "Resetuj wszystkie ustawienia", "すべての設定をリセット", "重置所有设置", "모든 설정 재설정"]),
    ("ALL settings in the application will be reset", ["Se restablecerán TODOS los ajustes de la aplicación", "Es restablirà TOTA la configuració de l'aplicació", "TOUS les réglages de l'application seront réinitialisés", "ALLE Einstellungen der App werden zurückgesetzt", "TODAS as definições da aplicação serão repostas", "TUTTE le impostazioni dell'app verranno ripristinate", "Restabeleceranse TODOS os axustes da aplicación", "Aplikazioaren ezarpen GUZTIAK berrezarriko dira", "TODOS os ajustes do aplicativo serão redefinidos", "ALLE instellingen van de app worden hersteld", "WSZYSTKIE ustawienia aplikacji zostaną zresetowane", "アプリケーションのすべての設定がリセットされます", "应用程序中的所有设置都将被重置", "응용 프로그램의 모든 설정이 재설정됩니다"]),
    ("Reset", ["Restablecer", "Restableix", "Réinitialiser", "Zurücksetzen", "Repor", "Ripristina", "Restabelecer", "Berrezarri", "Redefinir", "Herstel", "Resetuj", "リセット", "重置", "재설정"]),
    ("After screenshot:", ["Después de capturar:", "Després de capturar:", "Après la capture :", "Nach der Aufnahme:", "Depois da captura:", "Dopo l'istantanea:", "Despois de capturar:", "Argazkia atera ondoren:", "Após a captura:", "Na schermafbeelding:", "Po zrzucie ekranu:", "撮影後:", "截屏后：", "스크린샷 후:"]),
    ("Sound effects", ["Efectos de sonido", "Efectes de so", "Effets sonores", "Toneffekte", "Efeitos sonoros", "Effetti sonori", "Efectos de son", "Soinu-efektuak", "Efeitos sonoros", "Geluidseffecten", "Efekty dźwiękowe", "サウンドエフェクト", "声音效果", "사운드 효과"]),
    ("External Image Editor:", ["Editor de imágenes externo:", "Editor d'imatges extern:", "Éditeur d'images externe :", "Externer Bildeditor:", "Editor de imagem externo:", "Editor di immagini esterno:", "Editor de imaxes externo:", "Kanpoko irudi-editorea:", "Editor de imagens externo:", "Externe beeldeditor:", "Zewnętrzny edytor obrazów:", "外部イメージエディタ:", "外部图像编辑器：", "외부 이미지 편집기:"]),
    ("Choose application...", ["Elegir aplicación...", "Tria una aplicació...", "Choisir une application...", "App auswählen...", "Escolher aplicação...", "Scegli un'app...", "Escoller aplicación...", "Aukeratu aplikazioa...", "Escolher aplicativo...", "Kies app...", "Wybierz aplikację...", "アプリケーションを選択...", "选择应用程序...", "응용 프로그램 선택..."]),
    ("Choose application", ["Elegir aplicación", "Tria una aplicació", "Choisir une application", "App auswählen", "Escolher aplicação", "Scegli un'app", "Escoller aplicación", "Aukeratu aplikazioa", "Escolher aplicativo", "Kies app", "Wybierz aplikację", "アプリケーションを選択", "选择应用程序", "응용 프로그램 선택"]),
    ("Application", ["Aplicación", "Aplicació", "Application", "App", "Aplicação", "App", "Aplicación", "Aplikazioa", "Aplicativo", "App", "Aplikacja", "アプリケーション", "应用程序", "응용 프로그램"]),
    ("Status bar:", ["Barra de menús:", "Barra de menús:", "Barre des menus :", "Menüleiste:", "Barra de menus:", "Barra dei menu:", "Barra de menús:", "Menu-barra:", "Barra de menus:", "Menubalk:", "Pasek menu:", "メニューバー:", "菜单栏：", "메뉴 막대:"]),
    ("Launch at login", ["Abrir al iniciar sesión", "Obre en iniciar la sessió", "Ouvrir à l'ouverture de session", "Beim Anmelden öffnen", "Abrir ao iniciar sessão", "Apri al login", "Abrir ao iniciar sesión", "Ireki saioa hastean", "Abrir ao iniciar sessão", "Open bij inloggen", "Otwieraj po zalogowaniu", "ログイン時に開く", "登录时打开", "로그인 시 열기"]),
    ("Image File Format:", ["Formato de imagen:", "Format d'imatge:", "Format d'image :", "Bildformat:", "Formato de imagem:", "Formato immagine:", "Formato de imaxe:", "Irudi-formatua:", "Formato de imagem:", "Afbeeldingsformaat:", "Format pliku obrazu:", "イメージフォーマット:", "图像文件格式：", "이미지 파일 포맷:"]),
    ("Auto (smaller size)", ["Automático (menor tamaño)", "Automàtic (mida més petita)", "Automatique (taille réduite)", "Automatisch (kleinere Datei)", "Automático (menor tamanho)", "Automatico (dimensione minore)", "Automático (menor tamaño)", "Automatikoa (tamaina txikiagoa)", "Automático (menor tamanho)", "Automatisch (kleiner bestand)", "Automatyczny (mniejszy plik)", "自動(小さいサイズ)", "自动（较小文件）", "자동(더 작은 크기)"]),
    ("JPEG Quality:", ["Calidad JPEG:", "Qualitat JPEG:", "Qualité JPEG :", "JPEG-Qualität:", "Qualidade JPEG:", "Qualità JPEG:", "Calidade JPEG:", "JPEG kalitatea:", "Qualidade JPEG:", "JPEG-kwaliteit:", "Jakość JPEG:", "JPEG画質:", "JPEG 质量：", "JPEG 품질:"]),
    ("Save folder:", ["Carpeta de destino:", "Carpeta de destinació:", "Dossier d'enregistrement :", "Speicherordner:", "Pasta de destino:", "Cartella di salvataggio:", "Cartafol de destino:", "Gordetzeko karpeta:", "Pasta de destino:", "Bewaarmap:", "Folder zapisu:", "保存先フォルダ:", "存储文件夹：", "저장 폴더:"]),
    ("Choose...", ["Elegir...", "Tria...", "Choisir...", "Auswählen...", "Escolher...", "Scegli...", "Escoller...", "Aukeratu...", "Escolher...", "Kies...", "Wybierz...", "選択...", "选取...", "선택..."]),
    ("Choose save folder", ["Elegir la carpeta de destino", "Tria la carpeta de destinació", "Choisir le dossier d'enregistrement", "Speicherordner auswählen", "Escolher a pasta de destino", "Scegli la cartella di salvataggio", "Escoller o cartafol de destino", "Aukeratu gordetzeko karpeta", "Escolher a pasta de destino", "Kies bewaarmap", "Wybierz folder zapisu", "保存先フォルダを選択", "选取存储文件夹", "저장 폴더 선택"]),
    ("Autosave screenshots", ["Guardar capturas automáticamente", "Desa les captures automàticament", "Enregistrer les captures automatiquement", "Bildschirmfotos automatisch speichern", "Guardar capturas automaticamente", "Salva automaticamente le istantanee", "Gardar capturas automaticamente", "Gorde pantaila-argazkiak automatikoki", "Salvar capturas automaticamente", "Bewaar schermafbeeldingen automatisch", "Automatycznie zachowuj zrzuty", "スクリーンショットを自動保存", "自动存储截屏", "스크린샷 자동 저장"]),
    ("Shrink Retina snaps", ["Reducir capturas Retina", "Redueix les captures Retina", "Réduire les captures Retina", "Retina-Aufnahmen verkleinern", "Reduzir capturas Retina", "Riduci le istantanee Retina", "Reducir capturas Retina", "Txikitu Retina argazkiak", "Reduzir capturas Retina", "Verklein Retina-afbeeldingen", "Zmniejszaj zrzuty Retina", "Retinaの画像を縮小", "缩小 Retina 截屏", "Retina 이미지 축소"]),
    ("Video File Format:", ["Formato de vídeo:", "Format de vídeo:", "Format vidéo :", "Videoformat:", "Formato de vídeo:", "Formato video:", "Formato de vídeo:", "Bideo-formatua:", "Formato de vídeo:", "Videoformaat:", "Format pliku wideo:", "ビデオフォーマット:", "视频文件格式：", "비디오 파일 포맷:"]),
    ("GIF Quality:", ["Calidad GIF:", "Qualitat GIF:", "Qualité GIF :", "GIF-Qualität:", "Qualidade GIF:", "Qualità GIF:", "Calidade GIF:", "GIF kalitatea:", "Qualidade GIF:", "GIF-kwaliteit:", "Jakość GIF:", "GIF画質:", "GIF 质量：", "GIF 품질:"]),
    ("Autosave to \"Movies\" folder", ["Guardar automáticamente en «Películas»", "Desa automàticament a «Pel·lícules»", "Enregistrer automatiquement dans « Films »", "Automatisch in „Filme“ speichern", "Guardar automaticamente em «Filmes»", "Salva automaticamente in «Filmati»", "Gardar automaticamente en «Películas»", "Gorde automatikoki «Filmak» karpetan", "Salvar automaticamente em \"Filmes\"", "Bewaar automatisch in map ‘Films’", "Automatycznie zachowuj w folderze „Filmy”", "「ムービー」フォルダに自動保存", "自动存储到「影片」文件夹", "'동영상' 폴더에 자동 저장"]),
    ("Drag and drop copy:", ["Copia al arrastrar y soltar:", "Còpia en arrossegar i deixar anar:", "Copie par glisser-déposer :", "Kopie per Drag & Drop:", "Cópia ao arrastar e largar:", "Copia con trascinamento:", "Copia ao arrastrar e soltar:", "Arrastatuta kopiatzean:", "Cópia ao arrastar e soltar:", "Kopie bij slepen:", "Kopia przy przeciąganiu:", "ドラッグ&ドロップ時のコピー:", "拖放拷贝：", "드래그 앤 드롭 복사:"]),
    ("Copy to clipboard:", ["Copiar al portapapeles:", "Copia al porta-retalls:", "Copier dans le presse-papiers :", "In Zwischenablage kopieren:", "Copiar para a área de transferência:", "Copia negli appunti:", "Copiar no portapapeis:", "Kopiatu arbelean:", "Copiar para a Área de Transferência:", "Kopieer naar klembord:", "Kopiuj do schowka:", "クリップボードにコピー:", "拷贝到剪贴板：", "클립보드에 복사:"]),
    ("Filename template:", ["Plantilla de nombre de archivo:", "Plantilla de nom de fitxer:", "Modèle de nom de fichier :", "Dateinamenvorlage:", "Modelo de nome de ficheiro:", "Modello del nome file:", "Modelo de nome de ficheiro:", "Fitxategi-izenaren eredua:", "Modelo de nome de arquivo:", "Sjabloon bestandsnaam:", "Szablon nazwy pliku:", "ファイル名テンプレート:", "文件名模板：", "파일 이름 템플릿:"]),
    ("%Y — 4-digit year,  %y — 2-digit year, %m — month, %d — day,\n%H — hour, %M — minutes, %S — seconds, %W — window title,\n%R — 5 random symbols, %T - file type", ["%Y — año (4 cifras),  %y — año (2 cifras), %m — mes, %d — día,\n%H — hora, %M — minutos, %S — segundos, %W — título de la ventana,\n%R — 5 símbolos aleatorios, %T - tipo de archivo", "%Y — any (4 xifres),  %y — any (2 xifres), %m — mes, %d — dia,\n%H — hora, %M — minuts, %S — segons, %W — títol de la finestra,\n%R — 5 símbols aleatoris, %T - tipus de fitxer", "%Y — année (4 chiffres),  %y — année (2 chiffres), %m — mois, %d — jour,\n%H — heure, %M — minutes, %S — secondes, %W — titre de la fenêtre,\n%R — 5 symboles aléatoires, %T - type de fichier", "%Y — Jahr (4-stellig),  %y — Jahr (2-stellig), %m — Monat, %d — Tag,\n%H — Stunde, %M — Minuten, %S — Sekunden, %W — Fenstertitel,\n%R — 5 zufällige Zeichen, %T - Dateityp", "%Y — ano (4 algarismos),  %y — ano (2 algarismos), %m — mês, %d — dia,\n%H — hora, %M — minutos, %S — segundos, %W — título da janela,\n%R — 5 símbolos aleatórios, %T - tipo de ficheiro", "%Y — anno (4 cifre),  %y — anno (2 cifre), %m — mese, %d — giorno,\n%H — ora, %M — minuti, %S — secondi, %W — titolo della finestra,\n%R — 5 simboli casuali, %T - tipo di file", "%Y — ano (4 cifras),  %y — ano (2 cifras), %m — mes, %d — día,\n%H — hora, %M — minutos, %S — segundos, %W — título da xanela,\n%R — 5 símbolos aleatorios, %T - tipo de ficheiro", "%Y — urtea (4 digitu),  %y — urtea (2 digitu), %m — hila, %d — eguna,\n%H — ordua, %M — minutuak, %S — segundoak, %W — leihoaren izenburua,\n%R — 5 ausazko ikur, %T - fitxategi mota", "%Y — ano (4 dígitos),  %y — ano (2 dígitos), %m — mês, %d — dia,\n%H — hora, %M — minutos, %S — segundos, %W — título da janela,\n%R — 5 símbolos aleatórios, %T - tipo de arquivo", "%Y — jaar (4 cijfers),  %y — jaar (2 cijfers), %m — maand, %d — dag,\n%H — uur, %M — minuten, %S — seconden, %W — venstertitel,\n%R — 5 willekeurige tekens, %T - bestandstype", "%Y — rok (4 cyfry),  %y — rok (2 cyfry), %m — miesiąc, %d — dzień,\n%H — godzina, %M — minuty, %S — sekundy, %W — tytuł okna,\n%R — 5 losowych znaków, %T - typ pliku", "%Y — 西暦(4桁)、 %y — 西暦(2桁)、%m — 月、%d — 日、\n%H — 時、%M — 分、%S — 秒、%W — ウインドウのタイトル、\n%R — ランダムな5文字、%T - ファイルの種類", "%Y — 4 位年份，%y — 2 位年份，%m — 月，%d — 日，\n%H — 时，%M — 分，%S — 秒，%W — 窗口标题，\n%R — 5 个随机字符，%T - 文件类型", "%Y — 연도(4자리),  %y — 연도(2자리), %m — 월, %d — 일,\n%H — 시, %M — 분, %S — 초, %W — 윈도우 제목,\n%R — 임의의 기호 5개, %T - 파일 유형"]),
    ("Apply to reopened files", ["Aplicar a los archivos reabiertos", "Aplica als fitxers reoberts", "Appliquer aux fichiers rouverts", "Auf erneut geöffnete Dateien anwenden", "Aplicar aos ficheiros reabertos", "Applica ai file riaperti", "Aplicar aos ficheiros reabertos", "Aplikatu berriro irekitako fitxategiei", "Aplicar a arquivos reabertos", "Pas toe op heropende bestanden", "Stosuj do ponownie otwartych plików", "再度開いたファイルにも適用", "应用到重新打开的文件", "다시 연 파일에 적용"]),
    ("Theme:", ["Tema:", "Tema:", "Thème :", "Design:", "Tema:", "Tema:", "Tema:", "Gaia:", "Tema:", "Thema:", "Motyw:", "テーマ:", "主题：", "테마:"]),
    ("Automatic", ["Automático", "Automàtic", "Automatique", "Automatisch", "Automático", "Automatico", "Automático", "Automatikoa", "Automático", "Automatisch", "Automatyczny", "自動", "自动", "자동"]),
    ("Light", ["Claro", "Clar", "Clair", "Hell", "Claro", "Chiaro", "Claro", "Argia", "Claro", "Licht", "Jasny", "ライト", "浅色", "라이트"]),
    ("Dark", ["Oscuro", "Fosc", "Sombre", "Dunkel", "Escuro", "Scuro", "Escuro", "Iluna", "Escuro", "Donker", "Ciemny", "ダーク", "深色", "다크"]),
    ("Capture area:", ["Captura de área:", "Captura d'àrea:", "Capture de zone :", "Bereichsaufnahme:", "Captura de área:", "Cattura di un'area:", "Captura de área:", "Area-kaptura:", "Captura de área:", "Gebied vastleggen:", "Zrzut obszaru:", "範囲キャプチャ:", "截取区域：", "영역 캡처:"]),
    ("Show magnifier", ["Mostrar lupa", "Mostra la lupa", "Afficher la loupe", "Lupe anzeigen", "Mostrar lupa", "Mostra la lente", "Amosar lupa", "Erakutsi lupa", "Mostrar lupa", "Toon vergrootglas", "Pokazuj lupę", "拡大鏡を表示", "显示放大镜", "돋보기 표시"]),
    ("Highlight window under cursor", ["Resaltar la ventana bajo el cursor", "Ressalta la finestra sota el cursor", "Surligner la fenêtre sous le curseur", "Fenster unter dem Zeiger hervorheben", "Realçar a janela sob o cursor", "Evidenzia la finestra sotto il cursore", "Resaltar a xanela baixo o cursor", "Nabarmendu kurtsorearen azpiko leihoa", "Realçar a janela sob o cursor", "Markeer venster onder aanwijzer", "Wyróżniaj okno pod kursorem", "ポインタの下のウインドウを強調", "高亮显示指针下的窗口", "포인터 아래 윈도우 강조"]),
    ("Add shadow to window", ["Añadir sombra a la ventana", "Afegeix ombra a la finestra", "Ajouter une ombre à la fenêtre", "Fenster mit Schatten", "Adicionar sombra à janela", "Aggiungi un'ombra alla finestra", "Engadir sombra á xanela", "Gehitu itzala leihoari", "Adicionar sombra à janela", "Voeg schaduw toe aan venster", "Dodawaj cień do okna", "ウインドウに影を追加", "为窗口添加阴影", "윈도우에 그림자 추가"]),
    ("Click to snap a window", ["Clic para capturar una ventana", "Fes clic per capturar una finestra", "Cliquer pour capturer une fenêtre", "Klicken, um ein Fenster aufzunehmen", "Clicar para capturar uma janela", "Fai clic per catturare una finestra", "Clic para capturar unha xanela", "Egin klik leiho bat kapturatzeko", "Clicar para capturar uma janela", "Klik om een venster vast te leggen", "Kliknij, aby uchwycić okno", "クリックでウインドウを撮影", "点按以截取窗口", "클릭하여 윈도우 캡처"]),
    ("Show hints", ["Mostrar consejos", "Mostra consells", "Afficher les astuces", "Hinweise anzeigen", "Mostrar dicas", "Mostra i suggerimenti", "Amosar consellos", "Erakutsi aholkuak", "Mostrar dicas", "Toon tips", "Pokazuj wskazówki", "ヒントを表示", "显示提示", "힌트 표시"]),
    ("Hide animation", ["Ocultar animación", "Amaga l'animació", "Masquer l'animation", "Animation ausblenden", "Ocultar animação", "Nascondi l'animazione", "Ocultar animación", "Ezkutatu animazioa", "Ocultar animação", "Verberg animatie", "Ukryj animację", "アニメーションを隠す", "隐藏动画", "애니메이션 숨기기"]),
    ("Hide all notifications", ["Ocultar todas las notificaciones", "Amaga totes les notificacions", "Masquer toutes les notifications", "Alle Mitteilungen ausblenden", "Ocultar todas as notificações", "Nascondi tutte le notifiche", "Ocultar todas as notificacións", "Ezkutatu jakinarazpen guztiak", "Ocultar todas as notificações", "Verberg alle meldingen", "Ukryj wszystkie powiadomienia", "すべての通知を隠す", "隐藏所有通知", "모든 알림 숨기기"]),
    ("Editor:", ["Editor:", "Editor:", "Éditeur :", "Editor:", "Editor:", "Editor:", "Editor:", "Editorea:", "Editor:", "Editor:", "Edytor:", "エディタ:", "编辑器：", "편집기:"]),
    ("Show tooltips", ["Mostrar ayudas emergentes", "Mostra els indicadors de funció", "Afficher les bulles d'aide", "Hilfetexte anzeigen", "Mostrar dicas de ferramentas", "Mostra i suggerimenti dei comandi", "Amosar axudas emerxentes", "Erakutsi argibideak", "Mostrar dicas de ferramentas", "Toon help-tags", "Pokazuj podpowiedzi", "ツールチップを表示", "显示工具提示", "툴팁 표시"]),
    ("Video record hotkeys:", ["Atajos de grabación de vídeo:", "Dreceres d'enregistrament de vídeo:", "Raccourcis d'enregistrement vidéo :", "Kurzbefehle für Videoaufnahme:", "Atalhos de gravação de vídeo:", "Abbreviazioni per la registrazione video:", "Atallos de gravación:", "Grabazioko laster-teklak:", "Atalhos de gravação de vídeo:", "Sneltoetsen video-opname:", "Skróty nagrywania wideo:", "ビデオ収録のショートカット:", "视频录制快捷键：", "비디오 녹화 단축키:"]),
    ("Default", ["Por defecto", "Per defecte", "Par défaut", "Standard", "Predefinição", "Predefinito", "Predeterminado", "Lehenetsia", "Padrão", "Standaard", "Domyślne", "デフォルト", "默认", "기본값"]),
    ("Restore the default hotkeys", ["Restaurar los atajos por defecto", "Restaura les dreceres per defecte", "Restaurer les raccourcis par défaut", "Standard-Kurzbefehle wiederherstellen", "Repor os atalhos predefinidos", "Ripristina le abbreviazioni predefinite", "Restaurar os atallos predeterminados", "Leheneratu laster-tekla lehenetsiak", "Restaurar os atalhos padrão", "Herstel standaardsneltoetsen", "Przywróć domyślne skróty", "デフォルトのショートカットに戻す", "恢复默认快捷键", "기본 단축키로 복원"]),
    ("Press hotkey with ⌘/⌃", ["Pulsa un atajo con ⌘/⌃", "Prem una drecera amb ⌘/⌃", "Appuyez sur un raccourci avec ⌘/⌃", "Kurzbefehl mit ⌘/⌃ drücken", "Prima um atalho com ⌘/⌃", "Premi un'abbreviazione con ⌘/⌃", "Preme un atallo con ⌘/⌃", "Sakatu laster-tekla ⌘/⌃ erabiliz", "Pressione um atalho com ⌘/⌃", "Druk sneltoets met ⌘/⌃", "Naciśnij skrót z ⌘/⌃", "⌘/⌃を含むキーを押してください", "按下含 ⌘/⌃ 的快捷键", "⌘/⌃와 함께 단축키 누르기"]),
    ("Delete hotkey", ["Eliminar atajo", "Elimina la drecera", "Supprimer le raccourci", "Kurzbefehl löschen", "Apagar atalho", "Elimina l'abbreviazione", "Eliminar atallo", "Ezabatu laster-tekla", "Apagar atalho", "Verwijder sneltoets", "Usuń skrót", "ショートカットを削除", "删除快捷键", "단축키 삭제"]),
    ("Main font:", ["Tipo de letra principal:", "Tipus de lletra principal:", "Police principale :", "Hauptschrift:", "Tipo de letra principal:", "Font principale:", "Tipo de letra principal:", "Letra-tipo nagusia:", "Fonte principal:", "Hoofdlettertype:", "Czcionka główna:", "メインフォント:", "主字体：", "기본 서체:"]),
    ("Additional fonts:", ["Tipos de letra adicionales:", "Tipus de lletra addicionals:", "Polices supplémentaires :", "Weitere Schriften:", "Tipos de letra adicionais:", "Font aggiuntivi:", "Tipos de letra adicionais:", "Letra-tipo gehigarriak:", "Fontes adicionais:", "Extra lettertypen:", "Dodatkowe czcionki:", "追加フォント:", "附加字体：", "추가 서체:"]),
    ("No additional fonts", ["Sin tipos de letra adicionales", "Cap tipus de lletra addicional", "Aucune police supplémentaire", "Keine weiteren Schriften", "Sem tipos de letra adicionais", "Nessun font aggiuntivo", "Sen tipos de letra adicionais", "Ez dago letra-tipo gehigarririk", "Nenhuma fonte adicional", "Geen extra lettertypen", "Brak dodatkowych czcionek", "追加フォントはありません", "无附加字体", "추가 서체 없음"]),
    ("Add font", ["Añadir tipo de letra", "Afegeix un tipus de lletra", "Ajouter une police", "Schrift hinzufügen", "Adicionar tipo de letra", "Aggiungi un font", "Engadir tipo de letra", "Gehitu letra-tipoa", "Adicionar fonte", "Voeg lettertype toe", "Dodaj czcionkę", "フォントを追加", "添加字体", "서체 추가"]),
    ("Font", ["Tipo de letra", "Tipus de lletra", "Police", "Schrift", "Tipo de letra", "Font", "Tipo de letra", "Letra-tipoa", "Fonte", "Lettertype", "Czcionka", "フォント", "字体", "서체"]),
    ("Remove font", ["Quitar tipo de letra", "Treu el tipus de lletra", "Retirer la police", "Schrift entfernen", "Remover tipo de letra", "Rimuovi il font", "Quitar tipo de letra", "Kendu letra-tipoa", "Remover fonte", "Verwijder lettertype", "Usuń czcionkę", "フォントを削除", "移除字体", "서체 제거"]),
    ("Attention: some fonts could not correctly displayed in the editor.", ["Atención: algunos tipos de letra pueden no verse bien en el editor.", "Atenció: alguns tipus de lletra poden no veure's bé a l'editor.", "Attention : certaines polices peuvent mal s'afficher dans l'éditeur.", "Achtung: Manche Schriften werden im Editor eventuell nicht richtig angezeigt.", "Atenção: alguns tipos de letra podem não aparecer bem no editor.", "Attenzione: alcuni font potrebbero non essere visualizzati correttamente nell'editor.", "Atención: algúns tipos de letra poden non verse ben no editor.", "Kontuz: baliteke letra-tipo batzuk editorean ondo ez ikustea.", "Atenção: algumas fontes podem não ser exibidas corretamente no editor.", "Let op: sommige lettertypen worden mogelijk niet goed weergegeven in de editor.", "Uwaga: niektóre czcionki mogą nie wyświetlać się poprawnie w edytorze.", "注意: 一部のフォントはエディタで正しく表示されない場合があります。", "注意：某些字体可能无法在编辑器中正确显示。", "주의: 일부 서체는 편집기에서 올바르게 표시되지 않을 수 있습니다."]),
    ("Reset fonts", ["Restablecer tipos de letra", "Restableix els tipus de lletra", "Réinitialiser les polices", "Schriften zurücksetzen", "Repor os tipos de letra", "Ripristina i font", "Restabelecer tipos de letra", "Berrezarri letra-tipoak", "Redefinir fontes", "Herstel lettertypen", "Resetuj czcionki", "フォントをリセット", "重置字体", "서체 재설정"]),
    ("Image", ["Imagen", "Imatge", "Image", "Bild", "Imagem", "Immagine", "Imaxe", "Irudia", "Imagem", "Afbeelding", "Obraz", "イメージ", "图像", "이미지"]),
    ("Path", ["Ruta", "Camí", "Chemin", "Pfad", "Caminho", "Percorso", "Ruta", "Bide-izena", "Caminho", "Pad", "Ścieżka", "パス", "路径", "경로"]),
    ("Open {} editor", ["Abrir el editor de {}", "Obre l'editor de {}", "Ouvrir l'éditeur de {}", "{}-Editor öffnen", "Abrir o editor do {}", "Apri l'editor di {}", "Abrir o editor de {}", "Ireki {} editorea", "Abrir o editor do {}", "Open {}-editor", "Otwórz edytor {}", "{}エディタを開く", "打开 {} 编辑器", "{} 편집기 열기"]),
    ("Save screenshot", ["Guardar la captura", "Desa la captura", "Enregistrer la capture", "Bildschirmfoto speichern", "Guardar a captura", "Salva l'istantanea", "Gardar a captura", "Gorde pantaila-argazkia", "Salvar a captura de tela", "Bewaar schermafbeelding", "Zachowaj zrzut ekranu", "スクリーンショットを保存", "存储截屏", "스크린샷 저장"]),
    ("Open external editor", ["Abrir el editor externo", "Obre l'editor extern", "Ouvrir l'éditeur externe", "Externen Editor öffnen", "Abrir o editor externo", "Apri l'editor esterno", "Abrir o editor externo", "Ireki kanpoko editorea", "Abrir o editor externo", "Open externe editor", "Otwórz zewnętrzny edytor", "外部エディタを開く", "打开外部编辑器", "외부 편집기 열기"]),
    ("Copy image to clipboard", ["Copiar la imagen al portapapeles", "Copia la imatge al porta-retalls", "Copier l'image dans le presse-papiers", "Bild in die Zwischenablage kopieren", "Copiar a imagem para a área de transferência", "Copia l'immagine negli appunti", "Copiar a imaxe no portapapeis", "Kopiatu irudia arbelean", "Copiar a imagem para a Área de Transferência", "Kopieer afbeelding naar klembord", "Kopiuj obraz do schowka", "イメージをクリップボードにコピー", "将图像拷贝到剪贴板", "이미지를 클립보드에 복사"]),
    ("Capture frozen area:", ["Captura de área congelada:", "Captura d'àrea congelada:", "Capture de zone figée :", "Eingefrorener Bereich:", "Captura de área congelada:", "Cattura di un'area congelata:", "Captura de área conxelada:", "Kapturatu area izoztua:", "Capturar área congelada:", "Bevroren gebied vastleggen:", "Zrzut zamrożonego obszaru:", "静止画面で範囲を撮影:", "截取冻结区域：", "정지 화면 영역 캡처:"]),
    ("Capture fullscreen:", ["Captura de pantalla completa:", "Captura de pantalla completa:", "Capture de l'écran entier :", "Ganzer Bildschirm:", "Captura de ecrã inteiro:", "Cattura dello schermo intero:", "Captura de pantalla completa:", "Kapturatu pantaila osoa:", "Capturar tela inteira:", "Volledig scherm vastleggen:", "Zrzut całego ekranu:", "全画面をキャプチャ:", "截取全屏：", "전체 화면 캡처:"]),
    ("Fullscreen after 10s:", ["Pantalla completa en 10 s:", "Pantalla completa d'aquí a 10 s:", "Écran entier dans 10 s :", "Ganzer Bildschirm in 10 s:", "Ecrã inteiro daqui a 10 s:", "Schermo intero tra 10 s:", "Pantalla completa en 10 s:", "Pantaila osoa 10 s barru:", "Tela inteira em 10 s:", "Volledig scherm na 10 s:", "Cały ekran po 10 s:", "10秒後に全画面:", "10 秒后截取全屏：", "10초 후 전체 화면:"]),
    ("Open Latest Image:", ["Abrir la última imagen:", "Obre l'última imatge:", "Ouvrir la dernière image :", "Letztes Bild öffnen:", "Abrir a última imagem:", "Apri l'ultima immagine:", "Abrir a última imaxe:", "Ireki azken irudia:", "Abrir última imagem:", "Open laatste afbeelding:", "Otwórz ostatni obraz:", "最新のイメージを開く:", "打开最新图像：", "최근 이미지 열기:"]),
    ("Record video:", ["Grabar vídeo:", "Enregistra un vídeo:", "Enregistrer une vidéo :", "Video aufnehmen:", "Gravar vídeo:", "Registra un video:", "Gravar vídeo:", "Grabatu bideoa:", "Gravar vídeo:", "Video opnemen:", "Nagraj wideo:", "ビデオを収録:", "录制视频：", "비디오 녹화:"]),
    ("Capture previous area:", ["Capturar el área anterior:", "Captura l'àrea anterior:", "Capturer la zone précédente :", "Vorherigen Bereich aufnehmen:", "Capturar a área anterior:", "Cattura l'area precedente:", "Capturar a área anterior:", "Kapturatu aurreko area:", "Capturar a área anterior:", "Vorig gebied vastleggen:", "Zrzut poprzedniego obszaru:", "前回の範囲をキャプチャ:", "截取上次的区域：", "이전 영역 캡처:"]),
    ("Open from clipboard:", ["Abrir desde el portapapeles:", "Obre des del porta-retalls:", "Ouvrir depuis le presse-papiers :", "Aus Zwischenablage öffnen:", "Abrir da área de transferência:", "Apri dagli appunti:", "Abrir do portapapeis:", "Ireki arbeletik:", "Abrir da Área de Transferência:", "Open van klembord:", "Otwórz ze schowka:", "クリップボードから開く:", "从剪贴板打开：", "클립보드에서 열기:"]),
    ("Record/Pause:", ["Grabar/Pausa:", "Enregistra/Pausa:", "Enregistrer/Pause :", "Aufnehmen/Pause:", "Gravar/Pausa:", "Registra/Pausa:", "Gravar/Pausa:", "Grabatu/Pausatu:", "Gravar/Pausar:", "Opnemen/pauze:", "Nagrywaj/pauza:", "収録/一時停止:", "录制/暂停：", "녹화/일시 정지:"]),
    ("Stop:", ["Detener:", "Atura:", "Arrêter :", "Stoppen:", "Parar:", "Interrompi:", "Deter:", "Gelditu:", "Parar:", "Stop:", "Zatrzymaj:", "停止:", "停止：", "중단:"]),
    ("Arrow:", ["Flecha:", "Fletxa:", "Flèche :", "Pfeil:", "Seta:", "Freccia:", "Frecha:", "Gezia:", "Seta:", "Pijl:", "Strzałka:", "矢印:", "箭头：", "화살표:"]),
    ("Rectangle:", ["Rectángulo:", "Rectangle:", "Rectangle :", "Rechteck:", "Retângulo:", "Rettangolo:", "Rectángulo:", "Laukizuzena:", "Retângulo:", "Rechthoek:", "Prostokąt:", "長方形:", "矩形：", "직사각형:"]),
    ("Fullscreen / Windowed mode:", ["Pantalla completa / ventana:", "Pantalla completa / finestra:", "Plein écran / fenêtre :", "Vollbild / Fenster:", "Ecrã inteiro / janela:", "Schermo intero / finestra:", "Pantalla completa / xanela:", "Pantaila osoa / leihoa:", "Tela cheia / janela:", "Volledig scherm / venster:", "Pełny ekran / tryb okna:", "全画面/ウインドウ切替:", "全屏 / 窗口模式：", "전체 화면/윈도우 모드:"]),
];

fn table() -> &'static HashMap<&'static str, [&'static str; 14]> {
    static MAP: OnceLock<HashMap<&'static str, [&'static str; 14]>> = OnceLock::new();
    MAP.get_or_init(|| TABLE.iter().map(|(en, tr)| (*en, *tr)).collect())
}

/// The text in the current language.
pub fn t(en: &'static str) -> &'static str {
    match current() {
        Lang::En => en,
        lang => table().get(en).and_then(|tr| tr.get(lang as usize - 1)).copied().unwrap_or(en),
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
        Lang::Fr => vertical_tricolour(&p, r, [Color32::from_rgb(0, 35, 149), Color32::WHITE, Color32::from_rgb(237, 41, 57)]),
        Lang::It => vertical_tricolour(&p, r, [Color32::from_rgb(0, 146, 70), Color32::WHITE, Color32::from_rgb(206, 43, 55)]),
        Lang::De => {
            for (k, c) in [Color32::BLACK, Color32::from_rgb(221, 0, 0), Color32::from_rgb(255, 206, 0)].into_iter().enumerate() {
                p.rect_filled(Rect::from_min_size(pos2(r.min.x, r.min.y + k as f32 * h / 3.0), vec2(w, h / 3.0)), 0.0, c);
            }
        }
        Lang::Pt => {
            // green 2/5, red 3/5, and the armillary sphere as a yellow ring
            // around a small shield (simplified for small sizes)
            p.rect_filled(r, 0.0, Color32::from_rgb(218, 41, 28));
            p.rect_filled(Rect::from_min_size(r.min, vec2(w * 0.4, h)), 0.0, Color32::from_rgb(4, 106, 56));
            let c = pos2(r.min.x + w * 0.4, r.center().y);
            p.circle_stroke(c, h * 0.26, Stroke::new(h * 0.08, Color32::from_rgb(255, 225, 0)));
            p.rect_filled(Rect::from_center_size(c, vec2(h * 0.24, h * 0.28)), h * 0.05, Color32::WHITE);
            p.rect_filled(Rect::from_center_size(c, vec2(h * 0.16, h * 0.2)), h * 0.04, Color32::from_rgb(218, 41, 28));
        }
        Lang::Gl => {
            // white with a light-blue band from top-left to bottom-right
            p.rect_filled(r, 0.0, Color32::WHITE);
            p.line_segment([r.left_top(), r.right_bottom()], Stroke::new(h * 0.3, Color32::from_rgb(0, 154, 216)));
        }
        Lang::Eu => {
            // the ikurriña: red, a green saltire and a white cross over it
            p.rect_filled(r, 0.0, Color32::from_rgb(213, 43, 30));
            for (a, b) in [(r.left_top(), r.right_bottom()), (r.left_bottom(), r.right_top())] {
                p.line_segment([a, b], Stroke::new(h * 0.14, Color32::from_rgb(0, 155, 72)));
            }
            let c = r.center();
            p.rect_filled(Rect::from_center_size(c, vec2(w, h * 0.13)), 0.0, Color32::WHITE);
            p.rect_filled(Rect::from_center_size(c, vec2(h * 0.13, h)), 0.0, Color32::WHITE);
        }
        Lang::PtBr => {
            // green, a yellow rhombus and a blue globe
            p.rect_filled(r, 0.0, Color32::from_rgb(0, 156, 59));
            let c = r.center();
            let (dx, dy) = (w * 0.42, h * 0.38);
            p.add(egui::Shape::convex_polygon(
                vec![pos2(c.x - dx, c.y), pos2(c.x, c.y - dy), pos2(c.x + dx, c.y), pos2(c.x, c.y + dy)],
                Color32::from_rgb(255, 223, 0),
                Stroke::NONE,
            ));
            p.circle_filled(c, h * 0.22, Color32::from_rgb(0, 39, 118));
        }
        Lang::Nl => {
            for (k, c) in [Color32::from_rgb(174, 28, 40), Color32::WHITE, Color32::from_rgb(33, 70, 139)].into_iter().enumerate() {
                p.rect_filled(Rect::from_min_size(pos2(r.min.x, r.min.y + k as f32 * h / 3.0), vec2(w, h / 3.0)), 0.0, c);
            }
        }
        Lang::Pl => {
            p.rect_filled(r, 0.0, Color32::WHITE);
            p.rect_filled(Rect::from_min_size(pos2(r.min.x, r.min.y + h / 2.0), vec2(w, h / 2.0)), 0.0, Color32::from_rgb(220, 20, 60));
        }
        Lang::Ja => {
            p.rect_filled(r, 0.0, Color32::WHITE);
            p.circle_filled(r.center(), h * 0.3, Color32::from_rgb(188, 0, 45));
        }
        Lang::Zh => {
            // red with a large yellow star and four small ones in the canton
            p.rect_filled(r, 0.0, Color32::from_rgb(238, 28, 37));
            let yellow = Color32::from_rgb(255, 255, 0);
            let at = |x: f32, y: f32| pos2(r.min.x + x * w, r.min.y + y * h);
            star(&p, at(0.17, 0.27), h * 0.17, yellow);
            for (x, y) in [(0.33, 0.1), (0.4, 0.2), (0.4, 0.35), (0.33, 0.45)] {
                star(&p, at(x, y), h * 0.06, yellow);
            }
        }
        Lang::Ko => {
            // white, the red-and-blue taegeuk and four black trigrams
            p.rect_filled(r, 0.0, Color32::WHITE);
            let c = r.center();
            let rad = h * 0.25;
            let (red, blue) = (Color32::from_rgb(205, 46, 58), Color32::from_rgb(0, 71, 160));
            p.circle_filled(c, rad, blue);
            let top: Vec<Pos2> = (0..=24).map(|i| {
                let a = std::f32::consts::PI * (1.0 + i as f32 / 24.0);
                pos2(c.x + rad * a.cos(), c.y + rad * a.sin())
            }).collect();
            p.add(egui::Shape::convex_polygon(top, red, Stroke::NONE));
            p.circle_filled(c - vec2(rad / 2.0, 0.0), rad / 2.0, red);
            p.circle_filled(c + vec2(rad / 2.0, 0.0), rad / 2.0, blue);
            let black = Stroke::new(h * 0.05, Color32::BLACK);
            for (dx, dy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let m = c + vec2(dx * w * 0.34, dy * h * 0.3);
                let along = vec2(-dy, dx).normalized() * h * 0.09;
                let across = vec2(dx, dy).normalized() * h * 0.06;
                for k in -1..=1 {
                    let o = m + across * k as f32;
                    p.line_segment([o - along, o + along], black);
                }
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

/// A five-pointed star (China's flag), pointing up.
fn star(p: &Painter, c: Pos2, r: f32, col: Color32) {
    let pt = |k: usize, rr: f32| {
        let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
        pos2(c.x + rr * a.cos(), c.y + rr * a.sin())
    };
    let inner = r * 0.382;
    // the inner pentagon and one triangle per point (all convex)
    p.add(egui::Shape::convex_polygon((0..5).map(|k| pt(2 * k + 1, inner)).collect(), col, Stroke::NONE));
    for k in 0..5 {
        p.add(egui::Shape::convex_polygon(vec![pt(2 * k, r), pt(2 * k + 1, inner), pt((2 * k + 9) % 10, inner)], col, Stroke::NONE));
    }
}

/// Three vertical bands (France, Italy).
fn vertical_tricolour(p: &Painter, r: Rect, colours: [Color32; 3]) {
    let w = r.width() / 3.0;
    for (k, c) in colours.into_iter().enumerate() {
        p.rect_filled(Rect::from_min_size(pos2(r.min.x + k as f32 * w, r.min.y), vec2(w, r.height())), 0.0, c);
    }
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
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let vis = ui.style().interact(&resp);
    let pal = crate::theme::pal(ui.ctx());
    let hov = crate::theme::hover_amount(ui.ctx(), resp.id, resp.hovered());
    let p = ui.painter();
    p.rect_filled(r, 8.0, pal.card);
    p.rect_filled(r, 8.0, crate::theme::hover_veil(&pal, hov));
    p.rect_stroke(r, 8.0, Stroke::new(1.0_f32, pal.border), egui::StrokeKind::Inside);
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
        // names in the interface language, alphabetically; each language's
        // own name in grey on the right, so it can be found from any language
        let here = current();
        let small = FontId::proportional(12.0);
        for l in Lang::sorted_in(here) {
            let (rr, rsp) = ui.allocate_exact_size(vec2(330.0, 26.0), Sense::click());
            let p = ui.painter();
            if rsp.hovered() {
                p.rect_filled(rr, 6.0, ui.visuals().widgets.hovered.weak_bg_fill);
            }
            let fr = Rect::from_min_size(pos2(rr.min.x + 6.0, rr.center().y - FLAG.y / 2.0), FLAG);
            paint_flag(p, fr, l);
            let col = ui.visuals().text_color();
            let name = l.name_in(here);
            p.text(pos2(fr.max.x + 8.0, rr.center().y), Align2::LEFT_CENTER, name, font.clone(), col);
            if !name.to_lowercase().starts_with(&l.name().to_lowercase()) {
                let hint = crate::theme::pal(ui.ctx()).hint;
                p.text(pos2(rr.max.x - 28.0, rr.center().y), Align2::RIGHT_CENTER, l.name(), small.clone(), hint);
            }
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
        for (en, tr) in TABLE {
            for x in tr {
                assert_eq!(en.matches("{}").count(), x.matches("{}").count(), "{en} → {x}");
                assert_eq!(en.matches('\n').count(), x.matches('\n').count(), "{en} → {x}");
                assert!(!x.is_empty(), "{en}");
            }
        }
        // every language has a code, a name and a slot in the table
        for (i, l) in Lang::BY_INDEX.iter().enumerate() {
            assert_eq!(*l as usize, i);
            assert!(Lang::ALL.contains(l));
            set(*l);
            assert_eq!(current(), *l);
        }
        set(Lang::Es);
        assert_eq!(t("Save"), "Guardar");
        assert_eq!(tf("Saved to {}", &[&"/tmp/x.png"]), "Guardado en /tmp/x.png");
        assert_eq!(t("not in the table"), "not in the table");
        set(Lang::Ca);
        assert_eq!(t("Quit"), "Surt");
        set(Lang::Fr);
        assert_eq!(t("Quit"), "Quitter");
        set(Lang::De);
        assert_eq!(tf("Saved to {}", &[&"/tmp/x.png"]), "Gespeichert unter /tmp/x.png");
        set(Lang::Pt);
        assert_eq!(t("Save"), "Guardar");
        set(Lang::It);
        assert_eq!(t("Save"), "Salva");
        // the newer columns line up with their languages
        for (l, quit_differs) in [(Lang::Gl, true), (Lang::Eu, true), (Lang::PtBr, true), (Lang::Nl, true), (Lang::Pl, true), (Lang::Ja, true), (Lang::Zh, true), (Lang::Ko, true)] {
            set(l);
            assert_eq!(t("Quit") != "Quit", quit_differs, "{l:?}");
        }
        set(Lang::PtBr);
        assert_ne!(t("Save"), "Guardar", "Brazilian Portuguese is not European Portuguese");
        set(Lang::En);
    }

    #[test]
    fn language_names_sort_alphabetically_in_each_language() {
        // every language names itself as in its own row
        for l in Lang::ALL {
            assert!(l.name_in(l).to_lowercase().starts_with(&l.name().to_lowercase()) || l == Lang::Pt, "{l:?}");
            for ui in Lang::ALL {
                assert!(!l.name_in(ui).is_empty());
            }
        }
        let first = |ui| Lang::sorted_in(ui)[0];
        assert_eq!(first(Lang::En), Lang::Eu); // Basque
        assert_eq!(first(Lang::Es), Lang::De); // Alemán
        assert_eq!(first(Lang::De), Lang::Eu); // Baskisch
        assert_eq!(first(Lang::Ja), Lang::It); // イタリア語 (i…)
        // ガリシア語 (ga…) sorts among the カ (ka…), between カタルーニャ語 and 韓国語
        let ja = Lang::sorted_in(Lang::Ja);
        let at = |l| ja.iter().position(|x| *x == l).unwrap();
        assert!(at(Lang::Ca) < at(Lang::Gl) && at(Lang::Gl) < at(Lang::Ko));
        assert_eq!(first(Lang::Zh), Lang::Eu); // 巴斯克语 (ba…)
        assert_eq!(first(Lang::Ko), Lang::Gl); // 갈리시아어
        // accents do not push a name to the end: Galician's Éuscaro sits among the E's
        let gl = Lang::sorted_in(Lang::Gl);
        let pos = |l| gl.iter().position(|x| *x == l).unwrap();
        assert!(pos(Lang::Eu) < pos(Lang::Fr));
        assert_eq!(Lang::sorted_in(Lang::Fr).len(), 15);
    }
}
