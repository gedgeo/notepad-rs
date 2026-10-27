# Catalogue des commandes de Notepad RS

Source commune : src/commands.rs. Les menus et la palette utilisent les mêmes actions et conditions. Les réglages rejoignent leur contrôle existant. Les commandes d’écriture nécessitent une page modifiable. Les fichiers récents, pages et destinations de déplacement sont dynamiques.

| Identifiant | Commande | Groupe | Page texte | Note sélectionnée | Image sélectionnée | Accès |
|---|---|---|---|---|---|---|
| new | Nouvelle page | Fichier | Disponible | Disponible | Disponible | Action |
| new-board | Nouveau moodboard | Fichier | Disponible | Disponible | Disponible | Action |
| window | Nouvelle fenêtre | Fichier | Disponible | Disponible | Disponible | Action |
| open | Ouvrir… | Fichier | Disponible | Disponible | Disponible | Action |
| save | Enregistrer | Fichier | Disponible | Disponible | Disponible | Action |
| save-as | Enregistrer sous… | Fichier | Disponible | Disponible | Disponible | Action |
| save-all | Tout enregistrer | Fichier | Disponible | Disponible | Disponible | Action |
| page-setup | Mise en page… | Fichier | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| print | Imprimer… | Fichier | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| print-preview | Aperçu avant impression | Fichier | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| close | Fermer l’onglet | Fichier | Disponible | Disponible | Disponible | Action |
| close-window | Fermer la fenêtre | Fichier | Disponible | Disponible | Disponible | Action |
| quit | Quitter | Fichier | Disponible | Disponible | Disponible | Action |
| undo | Annuler | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| redo | Rétablir | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| cut | Couper | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| copy | Copier | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| paste | Coller | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| delete-text | Supprimer la sélection de texte | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| select-all | Tout sélectionner | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| find | Rechercher… | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| next | Rechercher suivant | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| previous | Rechercher précédent | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| replace | Remplacer… | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| goto | Atteindre une ligne… | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| font | Police… | Édition | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| date | Heure et date | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| zoom-in | Zoom avant | Affichage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| zoom-out | Zoom arrière | Affichage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| zoom-reset | Zoom du texte à 100 % | Affichage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| wrap | Retour à la ligne | Affichage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| status | Barre d’état | Affichage | Disponible | Disponible | Disponible | Action |
| markdown | Mode Markdown | Affichage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| preview | Aperçu Markdown | Affichage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-bar | Barre de mise en forme | Affichage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| about | À propos de Notepad RS | Aide | Disponible | Disponible | Disponible | Action |
| format-bold | Gras | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-italic | Italique | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-strike | Barré | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-h1 | Titre 1 | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-h2 | Titre 2 | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-h3 | Titre 3 | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-bullet | Liste à puces | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-ordered | Liste numérotée | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-quote | Citation | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-code | Code en ligne | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-code-block | Bloc de code | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| format-link | Lien Markdown | Insertion | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| encoding-Ansi | Enregistrer en ANSI | Encodage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| reopen-Ansi | Rouvrir les octets en ANSI | Encodage | Nécessite un fichier ouvert sans modifications non enregistrées | Disponible sur une page texte | Disponible sur une page texte | Action |
| encoding-Utf8 | Enregistrer en UTF-8 | Encodage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| reopen-Utf8 | Rouvrir les octets en UTF-8 | Encodage | Nécessite un fichier ouvert sans modifications non enregistrées | Disponible sur une page texte | Disponible sur une page texte | Action |
| encoding-Utf8Bom | Enregistrer en UTF-8 avec BOM | Encodage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| reopen-Utf8Bom | Rouvrir les octets en UTF-8 avec BOM | Encodage | Nécessite un fichier ouvert sans modifications non enregistrées | Disponible sur une page texte | Disponible sur une page texte | Action |
| encoding-Utf16Le | Enregistrer en UTF-16 LE | Encodage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| reopen-Utf16Le | Rouvrir les octets en UTF-16 LE | Encodage | Nécessite un fichier ouvert sans modifications non enregistrées | Disponible sur une page texte | Disponible sur une page texte | Action |
| encoding-Utf16Be | Enregistrer en UTF-16 BE | Encodage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| reopen-Utf16Be | Rouvrir les octets en UTF-16 BE | Encodage | Nécessite un fichier ouvert sans modifications non enregistrées | Disponible sur une page texte | Disponible sur une page texte | Action |
| eol-Crlf | Fins de ligne : Windows (CRLF) | Encodage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| eol-Lf | Fins de ligne : Unix (LF) | Encodage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| eol-Cr | Fins de ligne : Macintosh (CR) | Encodage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| theme-Système | Thème Système | Affichage | Disponible | Disponible | Disponible | Action |
| theme-Clair | Thème Clair | Affichage | Disponible | Disponible | Disponible | Action |
| theme-Sombre | Thème Sombre | Affichage | Disponible | Disponible | Disponible | Action |
| export-md | Exporter .md… | Export | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| export-html | Exporter .html… | Export | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| export-pdf | Exporter .pdf… | Export | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| export-json | Exporter .json… | Export | Ouvrez un moodboard | Disponible | Disponible | Action |
| export-png | Exporter .png… | Export | Ouvrez un moodboard | Disponible | Disponible | Action |
| export-jpg | Exporter .jpg… | Export | Ouvrez un moodboard | Disponible | Disponible | Action |
| search | Rechercher dans l’espace | Outils | Disponible | Disponible | Disponible | Action |
| history | Historique et versions… | Page | Disponible | Disponible | Disponible | Action |
| duplicate-page | Dupliquer la page | Page | Disponible | Disponible | Disponible | Action |
| rename | Renommer la page | Page | Disponible | Disponible | Disponible | Action |
| subpage | Créer une sous-page | Page | Disponible | Disponible | Disponible | Action |
| favorite | Ajouter / retirer des favoris | Page | Disponible | Disponible | Disponible | Action |
| trash-page | Déplacer la page vers la corbeille | Page | Disponible | Disponible | Disponible | Action |
| trash | Corbeille | Page | Disponible | Disponible | Disponible | Action |
| undo-trash | Annuler la mise à la corbeille | Page | Aucune mise à la corbeille à annuler | Aucune mise à la corbeille à annuler | Aucune mise à la corbeille à annuler | Action |
| properties | Propriétés de la page | Page | Disponible | Disponible | Disponible | Action |
| sidebar | Navigation des pages | Affichage | Disponible | Disponible | Disponible | Action |
| concentration | Mode concentration | Affichage | Disponible | Disponible | Disponible | Action |
| palette | Toutes les commandes… | Aide | Disponible | Disponible | Disponible | Action |
| board-note | Ajouter une note | Insertion | Ouvrez un moodboard | Disponible | Disponible | Action |
| board-image | Importer une image… | Insertion | Ouvrez un moodboard | Disponible | Disponible | Action |
| board-duplicate | Dupliquer l’élément | Édition | Ouvrez un moodboard | Disponible | Disponible | Action |
| board-delete | Supprimer l’élément | Édition | Ouvrez un moodboard | Disponible | Disponible | Action |
| board-fit | Ajuster le canevas | Affichage | Ouvrez un moodboard | Disponible | Disponible | Action |
| section-Élément sélectionné | Propriétés du moodboard · Élément sélectionné | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| section-Canevas | Propriétés du moodboard · Canevas | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| section-Grille et alignement | Propriétés du moodboard · Grille et alignement | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| section-Vue | Propriétés du moodboard · Vue | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| section-Calques et ajout | Propriétés du moodboard · Calques et ajout | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| section-Export | Propriétés du moodboard · Export | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| setting-Format et dimensions du canevas | Régler : Format et dimensions du canevas | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| setting-Fond et cadre du canevas | Régler : Fond et cadre du canevas | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| setting-Points, lignes et espacement | Régler : Points, lignes et espacement | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| setting-Magnétisme des déplacements | Régler : Magnétisme des déplacements | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| setting-Zoom et déplacement de la vue | Régler : Zoom et déplacement de la vue | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| setting-Masquer ou verrouiller un calque | Régler : Masquer ou verrouiller un calque | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| setting-Position, dimensions et proportions de l’élément | Régler : Position, dimensions et proportions de l’élément | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| setting-Alignement et adaptation au canevas | Régler : Alignement et adaptation au canevas | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| setting-Lien de l’élément | Régler : Lien de l’élément | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| setting-Couleur et texte de la note | Régler : Couleur et texte de la note | Moodboard | Ouvrez un moodboard | Disponible | Sélectionnez une note | Section du panneau |
| setting-Remplacer l’image | Régler : Remplacer l’image | Moodboard | Ouvrez un moodboard | Sélectionnez une image | Disponible | Section du panneau |
| setting-Disposition avant / arrière | Régler : Disposition avant / arrière | Moodboard | Ouvrez un moodboard | Disponible | Disponible | Section du panneau |
| settings-0 | Paramètres · Général | Outils | Disponible | Disponible | Disponible | Paramètres |
| settings-1 | Paramètres · Apparence | Outils | Disponible | Disponible | Disponible | Paramètres |
| settings-2 | Paramètres · Éditeur | Outils | Disponible | Disponible | Disponible | Paramètres |
| settings-3 | Paramètres · Moodboard | Outils | Disponible | Disponible | Disponible | Paramètres |
| settings-4 | Paramètres · Fichiers et données | Outils | Disponible | Disponible | Disponible | Paramètres |
| settings-5 | Paramètres · Sauvegardes | Outils | Disponible | Disponible | Disponible | Paramètres |
| settings-6 | Paramètres · Synchronisation | Outils | Disponible | Disponible | Disponible | Paramètres |
| settings-7 | Paramètres · Raccourcis | Outils | Disponible | Disponible | Disponible | Paramètres |
| settings-8 | Paramètres · Confidentialité | Outils | Disponible | Disponible | Disponible | Paramètres |
| preference-0-Démarrer sur une page vide | Réglage · Démarrer sur une page vide | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-0-Ouvrir dans une nouvelle fenêtre | Réglage · Ouvrir dans une nouvelle fenêtre | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-0-Type des nouvelles pages | Réglage · Type des nouvelles pages | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-0-Profils de travail | Réglage · Profils de travail | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-1-Thème | Réglage · Thème | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-1-Échelle de l’interface | Réglage · Échelle de l’interface | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-1-Navigation et barre d’état | Réglage · Navigation et barre d’état | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-1-Mode concentration | Réglage · Mode concentration | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-2-Police et taille | Réglage · Police et taille | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-2-Modèles personnalisés | Réglage · Modèles personnalisés | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-3-Afficher la grille | Réglage · Afficher la grille | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-3-Alignement magnétique | Réglage · Alignement magnétique | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-4-Encodage par défaut | Réglage · Encodage par défaut | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-4-Fins de ligne par défaut | Réglage · Fins de ligne par défaut | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-4-Conserver les fichiers récents | Réglage · Conserver les fichiers récents | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-4-Nombre de fichiers récents | Réglage · Nombre de fichiers récents | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-5-Sauvegarde portable | Réglage · Sauvegarde portable | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-5-Sauvegardes planifiées | Réglage · Sauvegardes planifiées | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-5-Versions automatiques | Réglage · Versions automatiques | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-5-Versions par page | Réglage · Versions par page | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-2-Largeur de lecture | Réglage · Largeur de lecture | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-2-Interligne | Réglage · Interligne | Réglages | Disponible | Disponible | Disponible | Paramètres |
| preference-3-Pas de grille | Réglage · Pas de grille | Réglages | Disponible | Disponible | Disponible | Paramètres |
| backup-export | Ouvrir : Sauvegarder l’espace de travail… | Outils | Disponible | Disponible | Disponible | Paramètres |
| backup-import | Ouvrir : Importer les pages d’une sauvegarde… | Outils | Disponible | Disponible | Disponible | Paramètres |
| backup-recover | Ouvrir : Récupérer les pages du fichier .bak… | Outils | Disponible | Disponible | Disponible | Paramètres |
| backup-schedule | Ouvrir : Planifier les sauvegardes… | Outils | Disponible | Disponible | Disponible | Paramètres |
| sync-folder | Ouvrir : Choisir le dossier de synchronisation… | Outils | Disponible | Disponible | Disponible | Paramètres |
| sync-now | Ouvrir : Synchroniser maintenant ou réessayer… | Outils | Disponible | Disponible | Disponible | Paramètres |
| sync-conflict | Ouvrir : Comparer les copies de conflit… | Outils | Disponible | Disponible | Disponible | Paramètres |
| sync-pause | Ouvrir : Pause et fréquence de synchronisation… | Outils | Disponible | Disponible | Disponible | Paramètres |
| shortcuts | Ouvrir : Affecter les raccourcis clavier… | Outils | Disponible | Disponible | Disponible | Paramètres |
| preferences-export | Ouvrir : Exporter les préférences uniquement… | Outils | Disponible | Disponible | Disponible | Paramètres |
| preferences-import | Ouvrir : Importer les préférences… | Outils | Disponible | Disponible | Disponible | Paramètres |
| preferences-reset | Ouvrir : Réinitialiser les préférences… | Outils | Disponible | Disponible | Disponible | Paramètres |
| diagnostic | Ouvrir : Copier un diagnostic sans contenu ni chemins… | Outils | Disponible | Disponible | Disponible | Paramètres |
| templates | Ouvrir : Modèles de pages… | Outils | Disponible | Disponible | Disponible | Paramètres |
| page-icon | Régler : Icône et couleur de repère de la page | Page | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| page-width | Régler : Largeur de lecture propre à cette page | Page | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| page-links | Régler : Liens sortants et liens entrants | Page | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| block-properties | Régler : Type, déplacement et duplication du bloc | Page | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| readonly | Lecture seule de la page | Page | Disponible | Disponible | Disponible | Action |
| page-blocks | Édition par blocs | Affichage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| page-text | Édition texte | Affichage | Disponible | Disponible sur une page texte | Disponible sur une page texte | Action |
| close-others | Fermer les autres onglets | Fichier | Disponible | Disponible | Disponible | Action |
| close-right | Fermer les onglets à droite | Fichier | Disponible | Aucun onglet à droite | Aucun onglet à droite | Action |
| copy-path | Copier le chemin du fichier | Fichier | Enregistrez d’abord le fichier | Enregistrez d’abord le fichier | Enregistrez d’abord le fichier | Action |
| reveal | Afficher le fichier dans son dossier | Fichier | Enregistrez d’abord le fichier | Enregistrez d’abord le fichier | Enregistrez d’abord le fichier | Action |
| clear-recent | Effacer la liste des récents | Fichier | Disponible | Disponible | Disponible | Action |

Les boutons de confirmation, valeurs des champs, actions sur une version et réglages des blocs restent dans leur écran dédié. Le catalogue offre leur point d’entrée, sans déclencher une opération destructive à la sélection d’un réglage.
