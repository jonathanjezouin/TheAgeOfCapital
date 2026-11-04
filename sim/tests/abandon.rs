//! Abandon, destruction, scission des terres et repousse (docs/07 §14).

use sim::monde::{Cible, Contenu};
use sim::terrain::Terrain;
use sim::{generer, Catalogue, ParamsVillage, Simulation};

fn village(graine: u64) -> Simulation {
    generer(Catalogue::defaut(), &ParamsVillage { graine, ..Default::default() }).unwrap()
}

/// Un bloc de `w × h` tiles ouvertes, libres et sans route, à au moins 12 tiles du premier hameau (coin haut-gauche).
fn bloc_libre(s: &Simulation, w: usize, h: usize) -> Vec<usize> {
    let (hx, hy) = s.monde.xy(s.monde.quartiers[0].tuile);
    for y in 2..s.monde.hauteur - h - 2 {
        for x in 2..s.monde.largeur - w - 2 {
            if x.abs_diff(hx).max(y.abs_diff(hy)) < 12 {
                continue;
            }
            let tuiles: Vec<usize> = (0..h).flat_map(|j| (0..w).map(move |i| (x + i, y + j))).map(|(x, y)| s.monde.idx(x, y)).collect();
            if tuiles.iter().all(|&t| {
                let tu = &s.monde.tuiles[t];
                tu.contenu == Contenu::Vide && tu.aretes == 0 && tu.ouvert()
            }) {
                return tuiles;
            }
        }
    }
    panic!("aucun bloc libre");
}

/// Ajoute une terre de ce type sur les tiles données (sans route) et renvoie son index.
fn poser_terre(s: &mut Simulation, nom: &str, tuiles: &[usize]) -> usize {
    let t = s.cat.bat_id(nom).unwrap();
    let modele = s.monde.batiments.iter().find(|b| b.type_id == t).unwrap().clone();
    let mut bat = modele;
    bat.emprise = tuiles.to_vec();
    bat.tuile = tuiles[0];
    bat.chantier = None;
    bat.abandon_mois = None;
    bat.inactif_mois = 0;
    let b = s.monde.batiments.len();
    s.monde.batiments.push(bat);
    for &i in tuiles {
        s.monde.tuiles[i].contenu = Contenu::Batiment(b);
    }
    s.reconstruire_reseau();
    b
}

#[test]
fn detruire_une_tile_coupe_la_terre_en_deux_terres() {
    let mut s = village(1);
    let bloc = bloc_libre(&s, 3, 3);
    let b = poser_terre(&mut s, "champ", &bloc);
    let g = s.cat.res_id("grain").unwrap();
    s.monde.batiments[b].stock_sortie[g] = 900_000;
    let avant = s.monde.batiments.len();
    // La colonne du milieu (tiles 1, 4, 7 du bloc) est détruite : deux morceaux de 3 tiles.
    let milieu = [bloc[1], bloc[4], bloc[7]];
    let touchees = s.detruire_tuiles(&milieu);
    assert_eq!(touchees, vec![b]);
    s.scinder(b, &[]);
    assert_eq!(s.monde.batiments.len(), avant + 1, "une terre de plus");
    let (a, n) = (&s.monde.batiments[b], &s.monde.batiments[avant]);
    assert_eq!((a.emprise.len(), n.emprise.len()), (3, 3));
    assert!(a.emprise.contains(&a.tuile), "la terre garde son nœud");
    assert_eq!(a.type_id, n.type_id);
    // Le stock se partage sans perte.
    assert_eq!(a.stock_sortie[g] + n.stock_sortie[g], 900_000);
    // Les tiles détruites sont libres ; chaque tile appartient à sa terre.
    for &t in &milieu {
        assert_eq!(s.monde.tuiles[t].contenu, Contenu::Vide);
    }
    for &t in &n.emprise {
        assert_eq!(s.monde.tuiles[t].contenu, Contenu::Batiment(avant));
    }
}

#[test]
fn une_route_a_travers_un_champ_detruit_les_tiles_traversees() {
    let mut s = village(1);
    let bloc = bloc_libre(&s, 7, 3); // 7 tiles de large, 3 de haut
    let b = poser_terre(&mut s, "champ", &bloc);
    // Un chemin qui traverse le champ de part en part, sur sa rangée du milieu, entre deux tiles libres.
    let (x0, y0) = s.monde.xy(bloc[0]);
    let rangee: Vec<usize> = (0..7).map(|i| s.monde.idx(x0 + i, y0 + 1)).collect();
    let avant = s.monde.batiments.len();
    let (de, vers) = (s.monde.idx(x0 - 1, y0 + 1), s.monde.idx(x0 + 7, y0 + 1));
    let cout = s.travail_route_pour_test(&rangee);
    assert!(cout > 0, "traverser un champ coûte du travail");
    s.tracer_route(de, &rangee, vers);
    // Les 7 tiles de la rangée sont des routes, hors de toute emprise : le champ est coupé en deux (haut et bas).
    for &t in &rangee {
        assert_ne!(s.monde.tuiles[t].aretes, 0);
        assert_eq!(s.monde.tuiles[t].contenu, Contenu::Vide, "tile détruite");
    }
    assert_eq!(s.monde.batiments.len(), avant + 1, "le champ se scinde en deux terres");
    let total: usize = [b, avant].iter().map(|&i| s.monde.batiments[i].emprise.len()).sum();
    assert_eq!(total, 14, "7 tiles du haut et du bas, la rangée du milieu détruite");
}

#[test]
fn les_villages_detruisent_des_tiles_de_terre_pour_poser_des_routes() {
    let mut detruites = 0;
    for graine in 1..=8u64 {
        let mut s = village(graine);
        s.avancer(365 * 30);
        detruites += s.evenements.iter().filter(|e| e.texte.contains("détruisant") || e.texte.contains("Une terre coupée")).count();
        // Jamais de route à travers une terre qui n'a pas été détruite.
        for (t, tu) in s.monde.tuiles.iter().enumerate() {
            if tu.aretes != 0 {
                if let Contenu::Batiment(x) = tu.contenu {
                    assert_eq!(s.monde.batiments[x].tuile, t);
                }
            }
        }
    }
    assert!(detruites > 0, "aucune destruction en 8 villages et 30 ans");
}

#[test]
fn une_terre_abandonnee_ne_produit_plus_puis_redevient_friche_puis_foret() {
    let mut s = village(1);
    s.cat.abandon.terre_friche_mois = 24;
    s.cat.abandon.foret_apres_mois = 120; // 10 ans
    let bloc = bloc_libre(&s, 3, 3);
    let b = poser_terre(&mut s, "champ", &bloc);
    s.monde.tuiles[bloc[0]].terrain = Terrain::Plaine;
    s.abandonner_terre(b);
    // Abandonnée : elle ne livre plus et ne produit plus.
    let g = s.cat.res_id("grain").unwrap();
    s.monde.batiments[b].stock_sortie[g] = 0;
    s.avancer(10);
    assert_eq!(s.monde.batiments[b].stock_sortie[g], 0);
    assert_eq!(s.monde.batiments[b].prod_ppm, 0);
    // Après `terre_friche_mois`, les tiles sont libres et comptent le temps de repousse.
    s.avancer(30 * 25);
    assert!(bloc.iter().all(|&t| s.monde.tuiles[t].contenu == Contenu::Vide), "tiles libérées");
    assert!(bloc.iter().all(|&t| s.monde.tuiles[t].vacant_mois > 0));
    assert!(s.repousse_restante_mois(bloc[4]).is_some());
    // La forêt reprend : 10 ans (×0,6 à ×1,5 selon la lisière) après la libération.
    s.avancer(30 * 12 * 16);
    assert!(
        bloc.iter().all(|&t| s.monde.tuiles[t].terrain == Terrain::Foret),
        "tiles redevenues forêt : {:?}",
        bloc.iter().map(|&t| (s.monde.tuiles[t].terrain, s.monde.tuiles[t].vacant_mois)).collect::<Vec<_>>()
    );
    // Conservation de la matière malgré le stock perdu à la libération.
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
}

#[test]
fn la_repousse_est_plus_rapide_en_lisiere_d_une_foret() {
    let mut s = village(1);
    let bloc = bloc_libre(&s, 5, 5);
    let centre = bloc[12];
    let isole = s.facteur_repousse_ppm(centre);
    // Trois tiles de forêt voisines : lisière forte.
    for d in [0usize, 2, 4] {
        let v = s.monde.voisin(centre, d).unwrap();
        s.monde.tuiles[v].terrain = Terrain::Foret;
    }
    let lisiere = s.facteur_repousse_ppm(centre);
    assert_eq!(isole, s.cat.abandon.lisiere_isolee_ppm);
    assert_eq!(lisiere, s.cat.abandon.lisiere_forte_ppm);
    assert!(lisiere < isole);
}

#[test]
fn un_hameau_vide_est_abandonne_puis_detruit() {
    let mut s = village(1);
    s.cat.abandon.hameau_vide_mois = 12;
    s.cat.abandon.hameau_ruine_mois = 24;
    // Hameau 0 : ses pops partent dans un autre hameau.
    for p in &mut s.pops {
        if p.residence == 0 {
            p.residence = 1;
        }
    }
    s.avancer(30 * 14);
    assert!(s.monde.quartiers[0].abandon_mois.is_some(), "hameau abandonné");
    assert_eq!(s.monde.quartiers[0].capacite_pops, 0, "plus de logement");
    assert!(s.evenements.iter().any(|e| e.texte.starts_with("Hameau abandonné")));
    s.avancer(30 * 26);
    let h = &s.monde.quartiers[0];
    assert!(h.detruit, "hameau détruit");
    assert_eq!(s.monde.tuiles[h.tuile].contenu, Contenu::Vide);
    assert!(s.ecart_conservation().iter().all(|&e| e == 0));
}

#[test]
fn un_chantier_sans_progres_est_abandonne_et_son_equipe_liberee() {
    let mut s = village(1);
    s.cat.abandon.chantier_bloque_mois = 6;
    // Plus de bois : la maison ne peut pas avancer.
    s.cat.peuplement.seuil_fondation_ppm = 1_000_000_000;
    let bois = s.cat.res_id("bois").unwrap();
    let seuil = s.seuil_croissance();
    s.pops[0].croissance = seuil;
    s.etendre();
    let Some(b) = (0..s.monde.batiments.len()).find(|&b| matches!(s.monde.batiments[b].chantier.as_ref().map(|c| c.cible), Some(Cible::Maison(_)))) else {
        return; // pas de maison possible dans ce scénario : rien à tester
    };
    for bat in &mut s.monde.batiments {
        bat.stock_sortie[bois] = 0;
    }
    // On interdit tout apport de bois : plus aucun producteur ne livre.
    for t in &mut s.cat.batiments {
        for e in &mut t.etapes {
            e.extrants.retain(|&(r, _)| r != bois);
        }
    }
    s.avancer(30 * 12);
    assert!(s.monde.batiments[b].chantier.is_none(), "chantier abandonné");
    assert!(s.evenements.iter().any(|e| e.texte.starts_with("Chantier abandonné")));
    assert!(s.pops.iter().all(|p| p.emploi != Some(b)), "équipe libérée");
}
