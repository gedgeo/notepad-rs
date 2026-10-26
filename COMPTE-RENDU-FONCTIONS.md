# Fonctions de pages — 26 octobre 2026

## Livré

Les options de page se trouvent dans le panneau droit. La navigation et le renommage restent dans le panneau gauche.

| Fonction | Comportement disponible |
| --- | --- |
| Liens et backlinks | Insertion de `[[Page]]`, navigation depuis les liens de la page et références entrantes dans la barre latérale. |
| Favoris et épinglées | Deux indicateurs persistants et une liste d'accès rapide. |
| Corbeille | Masquage dans la liste de pages et restauration sans suppression des données. |
| Blocs Markdown | Insertion de titres, tâches, citations, code, séparateurs et tableaux dans l'éditeur existant. |
| Glisser-déposer | Réorganisation et déplacement sous une page parente, contrôle des cycles et retour à la racine. |
| Icônes et couleurs | Personnalisation persistante de l'icône dans la navigation. |
| Export | Markdown, HTML mis en forme ; PDF via la boîte d'impression Windows et Microsoft Print to PDF. |
| Versions | Versions manuelles, captures espacées lors de l'édition, restauration ; conservation des 50 dernières captures automatiques. |
| Sauvegarde et échange | Export/import JSON de l'espace, copie .bak avant remplacement, publication/récupération manuelle dans un dossier partagé. |

## Corrections des ajouts précédents

- Mise à jour vers l'API Panel d'egui 0.37.
- Initialisation des champs manquants qui empêchaient la compilation.
- Sauvegarde et restauration réelles des noms, identifiants et parents des pages.
- Conservation des pages vierges et de leur texte dans la session.
- Réattribution des identifiants dupliqués dans les anciennes sessions.
- Marquage des modifications du moodboard pour l'autosauvegarde.

## Vérification

74 tests Rust réussis, dont trois nouveaux contrôles pour les liens, la persistance des métadonnées et l'échappement HTML. Compilation de développement réussie. La manipulation visuelle de tous les contrôles Windows reste à vérifier en utilisation réelle.

## Limites explicites

- Les liens utilisent les noms : renommer une page ne réécrit pas les références existantes ; des noms uniques sont préférables.
- Les blocs restent du Markdown, pas un éditeur visuel de blocs indépendant.
- Le PDF utilise le pilote d'impression Windows ; il n'y a pas de générateur PDF direct.
- La synchronisation est manuelle par fichiers ; aucune fusion collaborative ou synchronisation cloud automatique n'est implémentée. Une récupération ajoute les pages importées aux pages locales et peut donc créer des doublons.
- Les images des moodboards restent référencées par leur chemin ; l'échange JSON ne les embarque pas.
- La corbeille conserve les onglets et leurs données ; elle ne supprime pas les fichiers sur disque.
