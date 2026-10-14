# Newell — Spécification UI/UX des références ASCII

Ce document décrit l'intention des dix fichiers `.txt` actuels de ce dossier :
structure de l'application, grammaire des contrôles et états des cinq opérations.
Les libellés anglais cités correspondent aux maquettes, y compris leurs variantes
orthographiques. Les exemples sont des états d'interface, pas une preuve que les
opérations géométriques sont implémentées.

## 1. Portée et autorité

Ordre d'autorité : [ARCHITECTURE.md](../../ARCHITECTURE.md), ce README,
références ASCII, puis plans d'implémentation. Cette actualisation prend les
références actuelles comme source des intentions UX, sans modifier les `.txt`.
Les divergences non résolues sont explicites en §11 ; elles ne doivent pas être
transformées silencieusement en nouvelles fonctionnalités.

La spécification couvre l'interface cible au-delà du seul mock Extrude de M2.
Elle n'étend pas le périmètre d'exécution d'un jalon : dans un jalon de mock,
les valeurs, sélections et validations restent simulées, sans évaluation de
formules ni opération géométrique réelle.

| Référence | Contenu couvert |
|---|---|
| [Special_ASCII.txt](Special_ASCII.txt) | Palette de traits, jonctions, remplissages et symboles. |
| [Interface_Elements.txt](Interface_Elements.txt) | Progression, sélections, profils, inversion, défilement, six en-têtes, formule et saisie de nom. |
| [Interface_Exemple.txt](Interface_Exemple.txt) | Shell complet et panneau Extrude intégré. |
| [Application_Menu_Bar.txt](Application_Menu_Bar.txt) | File, Edit, View et Help. |
| [Command_Bar.txt](Command_Bar.txt) | Ateliers et groupes de commandes. |
| [Action_Box_Extrude.txt](Action_Box_Extrude.txt) | Extrusion, limites, décalages, symétrie et second côté. |
| [Action_Box_Revolve.txt](Action_Box_Revolve.txt) | Révolution complète ou partielle et limites angulaires. |
| [Action_Box_Loft.txt](Action_Box_Loft.txt) | Profils, conditions aux extrémités, guides, chemin et connexions. |
| [Action_Box_Sweep.txt](Action_Box_Sweep.txt) | Profil, chemin, orientation, torsion et échelle. |
| [Action_BoxBoolean.txt](Action_BoxBoolean.txt) | Union, soustraction, intersection, décalage et conservation des outils. |

## 2. Grammaire ASCII et lecture des dépendances

« ASCII » désigne ici une grille de caractères comprenant des glyphes Unicode.
Les espaces alignent les cellules ; ils ne constituent pas des contrôles. Les
légendes entre guillemets et `Same for Surfaces` annotent les exemples et ne sont
pas du texte à afficher dans le panneau. Des panneaux successifs montrent des
états alternatifs, pas plusieurs dialogues ouverts simultanément.

| Notation | Sens contextuel |
|---|---|
| `╔ ╗ ╚ ╝ ═ ║ ╠ ╣ ╦ ╩ ╬` | Cadre et grandes divisions du shell. |
| `╭ ╮ ╰ ╯`, `─ │`, `┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼` | Conteneurs, champs internes, séparateurs ou branches selon leur position. |
| `━ ┃ ┏ ┓ ┗ ┛ ┣ ┫ ┳ ┻ ╋` | Famille épaisse ; `┃` sert notamment au défilement. |
| `╞ ╡ ╤ ╧ ╪ ╓ ╒ ╖ ╕ ╙ ╘ ╜ ╛ ╟ ╢ ╥` | Jonctions mixtes de la palette ; pas des actions utilisateur. |
| `╴ ╵ ╶ ╷ ┈ ┊ ┄ ┆` | Traits partiels/discontinus ; `╴` et `┆` différencient notamment les segments des en-têtes. |
| `[ … ]` | Zone de saisie ou de sélection ; le contenu et le libellé déterminent le type. |
| `□`, `■` | Checkbox décochée/cochée. Dans les arbres, le sens métier reste à préciser. |
| `◀ valeur ▶` | Sélecteur cyclique : valeur précédente/suivante. |
| `▲ … ▼`, `┃`, `╻`, `╹` | Défilement et position du curseur de liste. |
| `░ ▒ ▓ █` | Remplissage progressif d'une cellule de progression. |
| `☰` | Action contextuelle détaillée en §3.4. |
| `×` | Retrait d'une entrée dans la liste de profils. |

### 2.1 Bordure externe, branche interne et séparateur de champ

```text
│ Extend   ◀     Blind     ▶  │
│ │ Depth   [+|0.0 mm       ☰]│
```

Lecture de la seconde ligne, de gauche à droite :

1. Le premier `│` est le bord gauche de l'Action Box.
2. Le second `│`, indenté, est une branche : `Depth` dépend du mode `Blind`
   de `Extend`. Ce n'est ni une seconde bordure ni un bouton.
3. `Depth` est le libellé du paramètre.
4. `[` ouvre le champ ; `+` est son contrôle d'orientation ; `|` sépare ce
   contrôle de la saisie. La variante inversée est `[-|…]`.
5. `0.0` est la valeur à renseigner et `mm` son unité.
6. `☰` ouvre l'éditeur de formule de cette valeur.
7. `]` ferme le champ ; le dernier `│` ferme l'Action Box.

Les branches `│` poursuivent un groupe de dépendances ; `└` marque son dernier
enfant. Une indentation supplémentaire indique une dépendance imbriquée :
`2nd side → Up to → End Offset → Dist/Target`, par exemple. Il faut lire ces
traits avec les libellés et les variantes : leur alignement est parfois imparfait.
Les branches des arbres Features/Elements expriment, elles, une hiérarchie de
document et non une condition d'affichage d'un paramètre.

### 2.2 Révélation progressive

Un contrôle dépendant apparaît lorsque son parent l'exige ; sinon il disparaît,
plutôt que de rester systématiquement grisé. Cocher une option peut révéler une
saisie, un sélecteur et/ou un groupe entier. Décocher `Merge with all` révèle au
contraire les cibles explicites. Les maquettes ne déterminent pas si une valeur
masquée est mémorisée lors d'un changement de mode.

## 3. Contrôles partagés

### 3.1 Sélecteurs segmentés et cycliques

Un sélecteur **segmenté** montre toutes ses alternatives en même temps :
`Solid | Surface`, `New | Add | Remove` ou `Add | Cut | Inter`.
Le tracé continu qui encadre/souligne le segment et les traits discontinus des
autres segments indiquent le choix actif dans les maquettes. Les six en-têtes de
`Interface_Elements.txt` couvrent Solid/New, Solid/Add, Solid/Remove,
Surface/New, Surface/Add et Surface/Remove. Les deux lignes sont deux choix
indépendants : nature du résultat, puis mode de combinaison.

Un sélecteur **cyclique** affiche une seule valeur entre `◀` et `▶`. Les flèches
parcourent les valeurs documentées, par exemple `Blind`/`Up to`. Le bouclage aux
extrémités n'est pas précisé. Le nombre de valeurs ne décide pas du type :
Perspective/Orthographic est cyclique malgré ses deux valeurs.

L'ancien contrat de rendu M2 utilisait des séparateurs `║` au contact du segment
actif. Cela reste une convention d'implémentation à distinguer du dessin actuel
avec `╴` et `┆` ; l'intention commune est un état actif lisible sans couleur.

### 3.2 Checkboxes et champs numériques

`□ Option` désactive l'option, `■ Option` l'active. L'état doit rester lisible
sans couleur. Un contrôle d'orientation intégré au champ n'est pas une checkbox.

| Forme | Intention |
|---|---|
| `[+|0.0 mm ☰]` / `[-|0.0 mm ☰]` | Distance et orientation de déplacement. |
| `[+|0.0 deg ☰]` / `[-|0.0 deg ☰]` | Angle et sens de rotation/torsion. |
| `[0.0 mm ☰]`, `[0.0 deg ☰]` | Valeur avec unité, sans bouton d'orientation dessiné. |
| `[0.0 ☰]` | Grandeur sans unité affichée : Magnitude ou Scale. |

`+`/`-` exprime le sens indépendamment de la zone éditable ; la convention d'axe
et l'acceptation d'une valeur numérique négative restent à définir. Ne pas ajouter
un bouton d'inversion aux champs où il n'est pas représenté. Les zéros illustrent
le format, sans garantir des valeurs par défaut géométriquement valides.

### 3.3 Champs de sélection

`[Select…]` invite à sélectionner ; `[1 Element ☰]` et `[N Elements ☰]`
résument la sélection par son nombre, pas par les noms des objets. `[none]`
apparaît aussi, notamment pour `Faces` de Boolean. Ces deux formes vides ne
signifient pas deux types de géométrie différents.

Le libellé porte le rôle : `Profile`, `Axis`, `Along`, `Path`, `Target`, `Tools`,
`Faces`, etc. Le champ ouvre/active la sélection adaptée à ce rôle ; le panneau
de sélection permet de consulter et modifier les éléments retenus. Les types
admissibles, cardinalités, filtres et erreurs de validation ne sont pas spécifiés
par les seuls dessins. Le singulier `Profile` ne suffit donc pas à imposer une
sélection unique, notamment pour Loft.

### 3.4 Rôle contextuel de `☰`

| Emplacement | Intention |
|---|---|
| Fin d'un champ numérique | Ouvrir Formula pour ce paramètre. |
| Fin d'un champ `1 Element` / `N Elements` | Accéder au détail et à l'édition de la sélection ; pas à Formula. |
| Début d'une ligne de la liste `Profils` | Poignée de manipulation/réordonnancement du profil, intention déduite de la liste ordonnée. Le geste exact reste à préciser. |

Le glyphe ne désigne donc pas un menu universel. Le `×` en fin de ligne retire
le profil de la liste ; il ne signifie pas supprimer l'objet du document.

### 3.5 Modales et listes

Toutes les boîtes illustrées portent un titre, un contenu et `OK`/`Cancel`.
La règle UX est de valider l'édition locale avec OK et de l'abandonner avec
Cancel, en revenant à l'état antérieur. Le panneau de commande reste le contexte
de travail ; l'ouverture d'une modale interrompt son interaction jusqu'à fermeture.
Les raccourcis clavier, clics hors boîte et règles de focus ne sont pas dessinés.

- **Selection** : liste défilante à checkboxes (`Rim_Body`, `Hub_Body`,
  `Inner_Surface`) pour modifier l'ensemble sélectionné. `Hub_Body` est coché
  dans l'exemple. `▲`/`▼` et le curseur indiquent le défilement.
- **Profils** : liste ordonnée (`Spline_1`, `Profil_3`, deux lignes `Profil_4`,
  `Compound_line_1`) avec poignée `☰`, retrait `×` et défilement. L'exemple ne
  tranche pas si les doublons sont autorisés ; ne pas les dédupliquer d'office.
- **Formula** : zone éditable multilignes. L'exemple associe des références
  `#EntraxeA`, `#Angle1`, `#Angle2`, des unités `mm`/`deg`, des parenthèses,
  des opérateurs et `sin`. Il illustre l'intention d'une expression paramétrique,
  pas une grammaire de calcul complète ou un moteur déjà disponible.
- **Enter Name** : champ texte initialement montré avec `none` ; il ne s'agit
  pas d'une sélection géométrique. **Rename** préremplit `Rim_Body` pour édition.
  Leur présence n'impose pas une demande de nom après chaque opération.

### 3.6 Barre de progression

La référence donne 21 états de 0 à 100 %, par pas de 5 %, avec cinq cellules.
Une cellule pleine représente 20 % ; `░`, `▒`, `▓` représentent respectivement
5, 10 et 15 % supplémentaires dans la cellule suivante. Les cellules restantes
sont des espaces :

```text
Progress [     ] 0%
Progress [░    ] 5%
Progress [▒    ] 10%
Progress [▓    ] 15%
Progress [█    ] 20%
Progress [███░ ] 65%
Progress [█████] 100%
```

Cette règle couvre tous les états intermédiaires du fichier. Le pourcentage
reste explicite ; aucune animation indéterminée ni politique d'arrondi pour
les valeurs hors pas de 5 % n'est définie.

## 4. Shell de l'application

`Interface_Exemple.txt` situe les régions suivantes dans un cadre double :

| Région canonique | Place et intention |
|---|---|
| MenuBar | Première ligne : File, Edit, View, Help ; à droite `[-] [□] [X]`, commandes de fenêtre (réduire, maximiser/restaurer, fermer). |
| CommandBar | Deuxième ligne : Workbench, Sketch, Create, Operate, Pattern, Refine, Construct, Inspect. |
| NavigationPanel | Colonne gauche divisée en Features et Elements. |
| Viewport3D | Zone principale de visualisation 3D. |
| NavCube | Aide d'orientation en haut à droite du viewport. |
| ActionPanel | Action Box contextuelle de la commande, ici Extrude, superposée à droite du viewport. |
| StatusBar | Ligne inférieure : document, atelier, statut, fréquence d'affichage, progression. |

Features représente l'historique/organisation des fonctions ; Elements représente
les entités résultantes, avec groupes et comptes (`Solid Bodies(2)`,
`Surfaces(0)`, `Construct`). Ce sont deux sources sémantiques distinctes.
`[-]` indique un nœud déployé, `[+]` un nœud replié ; `├`, `│`, `└` structurent
les descendants. Les marqueurs `□`/`■` ne permettent pas de décider entre
visibilité et sélection sans spécification complémentaire.

La barre de statut montre `Roue V1*`, `Part Design`, `Fully Constrained`, `98fps`
et une progression de 65 %. L'astérisque suggère un document modifié, sans que
le cycle de sauvegarde soit défini par cette maquette. Les valeurs et le NavCube
peuvent rester des données/éléments statiques dans le mock M2.

## 5. Application_Menu_Bar et Command_Bar

### 5.1 Menus de l'application

Les menus déroulants sont des listes contextuelles, distinctes des modales.

| Menu | Entrées exactes et intention |
|---|---|
| File | New, Open, Recent, Save, Save As, Import, Export, Option : création, accès aux documents, sauvegarde, échanges et réglages. Le contenu de Recent/Option n'est pas détaillé. |
| Edit | Undo, Redo, Recompute : annulation, rétablissement, recalcul. |
| Help | Tutorial, Docs, Shortcuts, Bug, Update, About : apprentissage, documentation, raccourcis, signalement, mise à jour et informations. |

View contient deux sélecteurs cycliques :

- Projection : `Perspective`, `Orthographic`.
- Affichage : `Points`, `Wireframe`, `Hidden line`, `No shading`, `Shaded`,
  `Flat line`, dans l'ordre de la référence.

L'état montré est Perspective/Shaded. Les checkboxes `Tree panel`, `Task bar`,
`Status bar`, `Navigation cube` sont cochées ; `Grid Overlay` est décochée.
Elles expriment la visibilité de ces éléments. `Tree panel` correspond au
NavigationPanel ; `Status bar` et `Navigation cube` au StatusBar et NavCube.
Le rattachement exact de `Task bar` est ambigu (voir §11).
`[N]ormal to`, `Reset Cam` et `Fullscreen` sont des actions de vue.

### 5.2 Groupes de commandes

| Groupe | Entrées de la référence |
|---|---|
| Workbench | Part design, Assemblies, Mesh, Parametric, Drawing, Simulation, Animation, Manufacture, PLM. |
| Create | `[E]xtrude`, `[R]evolve`, `[L]oft`, `[S]weep`. |
| Operate | `[B]oolean`, `[S]plit [F]ace`, `[S]plit [B]ody`, `[T]ransform`. |
| Pattern | `[Mi]rror`, `[Li]near`, `[Ci]rcular`, `[Cu]rve`. |
| Refine | `[F]illet`, `[C]hamfer`, `[S]hell`, `[D]raft`. |
| Construct | `[Pl]ane`, `[A]xis`, `[P]oint`. |
| Inspect | `[M]esure`, `[S]plit [V]iew`, Curvature, Properties. |

Workbench choisit l'atelier ; les autres groupes lancent leurs commandes.
`Sketch` est visible dans le shell mais son menu n'est pas développé dans
`Command_Bar.txt`. Les caractères entre crochets dans les noms sont des indices
mnémoniques, pas des champs de saisie. Leur résolution, les conflits (par exemple
`S`) et les éventuelles séquences de touches ne sont pas définis ici.

## 6. Structure commune des Action Boxes

Extrude, Revolve, Loft et Sweep partagent `Solid | Surface`, puis
`New | Add | Remove`, un corps conditionnel et les actions OK/Cancel.
`Same for Surfaces` étend les trois états illustrés en Solid aux trois états
Surface, sans introduire de contrôles supplémentaires. New crée un résultat ;
Add l'ajoute/associe aux cibles ; Remove exprime un retrait sur les cibles.
Les algorithmes et restrictions propres aux surfaces restent hors de ces dessins.

Dans les quatre opérations :

| État de résultat | Contrôles de portée |
|---|---|
| New | Aucun contrôle de fusion montré. |
| Add ou Remove, `■ Merge with all` | Portée globale ; pas de champ Targets. |
| Add ou Remove, `□ Merge with all` | Affiche l'enfant `Targets [Select…]` pour une portée explicite. |

`Merge with all` est le libellé source, même en Remove : il représente alors la
portée des cibles concernées, pas une union à substituer à la soustraction.
OK valide la commande et Cancel abandonne l'édition ; l'exécution réelle dépend
du jalon. Les règles d'erreur/activation d'OK ne sont pas dessinées.

## 7. Extrude — tous les états référencés

Les trois panneaux Solid New/Add/Remove montrent `Profile`, `Extend = Blind`,
`Depth`, `Along`, `Starting Offset`, `End Offset`, `Symetric` et
`Second end position`. Add/Remove ajoutent la portée de §6 ; les mêmes trois
états existent en Surface. Profile choisit la section à extruder ; Along choisit
une référence de direction, sans imposer ici ses types admissibles.

| Contrôle / état | Contenu dépendant et intention |
|---|---|
| Extend = Blind | `Depth [+|0.0 mm ☰]`, `Along [Select…]`, option de décalage initial. Les panneaux complets montrent aussi End Offset. |
| Extend = Up to | `Selection [Select…]` pour la limite, puis `□ Offset`. Depth et Along ne figurent pas dans cette variante. |
| Start Offset décoché | Pas de paramètre enfant. Le panneau complet le nomme Starting Offset. |
| Start Offset coché, Blind | `Dist [+|0.0 mm ☰]` : décalage initial par distance orientée. |
| Start Offset coché, Up to | `Target [Select…]` : référence du départ. |
| End Offset décoché | Pas de paramètre enfant. |
| End Offset coché, Blind, variante 1 | `Dist [+|0.0 mm ☰]` à la profondeur du groupe principal. |
| End Offset coché, Up to, variante 1 | `Target [Select…]`. |
| End Offset coché, Blind, variante 2 | Enfant plus profondément indenté : `Dist [0.0 mm ☰]`, sans orientation dessinée. |
| End Offset coché, Up to, variante 2 | Enfant plus profondément indenté : `Target [Select…]`. |
| Symetric décoché | Option 2nd side disponible. |
| Symetric coché | Option 2nd side absente ; intention d'une extrusion symétrique. La répartition de Depth entre côtés n'est pas précisée. |
| 2nd side décoché | Aucun champ de seconde limite. |
| 2nd side coché, Blind | `Depth [+|0.0 mm ☰]` propre au second côté. |
| 2nd side coché, Up to | `Target [Select…]` puis option End Offset, dont les enfants suivent la variante 2. |

Le fichier montre également `□ Invert` / `■ Invert` comme fragments autonomes.
Ils expriment l'inversion de sens ; leur coexistence avec `+|`/`-|` n'est pas
tranchée. Le rattachement exact du simple `Offset` sous Extend/Up to, ainsi que
l'omission d'End Offset dans le fragment Extend/Blind, sont consignés en §11.
Aucun état `Through all` ni champ `Direction [Normal to]` n'est présent dans
les `.txt` actuels : ils ne font pas partie de cette spécification actualisée.

## 8. Revolve — tous les états référencés

Les six combinaisons de §6 partagent `Profile [Select…]`, `Axis [Select…]` et
`■ Full Revolve` dans les panneaux complets. Profile fournit la section et Axis
la référence de rotation. Add/Remove ajoutent la portée commune.

| Contrôle / état | Contenu dépendant et intention |
|---|---|
| Full Revolve coché | Révolution complète ; aucun angle, décalage, Symetric ou 2nd side montré. |
| Full Revolve décoché, Blind | `Angle [+|0.0 deg ☰]`, Start Offset, Symetric, 2nd side. Pas d'End Offset principal dans ce fragment. |
| Full Revolve décoché, Up to | `Target [Select…]`, Start Offset, End Offset, Symetric, 2nd side. |
| Start Offset décoché | Aucun enfant. |
| Start Offset coché, Blind | `Angle [+|0.0 deg ☰]` pour le décalage initial. |
| Start Offset coché, Up to | `Target [Select…]`. |
| End Offset décoché | Aucun enfant. |
| End Offset coché, Blind | `Angle [0.0 deg ☰]`, sans orientation dessinée. |
| End Offset coché, Up to | `Target [Select…]`. |
| End Offset, variante 1 / variante 2 | Mêmes choix ; variante 1 dans le groupe principal, variante 2 imbriquée sous la limite Up to du second côté. |
| 2nd side décoché | Aucun enfant. |
| 2nd side coché, Blind | `Angle [+|0.0 deg ☰]` pour le second côté. |
| 2nd side coché, Up to | `Target [Select…]`, puis End Offset et ses variantes imbriquées. |
| Symetric décoché | 2nd side disponible. |
| Symetric coché | 2nd side masqué ; la répartition de l'angle entre côtés n'est pas spécifiée. |

Le mode partiel révèle donc une seconde famille de limites et de décalages,
conditionnée d'abord par Full Revolve. Revenir au mode complet masque ce groupe.

## 9. Loft et Sweep — tous les états référencés

### 9.1 Loft

Les six combinaisons de §6 partagent `Profile [Select…]`, Start type, End type,
Guide, Path et Connection, tous décochés dans les panneaux complets.
L'intention de Profile est de définir les sections du loft ; la liste ordonnée de
profils de §3.5 décrit l'édition de leur ordre. Add/Remove ont la portée commune.

| Option / état | Contenu dépendant et intention |
|---|---|
| Start type décoché | Aucune condition de départ explicite. |
| Start type coché, Normal | `Magnitude [0.0 ☰]` : intensité de la condition normale au départ. |
| Start type coché, Tengent | `Magnitude [0.0 ☰]` : intensité de la condition tangente au départ. |
| Start type coché, Match | `Entity [Select…]` : entité de référence à raccorder. |
| End type décoché | Aucune condition d'arrivée explicite. |
| End type coché, Normal | `Magnitude [0.0 ☰]` à l'arrivée. |
| End type coché, Tengent | `Magnitude [0.0 ☰]` à l'arrivée. |
| End type coché, Match | `Entity [Select…]` à l'arrivée. |
| Guide décoché / coché | Masque / affiche un champ `[Select…]` sur la ligne Guide. |
| Path décoché / coché | Masque / affiche un champ `[Select…]` sur la ligne Path. |
| Connection décoché | Aucun enfant. |
| Connection coché | Deux sélections enfants : Start et End, pour définir les connexions. |

Les variantes Normal/Tengent/Match sont cycliques. Les dessins ne précisent
ni le degré de continuité de Match, ni l'unité de Magnitude, ni les combinaisons
interdites entre Guide, Path et Connection : ne pas inventer de telles règles.

### 9.2 Sweep

Les six combinaisons de §6 partagent `Profile [Select…]`, `Path [Select…]`,
Full path, Keep orientation et Scale, initialement décochés. Profile fournit la
section et Path la trajectoire. Add/Remove ont la portée commune.

| Option / état | Contenu dépendant et intention |
|---|---|
| Full path décoché / coché | Deux états montrés, sans champ enfant. Coché exprime l'utilisation du chemin complet ; aucune borne partielle n'est dessinée pour l'autre état. |
| Keep orientation décoché | Aucun enfant Twist. |
| Keep orientation coché | Révèle `□ Twist` ; intention de conserver l'orientation avec réglage optionnel de torsion. |
| Twist décoché | Pas de saisie angulaire. |
| Twist coché | `Twist [+|0.0 deg ☰]` sur la même ligne, pour l'angle et le sens de torsion. |
| Scale décoché | Aucun champ de facteur. |
| Scale coché | `Scale [0.0 ☰]` sur la même ligne, sans unité affichée. |

Twist dépend de Keep orientation dans les dessins, même si d'autres logiciels
organisent ces options autrement. La distribution de la torsion et de l'échelle
le long du chemin n'est pas définie par ces variantes.

## 10. Boolean — tous les états référencés

Boolean a son propre en-tête segmenté `Add | Cut | Inter`, sans
Solid/Surface ni New/Add/Remove. Tous les états comportent
`Target [Select…]`, `Tools [Select…]`, Keep tools et OK/Cancel.
Target identifie la cible et Tools les opérandes outils.

| Segment actif | Intention et contrôles |
|---|---|
| Add | Union/ajout ; pas d'Offset montré. |
| Cut | Soustraction ; option Offset entre Tools et Keep tools. |
| Inter | Intersection ; pas d'Offset montré. |

| Option / état | Contenu dépendant et intention |
|---|---|
| Offset décoché | Pas de paramètres de décalage. |
| Offset coché | Révèle Offset all, montré coché, puis ses paramètres. |
| Offset all coché | `Dist [+|0.0 mm ☰]` : distance orientée appliquée globalement. |
| Offset all décoché | `Faces [none]`, puis `Dist [+|0.0 mm ☰]` : sélection des faces auxquelles appliquer le décalage. |
| Keep tools décoché | Intention de ne pas conserver les outils après l'opération. |
| Keep tools coché | Intention de conserver les outils après l'opération. Aucun champ supplémentaire. |

Offset all dépend d'Offset, lui-même montré uniquement en Cut. Les faces
admissibles et le côté géométrique auquel le décalage s'applique ne sont pas
explicités. Keep tools est indépendant du décalage et existe dans les trois modes.
Boolean ne possède pas le contrôle Merge with all des quatre autres opérations.

## 11. Incohérences et décisions encore ouvertes

### 11.1 Libellés à harmoniser ultérieurement

Les `.txt` restent inchangés. Les formes normalisées ci-dessous permettent de
parler d'un même concept ; elles ne prétendent pas être déjà présentes partout.

| Forme dans les références | Forme normalisée / correspondance | Observation |
|---|---|---|
| Symetric | Symmetric | Extrude, Revolve et shell ; faute orthographique. |
| Starting Offset / Start Offset | Start Offset | Panneaux complets Extrude/shell contre fragments de variantes et Revolve. |
| Second end position / 2nd side | 2nd side | Panneaux complets Extrude/shell contre variantes détaillées ; même intention de seconde limite. |
| Tengent | Tangent | Conditions Start type et End type de Loft. |
| Profils | Profiles | Titre français de la boîte de profils dans une interface majoritairement anglaise ; noms d'objets Profil_3/Profil_4 distincts du titre. |
| Mesure (`[M]esure`) | Measure (`[M]easure`) | Entrée Inspect ; orthographe anglaise attendue. |
| Part design / Part Design | Même atelier | Casse différente entre Workbench et StatusBar. |

### 11.2 Écarts structurels à ne pas masquer

- **Extrude / Extend** : les panneaux complets Blind montrent End Offset,
  mais le fragment Extend/Blind s'arrête à Starting Offset. La visibilité exacte
  d'End Offset principal en Blind reste à confirmer ; les deux dessins sont
  recensés, aucun n'est silencieusement supprimé.
- **Extrude / Up to** : le fragment présente `Selection` et un simple `Offset`,
  tandis que les autres variantes parlent de Target et End Offset. Le lien entre
  ce simple Offset et les variantes End Offset n'est pas explicitement établi.
- **Orientation des décalages** : Extrude End Offset variante 1 contient `+|`,
  variante 2 non ; Revolve End Offset n'en contient dans aucune des deux.
  Ne pas uniformiser ces champs sans décision UX.
- **Invert** : les checkboxes autonomes d'Extrude et l'inversion intégrée aux
  champs coexistent dans les sources ; décider ultérieurement si elles sont
  alternatives ou liées. Ne pas ajouter deux actions contradictoires.
- **Task bar** : le menu View conserve ce libellé sans identifier sans ambiguïté
  CommandBar ou ActionPanel. Garder les deux régions distinctes jusqu'à décision.
- **Arbres** : signification métier de leurs cases, ligne horizontale dans
  Features et règles de sélection/visibilité encore non précisées.
- **Profils** : poignée de réordonnancement déduite de la disposition ; geste,
  doublons et contraintes d'ordre restent à confirmer.
- **Dessin et rendu** : largeurs, tabulations, jonctions et espacements ne sont
  pas toujours cohérents, notamment dans Boolean et les menus. Les cadres
  expriment la structure ; ils ne fixent pas tous une largeur exacte en cellules.
- **Validation** : filtres de sélection, plages numériques, messages d'erreur,
  raccourcis effectifs, conservation des valeurs masquées et géométrie des modes
  Surface ne sont pas déduits de simples exemples.

Les anciennes mentions de `Revovle`, `Through all`, `Direction [Normal to]`
et `Merge scope` ne décrivent pas les fichiers actuels : le titre est Revolve,
les limites illustrées sont Blind/Up to et la portée explicite s'appelle Targets.

## 12. Contraintes de réalisation conservées

La mise en œuvre reste soumise à ARCHITECTURE.md : grille de cellules entières
au ratio logique 3:5, séparation MVC, état par fenêtre dans des structs/enums
Newell, représentation rendue en tableau plat de cellules sans arbre de widgets
retenu. Le hit-testing utilise la division pixels/cellules en O(1), avec modulo
pour les séparateurs partagés. Le rendu de la surbrillance appartient au shader,
et l'overlay ASCII est consommé en un seul appel de dessin.

L'état persistant (commande, sélections, options, expansion des arbres) et l'état
transitoire (focus, survol, saisie) restent hors du tampon de cellules. La
composition régénère ce tampon à partir de ces états. Les branches de dépendance
visuelles n'imposent donc pas un DOM ni un arbre de widgets retenu.

La palette de Special_ASCII n'est pas une garantie de support de tous ses glyphes
par le moteur de bordures. Le contrat M2 des bordures simples/doubles fusionne
les topologies de façon déterministe, promeut les jonctions mixtes en double,
applique le dernier StyleId dessiné et ignore les rectangles de moins de deux
cellules par dimension. Coins arrondis, traits épais/discontinus et curseurs de
liste doivent être pris en compte par le jalon de widgets concerné.

Aucun atlas factice n'est requis par ces références textuelles. Les futurs logos
respecteront l'atlas PNG RGBA de taille puissance de deux, les tranches de
256×256 (zone visible 240×240) et les allocations 5×3 / 3×2 cellules prévues
par l'architecture. Cette documentation n'ajoute aucune dépendance de rendu.
