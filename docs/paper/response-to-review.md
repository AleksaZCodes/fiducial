# Odgovor recenzentu — IEEESTEC 2026

**Rad:** Fiducial: Keeping Multidisciplinary Products Built with AI Agents Consistent
(ranije: *Fiducial: Build Any Robust Multidisciplinary Product with AI Agents*)

Zahvaljujem recenzentu na pažljivom čitanju i konkretnim predlozima. Ispod je
odgovor na svaku primedbu i opis izmene u radu.

**1. Evaluacija na istom skupu grešaka.** Slažem se: rezultat 20/20 dobijen je
na skupu koji je korišćen za unapređenje platforme. Zato je dodat nezavisan
(held-out) skup od 20 novih grešaka, napisan nakon što je verzija 0.9.11
zamrznuta, sačuvan u repozitorijumu pre prvog pokretanja i prijavljen bez
ikakvih izmena platforme. Na njemu su provere otkrile 15 od 20 grešaka. Rad
navodi i koje su greške promaknute (stranica deklarisana dvaput, nepostojeći
datum objave, deblja ploča, vreme čekanja senzora upisano kao broj i ručno
poređenje vlažnosti sa opsegom na stranici). Oba rezultata (15/20 → 20/20 na
prvom skupu i 15/20 na nezavisnom) sada su jasno navedena u apstraktu, u
evaluaciji (Tabela II) i u zaključku.

**2. Poređenje sa postojećim pristupima i standardnim CI procesom.** Oba skupa
su pokrenuta i kroz uobičajeni CI istog projekta — iste kompajlere, provere
tipova i testove, bez dva koraka koje dodaje Fiducial. Takav CI otkrio je 1
od 20 grešaka iz prvog skupa i 0 od 20 iz nezavisnog, jer ništa u njemu ne
upoređuje jedan artefakt sa drugim. Rezultat je u Tabeli II, a Tabela I i
poglavlje II porede Fiducial sa model-based inženjeringom, ECAD–MCAD alatima,
alatima „dizajn kao kod“, jezicima za opis interfejsa, build sistemima i
kontekst fajlovima za agente.

**3. Preširok naslov i tvrdnje.** Naslov je promenjen u *Fiducial: Keeping
Multidisciplinary Products Built with AI Agents Consistent*, a formulacije
poput „guarantees“ uklonjene su ili ublažene (npr. „many are caught“ umesto
tvrdnje da se greške uvek otkrivaju).

**4. Konzistentnost, a ne tačnost.** Dodat je poseban pasus u poglavlju III
(„The rules check agreement, not truth“): pogrešno deklarisana činjenica
prenosi se dosledno u sve artefakte, pa proizvod može biti dosledno pogrešan;
o tačnosti deklaracije i dalje odlučuju čovek, datasheet i merenje. Isto je
naglašeno u apstraktu i zaključku.

**5. Srodni radovi.** Poglavlje II prošireno je referencama iz model-based
inženjeringa (Brambilla i sar.; Friedenthal i sar., SysML; Nuseibeh i sar.)
i o pouzdanosti koda koji generišu veliki jezički modeli (Chen i sar.;
Liu i sar.; Pearce i sar.; Jimenez i sar.; Huang i sar.).

Rad je ostao u okviru od četiri strane; da bi se napravio prostor, uklonjena
je slika sa dve poruke o greškama, a uvod i opis primera su skraćeni.
Skripta testa, oba skupa grešaka i svi rezultati dostupni su u repozitorijumu.

S poštovanjem,
Aleksa Zdravković
