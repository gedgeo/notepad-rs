# Comparaison Notepad Windows / notepad-rs — 26 octobre 2026

## Résultat et périmètre

Comparaison statique du Bloc-notes moderne installé, via REA 6.8.0 et Ghidra 12.1.4, avec les sources actuelles de notepad-rs. Aucun comportement interactif, service IA ou impression physique n’a été exécuté. Les options présentes dans les ressources peuvent être conditionnées par un réglage, le matériel ou un déploiement progressif. Leur présence ne prouve pas leur activation sur cette machine.

La référence principale est Microsoft.WindowsNotepad **11.2606.15.0**, et non le binaire classique System32 **10.0.26100.8875** cité dans le README. Les deux ne doivent pas être confondus. Le présent examen ne redécompile pas le binaire classique et ne constitue pas une preuve de parité exhaustive.

- Exécutable installé : `C:\Program Files\WindowsApps\Microsoft.WindowsNotepad_11.2606.15.0_x64__8wekyb3d8bbwe\Notepad\Notepad.exe`.
- SHA-256 : `7146785db2b9c4fbdc114b4f31cf4d3c01168661f06ac6149709bc651529c934`.
- Copie locale analysée : `inputs/Notepad.exe`, même empreinte vérifiée par REA. La copie évite le refus d’admission Ghidra sur le dossier WindowsApps ; aucune permission système n’a été changée.
- NotepadXamlUI.dll : SHA-256 `1aec052eaf6b0653c1c130d42c3d7b9f397575a444752b62cf49e32975bea41c`.
- resources.pri : SHA-256 `ea6e8b324bcc77e53c69b22279ba16ad16a159251a46c24efe2fc71d5340a896`.
- Base locale : 70 tests réussis lors de la reprise, avant cette comparaison. Aucun changement du code applicatif.

## Écarts à traiter

Niveau A = chemin de code décompilé ; B = chaînes, identifiants et ressources livrés, fonctionnalité à confirmer à l’exécution. Les absences locales sont établies dans les modules examinés, pas dans un ancien README.

| Priorité | Option ou comportement | Preuve Windows | État notepad-rs / action proposée |
| --- | --- | --- | --- |
| P1 | Avertir avant une conversion ANSI avec perte | A : E5, fonction `0x140088fc0`, appelle GetACP et WideCharToMultiByte puis utilise SaveUnicodeAsANSIWarning si la conversion pose problème ; chemin d’annulation présent | `src/encoding.rs:75` ignore le signal d’erreur de WINDOWS_1252.encode ; `src/main.rs:655` écrit directement. Détecter les caractères non représentables, proposer UTF-8 ou annulation avant écriture. |
| P1 | Utiliser la page de code ANSI du système | A : E5, appel GetACP dans le chemin ANSI | Windows-1252 est fixé dans encoding.rs. Ajouter la page système et tester les conversions. |
| P1 | Compter les caractères sélectionnés | A : E4, fonction `0x14009c1a0`, choisit StatusBarCharacterCountWithSelection et fournit deux valeurs quand la quantité sélectionnée est positive | `src/main.rs:2441` affiche uniquement le total, bien que le modèle possède selection_chars. Ajouter sélection / total ; vérifier séparément l’unité de comptage Unicode. |
| P1 | Réglages d’ouverture des fichiers | B : E1/E3, OpenFile, OpenFileSetting ; ressources Opening files, OpenInNewTab | `main.rs:30` démarre sans restauration avec un fichier en argument ; pas de préférence onglet/fenêtre ni de routage vers une instance existante. Ajouter une préférence puis vérifier le comportement exact de Windows par test. |
| P1 | Politique de reprise au démarrage | B : ressources Opening Notepad et Continue previous session ; E2/E3 WindowState, TabState | Restauration prévue mais sans réglage utilisateur de politique de démarrage. Proposer session précédente / document vide, avec comportement explicite pour les modifications non enregistrées. |
| P1 | Sessions de plusieurs fenêtres | B : E1 SessionLifetimeEngine et types de contexte de session ; E2 WindowState/TabState ; E3 WindowBatchCreation | `session.rs:29,130` conserve un seul état et un seul session.json. Les fenêtres peuvent remplacer leur état mutuellement. Prévoir identifiants de fenêtres, coordination et restauration séparée. Détails exacts du protocole Windows non établis. |
| P2 | Orthographe, suggestions et dictionnaire | B : E1 SpellCheckState, SpellingAddToDictionary, SpellingIgnoreOnce/All, SpellingDeleteRepeated ; ressources SpellCheckSettingSection et SpellCheckFileCollection | Aucun moteur ou réglage correspondant dans src. Ajouter activation, suggestions et exclusions par type de fichier ; comportement linguistique à tester. |
| P2 | Correction automatique | B : E1 AutoCorrect et SpellingAutoCorrect ; ressources IsAutocorrectEnabled et section dédiée | Absente. Prévoir un réglage distinct de la vérification orthographique et une annulation cohérente. |
| P2 | Style de police de l’éditeur | B : E1 FontStyle / LocalizedFontStyle, E2 FontFamily et FontWeight ; ressources CurrentFontStyle/FontStyleCollection | `main.rs:2621` propose famille et taille, pas le style. Le gras de l’aperçu Markdown ne remplace pas ce réglage. |
| P2 | Détacher ou transférer un onglet vers une fenêtre | B : E1 types TabTearingOutWindowContext, DragTabOutToNewWindow ; E3 NotepadTabSourceWindow et NotepadTabIdInDragging | `main.rs:2246,2375` réordonne uniquement dans la même fenêtre ; nouvelle fenêtre = nouveau processus vide. |
| P2 | Restaurer la position de fenêtre | B : E3 WindowPositionTop/Left/Bottom/Right ; E2 GetWindowPlacement et SetWindowPlacement | Session ne stocke que la taille. Ajouter position/état avec contrôle des écrans disponibles. |
| P2 | Désactiver les fichiers récents | B : E2 RecentFilesEnabled et ressources IsRecentFilesEnabled | Liste et effacement présents, absence de préférence pour désactiver l’historique. |
| P2 | Sens droite-à-gauche et contrôles Unicode | B : E1 RightToLeft, E2 InsertUnicodeControlCharacters / ShowUnicodeControlCharacters, ressource ContextMenuItemRightToLeftTitle | Pas de commandes correspondantes dans l’éditeur. Le right_to_left utilisé pour aligner la barre d’état ne fournit pas cette fonction. |
| P2 | Markdown éditable en vue formatée | B : E3 OpenInFormattedView/OpenInMDSyntaxView, ressources MenuItemTextViewFormatted/Syntax | Édition de syntaxe + aperçu séparé présents ; pas d’édition directe dans le rendu. C’est une différence de fonctionnement, pas une absence totale de Markdown. |
| P2 | Outils Markdown supplémentaires | B : E3 opérations d’insertion/suppression de lignes et colonnes de tableau ; ressources ClearFormatting, Increase/Decrease indent, MenuItemFileNewMarkdownTab, MenuItemRemoveLink, images | Tableaux rendus mais pas d’outil d’édition de tableau ; pas de commande générale Effacer la mise en forme, ni d’insertion/rendu d’image identifié. Certaines actions restent réalisables manuellement dans le texte. |
| Hors périmètre actuel | Réécriture, résumé, génération IA | B : E1 routes rewrite/rewrite-stream, ressources MenuItemRewrite/Summarize/Generate, tons et formats | Exclusion volontaire du README. Ne pas compter comme régression ; disponibilité et conditions d’accès non testées. |

## Fonctions présentes : ne pas les redévelopper

Les sources locales comportent déjà les onglets et leur réordonnancement, les fermetures groupées, les récents, les opérations de fichiers, les recherches avant/arrière avec bouclage, le remplacement, les encodages UTF, les fins de ligne, les thèmes, le zoom, le retour à la ligne, les commandes Markdown de base, l’aperçu, la mise en page et l’impression.

Le README est notamment dépassé sur l’impression : `header.rs` et `page.rs` portent des en-têtes/pieds de page configurables ; `print.rs` traite `PD_SELECTION`, `PD_PAGENUMS` et `nCopies`. Ces fonctionnalités sont implémentées ; leur fidélité à l’imprimante n’a pas été vérifiée dans cet audit. Le README annonce 59 tests contre 70 réellement exécutés.

## Plan proposé

1. Fiabilité : avertissement de perte ANSI, page de code système, sessions distinctes par fenêtre.
2. Ergonomie : compteur de sélection, préférences d’ouverture/reprise, position de fenêtre, style de police, désactivation des récents.
3. Fonctions modernes : orthographe/correction, détachement des onglets, outils Markdown supplémentaires puis édition formatée si souhaitée.

Les priorités sont une proposition de développement, pas une observation de REA. Aucune de ces modifications n’est incluse dans cet audit.

## Preuves conservées

Le fichier `rea-evidence.json` contient les résultats REA complets, les empreintes, paramètres et limites, y compris les références et décompilations. Les adresses ci-dessus sont des adresses virtuelles du binaire identifié, pas des offsets de fichier.

- E1 : `ev_df6335a1f8b492a723dd1dc3e9a954472e5e8a02b5d3c49499f90447c000fba9` — recherche Ghidra orthographe, sessions, police, Markdown, IA.
- E2 : `ev_daec6916e78974aff228a8469cdf74e51140bdb63f23c6d807b35a132a7a2c74` — recherche Ghidra compteur, états, encodage, API.
- E3 : `ev_144f0d114bd0006d87407e5a3b758a524a865be5f6c2911dd073ab59d1b21ffb` — recherche Ghidra fenêtres, ouverture et opérations de tableau.
- E4 : `ev_95d9f7b46efdb82e68f24854969438e665d1af87aacfc1542263e9811aefe9ea` — décompilation du choix de compteur selon la sélection. Référence entrante `ev_659af48f5a3076137bc38ad3b183a36f11d2bbca7e46b14785610115d0a37f09`.
- E5 : `ev_c24b2479c6a1596794bf79f24834de6eb67fc190a69cc7990583cfb437ddc769` — décompilation du chemin d’enregistrement ANSI. Référence entrante `ev_97abb372f36c8561ecf38bc7ce9e8053f1c0fa9bf6e0c87f63c636a2c9bfec87`.
- `resource-string-candidates.json` et `ui-option-candidates.json` : extractions complémentaires locales ASCII/UTF-16LE, avec fichier, encodage et offset en octets. Ce ne sont pas des Evidence REA ni un décodage complet de PRI/XAML. Leur présence prouve uniquement les octets/identifiants relevés. Les chemins de fichiers source internes Microsoft sont des chaînes livrées, pas des sources récupérées.

## Questions restant ouvertes

- Visibilité et activation effective des options modernes sur ce compte ; validation par parcours UI nécessaire.
- Raccourcis exacts de navigation/réouverture des onglets : non établis, donc pas déclarés manquants par parité.
- Surveillance des modifications externes : message de ressource trouvé, politique exacte de rechargement/conflit non suivie.
- Détails de restauration, unité du compteur Unicode, rendu RTL, linguistique et fidélité de l’impression : tests ciblés nécessaires.
- Aucune conclusion exhaustive sur les performances, l’accessibilité, les grands fichiers ou l’identité de comportement entre versions.
