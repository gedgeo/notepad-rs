# Vérification de la refonte native

final result: passed

## Périmètre

Les trois maquettes ont été validées ensemble : notes, blocs et moodboard. La cible est l'application Windows Rust/egui existante, pas un prototype web. L'évaluation porte sur cette adaptation native et ses interactions ; elle ne revendique pas une reproduction pixel par pixel ni une certification WCAG.

## Références et normalisation

- Références : `samples/design-references/notes.png`, `blocks.png`, `moodboard.png`.
- Captures d'implémentation : `samples/previews/notes-final.png`, `blocks-final.png`, `moodboard-final.png`, `notes-narrow-final.png`.
- Les références font 1487 × 1058 pixels, barre de titre Windows comprise. Les 44 pixels supérieurs sont retirés, puis le contenu est normalisé à 1440 × 980 pour comparer les zones clientes.
- Captures natives : zone cliente 1440 × 980 pixels ; petite fenêtre 640 × 700 pixels. Échelle d'interface 125 %, indépendante de l'échelle du canevas. Le viewport de développement est donc 1152 × 784 points egui à 1440 × 980 pixels.
- Le moteur eframe effectue les captures via `Context::request_screenshot`. Il s'agit de pixels réellement rendus par l'application, pas d'images générées ou d'un montage simulant l'application.
- Les données d'exemple sont isolées et ne sont pas sauvegardées dans la session utilisateur. Le mode se ferme après capture.

## Preuves comparatives

Les deux vues sont réunies dans chaque image, référence à gauche et application à droite :

- Vue globale notes : `samples/previews/compare-notes-final.png`.
- Vue globale blocs : `samples/previews/compare-blocks-final.png`.
- Vue globale moodboard : `samples/previews/compare-moodboard-final.png`.
- Détail blocs/tableau : `samples/previews/compare-blocks-detail-final.png`.
- Détail navigation : `samples/previews/compare-navigation-final.png`.
- Détail inspecteur d'image : `samples/previews/compare-inspector-final.png`.

Ces vues globales et rapprochées ont été ouvertes pour vérifier le rendu, notamment la lisibilité des cellules, les états de sélection et les propriétés de l'image.

## Itérations et corrections

| Passe | Constat | Correction | Preuve après correction |
| --- | --- | --- | --- |
| Première capture | P1 : titres sans véritable graisse et page active insuffisamment visible. | Segoe UI Bold, état de sélection bleu et fond teinté explicite. | Captures finales notes/blocs et comparaison navigation. |
| Passe blocs v3 | P2 : cadres techniques, tâches affichant leur syntaxe brute, cellules tronquées. | Édition continue, cases à cocher et texte séparés, largeurs minimales de colonnes. | Comparaison finale détaillée blocs. |
| Passe v4 | P2 : densité et proportions différentes, barre du moodboard sur plusieurs lignes, éléments déplacés automatiquement en bord de viewport. | Échelle réglable, colonne de lecture centrée, zoom en bas du canevas, désactivation de la contrainte de position des éléments et écrêtage au canevas. | Comparaisons finales globales et petite fenêtre. |
| Dernière passe | P2 : propriétés d'image trop longues, absence d'état actif du type H2 ; contours trop nombreux après renforcement global. | Couleur réservée aux notes, lien repliable, aperçu d'image, niveau actif, contours renforcés limités aux cases à cocher et au focus. | Comparaisons finales inspecteur et blocs. |
| Icônes et thème sombre | P2 : l'étoile des favoris utilisait une famille grasse sans police d'icônes de repli ; couleurs secondaires sombres insuffisamment lisibles en thème sombre. | Séparation du libellé gras et de l'icône Phosphor, couleurs secondaires et accent adaptés au thème. | Comparaison finale navigation et capture `notes-dark-final.png`. |

## Surfaces examinées

- **Typographie** : Segoe UI chargée localement, titres réellement gras, texte de document agrandi, mode brut conservé. La hiérarchie notes/blocs est distincte de la navigation.
- **Espacement et disposition** : navigation repliable, largeur mémorisée, colonne centrale limitée, inspecteur contextuel et défilement des zones longues. Les petites fenêtres ne réservent pas en permanence la largeur des deux panneaux.
- **Couleurs et états** : surfaces blanches/gris clair, texte sombre, bleu pour l'action principale, sélection et focus ; case à cocher délimitée. Le message de synchronisation ne prétend pas que le dossier est à jour quand aucune synchronisation n'est configurée.
- **Images** : fichiers photographiques autonomes, sans faux éléments visuels ; textures mises en cache et écrêtées au canevas. La photo sélectionnée est reprise dans l'inspecteur. Les images d'exemple ne modifient pas les pages réelles.
- **Contenu et icônes** : libellés français ; icônes Phosphor cohérentes ; les outils historiques restent dans les menus et les paramètres.

## Interactions et contrôles

- 93 tests Rust réussis, compilation de développement réussie avec WinLibs.
- Tests egui : navigation par clic, conservation du document pendant l'affichage, trois modes à 640/1100/1440 points de viewport de test, persistance des préférences et ordre de l'arborescence, sélection/duplication/suppression/déplacement du moodboard et restauration, tâche à cocher et mise en forme de bloc réversible.
- Les contrôles d'export, de sauvegarde et de synchronisation utilisent les fonctions existantes ; les formats du menu Exporter dépendent du type de document.
- Le navigateur et sa console ne s'appliquent pas à cette application native. La capture via le service Windows/MXC reste indisponible ; la comparaison visuelle est réalisée avec les captures du moteur natif, et les gestes avec les événements egui simulés.
- Les boîtes de fichiers Windows et l'utilisation avec un lecteur d'écran réel ne sont pas couvertes par ce test automatisé.

## Écarts intentionnels et retouches P3

- Le texte brut ne simule pas des styles de paragraphe : la structure riche se travaille en mode Blocs. Les commandes de mise en forme des blocs modifient le Markdown et affichent sa syntaxe pendant l'édition.
- L'inspecteur emploie des commandes textuelles explicites pour avant/arrière et propose aussi Supprimer et Lien. La palette décorative du moodboard de référence n'est pas insérée comme faux contenu dans les pages.
- Les cellules du tableau restent de vrais champs éditables ; leur présentation est plus sobre que le tableau de lecture de la maquette.
- Les ombres, le détail des coins et certaines icônes diffèrent légèrement de l'image générée au profit des composants natifs et des icônes Phosphor.
- Le verrouillage des proportions n'est pas ajouté à cette passe ; largeur et hauteur restent réglables séparément, comme dans le comportement existant.

## Liste de validation

- [x] Trois références et captures natives comparées dans les mêmes images.
- [x] Régressions de hiérarchie, de débordement et de tableaux corrigées puis recapturées.
- [x] Petite fenêtre capturée et modes testés à plusieurs tailles.
- [x] Tests et compilation réussis.
- [x] Session utilisateur préservée pendant les captures.
- [ ] Vérification manuelle complémentaire des dialogues Windows et lecteurs d'écran.
