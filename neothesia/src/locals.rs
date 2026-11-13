//! Translated texts: one table per language, so adding a language is one block.
//!
//! A language lists only what it translates and takes the rest from English.
//! Texts with a value in them hold `{}` placeholders and are filled by the
//! functions of their section module.

use std::{fmt::Display, path::Path, sync::LazyLock};

pub struct Texts {
    pub menu: MenuTexts,
    pub freeplay: FreeplayTexts,
    pub settings: SettingsTexts,
    pub playing: PlayingTexts,
}

pub struct MenuTexts {
    pub exit_question: &'static str,
    pub no: &'static str,
    pub yes: &'static str,
    pub loading: &'static str,
    pub select_file: &'static str,
    pub settings: &'static str,
    pub exit: &'static str,
    pub freeplay: &'static str,
    pub play: &'static str,
    pub tracks: &'static str,
}

pub struct FreeplayTexts {
    pub no_note_events_recorded: &'static str,
    pub failed_to_write_midi_file: &'static str,
    pub recording: &'static str,
    pub recorded: &'static str,
    pub saved_recording_to: &'static str,
}

pub struct SettingsTexts {
    pub output: &'static str,
    pub input: &'static str,
    pub play_along_by_default: &'static str,
    pub play_along_by_default_subtitle: &'static str,
    pub input_min_velocity: &'static str,
    pub off: &'static str,
    pub note_range: &'static str,
    pub keyboard_size: &'static str,
    pub keys: &'static str,
    pub custom: &'static str,
    pub range_start: &'static str,
    pub range_end: &'static str,
    pub waterfall: &'static str,
    pub keyboard: &'static str,
    pub octave_lines: &'static str,
    pub octave_lines_subtitle: &'static str,
    pub bar_lines: &'static str,
    pub bar_lines_subtitle: &'static str,
    pub note_names: &'static str,
    pub note_names_subtitle: &'static str,
    pub naming_style: &'static str,
    pub naming_style_keyboard_subtitle: &'static str,
    pub naming_style_waterfall_subtitle: &'static str,
    pub note_naming_octaves: &'static str,
    pub note_naming_letters: &'static str,
    pub note_naming_solfege: &'static str,
    pub note_naming_latin: &'static str,
    pub key_glow: &'static str,
    pub key_glow_subtitle: &'static str,
    pub chord_name: &'static str,
    pub chord_name_subtitle: &'static str,
    pub soundfont: &'static str,
    pub select_file: &'static str,
    pub audio_gain: &'static str,
    pub separate_channels: &'static str,
    pub separate_channels_subtitle: &'static str,
    pub auto_detect: &'static str,
    pub auto_detect_subtitle: &'static str,
    pub detect: &'static str,
    pub detection_subtitle: &'static str,
    pub cancel: &'static str,
    pub detection_in_progress: &'static str,
}

pub struct PlayingTexts {
    pub speed: &'static str,
    pub animation_speed: &'static str,
    pub offset: &'static str,
    /// Shown while the app plays the song for the player to hear it
    pub listening: &'static str,
    /// Shown when the player takes over
    pub your_turn: &'static str,
    /// Goes under "12 / 16"
    pub results_caption: &'static str,
    pub results_previous: &'static str,
    pub play_again: &'static str,
    pub listen: &'static str,
    pub menu: &'static str,
}

const EN: Texts = Texts {
    menu: MenuTexts {
        exit_question: "Do you want to exit?",
        no: "No",
        yes: "Yes",
        loading: "Loading...",
        select_file: "Select File",
        settings: "Settings",
        exit: "Exit",
        freeplay: "FreePlay",
        play: "Play",
        tracks: "Tracks",
    },
    freeplay: FreeplayTexts {
        no_note_events_recorded: "No note events recorded",
        failed_to_write_midi_file: "Failed to write MIDI file",
        recording: "Recording {}s",
        recorded: "Recorded {}s",
        saved_recording_to: "Saved recording to {}",
    },
    settings: SettingsTexts {
        output: "Output",
        input: "Input",
        play_along_by_default: "Play Along by Default",
        play_along_by_default_subtitle: "New songs wait until you play the right keys",
        input_min_velocity: "Minimum Key Velocity",
        off: "Off",
        note_range: "Note Range",
        keyboard_size: "Keyboard Size",
        keys: "{} Keys",
        custom: "Custom",
        range_start: "Start",
        range_end: "End",
        waterfall: "Waterfall",
        keyboard: "Keyboard",
        octave_lines: "Octave Lines",
        octave_lines_subtitle: "Vertical lines at every C and F to help you find keys",
        bar_lines: "Bar Lines",
        bar_lines_subtitle: "Horizontal line at the start of each measure",
        note_names: "Note Names",
        note_names_subtitle: "Show the note name on each falling note",
        naming_style: "Naming Style",
        naming_style_keyboard_subtitle: "Choose how the keys are labeled",
        naming_style_waterfall_subtitle: "Choose how falling notes are labeled",
        note_naming_octaves: "Octaves",
        note_naming_letters: "English Note Names (C, C#, D)",
        note_naming_solfege: "Solfege (Do, Di, Mi)",
        note_naming_latin: "Latin (Do, Re, Mi)",
        key_glow: "Key Glow",
        key_glow_subtitle: "Light up keys as they are played",
        chord_name: "Chord Name",
        chord_name_subtitle: "Name the chord you are playing above the keys",
        soundfont: "SoundFont",
        select_file: "Select File",
        audio_gain: "Audio Gain",
        separate_channels: "Separate Channels",
        separate_channels_subtitle: "Assign different MIDI channel to each track",
        auto_detect: "Auto-detect",
        auto_detect_subtitle: "Auto-detect range from connected keyboard",
        detect: "Detect",
        detection_subtitle: "Play the far-left and far-right key on your keyboard...",
        cancel: "Cancel",
        detection_in_progress: "Detection in progress... Step {}/{}",
    },
    playing: PlayingTexts {
        speed: "Speed: {}",
        animation_speed: "Animation Speed: {}",
        offset: "Offset: {}",
        listening: "Listen",
        your_turn: "Your turn",
        results_caption: "notes right at the first try",
        results_previous: "Last time: {}",
        play_again: "Play again",
        listen: "Listen",
        menu: "Menu",
    },
};

const PL: Texts = Texts {
    menu: MenuTexts {
        exit_question: "Czy chcesz wyjść?",
        no: "Nie",
        yes: "Tak",
        loading: "Ładowanie...",
        select_file: "Wybierz plik",
        settings: "Ustawienia",
        exit: "Wyjdź",
        freeplay: "Swobodna gra",
        play: "Graj",
        tracks: "Ścieżki",
    },
    freeplay: FreeplayTexts {
        no_note_events_recorded: "Nie nagrano żadnych nut",
        failed_to_write_midi_file: "Nie udało się zapisać pliku MIDI",
        recording: "Nagrywanie {}s",
        recorded: "Nagrano {}s",
        saved_recording_to: "Zapisano nagranie w {}",
    },
    settings: SettingsTexts {
        output: "Wyjście",
        input: "Wejście",
        off: "Wyłączone",
        note_range: "Zakres nut",
        keyboard_size: "Rozmiar klawiatury",
        keys: "{} klawiszy",
        custom: "Własny",
        range_start: "Początek",
        range_end: "Koniec",
        waterfall: "Wodospad",
        keyboard: "Klawiatura",
        octave_lines: "Linie oktaw",
        octave_lines_subtitle: "Pionowe linie przy każdym C i F, ułatwiające odnalezienie klawiszy",
        bar_lines: "Linie taktów",
        bar_lines_subtitle: "Pozioma linia na początku każdego taktu",
        note_names: "Nazwy nut",
        note_names_subtitle: "Wyświetlaj nazwę nuty na każdej opadającej nucie",
        naming_style: "Styl nazw",
        naming_style_keyboard_subtitle: "Wybierz sposób nazywania klawiszy",
        naming_style_waterfall_subtitle: "Wybierz sposób nazywania opadających nut",
        note_naming_octaves: "Oktawy",
        note_naming_letters: "Angielskie nazwy nut (C, C#, D)",
        note_naming_solfege: "Solfeż (Do, Di, Mi)",
        note_naming_latin: "Łacińskie (Do, Re, Mi)",
        key_glow: "Poświata klawiszy",
        key_glow_subtitle: "Podświetlaj klawisze podczas gry",
        chord_name: "Nazwa akordu",
        chord_name_subtitle: "Wyświetlaj nazwę granego akordu nad klawiszami",
        select_file: "Wybierz plik",
        audio_gain: "Wzmocnienie dźwięku",
        separate_channels: "Oddzielne kanały",
        separate_channels_subtitle: "Przypisz osobny kanał MIDI do każdej ścieżki",
        auto_detect: "Automatyczne wykrywanie",
        auto_detect_subtitle: "Automatycznie wykryj zakres podłączonej klawiatury",
        detect: "Wykryj",
        detection_subtitle: "Naciśnij skrajnie lewy i skrajnie prawy klawisz na klawiaturze...",
        cancel: "Anuluj",
        detection_in_progress: "Wykrywanie w toku... Krok {}/{}",
        ..EN.settings
    },
    playing: PlayingTexts {
        speed: "Prędkość: {}",
        animation_speed: "Prędkość animacji: {}",
        offset: "Przesunięcie: {}",
        ..EN.playing
    },
};

const DE: Texts = Texts {
    menu: MenuTexts {
        exit_question: "Möchtest Du die App verlassen?",
        no: "Nein",
        yes: "Ja",
        loading: "Wird geladen...",
        select_file: "Datei auswählen",
        settings: "Einstellungen",
        exit: "Beenden",
        freeplay: "Freies Spiel",
        play: "Abspielen",
        tracks: "Stücke",
    },
    freeplay: FreeplayTexts {
        no_note_events_recorded: "Keine Notenereignisse aufgezeichnet",
        failed_to_write_midi_file: "MIDI-Datei konnte nicht geschrieben werden",
        recording: "Aufnahme läuft ({} s)",
        recorded: "Aufnahme abgeschlossen ({} s)",
        saved_recording_to: "Aufnahme gespeichert unter: {}",
    },
    settings: SettingsTexts {
        output: "Ausgang",
        input: "Eingang",
        play_along_by_default: "Standardmäßig mitspielen",
        play_along_by_default_subtitle: "Neue Stücke warten, bis Du die richtigen Tasten spielst",
        input_min_velocity: "Mindest-Anschlagstärke",
        off: "Aus",
        note_range: "Notenumfang",
        keyboard_size: "Tastaturgröße",
        keys: "{} Tasten",
        custom: "Eigene",
        range_end: "Ende",
        waterfall: "Wasserfall",
        keyboard: "Tastatur",
        octave_lines: "Oktavlinien",
        octave_lines_subtitle: "Vertikale Linien bei jedem C und F zur Orientierung",
        bar_lines: "Taktstriche",
        bar_lines_subtitle: "Horizontale Linie am Anfang jedes Takts",
        note_names: "Notennamen",
        note_names_subtitle: "Notennamen auf jeder fallenden Note anzeigen",
        naming_style: "Benennungsstil",
        naming_style_keyboard_subtitle: "Wähle, wie die Tasten beschriftet werden",
        naming_style_waterfall_subtitle: "Wähle, wie fallende Noten beschriftet werden",
        note_naming_octaves: "Oktaven",
        note_naming_letters: "Englische Notennamen (C, C#, D)",
        note_naming_solfege: "Solfège (Do, Di, Mi)",
        note_naming_latin: "Lateinisch (Do, Re, Mi)",
        key_glow: "Tastenleuchten",
        key_glow_subtitle: "Tasten beim Spielen aufleuchten lassen",
        chord_name: "Akkordname",
        chord_name_subtitle: "Gespielten Akkord über den Tasten anzeigen",
        select_file: "Datei auswählen",
        audio_gain: "Audio-Verstärkung",
        separate_channels: "Kanäle trennen",
        separate_channels_subtitle: "Jeder Spur einen eigenen MIDI-Kanal zuweisen",
        auto_detect: "Automatische Erkennung",
        auto_detect_subtitle: "Tonumfang vom angeschlossenen Keyboard automatisch erkennen",
        detect: "Erkennen",
        detection_subtitle: "Spiele die ganz linke und die ganz rechts liegende Taste auf Deinem Keyboard...",
        cancel: "Abbrechen",
        detection_in_progress: "Erkennung läuft... Schritt {}/{}",
        ..EN.settings
    },
    playing: PlayingTexts {
        speed: "Geschwindigkeit: {}",
        animation_speed: "Animationsgeschwindigkeit: {}",
        ..EN.playing
    },
};

const FR: Texts = Texts {
    menu: MenuTexts {
        exit_question: "Voulez-vous quitter ?",
        no: "Non",
        yes: "Oui",
        loading: "Chargement...",
        select_file: "Choisir un fichier",
        settings: "Paramètres",
        exit: "Quitter",
        freeplay: "Jeu libre",
        play: "Jouer",
        tracks: "Pistes",
    },
    freeplay: FreeplayTexts {
        no_note_events_recorded: "Aucune note enregistrée",
        failed_to_write_midi_file: "Impossible d'écrire le fichier MIDI",
        recording: "Enregistrement {} s",
        recorded: "Enregistré {} s",
        saved_recording_to: "Enregistrement sauvegardé : {}",
    },
    settings: SettingsTexts {
        output: "Sortie",
        input: "Entrée",
        play_along_by_default: "Attendre mes touches",
        play_along_by_default_subtitle: "Les nouveaux morceaux attendent la bonne touche pour avancer",
        input_min_velocity: "Force de frappe minimale",
        off: "Désactivée",
        note_range: "Plage de notes",
        keyboard_size: "Taille du clavier",
        keys: "{} touches",
        custom: "Personnalisée",
        range_start: "Début",
        range_end: "Fin",
        waterfall: "Cascade",
        keyboard: "Clavier",
        octave_lines: "Lignes d'octave",
        octave_lines_subtitle: "Lignes verticales à chaque do et fa pour repérer les touches",
        bar_lines: "Barres de mesure",
        bar_lines_subtitle: "Ligne horizontale au début de chaque mesure",
        note_names: "Nom des notes",
        note_names_subtitle: "Afficher le nom de la note sur chaque note qui tombe",
        naming_style: "Notation",
        naming_style_keyboard_subtitle: "Choisir comment les touches sont nommées",
        naming_style_waterfall_subtitle: "Choisir comment les notes qui tombent sont nommées",
        note_naming_letters: "Notation anglaise (C, C#, D)",
        note_naming_solfege: "Solfège (Do, Di, Mi)",
        note_naming_latin: "Notation latine (Do, Re, Mi)",
        key_glow: "Halo des touches",
        key_glow_subtitle: "Illuminer les touches quand elles sont jouées",
        chord_name: "Nom de l'accord",
        chord_name_subtitle: "Afficher le nom de l'accord joué au-dessus des touches",
        select_file: "Choisir un fichier",
        audio_gain: "Gain audio",
        separate_channels: "Canaux séparés",
        separate_channels_subtitle: "Attribuer un canal MIDI différent à chaque piste",
        auto_detect: "Détection automatique",
        auto_detect_subtitle: "Détecter la plage du clavier connecté",
        detect: "Détecter",
        detection_subtitle: "Jouez la touche la plus à gauche puis la plus à droite de votre clavier...",
        cancel: "Annuler",
        detection_in_progress: "Détection en cours... Étape {}/{}",
        ..EN.settings
    },
    playing: PlayingTexts {
        speed: "Vitesse : {}",
        animation_speed: "Vitesse de l'animation : {}",
        offset: "Décalage : {}",
        listening: "Écoute",
        your_turn: "À toi",
        results_caption: "notes justes du premier coup",
        results_previous: "La fois d'avant : {}",
        play_again: "Rejouer",
        listen: "Écouter",
        ..EN.playing
    },
};
/// Maps a BCP 47 locale such as "pl-PL" to the table of its language.
fn texts_for_locale(locale: &str) -> &'static Texts {
    match locale
        .split('-')
        .next()
        .unwrap_or("en")
        .to_lowercase()
        .as_str()
    {
        "pl" => &PL,
        "de" => &DE,
        "fr" => &FR,
        _ => &EN,
    }
}

/// The texts in the language of the user.
pub fn t() -> &'static Texts {
    static TEXTS: LazyLock<&'static Texts> = LazyLock::new(|| {
        let locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
        texts_for_locale(&locale)
    });

    *TEXTS
}

/// Replaces each `{}` of `template`, in order, with the matching argument.
fn fill(template: &str, args: &[&dyn Display]) -> String {
    let mut parts = template.split("{}");
    let mut out = parts.next().unwrap_or_default().to_string();
    for (arg, part) in args.iter().zip(parts) {
        out.push_str(&arg.to_string());
        out.push_str(part);
    }
    out
}

pub mod freeplay {
    use super::*;

    pub fn recording(seconds: f32) -> String {
        fill(t().freeplay.recording, &[&format!("{seconds:.1}")])
    }

    pub fn recorded(seconds: f32) -> String {
        fill(t().freeplay.recorded, &[&format!("{seconds:.1}")])
    }

    pub fn saved_recording_to(path: &Path) -> String {
        fill(t().freeplay.saved_recording_to, &[&path.display()])
    }
}

pub mod settings {
    use super::*;

    pub fn keys(count: u8) -> String {
        fill(t().settings.keys, &[&count])
    }

    pub fn detection_in_progress(step: usize, total: usize) -> String {
        fill(t().settings.detection_in_progress, &[&step, &total])
    }
}

pub mod playing {
    use super::*;

    pub fn speed(speed: f32) -> String {
        fill(t().playing.speed, &[&speed])
    }

    pub fn animation_speed(speed: f32) -> String {
        fill(t().playing.animation_speed, &[&speed])
    }

    pub fn offset(offset: f32) -> String {
        fill(t().playing.offset, &[&offset])
    }

    pub fn results_previous(first_try: usize) -> String {
        fill(t().playing.results_previous, &[&first_try])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Consts have no stable address, so tables are told apart by a text
    /// that differs in each language.
    fn is(a: &Texts, b: &Texts) -> bool {
        a.menu.yes == b.menu.yes
    }

    #[test]
    fn locale_maps_to_language_table() {
        assert!(is(texts_for_locale("fr-FR"), &FR));
        assert!(is(texts_for_locale("fr"), &FR));
        assert!(is(texts_for_locale("pl-PL"), &PL));
        assert!(is(texts_for_locale("de-AT"), &DE));
        assert!(is(texts_for_locale("en-US"), &EN));
        assert!(is(texts_for_locale("FR-ca"), &FR));
        assert!(is(texts_for_locale("xx-YY"), &EN));
        assert!(is(texts_for_locale(""), &EN));
    }

    #[test]
    fn missing_translation_falls_back_to_english() {
        assert_eq!(PL.playing.play_again, EN.playing.play_again);
        assert_eq!(FR.playing.play_again, "Rejouer");
    }

    #[test]
    fn templates_keep_their_placeholders_in_every_language() {
        type Getter = fn(&Texts) -> &'static str;
        let templated: [(&str, Getter); 9] = [
            ("recording", |t| t.freeplay.recording),
            ("recorded", |t| t.freeplay.recorded),
            ("saved_recording_to", |t| t.freeplay.saved_recording_to),
            ("keys", |t| t.settings.keys),
            ("detection_in_progress", |t| {
                t.settings.detection_in_progress
            }),
            ("speed", |t| t.playing.speed),
            ("animation_speed", |t| t.playing.animation_speed),
            ("offset", |t| t.playing.offset),
            ("results_previous", |t| t.playing.results_previous),
        ];

        for (name, get) in templated {
            let expected = get(&EN).matches("{}").count();
            assert!(expected > 0, "{name} has no placeholder in English");
            for (lang, table) in [("pl", &PL), ("de", &DE), ("fr", &FR)] {
                assert_eq!(
                    get(table).matches("{}").count(),
                    expected,
                    "{name} in {lang}"
                );
            }
        }
    }

    #[test]
    fn fill_formats_like_the_old_format_calls() {
        assert_eq!(fill("Speed: {}", &[&1.25f32]), "Speed: 1.25");
        assert_eq!(fill("Speed: {}", &[&1.0f32]), "Speed: 1");
        assert_eq!(fill("Step {}/{}", &[&2usize, &3usize]), "Step 2/3");
        assert_eq!(fill("{} Keys", &[&88u8]), "88 Keys");
    }
}
