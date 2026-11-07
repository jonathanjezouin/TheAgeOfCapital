# 08 — Concept de subsistance (une pop)

> Décisions du 2026-11-07, issues d'une discussion de concept **sans tenir compte du code actuel**. Elles décrivent le modèle cible de la production et de la consommation de subsistance pour **une seule pop** ; l'extension à plusieurs pops et au seigneur viendra ensuite. Les valeurs sont des paramètres de départ, à régler en jeu.

## 1. Cadre

| Élément | Décision |
|---|---|
| Tile | 250 × 250 m (6,25 ha) |
| Pop | 100 habitants, unité de travail et de consommation ; 1 pop = 1 pop-jour par jour |
| Calendrier | Seul le **dimanche** est chômé (52 jours/an, ~313 ouvrables) ; pas de jour férié pour l'instant |
| Météo | Un facteur annuel tiré au hasard entre 0,8 et 1,2 |
| Hors modèle | Semences, jachère, battage, outils, machines (les innovations techniques viendront en multiplicateurs de rendement ou de performance) |

Unités abstraites (échelle ×10 pour éviter les non-entiers) : 1 grain ≈ 2 t ; 1 unité de bois ≈ 5 t.

## 2. Territoire et propriété

- Une pop occupe **1 tile de ferme isolée**, **5 tiles de champs** et **3 tiles de forêt**.
- Chaque tile a pour **propriétaire le seigneur** (pas encore conceptualisé) et éventuellement un **occupant**.
- Champs et forêts sont **affectés à la pop** de la même façon.

## 3. Production

| Bien | Où | Travail | Production |
|---|---|---|---|
| **Grain** | 5 champs | Par tile : labour 5 j (oct.-nov.), semis 5 j (mars-avril), récolte 10 j (juil.-août) | 3 grains par tile, produits à la fin de la récolte : 15 par an avec météo 1 |
| **Bois** | 3 forêts | Coupe (2 j par unité, estimation) | 3 par tile et par an : 9 par an. La forêt n'est jamais rasée (rendement soutenable) |
| **Vêtement** | Maison | 20 j par unité | Pas de transport |

- Météo : facteur 1,1 = 10 % de chance que chaque grain récolté soit doublé ; facteur 0,9 = 10 % de chance que chaque grain récolté soit perdu.
- La moisson de 5 tiles demande 50 jours pour ~53 jours ouvrables en juillet-août : marge de 3 jours.

## 4. Prélèvement

- Un seul terme : **prélèvement**, de **15 %** par an, sur le grain et sur le bois, au profit du seigneur.
- Grain : 15 × 0,85 = 12,75 net. Bois : 9 × 0,85 = 7,65 net.

## 5. Transport

- Coût en jours = **quantité × distance (en tiles) × k**.
- Grain : k = **1 jour** par grain et par tile (équivalent à un transport à dos d'homme).
- Bois : k = **2 jours** par unité et par tile (estimation).
- Le vêtement, fabriqué à la maison, n'a pas de transport.
- Outils, routes et bêtes de somme viendront plus tard comme multiplicateurs de k.

## 6. Décision quotidienne d'une pop

1. **Dimanche** : chômé.
2. **Travail disponible sur la terre** (labour, semis, récolte, transport du grain) : la pop y va.
3. **Bois** si le stock est inférieur à (5 + prélèvement attendu) + 10 % : travail en forêt et transport à la maison.
4. **Vêtement** si le stock est inférieur à 2 : travail à la maison.
5. **Sinon** : chômage supplémentaire, sans effet sur le contentement et sans volonté d'accumulation.

## 7. Consommation (mensuelle, à pourcentage)

| Bien | Règle | Par an |
|---|---|---|
| Grain | 100 % chaque mois | 12 |
| Bois | 100 % les mois froids (codé) | 5 |
| Vêtement | 20 % chaque mois | ~2,4 en moyenne |

En cas de stock vide : **simple trace** pour l'instant, sans conséquence.

## 8. Vérification chiffrée

**Grain** : 12,75 produit contre 12 consommé laisse +0,75, donc très tendu. Toute année où le facteur météo est inférieur à ~0,94 (environ 35 % des années avec un tirage uniforme de 0,8 à 1,2) est déficitaire ; le stock se reporte d'une année sur l'autre et amortit en partie.

**Temps** (distance moyenne de 2 à 3 tiles) :

| Poste | Jours |
|---|---|
| Grain, culture | 100 |
| Grain, transport | 30 à 45 |
| Bois (~7 unités × 6 à 8 j) | 42 à 56 |
| Vêtements (~2,4 × 20 j) | ~48 |
| **Total** | **~220 à 250 sur 313** |

Il reste 60 à 90 jours de temps libre, que la corvée et la construction consommeront plus tard.

## 9. Points ouverts

1. **Moisson et transport** : le transport du grain ne doit pas passer avant la moisson restante, sinon la fenêtre de juillet-août est dépassée. Proposition : récolter d'abord toutes les tiles, transporter ensuite.
2. **Grain laissé dans le champ** : se perd-il avec le temps, ou attend-il sans risque ?
3. **Prélèvement** : quand et où est-il pris (sur le champ à la récolte, ou à la maison à date fixe) ? Qui paie son transport ?
4. **Bois** : la forêt se régénère-t-elle d'un coup (1ᵉʳ janvier) ou progressivement ? La météo l'affecte-t-elle ?
5. **Stock initial** : combien de grain, bois et vêtements au départ ? Sans stock en janvier, la pop jeûne avant la première récolte.
6. **Vêtement à 20 % par mois** : certaines années en donneront 0 et d'autres 5 ; alternative déterministe : un compteur d'usure.

## 10. Idées reportées (hors de ce périmètre)

- Plusieurs pops : affectation individuelle des champs et forêt collective, conflits d'usage.
- Seigneur et prélèvements sur le travail (corvées), forêts seigneuriales.
- Pops non agricoles : approvisionnement par prélèvement, échange ou salaire ; proto-industrie textile.
- Jours fériés, calendrier républicain, repos hebdomadaire et congés payés comme changements de règle historiques.
- Famine, mortalité et état d'esprit comme conséquences d'un stock vide.
