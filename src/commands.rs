//! Shared command catalogue for menus and searchable palette.
use super::*;

pub enum Target {
    Action(Action),
    Settings(usize, &'static str),
    Section(&'static str),
    Export(&'static str),
    Tool(&'static str),
    Page(usize),
}
pub struct Command {
    pub id: String,
    pub label: String,
    pub group: &'static str,
    pub target: Target,
    pub reason: Option<&'static str>,
    pub checked: bool,
    pub shortcut: String,
}
impl Command {
    fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        group: &'static str,
        target: Target,
        reason: Option<&'static str>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            group,
            target,
            reason,
            checked: false,
            shortcut: String::new(),
        }
    }
    pub fn matches(&self, query: &str) -> bool {
        let text = format!("{} {} {}", self.label, self.group, self.id).to_lowercase();
        query
            .to_lowercase()
            .split_whitespace()
            .all(|word| settings::matches(word, &text))
    }
}
impl NotepadApp {
    pub(crate) fn command_catalogue(&self) -> Vec<Command> {
        use Target::*;
        let mood = self.active_tab().doc.moodboard;
        let readonly = self.active_tab().workspace.read_only;
        let text = mood.then_some("Disponible sur une page texte");
        let writable = readonly.then_some("Page en lecture seule");
        let edit = text.or(writable);
        let board = (!mood).then_some("Ouvrez un moodboard");
        let board_edit = board.or(writable);
        let selection = board_edit.or_else(|| {
            self.selected_mood_item
                .and_then(|i| self.active_tab().mood_items.get(i))
                .is_none()
                .then_some("Sélectionnez une note ou une image")
        });
        let mut commands = Vec::new();
        macro_rules! add {
            ($id:expr,$label:expr,$group:expr,$target:expr,$reason:expr) => {
                commands.push(Command::new($id, $label, $group, $target, $reason));
            };
        }
        add!(
            "new",
            "Nouvelle page",
            "Fichier",
            Action(super::Action::New),
            None
        );
        add!(
            "new-board",
            "Nouveau moodboard",
            "Fichier",
            Action(super::Action::NewMoodboard),
            None
        );
        add!(
            "window",
            "Nouvelle fenêtre",
            "Fichier",
            Action(super::Action::NewWindow),
            None
        );
        add!(
            "open",
            "Ouvrir…",
            "Fichier",
            Action(super::Action::Open(None)),
            None
        );
        add!(
            "save",
            "Enregistrer",
            "Fichier",
            Action(super::Action::Save),
            None
        );
        add!(
            "save-as",
            "Enregistrer sous…",
            "Fichier",
            Action(super::Action::SaveAs),
            None
        );
        add!(
            "save-all",
            "Tout enregistrer",
            "Fichier",
            Action(super::Action::SaveAll),
            None
        );
        add!(
            "page-setup",
            "Mise en page…",
            "Fichier",
            Action(super::Action::PageSetup),
            text
        );
        add!(
            "print",
            "Imprimer…",
            "Fichier",
            Action(super::Action::Print),
            text
        );
        add!(
            "print-preview",
            "Aperçu avant impression",
            "Fichier",
            Action(super::Action::PrintPreview),
            text
        );
        add!(
            "close",
            "Fermer l’onglet",
            "Fichier",
            Action(super::Action::CloseTab),
            None
        );
        add!(
            "close-window",
            "Fermer la fenêtre",
            "Fichier",
            Action(super::Action::CloseWindow),
            None
        );
        add!(
            "quit",
            "Quitter",
            "Fichier",
            Action(super::Action::Quit),
            None
        );
        add!(
            "undo",
            "Annuler",
            "Édition",
            Action(super::Action::Undo),
            edit
        );
        add!(
            "redo",
            "Rétablir",
            "Édition",
            Action(super::Action::Redo),
            edit
        );
        add!("cut", "Couper", "Édition", Action(super::Action::Cut), edit);
        add!(
            "copy",
            "Copier",
            "Édition",
            Action(super::Action::Copy),
            text
        );
        add!(
            "paste",
            "Coller",
            "Édition",
            Action(super::Action::Paste),
            edit
        );
        add!(
            "delete-text",
            "Supprimer la sélection de texte",
            "Édition",
            Action(super::Action::DeleteSelection),
            edit
        );
        add!(
            "select-all",
            "Tout sélectionner",
            "Édition",
            Action(super::Action::SelectAll),
            text
        );
        add!(
            "find",
            "Rechercher…",
            "Édition",
            Action(super::Action::Find),
            text
        );
        add!(
            "next",
            "Rechercher suivant",
            "Édition",
            Action(super::Action::FindNext),
            text
        );
        add!(
            "previous",
            "Rechercher précédent",
            "Édition",
            Action(super::Action::FindPrevious),
            text
        );
        add!(
            "replace",
            "Remplacer…",
            "Édition",
            Action(super::Action::Replace),
            edit
        );
        add!(
            "goto",
            "Atteindre une ligne…",
            "Édition",
            Action(super::Action::Goto),
            text
        );
        add!(
            "font",
            "Police…",
            "Édition",
            Action(super::Action::Font),
            text
        );
        add!(
            "date",
            "Heure et date",
            "Insertion",
            Action(super::Action::InsertDate),
            edit
        );
        add!(
            "zoom-in",
            "Zoom avant",
            "Affichage",
            Action(super::Action::Zoom(1)),
            text
        );
        add!(
            "zoom-out",
            "Zoom arrière",
            "Affichage",
            Action(super::Action::Zoom(-1)),
            text
        );
        add!(
            "zoom-reset",
            "Zoom du texte à 100 %",
            "Affichage",
            Action(super::Action::ResetZoom),
            text
        );
        add!(
            "wrap",
            "Retour à la ligne",
            "Affichage",
            Action(super::Action::ToggleWrap),
            text
        );
        add!(
            "status",
            "Barre d’état",
            "Affichage",
            Action(super::Action::ToggleStatus),
            None
        );
        add!(
            "markdown",
            "Mode Markdown",
            "Affichage",
            Action(super::Action::ToggleMarkdown),
            text
        );
        add!(
            "preview",
            "Aperçu Markdown",
            "Affichage",
            Action(super::Action::ToggleMarkdownPreview),
            text
        );
        add!(
            "format-bar",
            "Barre de mise en forme",
            "Affichage",
            Action(super::Action::ToggleFormatBar),
            text
        );
        add!(
            "about",
            "À propos de Notepad RS",
            "Aide",
            Action(super::Action::About),
            None
        );
        add!(
            "format-bold",
            "Gras",
            "Insertion",
            Action(super::Action::Format(FormatKind::Bold)),
            edit
        );
        add!(
            "format-italic",
            "Italique",
            "Insertion",
            Action(super::Action::Format(FormatKind::Italic)),
            edit
        );
        add!(
            "format-strike",
            "Barré",
            "Insertion",
            Action(super::Action::Format(FormatKind::Strike)),
            edit
        );
        add!(
            "format-h1",
            "Titre 1",
            "Insertion",
            Action(super::Action::Format(FormatKind::Heading(1))),
            edit
        );
        add!(
            "format-h2",
            "Titre 2",
            "Insertion",
            Action(super::Action::Format(FormatKind::Heading(2))),
            edit
        );
        add!(
            "format-h3",
            "Titre 3",
            "Insertion",
            Action(super::Action::Format(FormatKind::Heading(3))),
            edit
        );
        add!(
            "format-bullet",
            "Liste à puces",
            "Insertion",
            Action(super::Action::Format(FormatKind::BulletList)),
            edit
        );
        add!(
            "format-ordered",
            "Liste numérotée",
            "Insertion",
            Action(super::Action::Format(FormatKind::OrderedList)),
            edit
        );
        add!(
            "format-quote",
            "Citation",
            "Insertion",
            Action(super::Action::Format(FormatKind::Quote)),
            edit
        );
        add!(
            "format-code",
            "Code en ligne",
            "Insertion",
            Action(super::Action::Format(FormatKind::Code)),
            edit
        );
        add!(
            "format-code-block",
            "Bloc de code",
            "Insertion",
            Action(super::Action::Format(FormatKind::CodeBlock)),
            edit
        );
        add!(
            "format-link",
            "Lien Markdown",
            "Insertion",
            Action(super::Action::Format(FormatKind::Link)),
            edit
        );
        for enc in ENCODINGS {
            add!(
                format!("encoding-{:?}", enc),
                format!("Enregistrer en {}", enc.label()),
                "Encodage",
                Action(super::Action::SetEncoding(enc)),
                edit
            );
            let reason = text.or(writable).or_else(|| {
                (self.active_tab().doc.dirty || self.active_tab().doc.raw.is_none())
                    .then_some("Nécessite un fichier ouvert sans modifications non enregistrées")
            });
            add!(
                format!("reopen-{:?}", enc),
                format!("Rouvrir les octets en {}", enc.label()),
                "Encodage",
                Action(super::Action::ReopenEncoding(enc)),
                reason
            );
        }
        for eol in EOL_ALL {
            add!(
                format!("eol-{:?}", eol),
                format!("Fins de ligne : {}", eol.label()),
                "Encodage",
                Action(super::Action::SetEol(eol)),
                edit
            );
        }
        for theme in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
            add!(
                format!("theme-{}", theme.label()),
                format!("Thème {}", theme.label()),
                "Affichage",
                Action(super::Action::SetTheme(theme)),
                None
            );
        }
        for ext in ["md", "html", "pdf", "json", "png", "jpg"] {
            let for_board = matches!(ext, "json" | "png" | "jpg");
            add!(
                format!("export-{ext}"),
                format!("Exporter .{ext}…"),
                "Export",
                Export(ext),
                if for_board { board } else { text }
            );
        }
        for (id, label, group, reason) in [
            ("search", "Rechercher dans l’espace", "Outils", None),
            ("history", "Historique et versions…", "Page", None),
            ("duplicate-page", "Dupliquer la page", "Page", None),
            ("rename", "Renommer la page", "Page", writable),
            ("subpage", "Créer une sous-page", "Page", None),
            ("favorite", "Ajouter / retirer des favoris", "Page", None),
            (
                "trash-page",
                "Déplacer la page vers la corbeille",
                "Page",
                None,
            ),
            ("trash", "Corbeille", "Page", None),
            (
                "undo-trash",
                "Annuler la mise à la corbeille",
                "Page",
                self.options
                    .undo_trash
                    .is_empty()
                    .then_some("Aucune mise à la corbeille à annuler"),
            ),
            ("properties", "Propriétés de la page", "Page", None),
            ("sidebar", "Navigation des pages", "Affichage", None),
            ("concentration", "Mode concentration", "Affichage", None),
            ("palette", "Toutes les commandes…", "Aide", None),
            ("board-note", "Ajouter une note", "Insertion", board_edit),
            (
                "board-image",
                "Importer une image…",
                "Insertion",
                board_edit,
            ),
            (
                "board-duplicate",
                "Dupliquer l’élément",
                "Édition",
                selection,
            ),
            ("board-delete", "Supprimer l’élément", "Édition", selection),
            ("board-fit", "Ajuster le canevas", "Affichage", board),
        ] {
            add!(id, label, group, Tool(id), reason);
        }
        for section in [
            "Élément sélectionné",
            "Canevas",
            "Grille et alignement",
            "Vue",
            "Calques et ajout",
            "Export",
        ] {
            add!(
                format!("section-{section}"),
                format!("Propriétés du moodboard · {section}"),
                "Moodboard",
                Section(section),
                board
            );
        }
        for (label, section, reason) in [
            ("Format et dimensions du canevas", "Canevas", board),
            ("Fond et cadre du canevas", "Canevas", board),
            (
                "Points, lignes et espacement",
                "Grille et alignement",
                board,
            ),
            ("Magnétisme des déplacements", "Grille et alignement", board),
            ("Zoom et déplacement de la vue", "Vue", board),
            (
                "Masquer ou verrouiller un calque",
                "Calques et ajout",
                board,
            ),
            (
                "Position, dimensions et proportions de l’élément",
                "Élément sélectionné",
                selection,
            ),
            (
                "Alignement et adaptation au canevas",
                "Élément sélectionné",
                selection,
            ),
            ("Lien de l’élément", "Élément sélectionné", selection),
            (
                "Couleur et texte de la note",
                "Élément sélectionné",
                selection.or_else(|| {
                    self.selected_mood_item
                        .and_then(|i| self.active_tab().mood_items.get(i))
                        .is_some_and(|item| item.image_path.is_some())
                        .then_some("Sélectionnez une note")
                }),
            ),
            (
                "Remplacer l’image",
                "Élément sélectionné",
                selection.or_else(|| {
                    self.selected_mood_item
                        .and_then(|i| self.active_tab().mood_items.get(i))
                        .is_some_and(|item| item.image_path.is_none())
                        .then_some("Sélectionnez une image")
                }),
            ),
            (
                "Disposition avant / arrière",
                "Élément sélectionné",
                selection,
            ),
        ] {
            add!(
                format!("setting-{label}"),
                format!("Régler : {label}"),
                "Moodboard",
                Section(section),
                reason
            );
        }
        for (i, label) in settings::CATEGORIES.iter().enumerate() {
            add!(
                format!("settings-{i}"),
                format!("Paramètres · {label}"),
                "Outils",
                Settings(i, ""),
                None
            );
        }
        add!(
            "preference-0-Démarrer sur une page vide",
            "Réglage · Démarrer sur une page vide",
            "Réglages",
            Settings(0, "Démarrer sur une page vide"),
            None
        );
        add!(
            "preference-0-Ouvrir dans une nouvelle fenêtre",
            "Réglage · Ouvrir dans une nouvelle fenêtre",
            "Réglages",
            Settings(0, "Ouvrir dans une nouvelle fenêtre"),
            None
        );
        add!(
            "preference-0-Type des nouvelles pages",
            "Réglage · Type des nouvelles pages",
            "Réglages",
            Settings(0, "Type des nouvelles pages"),
            None
        );
        add!(
            "preference-0-Profils de travail",
            "Réglage · Profils de travail",
            "Réglages",
            Settings(0, "Profils de travail"),
            None
        );
        add!(
            "preference-1-Thème",
            "Réglage · Thème",
            "Réglages",
            Settings(1, "Thème"),
            None
        );
        add!(
            "preference-1-Échelle de l’interface",
            "Réglage · Échelle de l’interface",
            "Réglages",
            Settings(1, "Échelle de l’interface"),
            None
        );
        add!(
            "preference-1-Navigation et barre d’état",
            "Réglage · Navigation et barre d’état",
            "Réglages",
            Settings(1, "Navigation et barre d’état"),
            None
        );
        add!(
            "preference-1-Mode concentration",
            "Réglage · Mode concentration",
            "Réglages",
            Settings(1, "Mode concentration"),
            None
        );
        add!(
            "preference-2-Police et taille",
            "Réglage · Police et taille",
            "Réglages",
            Settings(2, "Police et taille"),
            None
        );
        add!(
            "preference-2-Modèles personnalisés",
            "Réglage · Modèles personnalisés",
            "Réglages",
            Settings(2, "Modèles personnalisés"),
            None
        );
        add!(
            "preference-3-Afficher la grille",
            "Réglage · Afficher la grille",
            "Réglages",
            Settings(3, "Afficher la grille"),
            None
        );
        add!(
            "preference-3-Alignement magnétique",
            "Réglage · Alignement magnétique",
            "Réglages",
            Settings(3, "Alignement magnétique"),
            None
        );
        add!(
            "preference-4-Encodage par défaut",
            "Réglage · Encodage par défaut",
            "Réglages",
            Settings(4, "Encodage par défaut"),
            None
        );
        add!(
            "preference-4-Fins de ligne par défaut",
            "Réglage · Fins de ligne par défaut",
            "Réglages",
            Settings(4, "Fins de ligne par défaut"),
            None
        );
        add!(
            "preference-4-Conserver les fichiers récents",
            "Réglage · Conserver les fichiers récents",
            "Réglages",
            Settings(4, "Conserver les fichiers récents"),
            None
        );
        add!(
            "preference-4-Nombre de fichiers récents",
            "Réglage · Nombre de fichiers récents",
            "Réglages",
            Settings(4, "Nombre de fichiers récents"),
            None
        );
        add!(
            "preference-5-Sauvegarde portable",
            "Réglage · Sauvegarde portable",
            "Réglages",
            Settings(5, "Sauvegarde portable"),
            None
        );
        add!(
            "preference-5-Sauvegardes planifiées",
            "Réglage · Sauvegardes planifiées",
            "Réglages",
            Settings(5, "Sauvegardes planifiées"),
            None
        );
        add!(
            "preference-5-Versions automatiques",
            "Réglage · Versions automatiques",
            "Réglages",
            Settings(5, "Versions automatiques"),
            None
        );
        add!(
            "preference-5-Versions par page",
            "Réglage · Versions par page",
            "Réglages",
            Settings(5, "Versions par page"),
            None
        );
        add!(
            "preference-2-Largeur de lecture",
            "Réglage · Largeur de lecture",
            "Réglages",
            Settings(2, "Largeur de lecture"),
            None
        );
        add!(
            "preference-2-Interligne",
            "Réglage · Interligne",
            "Réglages",
            Settings(2, "Interligne"),
            None
        );
        add!(
            "preference-3-Pas de grille",
            "Réglage · Pas de grille",
            "Réglages",
            Settings(3, "Pas de grille"),
            None
        );
        add!(
            "backup-export",
            "Ouvrir : Sauvegarder l’espace de travail…",
            "Outils",
            Settings(5, ""),
            None
        );
        add!(
            "backup-import",
            "Ouvrir : Importer les pages d’une sauvegarde…",
            "Outils",
            Settings(5, ""),
            None
        );
        add!(
            "backup-recover",
            "Ouvrir : Récupérer les pages du fichier .bak…",
            "Outils",
            Settings(5, ""),
            None
        );
        add!(
            "backup-schedule",
            "Ouvrir : Planifier les sauvegardes…",
            "Outils",
            Settings(5, ""),
            None
        );
        add!(
            "sync-folder",
            "Ouvrir : Choisir le dossier de synchronisation…",
            "Outils",
            Settings(6, ""),
            None
        );
        add!(
            "sync-now",
            "Ouvrir : Synchroniser maintenant ou réessayer…",
            "Outils",
            Settings(6, ""),
            None
        );
        add!(
            "sync-conflict",
            "Ouvrir : Comparer les copies de conflit…",
            "Outils",
            Settings(6, ""),
            None
        );
        add!(
            "sync-pause",
            "Ouvrir : Pause et fréquence de synchronisation…",
            "Outils",
            Settings(6, ""),
            None
        );
        add!(
            "shortcuts",
            "Ouvrir : Affecter les raccourcis clavier…",
            "Outils",
            Settings(7, ""),
            None
        );
        add!(
            "preferences-export",
            "Ouvrir : Exporter les préférences uniquement…",
            "Outils",
            Settings(8, ""),
            None
        );
        add!(
            "preferences-import",
            "Ouvrir : Importer les préférences…",
            "Outils",
            Settings(8, ""),
            None
        );
        add!(
            "preferences-reset",
            "Ouvrir : Réinitialiser les préférences…",
            "Outils",
            Settings(8, ""),
            None
        );
        add!(
            "diagnostic",
            "Ouvrir : Copier un diagnostic sans contenu ni chemins…",
            "Outils",
            Settings(8, ""),
            None
        );
        add!(
            "templates",
            "Ouvrir : Modèles de pages…",
            "Outils",
            Settings(2, ""),
            None
        );
        add!(
            "page-icon",
            "Régler : Icône et couleur de repère de la page",
            "Page",
            Tool("properties"),
            text
        );
        add!(
            "page-width",
            "Régler : Largeur de lecture propre à cette page",
            "Page",
            Tool("properties"),
            text
        );
        add!(
            "page-links",
            "Régler : Liens sortants et liens entrants",
            "Page",
            Tool("properties"),
            text
        );
        add!(
            "block-properties",
            "Régler : Type, déplacement et duplication du bloc",
            "Page",
            Tool("properties"),
            text
        );
        add!(
            "readonly",
            "Lecture seule de la page",
            "Page",
            Tool("readonly"),
            None
        );
        add!(
            "page-blocks",
            "Édition par blocs",
            "Affichage",
            Tool("blocks"),
            text
        );
        add!(
            "page-text",
            "Édition texte",
            "Affichage",
            Tool("text"),
            text
        );
        add!(
            "close-others",
            "Fermer les autres onglets",
            "Fichier",
            Action(super::Action::CloseOthers(self.active)),
            None
        );
        add!(
            "close-right",
            "Fermer les onglets à droite",
            "Fichier",
            Action(super::Action::CloseToTheRight(self.active)),
            (self.active + 1 >= self.tabs.len()).then_some("Aucun onglet à droite")
        );
        for (id, label, a) in [
            (
                "copy-path",
                "Copier le chemin du fichier",
                super::Action::CopyPath(self.active),
            ),
            (
                "reveal",
                "Afficher le fichier dans son dossier",
                super::Action::RevealPath(self.active),
            ),
        ] {
            add!(
                id,
                label,
                "Fichier",
                Action(a),
                self.active_tab()
                    .doc
                    .path
                    .is_none()
                    .then_some("Enregistrez d’abord le fichier")
            );
        }
        add!(
            "clear-recent",
            "Effacer la liste des récents",
            "Fichier",
            Action(super::Action::ClearRecent),
            None
        );
        for (i, t) in self.tabs.iter().enumerate() {
            if !t.workspace.trashed {
                add!(
                    format!("page-{}", t.doc.id),
                    ui_design::page_title(t),
                    "Pages ouvertes",
                    Page(i),
                    None
                );
            }
        }
        for entry in &self.recent {
            add!(
                format!("recent-{entry}"),
                format!("Ouvrir récent · {entry}"),
                "Fichiers récents",
                Action(super::Action::OpenRecent(PathBuf::from(entry))),
                None
            );
        }
        for c in &mut commands {
            c.checked = match c.id.as_str() {
                "readonly" => readonly,
                "favorite" => self.active_tab().workspace.favorite,
                "wrap" => self.active_tab().doc.wrap,
                "status" => self.status_bar,
                "markdown" => self.active_tab().doc.markdown,
                "preview" => self.show_markdown_preview,
                "format-bar" => self.show_format_bar,
                "sidebar" => self.design.sidebar,
                "concentration" => self.options.concentration,
                "page-blocks" => self.active_tab().workspace.visual_blocks,
                "page-text" => !self.active_tab().workspace.visual_blocks,
                _ => false,
            };
            c.shortcut = match c.id.as_str() {
                "new" => "Ctrl+N",
                "window" => "Ctrl+Maj+N",
                "open" => "Ctrl+O",
                "save" => "Ctrl+S",
                "save-as" => "Ctrl+Maj+S",
                "close" => "Ctrl+W",
                "close-window" => "Ctrl+Maj+W",
                "print" => "Ctrl+P",
                "undo" => "Ctrl+Z",
                "redo" => "Ctrl+Y",
                "cut" => "Ctrl+X",
                "copy" => "Ctrl+C",
                "paste" => "Ctrl+V",
                "select-all" => "Ctrl+A",
                "find" => "Ctrl+F",
                "replace" => "Ctrl+H",
                "goto" => "Ctrl+G",
                "next" => "F3",
                "previous" => "Maj+F3",
                "date" => "F5",
                "preview" => "Ctrl+Maj+M",
                "palette" => "Ctrl+Maj+P",
                _ => "",
            }
            .into();
            let custom = match c.id.as_str() {
                "concentration" => "focus",
                "settings-0" => "settings",
                id => id,
            };
            if let Some(key) = self.preferences.shortcuts.get(custom) {
                c.shortcut = key.clone();
            }
        }
        commands
    }

    pub(crate) fn execute_command(&mut self, ctx: &egui::Context, id: &str) {
        let Some(command) = self.command_catalogue().into_iter().find(|c| c.id == id) else {
            return;
        };
        if let Some(reason) = command.reason {
            self.status = reason.into();
            return;
        }
        match command.target {
            Target::Action(a) => self.handle_action(ctx, a),
            Target::Settings(i, q) => {
                self.options.category = i;
                self.options.query = q.into();
                self.options.modified_only = false;
                self.design.settings = true;
            }
            Target::Section(s) => {
                self.design.inspector = true;
                self.design.inspector_section = Some(s);
            }
            Target::Export(ext) => self.export_page(ext, ctx),
            Target::Page(i) => {
                self.active = i;
                self.selected_mood_item = None;
                self.design.selected_block = None;
                self.pending_focus = true;
            }
            Target::Tool(tool) => match tool {
                "readonly" => {
                    self.active_tab_mut().workspace.read_only =
                        !self.active_tab().workspace.read_only;
                    self.session_dirty = true;
                }
                "blocks" | "text" => {
                    self.active_tab_mut().workspace.visual_blocks = tool == "blocks";
                    self.design.selected_block = None;
                    self.session_dirty = true;
                }
                "search" => self.options.search = true,
                "history" => self.options.history = true,
                "duplicate-page" => self.duplicate_page(),
                "rename" => {
                    self.design.rename = Some(self.active_tab().doc.id);
                    self.design.rename_draft = ui_design::page_title(self.active_tab());
                    self.design.sidebar = true;
                }
                "subpage" => {
                    let id = self.active_tab().doc.id;
                    self.new_tab();
                    self.active_tab_mut().page_parent = Some(id);
                    self.session_dirty = true;
                }
                "favorite" => {
                    self.active_tab_mut().workspace.favorite =
                        !self.active_tab().workspace.favorite;
                    self.session_dirty = true;
                }
                "trash-page" => self.trash_page(self.active_tab().doc.id),
                "trash" => self.design.trash = true,
                "undo-trash" => self.undo_trash(),
                "properties" => {
                    self.design.inspector = true;
                    if self.active_tab().doc.moodboard {
                        self.design.inspector_section = Some("Canevas");
                    }
                }
                "sidebar" => {
                    self.design.sidebar = !self.design.sidebar;
                    self.session_dirty = true;
                }
                "concentration" => self.options.concentration = !self.options.concentration,
                "palette" => self.open_palette(),
                "board-image" => self.design.board_import_requested = true,
                "board-fit" => self.design.board_fit_requested = true,
                "board-note" => {
                    let offset = 40.0 + self.active_tab().mood_items.len() as f32 * 20.0;
                    self.active_tab_mut().mood_items.push(session::MoodItem {
                        text: "Nouvelle note".into(),
                        x: offset,
                        y: offset,
                        ..Default::default()
                    });
                    self.selected_mood_item = Some(self.active_tab().mood_items.len() - 1);
                    self.active_tab_mut().doc.dirty = true;
                    self.session_dirty = true;
                }
                "board-duplicate" | "board-delete" => {
                    if let Some(i) = self.selected_mood_item {
                        if tool == "board-delete" {
                            self.active_tab_mut().mood_items.remove(i);
                            self.selected_mood_item = None;
                        } else {
                            let mut item = self.active_tab().mood_items[i].clone();
                            item.x += 24.0;
                            item.y += 24.0;
                            self.active_tab_mut().mood_items.push(item);
                            self.selected_mood_item = Some(self.active_tab().mood_items.len() - 1);
                        }
                        self.active_tab_mut().doc.dirty = true;
                        self.session_dirty = true;
                    }
                }
                _ => {}
            },
        }
    }

    pub(crate) fn catalog_menu_bar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<super::Action>) {
        let catalog = self.command_catalogue();
        let mut picked = None;
        ui.horizontal_wrapped(|ui| {
            for group in [
                "Fichier",
                "Édition",
                "Page",
                "Insertion",
                "Affichage",
                "Outils",
                "Aide",
            ] {
                ui.menu_button(group, |ui| {
                    for c in catalog.iter().filter(|c| c.group == group) {
                        if command_button(ui, c).clicked() {
                            picked = Some(c.id.clone());
                            ui.close();
                        }
                    }
                    if group == "Fichier" {
                        self.recent_menu(ui, actions);
                        ui.menu_button("Exporter", |ui| {
                            for c in catalog.iter().filter(|c| c.group == "Export") {
                                if command_button(ui, c).clicked() {
                                    picked = Some(c.id.clone());
                                    ui.close();
                                }
                            }
                        });
                        ui.menu_button("Encodage et fins de ligne", |ui| {
                            for c in catalog.iter().filter(|c| c.group == "Encodage") {
                                if command_button(ui, c).clicked() {
                                    picked = Some(c.id.clone());
                                    ui.close();
                                }
                            }
                        });
                    }
                    if group == "Page" {
                        ui.menu_button("Déplacer vers…", |ui| self.move_page_menu(ui));
                    }
                    if group == "Affichage" {
                        ui.menu_button("Onglets ouverts", |ui| self.tab_strip(ui, actions));
                        ui.menu_button("Échelle de l’interface", |ui| {
                            let ctx = ui.ctx().clone();
                            self.interface_menu_items(ui, &ctx);
                        });
                        ui.menu_button("Propriétés du moodboard", |ui| {
                            for c in catalog.iter().filter(|c| c.group == "Moodboard") {
                                if command_button(ui, c).clicked() {
                                    picked = Some(c.id.clone());
                                    ui.close();
                                }
                            }
                        });
                    }
                });
            }
            if ui.button("Rechercher une commande…").clicked() {
                self.open_palette();
            }
        });
        if let Some(id) = picked {
            self.execute_command(&ui.ctx().clone(), &id);
        }
    }
}
pub fn command_button(ui: &mut egui::Ui, c: &Command) -> egui::Response {
    let mut label = c.label.clone();
    if c.checked {
        label = format!("✓ {label}");
    }
    if !c.shortcut.is_empty() {
        label.push_str(&format!("    {}", c.shortcut));
    }
    let response = ui.add_enabled(c.reason.is_none(), egui::Button::new(label));
    if let Some(reason) = c.reason {
        response.on_disabled_hover_text(reason)
    } else {
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(mode: &str) -> (NotepadApp, egui::Context) {
        let ctx = egui::Context::default();
        let mut app = NotepadApp::new(
            &eframe::CreationContext::_new_kittest(ctx.clone()),
            None,
            std::env::temp_dir().join(format!("commands-{}.json", workspace_io::uid())),
        );
        app.setup_preview(mode, "unused-command-preview.png");
        (app, ctx)
    }
    #[test]
    fn catalogue_unique_searchable_and_contextual() {
        for mode in [
            "notes",
            "moodboard-canvas-note-selection",
            "moodboard-canvas",
        ] {
            let (mut app, ctx) = fixture(mode);
            let all = app.command_catalogue();
            let mut ids = std::collections::HashSet::new();
            for c in &all {
                assert!(ids.insert(&c.id), "duplicate {}", c.id);
                assert!(c.matches(&c.label));
            }
            assert!(all.len() > 100);
            assert!(all.iter().any(|c| c.matches("encodage")));
            let available = |id: &str| all.iter().find(|c| c.id == id).unwrap().reason.is_none();
            assert_eq!(available("find"), mode == "notes");
            assert_eq!(available("board-note"), mode != "notes");
            assert_eq!(available("export-png"), mode != "notes");
            assert_eq!(available("export-pdf"), mode == "notes");
            app.active_tab_mut().workspace.read_only = true;
            let before = serde_json::to_vec(&app.active_tab().mood_items).unwrap();
            app.execute_command(&ctx, "board-delete");
            assert_eq!(
                before,
                serde_json::to_vec(&app.active_tab().mood_items).unwrap()
            );
        }
    }
    #[test]
    fn note_commands_duplicate_delete_and_persist() {
        let (mut app, ctx) = fixture("moodboard-canvas");
        let n = app.active_tab().mood_items.len();
        app.execute_command(&ctx, "board-note");
        assert_eq!(app.active_tab().mood_items.len(), n + 1);
        app.execute_command(&ctx, "board-duplicate");
        assert_eq!(app.active_tab().mood_items.len(), n + 2);
        app.execute_command(&ctx, "board-delete");
        assert_eq!(app.active_tab().mood_items.len(), n + 1);
        let saved = serde_json::to_vec(&app.session_snapshot(&ctx)).unwrap();
        let restored: Session = serde_json::from_slice(&saved).unwrap();
        assert_eq!(restored.tabs[app.active].mood_items.len(), n + 1);
    }
    #[test]
    fn all_settings_routes_clear_hidden_filters() {
        let (mut app, ctx) = fixture("notes");
        for c in app.command_catalogue() {
            if let Target::Settings(category, query) = c.target {
                app.options.modified_only = true;
                app.execute_command(&ctx, &c.id);
                assert!(app.design.settings);
                assert!(!app.options.modified_only);
                assert_eq!(app.options.category, category);
                assert_eq!(app.options.query, query);
            }
        }
    }
    #[test]
    fn permanent_summary_reaches_all_sections_for_note_and_image() {
        for mode in ["moodboard-canvas-note-selection", "moodboard-canvas"] {
            let (mut app, ctx) = fixture(mode);
            for title in [
                "Élément sélectionné",
                "Canevas",
                "Grille et alignement",
                "Vue",
                "Calques et ajout",
                "Export",
            ] {
                app.execute_command(&ctx, &format!("section-{title}"));
                for _ in 0..4 {
                    let mut out = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(1100.0, 800.0),
                            )),
                            ..Default::default()
                        },
                        |ui| app.contextual_inspector(ui),
                    );
                    out.textures_delta.clear();
                }
                assert!(app.design.inspector_section.is_none(), "{title}");
                assert!(
                    ctx.data(|d| d.get_temp::<(&'static str, u8)>(egui::Id::unique(
                        "moodboard-section-target"
                    )))
                    .is_none(),
                    "anchor not reached: {title}"
                );
            }
        }
    }
    #[test]
    fn every_main_menu_opens_for_text_note_and_image() {
        fn frame(
            app: &mut NotepadApp,
            ctx: &egui::Context,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440.0, 980.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let mut actions = Vec::new();
                    app.horizontal_menu_ui(ui, &mut actions);
                },
            );
            output.textures_delta.clear();
            output
        }
        for mode in [
            "notes",
            "moodboard-canvas-note-selection",
            "moodboard-canvas",
        ] {
            for group in [
                "Fichier",
                "Édition",
                "Page",
                "Insertion",
                "Affichage",
                "Outils",
                "Aide",
            ] {
                let (mut app, ctx) = fixture(mode);
                let mut output = frame(&mut app, &ctx, vec![]);
                output.textures_delta.clear();
                output = frame(&mut app, &ctx, vec![]);
                let position = output
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t) if t.galley.text() == group => {
                            Some(t.pos + egui::vec2(5.0, 5.0))
                        }
                        _ => None,
                    })
                    .expect("menu visible");
                for pressed in [true, false] {
                    frame(
                        &mut app,
                        &ctx,
                        vec![
                            egui::Event::PointerMoved(position),
                            egui::Event::PointerButton {
                                pos: position,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: Default::default(),
                            },
                        ],
                    );
                }
                let output = frame(&mut app, &ctx, vec![]);
                let expected = app
                    .command_catalogue()
                    .into_iter()
                    .find(|c| c.group == group)
                    .unwrap()
                    .label;
                assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains(&expected) && s.clip_rect.contains(t.pos))),"{mode}: {group} / {expected}");
            }
        }
    }

    #[test]
    fn catalogue_documentation() {
        if let Ok(path) = std::env::var("NOTEPAD_COMMAND_CATALOGUE") {
            let (app, _) = fixture("notes");
            let (note, _) = fixture("moodboard-canvas-note-selection");
            let (image, _) = fixture("moodboard-canvas");
            let note_commands = note.command_catalogue();
            let image_commands = image.command_catalogue();
            let mut body = String::from(
                "# Catalogue des commandes de Notepad RS\n\nSource commune : src/commands.rs. Les menus et la palette utilisent les mêmes actions et conditions. Les réglages rejoignent leur contrôle existant. Les commandes d’écriture nécessitent une page modifiable. Les fichiers récents, pages et destinations de déplacement sont dynamiques.\n\n| Identifiant | Commande | Groupe | Page texte | Note sélectionnée | Image sélectionnée | Accès |\n|---|---|---|---|---|---|---|\n",
            );
            for c in app.command_catalogue() {
                if matches!(c.target, Target::Page(_)) {
                    continue;
                }
                let route = match c.target {
                    Target::Settings(_, _) => "Paramètres",
                    Target::Section(_) => "Section du panneau",
                    _ => "Action",
                };
                let state = |list: &Vec<Command>| {
                    list.iter()
                        .find(|x| x.id == c.id)
                        .map(|x| x.reason.unwrap_or("Disponible"))
                        .unwrap_or("Selon le contexte")
                };
                body.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} | {} |\n",
                    c.id,
                    c.label,
                    c.group,
                    c.reason.unwrap_or("Disponible"),
                    state(&note_commands),
                    state(&image_commands),
                    route
                ));
            }
            body.push_str("\nLes boutons de confirmation, valeurs des champs, actions sur une version et réglages des blocs restent dans leur écran dédié. Le catalogue offre leur point d’entrée, sans déclencher une opération destructive à la sélection d’un réglage.\n");
            std::fs::write(path, body).unwrap();
        }
    }
}
