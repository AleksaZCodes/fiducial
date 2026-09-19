//! Pure rendering: `[legal]` + `[brand]` + `[i18n]` in, TypeScript out.
//!
//! The `fid-legal` executor in `commands/derive.rs` owns the I/O — reading
//! `fiducial.toml` and writing outputs. Everything here is a pure function of
//! the declared facts, so it is testable without a scaffolded product, the
//! same split `brand.rs` uses for its rendering.
//!
//! **GDPR compliance is a legal state, not a code state.** The generated file
//! carries a checklist comment at the top naming every decision a human must
//! still make. `render_legal_ts` does not grant compliance; it generates a
//! typed starting point that a lawyer must review.

use anyhow::Result;

use crate::config::{Brand, Legal};

/// Pages included per jurisdiction.
///
/// EU and UK include `imprint` — a legal disclosure requirement in Germany and
/// other EU/UK jurisdictions. US drops it (no equivalent requirement). RS
/// (Serbia) has no Impressum equivalent, so it takes the minimal set plus an
/// accessibility statement, which Serbian public-sector procurement commonly
/// asks for. `other` produces the minimal set every product needs.
fn pages_for_jurisdiction(jurisdiction: &str) -> &'static [&'static str] {
    match jurisdiction {
        "EU" | "UK" => &["privacy", "terms", "cookies", "imprint", "accessibility"],
        "US" => &["privacy", "terms", "cookies", "accessibility"],
        "RS" => &["privacy", "terms", "cookies", "accessibility"],
        _ => &["privacy", "terms", "cookies"],
    }
}

/// Languages this pipeline can render legal copy in.
///
/// A locale outside this list still gets a catalog entry — omitting one would
/// break the `Record<Locale, …>` contract — but the entry is marked untranslated
/// rather than silently served as English. See `render_legal_ts`.
pub const TRANSLATED_LANGS: &[&str] = &["en", "sr"];

/// `sr-Latn-RS` → `sr`. Legal copy varies by language, not by region.
fn lang_of(locale: &str) -> &str {
    locale.split(['-', '_']).next().unwrap_or(locale)
}

/// True when legal copy exists in this locale's language.
pub fn is_translated(locale: &str) -> bool {
    TRANSLATED_LANGS.contains(&lang_of(locale))
}

/// Human-readable jurisdiction name, per language.
///
/// The enum token is never interpolated into prose: "Jurisdiction: other" is
/// not a sentence, and "governed by the laws of other" is worse. An undeclared
/// jurisdiction renders as a bracketed placeholder so that it is visibly unfit
/// to publish rather than plausibly wrong.
fn jurisdiction_name(jurisdiction: &str, lang: &str) -> &'static str {
    match (jurisdiction, lang) {
        ("EU", "sr") => "Evropska unija",
        ("EU", _) => "the European Union",
        ("UK", "sr") => "Ujedinjeno Kraljevstvo",
        ("UK", _) => "the United Kingdom",
        ("US", "sr") => "Sjedinjene Američke Države",
        ("US", _) => "the United States",
        ("RS", "sr") => "Srbija",
        ("RS", _) => "Serbia",
        (_, "sr") => "[jurisdikcija nije navedena]",
        _ => "[jurisdiction not declared]",
    }
}

/// The supervisory authority a reader complains to, named and addressed.
///
/// GDPR Art. 13(2)(d) requires telling the data subject they may lodge a
/// complaint with a supervisory authority. "Contact the relevant national
/// enforcement body for your jurisdiction", which this used to say, discharges
/// nothing: the whole point of the disclosure is that the reader should not
/// have to go and find out who that is.
fn supervisory_authority(jurisdiction: &str, lang: &str) -> &'static str {
    match (jurisdiction, lang) {
        ("RS", "sr") => {
            "Poverenik za informacije od javnog značaja i zaštitu podataka o ličnosti, \
             Bulevar kralja Aleksandra 15, 11120 Beograd, office@poverenik.rs, \
             www.poverenik.rs"
        }
        ("RS", _) => {
            "the Commissioner for Information of Public Importance and Personal Data \
             Protection, Bulevar kralja Aleksandra 15, 11120 Belgrade, Serbia, \
             office@poverenik.rs, www.poverenik.rs"
        }
        ("UK", "sr") => {
            "Information Commissioner's Office (ICO), Wycliffe House, Water Lane, \
             Wilmslow SK9 5AF, Ujedinjeno Kraljevstvo, ico.org.uk"
        }
        ("UK", _) => {
            "the Information Commissioner's Office (ICO), Wycliffe House, Water Lane, \
             Wilmslow SK9 5AF, United Kingdom, ico.org.uk"
        }
        ("EU", "sr") => {
            "nadzorni organ za zaštitu podataka države članice EU u kojoj imate \
             uobičajeno boravište, u kojoj radite ili u kojoj je došlo do navodne \
             povrede (spisak: edpb.europa.eu/about-edpb/about-edpb/members)"
        }
        ("EU", _) => {
            "the data protection supervisory authority of the EU member state where you \
             habitually reside, where you work, or where the alleged infringement took \
             place (list: edpb.europa.eu/about-edpb/about-edpb/members)"
        }
        (_, "sr") => "[nadzorni organ nije određen — jurisdikcija nije navedena]",
        _ => "[supervisory authority undetermined — jurisdiction not declared]",
    }
}

/// The data-protection statute the pages are written against, named.
///
/// Serbia's ZZPL is a close transposition of the GDPR, which is why one set of
/// templates can serve both — but a Serbian reader is owed the Serbian citation,
/// not a reference to a regulation that does not directly bind the controller.
fn statute_name(jurisdiction: &str, lang: &str) -> &'static str {
    match (jurisdiction, lang) {
        ("RS", "sr") => {
            "Zakon o zaštiti podataka o ličnosti („Službeni glasnik RS“, br. 87/2018), \
             koji je usklađen sa Opštom uredbom EU o zaštiti podataka (GDPR)"
        }
        ("RS", _) => {
            "the Serbian Personal Data Protection Act (Zakon o zaštiti podataka o \
             ličnosti, Official Gazette RS 87/2018), which transposes the EU General \
             Data Protection Regulation (GDPR)"
        }
        ("UK", "sr") => "UK GDPR i Data Protection Act 2018",
        ("UK", _) => "the UK GDPR and the Data Protection Act 2018",
        ("EU", "sr") => "Opšta uredba o zaštiti podataka (GDPR, Uredba (EU) 2016/679)",
        ("EU", _) => "the General Data Protection Regulation (GDPR, Regulation (EU) 2016/679)",
        (_, "sr") => "važeći propisi o zaštiti podataka o ličnosti",
        _ => "applicable data protection law",
    }
}

/// Everything a page template may interpolate.
///
/// A struct rather than eight positional `&str`s: the previous signature was
/// four, adding four more is where a caller starts passing `domain` where
/// `email` goes and nothing catches it, because they are all strings.
pub struct Facts<'a> {
    pub legal_name: &'a str,
    pub domain: &'a str,
    /// Human-readable jurisdiction, already resolved for the language.
    pub jurisdiction: &'a str,
    pub email: &'a str,
    pub authority: &'a str,
    pub statute: &'a str,
    /// Rendered "last reviewed" line, or a loud marker when undeclared.
    pub last_updated: String,
    /// Rendered registration/tax identifier lines, or empty.
    pub identifiers: String,
    /// Rendered postal address, or empty.
    pub postal_address: String,
}

/// The "last reviewed" line, or a marker saying nobody has.
///
/// The undeclared case is deliberately ugly. A legal page with no review date
/// looks finished, and a page that looks finished is a page nobody reviews —
/// so the absence is rendered into the reader's view rather than left as a
/// silent gap. It is meant to be embarrassing enough to fix.
fn last_updated_line(date: Option<&str>, lang: &str) -> String {
    match (date, lang) {
        (Some(d), "sr") => format!("*Poslednja provera: {d}.*"),
        (Some(d), _) => format!("*Last reviewed: {d}.*"),
        (None, "sr") => "*Ovu stranicu nije proverilo stručno lice. Datum provere nije unet u \
             `[legal] last_updated`.*"
            .to_string(),
        (None, _) => "*This page has not been reviewed by a qualified professional. No review \
             date is declared in `[legal] last_updated`.*"
            .to_string(),
    }
}

/// Registration and tax numbers, each on its own line, or nothing.
///
/// Nothing is the right output for an unincorporated project. Serbia's Zakon o
/// elektronskoj trgovini requires these of a trader; someone who is not one has
/// no number to publish, and a placeholder that looks like a number is worse
/// than an absence.
fn identifier_lines(legal: &Legal, lang: &str) -> String {
    let mut s = String::new();
    if let Some(reg) = legal
        .registration_number
        .as_deref()
        .filter(|v| !v.is_empty())
    {
        let label = if lang == "sr" {
            "Matični broj"
        } else {
            "Registration number"
        };
        s.push_str(&format!("{label}: {reg}\n"));
    }
    if let Some(tax) = legal.tax_number.as_deref().filter(|v| !v.is_empty()) {
        let label = if lang == "sr" { "PIB" } else { "Tax number" };
        s.push_str(&format!("{label}: {tax}\n"));
    }
    s
}

/// The postal address as its own line, or nothing.
fn address_line(address: Option<&str>) -> String {
    match address.filter(|v| !v.is_empty()) {
        Some(a) => format!("{a}\n"),
        None => String::new(),
    }
}

fn substitute(template: &str, f: &Facts<'_>) -> String {
    template
        .replace("{legal_name}", f.legal_name)
        .replace("{domain}", f.domain)
        .replace("{jurisdiction}", f.jurisdiction)
        .replace("{email}", f.email)
        .replace("{authority}", f.authority)
        .replace("{statute}", f.statute)
        .replace("{last_updated}", &f.last_updated)
        .replace("{identifiers}", &f.identifiers)
        .replace("{postal_address}", &f.postal_address)
}

/// Title and body template for each page, in the given language, before
/// substitution.
///
/// Every arm returns owned strings so that translated and formatted variants
/// can share one signature.
fn page_content(page: &str, lang: &str, categories: &[&str]) -> (String, String) {
    let cookie_list = categories.join(", ");
    match lang {
        "sr" => page_content_sr(page, categories, &cookie_list),
        _ => page_content_en(page, categories, &cookie_list),
    }
}

fn page_content_en(page: &str, categories: &[&str], cookie_list: &str) -> (String, String) {
    match page {
        "privacy" => (
            "Privacy Policy".to_string(),
            "This policy explains what {legal_name} does with personal data collected \
             through {domain}, and what you can require of us. It is written against \
             {statute}.\n\n\
             {last_updated}\n\n\
             ## Who is responsible\n\n\
             The controller is {legal_name}.\n\
             {postal_address}\
             {identifiers}\
             Contact for any data protection matter: {email}.\n\n\
             ## What is collected, and on what legal basis\n\n\
             **Contact details you send us.** If you write to {email}, we hold your \
             address and whatever you put in the message for as long as the exchange \
             is live and for two years afterwards, so that a later reply has its \
             context. Legal basis: our legitimate interest in answering enquiries \
             addressed to us (GDPR Art. 6(1)(f)), or steps taken at your request \
             before entering an agreement (Art. 6(1)(b)).\n\n\
             **Server request logs.** Our hosting provider records the IP address, \
             user agent, requested path and timestamp of each request. These exist to \
             keep the site available and to detect abuse. Legal basis: legitimate \
             interest in the security and availability of the service (Art. 6(1)(f)). \
             Retention is the provider's log window, typically under 30 days, and we \
             do not build profiles from them.\n\n\
             **Cookies.** See the Cookie Policy. Where a cookie is not strictly \
             necessary, it is set only after you consent, and consent is the legal \
             basis for it (Art. 6(1)(a)).\n\n\
             We do not buy personal data, we do not collect it from third parties, and \
             we do not knowingly collect anything from children.\n\n\
             ## Who else sees it\n\n\
             Only processors acting on our written instructions, principally our \
             hosting and email providers. They may not use the data for their own \
             purposes. We do not sell personal data and we do not share it for \
             advertising.\n\n\
             ## Transfers outside the country\n\n\
             Our infrastructure providers may process data on servers outside \
             {jurisdiction}. Where that happens, the transfer relies on an adequacy \
             decision or on standard contractual clauses. Write to {email} for a copy \
             of the safeguards that apply to a specific transfer.\n\n\
             ## Automated decisions\n\n\
             None. We do not carry out profiling or automated decision-making that \
             produces legal effects for you.\n\n\
             ## Your rights\n\n\
             You may ask us to give you a copy of your data, correct it, delete it, \
             restrict what we do with it, or hand it over in a portable form. You may \
             object to processing we base on legitimate interest. Where processing \
             rests on your consent, you may withdraw it at any time, and doing so does \
             not affect what was lawful before you withdrew it.\n\n\
             Write to {email}. We answer within 30 days. We will not charge you for \
             this and we will not ask you to justify the request.\n\n\
             ## Complaints\n\n\
             If our answer does not satisfy you, you can complain to {authority}. You \
             can do that without coming to us first.\n\n\
             ## Changes\n\n\
             When this policy changes materially, the new version is published here \
             before it takes effect, and the review date above changes with it."
                .to_string(),
        ),
        "terms" => (
            "Terms of Service".to_string(),
            "These terms govern your use of {domain}, operated by {legal_name}. Using \
             the site means accepting them.\n\n\
             {last_updated}\n\n\
             ## Who operates this site\n\n\
             {legal_name}.\n\
             {postal_address}\
             {identifiers}\
             Contact: {email}.\n\n\
             ## What this site is\n\n\
             An information site. It is not an offer, a quotation, or a commitment to \
             supply anything, and nothing on it forms a contract by itself. Where the \
             site describes work that is planned rather than built, it says so, and \
             you should read those descriptions as intentions rather than as available \
             capability.\n\n\
             ## What you may do with it\n\n\
             Read it, quote it with attribution, and link to it. Do not use it to \
             break the law, do not attempt to disrupt or gain unauthorised access to \
             it, and do not scrape it in a way that degrades it for other readers.\n\n\
             ## Accuracy\n\n\
             We keep the site as accurate as we can and correct errors when we find \
             them. We do not warrant that it is complete or current, and technical \
             detail here may be superseded without notice.\n\n\
             ## Intellectual property\n\n\
             Text, drawings and code on this site belong to {legal_name} or to the \
             people credited beside them. Third-party material is used under its own \
             licence, which the relevant page names.\n\n\
             ## Liability\n\n\
             To the extent the law allows, {legal_name} is not liable for indirect or \
             consequential loss arising from use of this site. Nothing here limits \
             liability for death, personal injury, or anything else that cannot \
             lawfully be limited. If you are a consumer, your statutory rights are \
             unaffected by these terms.\n\n\
             ## Governing law\n\n\
             The law of {jurisdiction} applies, and its courts have jurisdiction. If \
             you are a consumer resident elsewhere, this does not deprive you of the \
             protection of the mandatory rules of the country you live in.\n\n\
             ## Changes\n\n\
             We may amend these terms. The version published here when you use the \
             site is the one that applies."
                .to_string(),
        ),
        "cookies" => (
            "Cookie Policy".to_string(),
            format!(
                "What {{legal_name}} stores on your device when you visit {{domain}}, and \
                 what you can do about it.\n\n\
                 {{last_updated}}\n\n\
                 ## Consent\n\n\
                 Only strictly necessary cookies are set before you choose. Everything \
                 else waits for your consent, and you can withdraw that consent at any \
                 time from the cookie settings link in the footer. Withdrawing is as \
                 easy as giving, which is a requirement rather than a courtesy.\n\n\
                 Declining non-necessary cookies costs you nothing: the site works the \
                 same either way.\n\n\
                 ## Categories in use\n\n\
                 {cookie_list}.\n\n\
                 **Strictly necessary.** Required for the site to work at all, \
                 including remembering your cookie choice so that you are not asked \
                 again on every page. These have no alternative and are set without \
                 consent, which the law permits precisely because they are \
                 unavoidable.\n\n\
                 {analytics}\
                 {marketing}\
                 {functional}\
                 ## Your browser\n\n\
                 Every browser can block or delete cookies, and blocking ours will not \
                 break the site. Doing so also removes the record of your cookie \
                 choice, so you will be asked again.\n\n\
                 ## Questions\n\n\
                 {{email}}.",
                analytics = if categories.contains(&"analytics") {
                    "**Analytics.** Counts of visits and pages, used to see which parts \
                     of the site are read. Set only with your consent, and withdrawable \
                     at any time.\n\n"
                } else {
                    ""
                },
                marketing = if categories.contains(&"marketing") {
                    "**Marketing.** Used to measure campaigns and to show relevant \
                     advertising. Set only with your consent, and withdrawable at any \
                     time.\n\n"
                } else {
                    ""
                },
                functional = if categories.contains(&"functional") {
                    "**Functional.** Remembers preferences such as language or layout. \
                     Set only with your consent, and withdrawable at any time.\n\n"
                } else {
                    ""
                },
            ),
        ),
        "imprint" => (
            "Imprint".to_string(),
            "The disclosure required of the operator of this site under the law of \
             {jurisdiction}.\n\n\
             {last_updated}\n\n\
             ## Responsible for this site\n\n\
             {legal_name}\n\
             {postal_address}\
             {identifiers}\
             {domain}\n\n\
             Contact: {email}\n\n\
             ## Dispute resolution\n\n\
             The European Commission runs an online dispute resolution platform at \
             https://ec.europa.eu/consumers/odr. We are not obliged to use it and are \
             willing to settle disputes out of court."
                .to_string(),
        ),
        "accessibility" => (
            "Accessibility Statement".to_string(),
            "{legal_name} intends {domain} to be usable by everyone, including people \
             using a screen reader, a keyboard alone, magnification, or reduced \
             motion.\n\n\
             {last_updated}\n\n\
             ## Standard\n\n\
             We target WCAG 2.2 Level AA, the standard referenced by EN 301 549 and by \
             public-sector accessibility rules across Europe.\n\n\
             ## Status\n\n\
             Partially conformant, which means most of the site meets the standard and \
             some of it has not been audited by anyone but us. We have not commissioned \
             an independent audit. Saying \"fully conformant\" without one would be a \
             claim we cannot support.\n\n\
             What we have done deliberately: text contrast is measured rather than \
             eyeballed and the build fails when a pair falls short; every animation \
             respects prefers-reduced-motion; the page works at 400 percent zoom and \
             from the keyboard alone; and there is a skip link to the main content.\n\n\
             ## Tell us where it fails\n\n\
             If something blocks you, write to {email} with the page and what happened. \
             We reply within five working days, and we would rather hear about a small \
             problem than have you work around it.\n\n\
             ## If we do not fix it\n\n\
             You can raise the matter with {authority}, or with the body responsible \
             for accessibility enforcement in {jurisdiction}."
                .to_string(),
        ),
        _ => (String::new(), String::new()),
    }
}

fn page_content_sr(page: &str, categories: &[&str], cookie_list: &str) -> (String, String) {
    match page {
        "privacy" => (
            "Politika privatnosti".to_string(),
            "Ovde piše šta {legal_name} radi sa podacima o ličnosti prikupljenim preko \
             {domain} i šta možete da tražite od nas. Dokument je pisan prema propisu: \
             {statute}.\n\n\
             {last_updated}\n\n\
             ## Ko je odgovoran\n\n\
             Rukovalac je {legal_name}.\n\
             {postal_address}\
             {identifiers}\
             Kontakt za sva pitanja zaštite podataka: {email}.\n\n\
             ## Šta se prikuplja i po kom pravnom osnovu\n\n\
             **Podaci koje nam sami pošaljete.** Ako nam pišete na {email}, čuvamo vašu \
             adresu i sadržaj poruke dok traje prepiska i još dve godine posle nje, da \
             bi kasniji odgovor imao kontekst. Pravni osnov: naš legitimni interes da \
             odgovorimo na upit upućen nama (član 12 stav 1 tačka 6) ZZPL, odnosno \
             član 6(1)(f) GDPR), ili radnje preduzete na vaš zahtev pre zaključenja \
             ugovora.\n\n\
             **Zapisi o pristupu serveru.** Naš pružalac usluge hostinga beleži IP \
             adresu, tip pregledača, traženu adresu i vreme svakog zahteva. Ti zapisi \
             postoje da bi sajt bio dostupan i da bi se otkrila zloupotreba. Pravni \
             osnov: legitimni interes u bezbednosti i dostupnosti usluge. Čuvaju se \
             onoliko koliko ih pružalac čuva, po pravilu kraće od 30 dana, i od njih ne \
             pravimo profile.\n\n\
             **Kolačići.** Videti Politiku kolačića. Kolačić koji nije neophodan \
             postavlja se tek pošto pristanete, i pristanak je pravni osnov za \
             njega.\n\n\
             Ne kupujemo podatke o ličnosti, ne pribavljamo ih od trećih lica i svesno \
             ne prikupljamo podatke o deci.\n\n\
             ## Ko još ima pristup\n\n\
             Samo obrađivači koji postupaju po našem pisanom nalogu, pre svega pružaoci \
             usluga hostinga i elektronske pošte. Oni te podatke ne smeju koristiti za \
             sopstvene svrhe. Podatke o ličnosti ne prodajemo i ne ustupamo ih za \
             oglašavanje.\n\n\
             ## Iznošenje podataka iz zemlje\n\n\
             Naši pružaoci infrastrukture mogu obrađivati podatke na serverima van \
             države {jurisdiction}. Kada se to dešava, iznošenje počiva na odluci o \
             primerenom nivou zaštite ili na standardnim ugovornim klauzulama. Pišite \
             na {email} za primerak zaštitnih mera koje važe za konkretno \
             iznošenje.\n\n\
             ## Automatizovano odlučivanje\n\n\
             Nema ga. Ne sprovodimo profilisanje ni automatizovano odlučivanje koje \
             proizvodi pravne posledice po vas.\n\n\
             ## Vaša prava\n\n\
             Možete tražiti kopiju svojih podataka, njihovu ispravku, brisanje, \
             ograničenje obrade ili prenos u prenosivom obliku. Možete se usprotiviti \
             obradi zasnovanoj na legitimnom interesu. Kada se obrada zasniva na vašem \
             pristanku, pristanak možete opozvati u svakom trenutku, a opoziv ne utiče \
             na zakonitost obrade pre opoziva.\n\n\
             Pišite na {email}. Odgovaramo u roku od 30 dana. Ne naplaćujemo to i ne \
             tražimo od vas da obrazložite zahtev.\n\n\
             ## Pritužba\n\n\
             Ako niste zadovoljni našim odgovorom, možete podneti pritužbu: {authority}. \
             To možete i bez prethodnog obraćanja nama.\n\n\
             ## Izmene\n\n\
             Kada se ova politika bitno promeni, nova verzija se objavljuje ovde pre \
             nego što počne da važi, a datum poslednje provere se menja sa njom."
                .to_string(),
        ),
        "terms" => (
            "Uslovi korišćenja".to_string(),
            "Ovi uslovi uređuju korišćenje sajta {domain}, koji vodi {legal_name}. \
             Korišćenjem sajta prihvatate ih.\n\n\
             {last_updated}\n\n\
             ## Ko vodi ovaj sajt\n\n\
             {legal_name}.\n\
             {postal_address}\
             {identifiers}\
             Kontakt: {email}.\n\n\
             ## Šta je ovaj sajt\n\n\
             Informativni sajt. Nije ponuda, nije predračun i nije obavezivanje da se \
             bilo šta isporuči, i ništa na njemu samo po sebi ne zaključuje ugovor. \
             Tamo gde sajt opisuje ono što je planirano a nije napravljeno, to i piše, \
             pa takve opise treba čitati kao nameru, a ne kao raspoloživu \
             mogućnost.\n\n\
             ## Šta smete\n\n\
             Da ga čitate, citirate uz navođenje izvora i povezujete na njega. Nemojte \
             ga koristiti za kršenje propisa, nemojte pokušavati da ga onesposobite ili \
             da mu neovlašćeno pristupite i nemojte ga preuzimati automatski na način \
             koji ga usporava drugim čitaocima.\n\n\
             ## Tačnost\n\n\
             Trudimo se da sadržaj bude tačan i ispravljamo greške kada ih uočimo. Ne \
             garantujemo potpunost ni ažurnost, a tehnički podaci ovde mogu biti \
             prevaziđeni bez najave.\n\n\
             ## Autorska prava\n\n\
             Tekst, crteži i kod na ovom sajtu pripadaju licu {legal_name} ili licima \
             navedenim uz njih. Materijal trećih lica koristi se pod sopstvenom \
             licencom, koja je navedena na odgovarajućoj stranici.\n\n\
             ## Odgovornost\n\n\
             U meri u kojoj to zakon dopušta, {legal_name} ne odgovara za posrednu \
             štetu nastalu korišćenjem ovog sajta. Ništa ovde ne isključuje odgovornost \
             za smrt, telesnu povredu ni bilo šta drugo što se po zakonu ne može \
             isključiti. Ako ste potrošač, ovi uslovi ne diraju u vaša zakonska prava \
             po Zakonu o zaštiti potrošača.\n\n\
             ## Merodavno pravo\n\n\
             Primenjuje se pravo države {jurisdiction} i nadležni su njeni sudovi. Ako \
             ste potrošač sa prebivalištem u drugoj državi, ovo vas ne lišava zaštite \
             koju vam daju prinudni propisi države u kojoj živite.\n\n\
             ## Izmene\n\n\
             Uslove možemo menjati. Važi verzija objavljena ovde u trenutku kada \
             koristite sajt."
                .to_string(),
        ),
        "cookies" => (
            "Politika kolačića".to_string(),
            format!(
                "Šta {{legal_name}} čuva na vašem uređaju kada posetite {{domain}} i šta \
                 povodom toga možete da uradite.\n\n\
                 {{last_updated}}\n\n\
                 ## Pristanak\n\n\
                 Pre nego što izaberete, postavljaju se samo neophodni kolačići. Sve \
                 ostalo čeka vaš pristanak, a pristanak možete opozvati u svakom \
                 trenutku preko veze za podešavanje kolačića u podnožju stranice. \
                 Opoziv je jednako lak kao i davanje pristanka, što je zahtev propisa, \
                 a ne ljubaznost.\n\n\
                 Odbijanje kolačića koji nisu neophodni ne košta vas ništa: sajt radi \
                 isto u oba slučaja.\n\n\
                 ## Kategorije u upotrebi\n\n\
                 {cookie_list}.\n\n\
                 **Neophodni.** Potrebni su da bi sajt uopšte radio, uključujući \
                 pamćenje vašeg izbora o kolačićima da vas ne bismo pitali ponovo na \
                 svakoj stranici. Za njih ne postoji alternativa i postavljaju se bez \
                 pristanka, što zakon dopušta upravo zato što su neizbežni.\n\n\
                 {analytics}\
                 {marketing}\
                 {functional}\
                 ## Vaš pregledač\n\n\
                 Svaki pregledač može da blokira ili obriše kolačiće, i blokiranje \
                 naših neće pokvariti sajt. Time se briše i zapis o vašem izboru, pa \
                 ćemo vas pitati ponovo.\n\n\
                 ## Pitanja\n\n\
                 {{email}}.",
                analytics = if categories.contains(&"analytics") {
                    "**Analitika.** Broj poseta i pregleda stranica, da bismo videli \
                     koji delovi sajta se čitaju. Postavljaju se samo uz vaš pristanak \
                     i mogu se opozvati u svakom trenutku.\n\n"
                } else {
                    ""
                },
                marketing = if categories.contains(&"marketing") {
                    "**Marketing.** Služe za merenje kampanja i prikazivanje relevantnih \
                     oglasa. Postavljaju se samo uz vaš pristanak i mogu se opozvati u \
                     svakom trenutku.\n\n"
                } else {
                    ""
                },
                functional = if categories.contains(&"functional") {
                    "**Funkcionalni.** Pamte podešavanja kao što su jezik ili raspored. \
                     Postavljaju se samo uz vaš pristanak i mogu se opozvati u svakom \
                     trenutku.\n\n"
                } else {
                    ""
                },
            ),
        ),
        "imprint" => (
            "Impresum".to_string(),
            "Obaveštenje koje je onaj ko vodi ovaj sajt dužan da objavi prema propisima \
             države {jurisdiction}.\n\n\
             {last_updated}\n\n\
             ## Odgovorni za ovaj sajt\n\n\
             {legal_name}\n\
             {postal_address}\
             {identifiers}\
             {domain}\n\n\
             Kontakt: {email}\n\n\
             ## Rešavanje sporova\n\n\
             Evropska komisija vodi platformu za onlajn rešavanje sporova na adresi \
             https://ec.europa.eu/consumers/odr. Nismo obavezni da je koristimo, a \
             spremni smo na vansudsko rešavanje sporova."
                .to_string(),
        ),
        "accessibility" => (
            "Izjava o pristupačnosti".to_string(),
            "{legal_name} želi da {domain} mogu da koriste svi, uključujući one koji \
             koriste čitač ekrana, samo tastaturu, uvećanje ili smanjeno kretanje na \
             ekranu.\n\n\
             {last_updated}\n\n\
             ## Standard\n\n\
             Ciljamo WCAG 2.2 nivo AA, standard na koji upućuje EN 301 549 i propisi o \
             pristupačnosti u javnom sektoru širom Evrope.\n\n\
             ## Status\n\n\
             Delimično usklađeno. To znači da najveći deo sajta ispunjava standard, a \
             da deo nije proverio niko osim nas. Nezavisnu proveru nismo naručili. Da \
             smo bez nje napisali „potpuno usklađeno“, to bi bila tvrdnja koju ne \
             možemo da potkrepimo.\n\n\
             Šta smo namerno uradili: kontrast teksta se meri a ne procenjuje, i \
             izgradnja sajta pada kada neki par boja ne prolazi; svaka animacija poštuje \
             podešavanje prefers-reduced-motion; stranica radi na uvećanju od 400 odsto \
             i samo sa tastature; i postoji veza za preskakanje na glavni sadržaj.\n\n\
             ## Javite nam gde ne radi\n\n\
             Ako vas nešto blokira, pišite na {email} i navedite stranicu i šta se \
             dogodilo. Odgovaramo u roku od pet radnih dana, i draže nam je da čujemo \
             za mali problem nego da ga zaobilazite.\n\n\
             ## Ako ne ispravimo\n\n\
             Možete se obratiti: {authority}, ili organu nadležnom za nadzor nad \
             pristupačnošću u državi {jurisdiction}."
                .to_string(),
        ),
        _ => (String::new(), String::new()),
    }
}

/// Generate `src/generated/legal.ts` from the declarations.
///
/// `locales` is the full list from `[i18n]`; `default_locale` is the fallback.
/// The generated file exports a `LegalPage` union type and, for each locale,
/// a `Record<LegalPage, { title: string; body: string }>` with placeholder
/// values substituted from `legal` and `brand`.
pub fn render_legal_ts(
    legal: &Legal,
    brand: &Brand,
    locales: &[String],
    default_locale: &str,
) -> Result<String> {
    let pages = pages_for_jurisdiction(&legal.jurisdiction);
    let categories = legal.effective_categories();

    let page_union: Vec<String> = pages.iter().map(|p| format!("\"{p}\"")).collect();

    let mut out = String::new();

    // ── Header / GDPR checklist ────────────────────────────────────────────
    out.push_str("// generated by `fid derive` — do not edit\n");
    out.push_str("// source of truth: [legal], [brand], and [i18n] in fiducial.toml\n");
    out.push_str("//\n");
    out.push_str("// ┌─────────────────────────────────────────────────────────────────────┐\n");
    out.push_str("// │  GDPR COMPLIANCE CHECKLIST — decisions a human must make            │\n");
    out.push_str("// │  GDPR compliance is a legal state, not a code state.                │\n");
    out.push_str("// │  No tool grants it. A qualified legal professional must review       │\n");
    out.push_str("// │  these pages before they are published.                              │\n");
    out.push_str("// │                                                                      │\n");
    out.push_str("// │  [ ] Replace every placeholder in [legal] and [brand] with real     │\n");
    out.push_str("// │      values (data_protection_email, legal_name, domain, etc.).       │\n");
    out.push_str("// │  [ ] Have a qualified legal professional review all generated pages. │\n");
    out.push_str("// │  [ ] Confirm the jurisdiction clause matches where your entity is    │\n");
    out.push_str("// │      registered — not just where your users are.                    │\n");
    out.push_str("// │  [ ] Verify cookie_categories lists every tracking purpose in use;  │\n");
    out.push_str("// │      omitting one is a legal, not a technical, error.               │\n");
    out.push_str("// │  [ ] Record the date and reviewer for each page in your legal log.  │\n");
    out.push_str("// │  [ ] Re-review whenever jurisdiction, entity, or cookie categories  │\n");
    out.push_str("// │      change — a new `fid derive` regenerates the template, but      │\n");
    out.push_str("// │      the human review must follow.                                  │\n");
    if legal.jurisdiction == "EU" || legal.jurisdiction == "UK" {
        out.push_str("// │  [ ] Appoint a Data Protection Officer if required (Art. 37 GDPR). │\n");
        out.push_str(
            "// │  [ ] Register with the supervisory authority if required.           │\n",
        );
        out.push_str(
            "// │  [ ] Complete a DPIA for high-risk processing activities.           │\n",
        );
    }
    if legal.jurisdiction == "US" {
        out.push_str("// │  [ ] Check state-specific requirements (CCPA, CPRA, VCDPA, etc.).  │\n");
        out.push_str(
            "// │  [ ] Add a \"Do Not Sell or Share My Personal Information\" link     │\n",
        );
        out.push_str("// │      if subject to CCPA/CPRA.                                      │\n");
    }
    out.push_str("// └─────────────────────────────────────────────────────────────────────┘\n");
    out.push('\n');

    // ── LegalPage type ─────────────────────────────────────────────────────
    out.push_str(&format!(
        "export type LegalPage = {};\n\n",
        page_union.join(" | ")
    ));

    // ── Per-locale content ─────────────────────────────────────────────────
    out.push_str("export interface LegalPageContent {\n");
    out.push_str("  title: string;\n");
    out.push_str("  body: string;\n");
    out.push_str("}\n\n");

    out.push_str("export type LegalCatalog = Record<LegalPage, LegalPageContent>;\n\n");

    // One constant per locale, rendered in that locale's language. A locale whose
    // language has no templates still gets an entry — omitting one would break the
    // Record<Locale, …> contract downstream — but it is emitted in the default
    // locale's language under a loud marker, never silently as English. Principle
    // 1c: a missing translation is a missing artifact, not a fallback.
    for locale in locales {
        let const_name = locale.replace('-', "_");
        let lang = lang_of(locale);
        let translated = is_translated(locale);
        if !translated {
            out.push_str(&format!(
                "// UNTRANSLATED — no legal templates exist for \"{lang}\".\n\
                 // The text below is {fallback_lang}. It MUST NOT be published to a reader of\n\
                 // \"{lang}\". Supply a translation before routing this locale's pages.\n",
                fallback_lang = lang_of(default_locale),
            ));
        }
        let render_lang = if translated {
            lang
        } else {
            lang_of(default_locale)
        };
        out.push_str(&format!("export const {const_name}: LegalCatalog = {{\n"));
        for page in pages {
            let (title_template, body_template) = page_content(page, render_lang, &categories);
            let facts = Facts {
                legal_name: &brand.legal_name,
                domain: &brand.domain,
                jurisdiction: jurisdiction_name(&legal.jurisdiction, render_lang),
                email: &legal.data_protection_email,
                authority: supervisory_authority(&legal.jurisdiction, render_lang),
                statute: statute_name(&legal.jurisdiction, render_lang),
                last_updated: last_updated_line(legal.last_updated.as_deref(), render_lang),
                identifiers: identifier_lines(legal, render_lang),
                postal_address: address_line(legal.postal_address.as_deref()),
            };
            let title = substitute(&title_template, &facts);
            let body = substitute(&body_template, &facts);
            // Escape backticks and `${` for template literals.
            let body_escaped = body.replace('`', "\\`").replace("${", "\\${");
            out.push_str(&format!("  {page}: {{\n"));
            out.push_str(&format!("    title: \"{title}\",\n"));
            out.push_str(&format!("    body: `{body_escaped}`,\n"));
            out.push_str("  },\n");
        }
        out.push_str("};\n\n");
    }

    // ── Default export keyed by locale ─────────────────────────────────────
    out.push_str("export const legalCatalogs: Record<string, LegalCatalog> = {\n");
    for locale in locales {
        let const_name = locale.replace('-', "_");
        out.push_str(&format!("  \"{locale}\": {const_name},\n"));
    }
    out.push_str("};\n\n");

    out.push_str(&format!(
        "export const defaultLocale = \"{default_locale}\";\n"
    ));

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_brand() -> Brand {
        Brand {
            legal_name: "Example LLC".into(),
            trading_name: "Example".into(),
            domain: "example.com".into(),
            contact_email: "hello@example.com".into(),
            primary_color: String::new(),
            background_color: String::new(),
            short_name: None,
            favicon: None,
        }
    }

    fn test_legal_eu() -> Legal {
        Legal {
            jurisdiction: "EU".into(),
            data_protection_email: "privacy@example.com".into(),
            cookie_categories: vec!["necessary".into(), "analytics".into()],
            ..Legal::default()
        }
    }

    #[test]
    fn eu_pages_include_imprint() {
        assert!(pages_for_jurisdiction("EU").contains(&"imprint"));
        assert!(pages_for_jurisdiction("UK").contains(&"imprint"));
    }

    #[test]
    fn us_pages_omit_imprint() {
        assert!(!pages_for_jurisdiction("US").contains(&"imprint"));
    }

    #[test]
    fn other_jurisdiction_has_minimal_set() {
        let pages = pages_for_jurisdiction("other");
        assert!(pages.contains(&"privacy"));
        assert!(pages.contains(&"terms"));
        assert!(pages.contains(&"cookies"));
        assert!(!pages.contains(&"imprint"));
        assert!(!pages.contains(&"accessibility"));
    }

    #[test]
    fn necessary_is_always_in_effective_categories() {
        let legal = Legal {
            jurisdiction: "EU".into(),
            data_protection_email: "p@example.com".into(),
            cookie_categories: vec!["analytics".into()],
            ..Legal::default()
        };
        assert!(legal.effective_categories().contains(&"necessary"));
    }

    #[test]
    fn render_produces_valid_exports() {
        let legal = test_legal_eu();
        let brand = test_brand();
        let locales = vec!["en".into(), "de".into()];
        let ts = render_legal_ts(&legal, &brand, &locales, "en").unwrap();

        assert!(ts.contains("export type LegalPage ="), "{ts}");
        assert!(ts.contains("\"privacy\""), "{ts}");
        assert!(ts.contains("\"imprint\""), "{ts}");
        assert!(ts.contains("export const en: LegalCatalog"), "{ts}");
        assert!(ts.contains("export const de: LegalCatalog"), "{ts}");
        assert!(ts.contains("Example LLC"), "{ts}");
        assert!(ts.contains("example.com"), "{ts}");
        assert!(ts.contains("privacy@example.com"), "{ts}");
        assert!(ts.contains("GDPR COMPLIANCE CHECKLIST"), "{ts}");
    }

    #[test]
    fn us_render_does_not_include_imprint() {
        let legal = Legal {
            jurisdiction: "US".into(),
            data_protection_email: "p@example.com".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["en".into()], "en").unwrap();
        assert!(!ts.contains("\"imprint\""), "{ts}");
        assert!(ts.contains("CCPA"), "{ts}");
    }

    #[test]
    fn substitute_replaces_all_placeholders() {
        let facts = Facts {
            legal_name: "Acme LLC",
            domain: "acme.dev",
            jurisdiction: "EU",
            email: "dpo@acme.dev",
            authority: "the ICO",
            statute: "the GDPR",
            last_updated: "reviewed".to_string(),
            identifiers: "reg\n".to_string(),
            postal_address: "addr\n".to_string(),
        };
        let result = substitute(
            "Hello {legal_name} at {domain} ({jurisdiction}) — {email} / {authority} / \
             {statute} / {last_updated} / {identifiers}{postal_address}",
            &facts,
        );
        assert_eq!(
            result,
            "Hello Acme LLC at acme.dev (EU) — dpo@acme.dev / the ICO / the GDPR / \
             reviewed / reg\naddr\n"
        );
    }

    // ── Serbian as a first-class locale ────────────────────────────────────

    #[test]
    fn serbian_renders_in_serbian_not_english() {
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into(), "en".into()], "sr").unwrap();

        // The Serbian catalog carries Serbian titles, the English one English.
        assert!(ts.contains("Politika privatnosti"), "{ts}");
        assert!(ts.contains("Uslovi korišćenja"), "{ts}");
        assert!(ts.contains("Privacy Policy"), "{ts}");

        // The regression this guards: sr and en must not be the same bytes.
        let sr = ts.split("export const sr").nth(1).unwrap();
        let sr_body = sr.split("export const").next().unwrap();
        assert!(
            !sr_body.contains("This Privacy Policy describes"),
            "sr catalog must not contain the English template: {sr_body}"
        );
    }

    #[test]
    fn jurisdiction_renders_as_a_name_never_the_enum_token() {
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into(), "en".into()], "sr").unwrap();
        assert!(ts.contains("Srbija"), "{ts}");
        assert!(ts.contains("Serbia"), "{ts}");
        // "Jurisdiction: RS." is not a sentence a reader should ever see.
        assert!(!ts.contains("**Jurisdiction:** RS"), "{ts}");
        assert!(!ts.contains("**Jurisdikcija:** RS"), "{ts}");
    }

    #[test]
    fn undeclared_jurisdiction_is_visibly_unfit_to_publish() {
        let legal = Legal {
            jurisdiction: "other".into(),
            data_protection_email: "p@example.com".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["en".into()], "en").unwrap();
        // Never the bare token: "governed by the laws of other" is worse than a gap.
        assert!(!ts.contains("laws of other"), "{ts}");
        assert!(ts.contains("[jurisdiction not declared]"), "{ts}");
    }

    #[test]
    fn rs_includes_accessibility_but_not_imprint() {
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into()], "sr").unwrap();
        assert!(ts.contains("\"accessibility\""), "{ts}");
        assert!(!ts.contains("\"imprint\""), "Serbia has no Impressum: {ts}");
        assert!(ts.contains("Izjava o pristupačnosti"), "{ts}");
    }

    #[test]
    fn an_untranslated_locale_is_marked_never_silently_english() {
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        // German has no templates.
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into(), "de".into()], "sr").unwrap();
        assert!(ts.contains("export const de"), "de must still exist: {ts}");
        assert!(ts.contains("UNTRANSLATED"), "de must be marked: {ts}");
        assert!(ts.contains("MUST NOT be published"), "{ts}");
    }

    #[test]
    fn region_tagged_locales_resolve_to_their_language() {
        assert_eq!(lang_of("sr-Latn-RS"), "sr");
        assert_eq!(lang_of("en_US"), "en");
        assert_eq!(lang_of("sr"), "sr");
        assert!(is_translated("sr-Latn-RS"));
        assert!(!is_translated("de-AT"));
    }

    #[test]
    fn imprint_and_accessibility_have_no_stray_braces() {
        // Regression: these two templates used `{{name}}` inside a plain
        // .to_string(), where the doubled braces are literal, not escapes —
        // so substitution left `{Acme LLC}` in the rendered page.
        let legal = Legal {
            jurisdiction: "EU".into(),
            data_protection_email: "p@example.com".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["en".into()], "en").unwrap();
        assert!(
            !ts.contains("{Acme"),
            "stray braces around substitution: {ts}"
        );
        assert!(!ts.contains("{{"), "unsubstituted doubled braces: {ts}");
    }

    // ── Disclosures that have to name something ────────────────────────────

    #[test]
    fn the_supervisory_authority_is_named_not_gestured_at() {
        // GDPR Art. 13(2)(d) is discharged by telling the reader who to
        // complain to. "The relevant national enforcement body for your
        // jurisdiction", which this used to say, tells them to go and find out.
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into(), "en".into()], "sr").unwrap();
        assert!(
            ts.contains("Poverenik za informacije od javnog značaja"),
            "{ts}"
        );
        assert!(ts.contains("office@poverenik.rs"), "{ts}");
        assert!(
            !ts.contains("relevant national enforcement body"),
            "the old non-disclosure survived: {ts}"
        );
    }

    #[test]
    fn serbian_pages_cite_the_serbian_statute() {
        // The ZZPL transposes the GDPR, which is why one template set serves
        // both. A Serbian reader is still owed the citation that binds the
        // controller, not only the regulation it was copied from.
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into()], "sr").unwrap();
        assert!(ts.contains("Službeni glasnik RS"), "{ts}");
        assert!(ts.contains("87/2018"), "{ts}");
    }

    #[test]
    fn an_unreviewed_page_says_so_to_the_reader() {
        // Not in a comment a developer reads. In the page, where whoever is
        // about to publish it has to look at it.
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into(), "en".into()], "sr").unwrap();
        assert!(
            ts.contains("has not been reviewed by a qualified professional"),
            "{ts}"
        );
        assert!(ts.contains("nije proverilo stručno lice"), "{ts}");

        let reviewed = Legal {
            last_updated: Some("2026-09-18".into()),
            ..legal
        };
        let ts = render_legal_ts(&reviewed, &test_brand(), &["en".into()], "en").unwrap();
        assert!(ts.contains("Last reviewed: 2026-09-18"), "{ts}");
        assert!(!ts.contains("has not been reviewed"), "{ts}");
    }

    #[test]
    fn absent_identifiers_render_as_absence_never_as_a_placeholder() {
        // An unincorporated project has no registration number. A line reading
        // "Registration number: [TBD]" is worse than no line: it looks like a
        // fact that has not been filled in, which is how a placeholder ships.
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into()], "sr").unwrap();
        assert!(!ts.contains("Matični broj"), "{ts}");
        assert!(!ts.contains("PIB"), "{ts}");

        let incorporated = Legal {
            registration_number: Some("21234567".into()),
            tax_number: Some("112233445".into()),
            ..legal
        };
        let ts = render_legal_ts(&incorporated, &test_brand(), &["sr".into()], "sr").unwrap();
        assert!(ts.contains("Matični broj: 21234567"), "{ts}");
        assert!(ts.contains("PIB: 112233445"), "{ts}");
    }

    #[test]
    fn the_cookie_page_promises_a_withdrawal_route() {
        // The banner component's footer link is the other half of this. If the
        // page says consent can be withdrawn and nothing in the product lets
        // anyone withdraw it, the page is the violation.
        let legal = Legal {
            jurisdiction: "RS".into(),
            data_protection_email: "p@example.rs".into(),
            cookie_categories: vec!["necessary".into(), "analytics".into()],
            ..Legal::default()
        };
        let ts = render_legal_ts(&legal, &test_brand(), &["sr".into(), "en".into()], "sr").unwrap();
        assert!(ts.contains("withdraw that consent at any"), "{ts}");
        assert!(ts.contains("opozvati u svakom"), "{ts}");
    }
}
