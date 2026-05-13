//! Frisian/Dutch aliases for Fryslân localities.
//!
//! Source: provincie Fryslân, "Alfabetyske list fan de plaknammen yn Fryslân"
//! (bywurke 18 maart 2024). Each entry pairs the official BAG locality name
//! with its alternative-language form: the Frisian name when the official is
//! Dutch, the Dutch name when the official is Frisian. Localities whose
//! official name is identical in both languages are omitted.

use std::{collections::HashMap, sync::LazyLock};

pub struct LocalityAlias {
    pub name: &'static str,
    pub alias: &'static str,
}

static ALIAS_INDEX: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    FRYSLAN_LOCALITY_ALIASES
        .iter()
        .map(|entry| (entry.name, entry.alias))
        .collect()
});

/// Look up the Frisian/Dutch alias for the given official BAG locality name.
pub fn lookup_alias(name: &str) -> Option<&'static str> {
    ALIAS_INDEX.get(name).copied()
}

pub static FRYSLAN_LOCALITY_ALIASES: &[LocalityAlias] = &[
    LocalityAlias {
        name: "Aalsum",
        alias: "Ealsum",
    },
    LocalityAlias {
        name: "Abbega",
        alias: "Abbegea",
    },
    LocalityAlias {
        name: "Akmarijp",
        alias: "Eagmaryp",
    },
    LocalityAlias {
        name: "Alde Leie",
        alias: "Oude Leije",
    },
    LocalityAlias {
        name: "Aldeboarn",
        alias: "Oldeboorn",
    },
    LocalityAlias {
        name: "Aldtsjerk",
        alias: "Oudkerk",
    },
    LocalityAlias {
        name: "Aldwâld",
        alias: "Oudwoude",
    },
    LocalityAlias {
        name: "Appelscha",
        alias: "Appelskea",
    },
    LocalityAlias {
        name: "Augsbuert-Lytsewâld",
        alias: "Augsbuurt",
    },
    LocalityAlias {
        name: "Augustinusga",
        alias: "Stynsgea",
    },
    LocalityAlias {
        name: "Baaiduinen",
        alias: "Baaidunen",
    },
    LocalityAlias {
        name: "Baaium",
        alias: "Baijum",
    },
    LocalityAlias {
        name: "Bakhuizen",
        alias: "Bakhuzen",
    },
    LocalityAlias {
        name: "Bakkeveen",
        alias: "Bakkefean",
    },
    LocalityAlias {
        name: "Bantega",
        alias: "Bantegea",
    },
    LocalityAlias {
        name: "Bears",
        alias: "Beers",
    },
    LocalityAlias {
        name: "Beetsterzwaag",
        alias: "Beetstersweach",
    },
    LocalityAlias {
        name: "Berltsum",
        alias: "Berlikum",
    },
    LocalityAlias {
        name: "Bitgum",
        alias: "Beetgum",
    },
    LocalityAlias {
        name: "Bitgummole",
        alias: "Beetgumermolen",
    },
    LocalityAlias {
        name: "Blauwhuis",
        alias: "Blauhús",
    },
    LocalityAlias {
        name: "Blesdijke",
        alias: "Blesdike",
    },
    LocalityAlias {
        name: "Blije",
        alias: "Blija",
    },
    LocalityAlias {
        name: "Boarnwert",
        alias: "Bornwird",
    },
    LocalityAlias {
        name: "Boazum",
        alias: "Bozum",
    },
    LocalityAlias {
        name: "Boelenslaan",
        alias: "Boelensloane",
    },
    LocalityAlias {
        name: "Boijl",
        alias: "Boyl",
    },
    LocalityAlias {
        name: "Bolsward",
        alias: "Boalsert",
    },
    LocalityAlias {
        name: "Boornbergum",
        alias: "Boarnburgum",
    },
    LocalityAlias {
        name: "Boornzwaag",
        alias: "Boarnsweach",
    },
    LocalityAlias {
        name: "Breezanddijk",
        alias: "Breesândyk",
    },
    LocalityAlias {
        name: "Britswert",
        alias: "Britswerd",
    },
    LocalityAlias {
        name: "Broek",
        alias: "De Broek",
    },
    LocalityAlias {
        name: "Broeksterwâld",
        alias: "Broeksterwoude",
    },
    LocalityAlias {
        name: "Buitenpost",
        alias: "Bûtenpost",
    },
    LocalityAlias {
        name: "Burdaard",
        alias: "Birdaard",
    },
    LocalityAlias {
        name: "Burgum",
        alias: "Bergum",
    },
    LocalityAlias {
        name: "Burgwerd",
        alias: "Burchwert",
    },
    LocalityAlias {
        name: "Burum",
        alias: "Boerum",
    },
    LocalityAlias {
        name: "Cornwerd",
        alias: "Koarnwert",
    },
    LocalityAlias {
        name: "Damwâld",
        alias: "Damwoude",
    },
    LocalityAlias {
        name: "De Falom",
        alias: "De Valom",
    },
    LocalityAlias {
        name: "De Trieme",
        alias: "Triemen",
    },
    LocalityAlias {
        name: "De Veenhoop",
        alias: "De Feanhoop",
    },
    LocalityAlias {
        name: "De Westereen",
        alias: "Zwaagwesteinde",
    },
    LocalityAlias {
        name: "De Wilgen",
        alias: "De Wylgen",
    },
    LocalityAlias {
        name: "Dearsum",
        alias: "Deersum",
    },
    LocalityAlias {
        name: "Dedgum",
        alias: "Dedzjum",
    },
    LocalityAlias {
        name: "Delfstrahuizen",
        alias: "Dolsterhuzen",
    },
    LocalityAlias {
        name: "Dijken",
        alias: "Diken",
    },
    LocalityAlias {
        name: "Dongjum",
        alias: "Doanjum",
    },
    LocalityAlias {
        name: "Doniaga",
        alias: "Dunegea",
    },
    LocalityAlias {
        name: "Drachtstercompagnie",
        alias: "Drachtsterkompenije",
    },
    LocalityAlias {
        name: "Driezum",
        alias: "Driesum",
    },
    LocalityAlias {
        name: "Drogeham",
        alias: "Droegeham",
    },
    LocalityAlias {
        name: "Dronryp",
        alias: "Dronrijp",
    },
    LocalityAlias {
        name: "Eagum",
        alias: "Aegum",
    },
    LocalityAlias {
        name: "Eanjum",
        alias: "Anjum",
    },
    LocalityAlias {
        name: "Earnewâld",
        alias: "Eernewoude",
    },
    LocalityAlias {
        name: "Easterein",
        alias: "Oosterend",
    },
    LocalityAlias {
        name: "Easterlittens",
        alias: "Oosterlittens",
    },
    LocalityAlias {
        name: "Eastermar",
        alias: "Oostermeer",
    },
    LocalityAlias {
        name: "Easternijtsjerk",
        alias: "Oosternijkerk",
    },
    LocalityAlias {
        name: "Easterwierrum",
        alias: "Oosterwierum",
    },
    LocalityAlias {
        name: "Eastrum",
        alias: "Oostrum",
    },
    LocalityAlias {
        name: "Echten",
        alias: "Ychten",
    },
    LocalityAlias {
        name: "Echtenerbrug",
        alias: "Ychtenbrêge",
    },
    LocalityAlias {
        name: "Eesterga",
        alias: "Jistergea",
    },
    LocalityAlias {
        name: "Elahuizen",
        alias: "Ealahuzen",
    },
    LocalityAlias {
        name: "Elsloo",
        alias: "Elslo",
    },
    LocalityAlias {
        name: "Exmorra",
        alias: "Eksmoarre",
    },
    LocalityAlias {
        name: "Feankleaster",
        alias: "Veenklooster",
    },
    LocalityAlias {
        name: "Feanwâlden",
        alias: "Veenwouden",
    },
    LocalityAlias {
        name: "Feanwâldsterwâl",
        alias: "Veenwoudsterwal",
    },
    LocalityAlias {
        name: "Feinsum",
        alias: "Finkum",
    },
    LocalityAlias {
        name: "Ferwert",
        alias: "Ferwerd",
    },
    LocalityAlias {
        name: "Ferwoude",
        alias: "Ferwâlde",
    },
    LocalityAlias {
        name: "Firdgum",
        alias: "Furdgum",
    },
    LocalityAlias {
        name: "Fochteloo",
        alias: "De Fochtel",
    },
    LocalityAlias {
        name: "Follega",
        alias: "Follegea",
    },
    LocalityAlias {
        name: "Folsgare",
        alias: "Folsgeare",
    },
    LocalityAlias {
        name: "Formerum",
        alias: "Formearum",
    },
    LocalityAlias {
        name: "Franeker",
        alias: "Frjentsjer",
    },
    LocalityAlias {
        name: "Frieschepalen",
        alias: "Fryske Peallen",
    },
    LocalityAlias {
        name: "Gaastmeer",
        alias: "De Gaastmar",
    },
    LocalityAlias {
        name: "Garyp",
        alias: "Garijp",
    },
    LocalityAlias {
        name: "Gauw",
        alias: "Gau",
    },
    LocalityAlias {
        name: "Gerkesklooster",
        alias: "Gerkeskleaster",
    },
    LocalityAlias {
        name: "Gersloot",
        alias: "Gersleat",
    },
    LocalityAlias {
        name: "Ginnum",
        alias: "Genum",
    },
    LocalityAlias {
        name: "Goënga",
        alias: "Goaiïngea",
    },
    LocalityAlias {
        name: "Goëngahuizen",
        alias: "Goaiïngahuzen",
    },
    LocalityAlias {
        name: "Goingarijp",
        alias: "Goaiïngaryp",
    },
    LocalityAlias {
        name: "Gorredijk",
        alias: "De Gordyk",
    },
    LocalityAlias {
        name: "Grou",
        alias: "Grouw",
    },
    LocalityAlias {
        name: "Gytsjerk",
        alias: "Giekerk",
    },
    LocalityAlias {
        name: "Hantumerútbuorren",
        alias: "Hantumeruitburen",
    },
    LocalityAlias {
        name: "Hantumhuzen",
        alias: "Hantumhuizen",
    },
    LocalityAlias {
        name: "Harkema",
        alias: "De Harkema",
    },
    LocalityAlias {
        name: "Harlingen",
        alias: "Harns",
    },
    LocalityAlias {
        name: "Hartwerd",
        alias: "Hartwert",
    },
    LocalityAlias {
        name: "Haskerdijken",
        alias: "Haskerdiken",
    },
    LocalityAlias {
        name: "Haskerhorne",
        alias: "Haskerhoarne",
    },
    LocalityAlias {
        name: "Haule",
        alias: "De Haule",
    },
    LocalityAlias {
        name: "Haulerwijk",
        alias: "Haulerwyk",
    },
    LocalityAlias {
        name: "Heeg",
        alias: "Heech",
    },
    LocalityAlias {
        name: "Heerenveen",
        alias: "It Hearrenfean",
    },
    LocalityAlias {
        name: "Hegebeintum",
        alias: "Hogebeintum",
    },
    LocalityAlias {
        name: "Hemelum",
        alias: "Himmelum",
    },
    LocalityAlias {
        name: "Hempens",
        alias: "Himpens",
    },
    LocalityAlias {
        name: "Hemrik",
        alias: "De Himrik",
    },
    LocalityAlias {
        name: "Herbaijum",
        alias: "Hjerbeam",
    },
    LocalityAlias {
        name: "Hiaure",
        alias: "De Lytse Jouwer",
    },
    LocalityAlias {
        name: "Hilaard",
        alias: "Hijlaard",
    },
    LocalityAlias {
        name: "Hindeloopen",
        alias: "Hylpen",
    },
    LocalityAlias {
        name: "Hinnaard",
        alias: "Hennaard",
    },
    LocalityAlias {
        name: "Hitzum",
        alias: "Hitsum",
    },
    LocalityAlias {
        name: "Holwert",
        alias: "Holwerd",
    },
    LocalityAlias {
        name: "Hommerts",
        alias: "De Hommerts",
    },
    LocalityAlias {
        name: "Hoorn",
        alias: "Hoarne",
    },
    LocalityAlias {
        name: "Hoornsterzwaag",
        alias: "Hoarnsterswaech",
    },
    LocalityAlias {
        name: "Houtigehage",
        alias: "De Houtigehage",
    },
    LocalityAlias {
        name: "Húns",
        alias: "Huins",
    },
    LocalityAlias {
        name: "Hurdegaryp",
        alias: "Hardegarijp",
    },
    LocalityAlias {
        name: "Idaerd",
        alias: "Idaard",
    },
    LocalityAlias {
        name: "Idsegahuizum",
        alias: "Skuzum",
    },
    LocalityAlias {
        name: "Idskenhuizen",
        alias: "Jiskenhuzen",
    },
    LocalityAlias {
        name: "Idzega",
        alias: "Idzegea",
    },
    LocalityAlias {
        name: "Ie",
        alias: "Ee",
    },
    LocalityAlias {
        name: "Iens",
        alias: "Edens",
    },
    LocalityAlias {
        name: "IJlst",
        alias: "Drylts",
    },
    LocalityAlias {
        name: "Indijk",
        alias: "Yndyk",
    },
    LocalityAlias {
        name: "Ingelum",
        alias: "Engelum",
    },
    LocalityAlias {
        name: "Ingwierrum",
        alias: "Engwierum",
    },
    LocalityAlias {
        name: "It Heidenskip",
        alias: "Het Heidenschap",
    },
    LocalityAlias {
        name: "Jannum",
        alias: "Janum",
    },
    LocalityAlias {
        name: "Jirnsum",
        alias: "Irnsum",
    },
    LocalityAlias {
        name: "Jistrum",
        alias: "Eestrum",
    },
    LocalityAlias {
        name: "Jonkerslân",
        alias: "Jonkerland",
    },
    LocalityAlias {
        name: "Jorwert",
        alias: "Jorwerd",
    },
    LocalityAlias {
        name: "Joure",
        alias: "De Jouwer",
    },
    LocalityAlias {
        name: "Jubbega",
        alias: "Jobbegea",
    },
    LocalityAlias {
        name: "Jutrijp",
        alias: "Jutryp",
    },
    LocalityAlias {
        name: "Katlijk",
        alias: "Ketlik",
    },
    LocalityAlias {
        name: "Kimswerd",
        alias: "Kimswert",
    },
    LocalityAlias {
        name: "Kinnum",
        alias: "Kinum",
    },
    LocalityAlias {
        name: "Klooster-Lidlum",
        alias: "Kleaster-Lidlum",
    },
    LocalityAlias {
        name: "Koarnjum",
        alias: "Cornjum",
    },
    LocalityAlias {
        name: "Kolderwolde",
        alias: "Kolderwâlde",
    },
    LocalityAlias {
        name: "Kollumerpomp",
        alias: "De Pomp",
    },
    LocalityAlias {
        name: "Kollumersweach",
        alias: "Kollumerzwaag",
    },
    LocalityAlias {
        name: "Kootstertille",
        alias: "Koatstertille",
    },
    LocalityAlias {
        name: "Kornwerderzand",
        alias: "Koarnwertersân",
    },
    LocalityAlias {
        name: "Kortehemmen",
        alias: "Koartehimmen",
    },
    LocalityAlias {
        name: "Kûbaard",
        alias: "Kubaard",
    },
    LocalityAlias {
        name: "Langedijke",
        alias: "Langedike",
    },
    LocalityAlias {
        name: "Langelille",
        alias: "De Langelille",
    },
    LocalityAlias {
        name: "Langezwaag",
        alias: "Langsweagen",
    },
    LocalityAlias {
        name: "Langweer",
        alias: "Langwar",
    },
    LocalityAlias {
        name: "Leeuwarden",
        alias: "Ljouwert",
    },
    LocalityAlias {
        name: "Legemeer",
        alias: "Legemar",
    },
    LocalityAlias {
        name: "Lemmer",
        alias: "De Lemmer",
    },
    LocalityAlias {
        name: "Leons",
        alias: "Lions",
    },
    LocalityAlias {
        name: "Lippenhuizen",
        alias: "Lippenhuzen",
    },
    LocalityAlias {
        name: "Ljussens",
        alias: "Lioessens",
    },
    LocalityAlias {
        name: "Loënga",
        alias: "Loaiïngea",
    },
    LocalityAlias {
        name: "Longerhouw",
        alias: "Longerhou",
    },
    LocalityAlias {
        name: "Luinjeberd",
        alias: "Lúnbert",
    },
    LocalityAlias {
        name: "Luxwoude",
        alias: "Lúkswâld",
    },
    LocalityAlias {
        name: "Lytsewierrum",
        alias: "Lutkewierum",
    },
    LocalityAlias {
        name: "Makkinga",
        alias: "Makkingea",
    },
    LocalityAlias {
        name: "Marsum",
        alias: "Marssum",
    },
    LocalityAlias {
        name: "Menaam",
        alias: "Menaldum",
    },
    LocalityAlias {
        name: "Midlum",
        alias: "Mullum",
    },
    LocalityAlias {
        name: "Midsland",
        alias: "Midslân",
    },
    LocalityAlias {
        name: "Mildam",
        alias: "Mildaam",
    },
    LocalityAlias {
        name: "Minnertsga",
        alias: "Minnertsgea",
    },
    LocalityAlias {
        name: "Mirns",
        alias: "Murns",
    },
    LocalityAlias {
        name: "Mitselwier",
        alias: "Metslawier",
    },
    LocalityAlias {
        name: "Moarre",
        alias: "Morra",
    },
    LocalityAlias {
        name: "Molkwerum",
        alias: "Molkwar",
    },
    LocalityAlias {
        name: "Mûnein",
        alias: "Molenend",
    },
    LocalityAlias {
        name: "Munnekeburen",
        alias: "Munnikebuorren",
    },
    LocalityAlias {
        name: "Munnekezijl",
        alias: "Muntsjesyl",
    },
    LocalityAlias {
        name: "Nieuwebrug",
        alias: "Nijbrêge",
    },
    LocalityAlias {
        name: "Nieuwehorne",
        alias: "Nijhoarne",
    },
    LocalityAlias {
        name: "Nieuweschoot",
        alias: "Nijskoat",
    },
    LocalityAlias {
        name: "Nij Altoenae",
        alias: "Nij Altena",
    },
    LocalityAlias {
        name: "Nijeberkoop",
        alias: "Nijeberkeap",
    },
    LocalityAlias {
        name: "Nijega",
        alias: "Nyegea",
    },
    LocalityAlias {
        name: "Nijeholtpade",
        alias: "Nijeholtpea",
    },
    LocalityAlias {
        name: "Nijeholtwolde",
        alias: "Nijeholtwâlde",
    },
    LocalityAlias {
        name: "Nijelamer",
        alias: "Nijlemmer",
    },
    LocalityAlias {
        name: "Nijemirdum",
        alias: "Nijemardum",
    },
    LocalityAlias {
        name: "Nijetrijne",
        alias: "Nijetrine",
    },
    LocalityAlias {
        name: "Nijewier",
        alias: "Niawier",
    },
    LocalityAlias {
        name: "Nijhuizum",
        alias: "Nijhuzum",
    },
    LocalityAlias {
        name: "Nijland",
        alias: "Nijlân",
    },
    LocalityAlias {
        name: "Noardburgum",
        alias: "Noordbergum",
    },
    LocalityAlias {
        name: "Noordwolde",
        alias: "Noardwâlde",
    },
    LocalityAlias {
        name: "Oentsjerk",
        alias: "Oenkerk",
    },
    LocalityAlias {
        name: "Offingawier",
        alias: "Offenwier",
    },
    LocalityAlias {
        name: "Oldeberkoop",
        alias: "Aldeberkeap",
    },
    LocalityAlias {
        name: "Oldeholtpade",
        alias: "Aldeholtpea",
    },
    LocalityAlias {
        name: "Oldeholtwolde",
        alias: "Aldeholtwâlde",
    },
    LocalityAlias {
        name: "Oldelamer",
        alias: "Aldlemmer",
    },
    LocalityAlias {
        name: "Oldeouwer",
        alias: "Aldeouwer",
    },
    LocalityAlias {
        name: "Oldetrijne",
        alias: "Aldetrine",
    },
    LocalityAlias {
        name: "Oosterbierum",
        alias: "Easterbierrum",
    },
    LocalityAlias {
        name: "Oosterend",
        alias: "Aasterein",
    },
    LocalityAlias {
        name: "Oosterstreek",
        alias: "Easterstreek",
    },
    LocalityAlias {
        name: "Oosterwolde",
        alias: "Easterwâlde",
    },
    LocalityAlias {
        name: "Oosterzee",
        alias: "Eastersee",
    },
    LocalityAlias {
        name: "Oosthem",
        alias: "Easthim",
    },
    LocalityAlias {
        name: "Oost-Vlieland",
        alias: "East-Flylân",
    },
    LocalityAlias {
        name: "Opeinde",
        alias: "De Pein",
    },
    LocalityAlias {
        name: "Oppenhuizen",
        alias: "Toppenhuzen",
    },
    LocalityAlias {
        name: "Oranjewoud",
        alias: "Oranjewâld",
    },
    LocalityAlias {
        name: "Oudebildtzijl",
        alias: "Aldebiltsyl",
    },
    LocalityAlias {
        name: "Oudega",
        alias: "Aldegea",
    },
    LocalityAlias {
        name: "Oudehaske",
        alias: "Aldehaske",
    },
    LocalityAlias {
        name: "Oudehorne",
        alias: "Aldhoarne",
    },
    LocalityAlias {
        name: "Oudemirdum",
        alias: "Aldemardum",
    },
    LocalityAlias {
        name: "Oudeschoot",
        alias: "Aldskoat",
    },
    LocalityAlias {
        name: "Ouwsterhaule",
        alias: "Ousterhaule",
    },
    LocalityAlias {
        name: "Ouwster-Nijega",
        alias: "Ousternijegea",
    },
    LocalityAlias {
        name: "Parrega",
        alias: "Parregea",
    },
    LocalityAlias {
        name: "Peazens",
        alias: "Paesens",
    },
    LocalityAlias {
        name: "Peperga",
        alias: "Pepergea",
    },
    LocalityAlias {
        name: "Pietersbierum",
        alias: "Pitersbierrum",
    },
    LocalityAlias {
        name: "Pingjum",
        alias: "Penjum",
    },
    LocalityAlias {
        name: "Poppenwier",
        alias: "Poppingawier",
    },
    LocalityAlias {
        name: "Raerd",
        alias: "Rauwerd",
    },
    LocalityAlias {
        name: "Ravenswoud",
        alias: "Ravenswâld",
    },
    LocalityAlias {
        name: "Readtsjerk",
        alias: "Roodkerk",
    },
    LocalityAlias {
        name: "Reahûs",
        alias: "Roodhuis",
    },
    LocalityAlias {
        name: "Reduzum",
        alias: "Roordahuizum",
    },
    LocalityAlias {
        name: "Ried",
        alias: "Rie",
    },
    LocalityAlias {
        name: "Rijs",
        alias: "Riis",
    },
    LocalityAlias {
        name: "Rinsumageast",
        alias: "Rinsumageest",
    },
    LocalityAlias {
        name: "Rohel",
        alias: "Reahel",
    },
    LocalityAlias {
        name: "Rottevalle",
        alias: "De Rottefalle",
    },
    LocalityAlias {
        name: "Ruigahuizen",
        alias: "Rûgehuzen",
    },
    LocalityAlias {
        name: "Ryptsjerk",
        alias: "Rijperkerk",
    },
    LocalityAlias {
        name: "Sandfirden",
        alias: "Sânfurd",
    },
    LocalityAlias {
        name: "Schalsum",
        alias: "Skalsum",
    },
    LocalityAlias {
        name: "Scharnegoutum",
        alias: "Skearnegoutum",
    },
    LocalityAlias {
        name: "Scharsterbrug",
        alias: "Skarsterbrêge",
    },
    LocalityAlias {
        name: "Scherpenzeel",
        alias: "Skerpenseel",
    },
    LocalityAlias {
        name: "Schettens",
        alias: "Skettens",
    },
    LocalityAlias {
        name: "Schiermonnikoog",
        alias: "Skiermûntseach",
    },
    LocalityAlias {
        name: "Schraard",
        alias: "Skraard",
    },
    LocalityAlias {
        name: "Sexbierum",
        alias: "Seisbierrum",
    },
    LocalityAlias {
        name: "Sibrandabuorren",
        alias: "Sijbrandaburen",
    },
    LocalityAlias {
        name: "Sibrandahûs",
        alias: "Sijbrandahuis",
    },
    LocalityAlias {
        name: "Siegerswoude",
        alias: "Sigerswâld",
    },
    LocalityAlias {
        name: "Sint Nicolaasga",
        alias: "Sint Nyk",
    },
    LocalityAlias {
        name: "Sintjohannesga",
        alias: "Sint Jânsgea",
    },
    LocalityAlias {
        name: "Skingen",
        alias: "Schingen",
    },
    LocalityAlias {
        name: "Slijkenburg",
        alias: "Slikenboarch",
    },
    LocalityAlias {
        name: "Sloten",
        alias: "Sleat",
    },
    LocalityAlias {
        name: "Smalle Ee",
        alias: "Smelle Ie",
    },
    LocalityAlias {
        name: "Smallebrugge",
        alias: "Smelbrêge",
    },
    LocalityAlias {
        name: "Snakkerburen",
        alias: "Snakkerbuorren",
    },
    LocalityAlias {
        name: "Sneek",
        alias: "Snits",
    },
    LocalityAlias {
        name: "Snikzwaag",
        alias: "Sniksweach",
    },
    LocalityAlias {
        name: "Sonnega",
        alias: "Sonnegea",
    },
    LocalityAlias {
        name: "Spanga",
        alias: "Spangea",
    },
    LocalityAlias {
        name: "St.-Annaparochie",
        alias: "Sint Anne",
    },
    LocalityAlias {
        name: "St.-Jacobiparochie",
        alias: "Sint Jabik",
    },
    LocalityAlias {
        name: "Stavoren",
        alias: "Starum",
    },
    LocalityAlias {
        name: "Striep",
        alias: "Stryp",
    },
    LocalityAlias {
        name: "Stroobos",
        alias: "Strobos",
    },
    LocalityAlias {
        name: "Sumar",
        alias: "Suameer",
    },
    LocalityAlias {
        name: "Surhuisterveen",
        alias: "Surhústerfean",
    },
    LocalityAlias {
        name: "Surhuizum",
        alias: "Surhuzum",
    },
    LocalityAlias {
        name: "Suwâld",
        alias: "Suawoude",
    },
    LocalityAlias {
        name: "Sweagerbosk",
        alias: "Zwagerbosch",
    },
    LocalityAlias {
        name: "Teerns",
        alias: "Tearns",
    },
    LocalityAlias {
        name: "Ter Idzard",
        alias: "Teridzert",
    },
    LocalityAlias {
        name: "Terband",
        alias: "Terbant",
    },
    LocalityAlias {
        name: "Terherne",
        alias: "Terhorne",
    },
    LocalityAlias {
        name: "Tersoal",
        alias: "Terzool",
    },
    LocalityAlias {
        name: "Tijnje",
        alias: "De Tynje",
    },
    LocalityAlias {
        name: "Tirns",
        alias: "Turns",
    },
    LocalityAlias {
        name: "Tjalhuizum",
        alias: "Tsjalhuzum",
    },
    LocalityAlias {
        name: "Tjalleberd",
        alias: "Tsjalbert",
    },
    LocalityAlias {
        name: "Tjerkgaast",
        alias: "Tsjerkgaast",
    },
    LocalityAlias {
        name: "Tjerkwerd",
        alias: "Tsjerkwert",
    },
    LocalityAlias {
        name: "Twijzel",
        alias: "Twizel",
    },
    LocalityAlias {
        name: "Twijzelerheide",
        alias: "Twizelerheide",
    },
    LocalityAlias {
        name: "Tytsjerk",
        alias: "Tietjerk",
    },
    LocalityAlias {
        name: "Tzum",
        alias: "Tsjom",
    },
    LocalityAlias {
        name: "Tzummarum",
        alias: "Tsjummearum",
    },
    LocalityAlias {
        name: "Uitwellingerga",
        alias: "Twellingea",
    },
    LocalityAlias {
        name: "Ureterp",
        alias: "Oerterp",
    },
    LocalityAlias {
        name: "Vegelinsoord",
        alias: "Vegelinsoard",
    },
    LocalityAlias {
        name: "Vinkega",
        alias: "Finkegea",
    },
    LocalityAlias {
        name: "Vrouwenparochie",
        alias: "Froubuorren",
    },
    LocalityAlias {
        name: "Waaxens",
        alias: "Waaksens",
    },
    LocalityAlias {
        name: "Wâlterswâld",
        alias: "Wouterswoude",
    },
    LocalityAlias {
        name: "Wânswert",
        alias: "Wanswerd",
    },
    LocalityAlias {
        name: "Warfstermolen",
        alias: "Warfstermûne",
    },
    LocalityAlias {
        name: "Warten",
        alias: "Wartena",
    },
    LocalityAlias {
        name: "Waskemeer",
        alias: "Waskemar",
    },
    LocalityAlias {
        name: "Wergea",
        alias: "Warga",
    },
    LocalityAlias {
        name: "Westergeast",
        alias: "Westergeest",
    },
    LocalityAlias {
        name: "Westhem",
        alias: "Westhim",
    },
    LocalityAlias {
        name: "Westhoek",
        alias: "De Westhoek",
    },
    LocalityAlias {
        name: "West-Terschelling",
        alias: "West-Skylge",
    },
    LocalityAlias {
        name: "Wijckel",
        alias: "Wikel",
    },
    LocalityAlias {
        name: "Wijnaldum",
        alias: "Winaam",
    },
    LocalityAlias {
        name: "Wijnjewoude",
        alias: "Wynjewâld",
    },
    LocalityAlias {
        name: "Wirdum",
        alias: "Wurdum",
    },
    LocalityAlias {
        name: "Witmarsum",
        alias: "Wytmarsum",
    },
    LocalityAlias {
        name: "Wiuwert",
        alias: "Wieuwerd",
    },
    LocalityAlias {
        name: "Wjelsryp",
        alias: "Welsrijp",
    },
    LocalityAlias {
        name: "Wolvega",
        alias: "Wolvegea",
    },
    LocalityAlias {
        name: "Wons",
        alias: "Wûns",
    },
    LocalityAlias {
        name: "Workum",
        alias: "Warkum",
    },
    LocalityAlias {
        name: "Woudsend",
        alias: "Wâldsein",
    },
    LocalityAlias {
        name: "Wyns",
        alias: "Wijns",
    },
    LocalityAlias {
        name: "Wytgaard",
        alias: "Wijtgaard",
    },
    LocalityAlias {
        name: "Ypecolsga",
        alias: "Ypekolsgea",
    },
    LocalityAlias {
        name: "Ysbrechtum",
        alias: "IJsbrechtum",
    },
    LocalityAlias {
        name: "Zandhuizen",
        alias: "Sânhuzen",
    },
    LocalityAlias {
        name: "Zurich",
        alias: "Surch",
    },
    LocalityAlias {
        name: "Zweins",
        alias: "Sweins",
    },
];
