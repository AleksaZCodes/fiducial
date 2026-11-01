# Odgovor recenzentu — IEEESTEC 2026

**Rad:** Fiducial: Keeping Multidisciplinary Products Built with AI Agents Consistent
(ranije: *Fiducial: Build Any Robust Multidisciplinary Product with AI Agents*)

Zahvaljujem recenzentu na pažljivom čitanju i konkretnim predlozima. Ispod je
odgovor na svaku primedbu i opis izmene u radu.

**1. Evaluacija na istom skupu grešaka.** Slažem se: rezultat 20/20 dobijen je
na skupu koji je korišćen za unapređenje platforme. Zato su dodata još tri
skupa od po 20 grešaka, svaki sačuvan u repozitorijumu pre prvog pokretanja:

- drugi skup napisao je autor nakon što je verzija 0.9.11 zamrznuta;
- treći i četvrti napisali su zasebni agenti koji su čitali samo fajlove dva
  proizvoda, a ne platformu, njena pravila ni istoriju.

Pri prvom pokretanju provere su otkrile 15, 16 i 14 od 20 grešaka iz ta tri
skupa.

Sve promaknute greške spadaju u pet klasa:

- vrednost iz deklaracije ponovo upisana u kod;
- uklonjena ili zaobiđena provera u CI-ju;
- polje koje prima bilo koju vrednost ispravnog oblika;
- pravilo spolja (zakon, datasheet, mogućnosti proizvođača);
- generator koji čita nedeklarisan fajl.

Popravke jedna po jedna dovele su drugi skup do 20/20, ali treći je i dalje
ostao na 16. Zato je svaka klasa zatvorena jednim mehanizmom. Verzija 0.10.1
otkriva sve greške iz prva tri skupa i 19 od 20 iz četvrtog. Preostala greška
je značenjska: par boja preimenovan u „veliki tekst“, a nigde nije deklarisano
koji je tekst veliki.

Rezultati prvih pokretanja i rezultati posle popravki navedeni su odvojeno u
apstraktu, evaluaciji (Tabela II) i zaključku.

**2. Poređenje sa postojećim pristupima i standardnim CI procesom.** Sva četiri
skupa pokrenuta su i kroz uobičajeni CI istog projekta — iste kompajlere,
provere tipova i testove, bez dva koraka koje dodaje Fiducial. Takav CI otkrio
je 1 od 20 grešaka iz prvog skupa i nijednu od 60 iz ostala tri, jer ništa u
njemu ne upoređuje jedan artefakt sa drugim. Rezultat je u Tabeli II, a Tabela I
i poglavlje II porede Fiducial sa model-based inženjeringom, ECAD–MCAD alatima,
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
Skripta testa, sva četiri skupa grešaka i svi rezultati dostupni su u repozitorijumu.

S poštovanjem,
Aleksa Zdravković
